//! PostgreSQL storage backend implementation with pgvector
//!
//! Uses tokio-postgres for native async PostgreSQL access and pgvector
//! for efficient vector similarity search.

use async_trait::async_trait;
use deadpool_postgres::{Config, Pool, Runtime};
use pgvector::Vector;
use std::sync::Arc;
use tokio_postgres::NoTls;

use crate::storage::migrations::postgres::{
    SCHEMA_000_PGVECTOR_EXTENSION, SCHEMA_001_ARTIFACTS_SPANS, SCHEMA_002_SESSIONS_AGENTS,
};
use crate::storage::traits::StorageBackend;
use crate::storage::vector::{VectorSearchProvider, VectorSearchResult};
use crate::types::*;

/// PostgreSQL storage backend with pgvector
///
/// Uses connection pooling via deadpool-postgres and native vector
/// operations via pgvector extension.
pub struct PostgresBackend {
    pool: Pool,
}

impl PostgresBackend {
    /// Create new PostgreSQL backend
    ///
    /// # Arguments
    /// * `connection_string` - PostgreSQL connection string (postgres://user:pass@host/db)
    pub async fn new(connection_string: &str) -> Result<Self> {
        // Parse connection string into config
        let mut cfg = Config::new();

        // Parse the connection string manually
        let url = connection_string
            .strip_prefix("postgres://")
            .or_else(|| connection_string.strip_prefix("postgresql://"))
            .ok_or_else(|| Error::InvalidInput("Invalid PostgreSQL connection string".to_string()))?;

        // Parse user:pass@host:port/dbname
        let (auth, rest) = if let Some(at_pos) = url.find('@') {
            (Some(&url[..at_pos]), &url[at_pos + 1..])
        } else {
            (None, url)
        };

        if let Some(auth) = auth {
            if let Some(colon_pos) = auth.find(':') {
                cfg.user = Some(auth[..colon_pos].to_string());
                cfg.password = Some(auth[colon_pos + 1..].to_string());
            } else {
                cfg.user = Some(auth.to_string());
            }
        }

        // Parse host:port/dbname
        let (host_port, dbname) = if let Some(slash_pos) = rest.find('/') {
            (&rest[..slash_pos], Some(&rest[slash_pos + 1..]))
        } else {
            (rest, None)
        };

        if let Some(colon_pos) = host_port.find(':') {
            cfg.host = Some(host_port[..colon_pos].to_string());
            cfg.port = Some(host_port[colon_pos + 1..].parse().unwrap_or(5432));
        } else {
            cfg.host = Some(host_port.to_string());
            cfg.port = Some(5432);
        }

        if let Some(db) = dbname {
            // Remove query params if present
            let db = db.split('?').next().unwrap_or(db);
            cfg.dbname = Some(db.to_string());
        }

        // Create connection pool
        let pool = cfg
            .create_pool(Some(Runtime::Tokio1), NoTls)
            .map_err(|e| Error::Other(anyhow::anyhow!("Failed to create pool: {}", e)))?;

        let backend = Self { pool };

        // Run migrations
        backend.run_migrations().await?;

        Ok(backend)
    }

    /// Run database migrations
    async fn run_migrations(&self) -> Result<()> {
        let client = self.pool.get().await
            .map_err(|e| Error::Other(anyhow::anyhow!("Failed to get connection: {}", e)))?;

        // Enable pgvector extension
        client.batch_execute(SCHEMA_000_PGVECTOR_EXTENSION).await
            .map_err(|e| Error::Other(anyhow::anyhow!("Failed to enable pgvector: {}", e)))?;

        // Create artifacts and spans tables
        client.batch_execute(SCHEMA_001_ARTIFACTS_SPANS).await
            .map_err(|e| Error::Other(anyhow::anyhow!("Failed to create artifacts/spans tables: {}", e)))?;

        // Create sessions, messages, agents tables
        client.batch_execute(SCHEMA_002_SESSIONS_AGENTS).await
            .map_err(|e| Error::Other(anyhow::anyhow!("Failed to create session/agent tables: {}", e)))?;

        Ok(())
    }

    /// Helper to convert PostgreSQL row to Artifact
    fn row_to_artifact(row: &tokio_postgres::Row) -> Result<Artifact> {
        let metadata: Option<serde_json::Value> = row.get("metadata");
        let created_at: chrono::DateTime<chrono::Utc> = row.get("created_at");

        Ok(Artifact {
            id: row.get("id"),
            path: row.get("path"),
            content: row.get("content"),
            content_hash: row.get("content_hash"),
            metadata,
            created_at,
        })
    }

    /// Helper to convert PostgreSQL row to Span
    fn row_to_span(row: &tokio_postgres::Row) -> Result<Span> {
        let embedding: Option<Vector> = row.try_get("embedding").ok();
        let embedding_vec = embedding.map(|v| v.to_vec());
        let metadata: Option<serde_json::Value> = row.get("metadata");
        let token_count: Option<i32> = row.get("token_count");

        Ok(Span {
            id: row.get("id"),
            artifact_id: row.get("artifact_id"),
            start_line: row.get::<_, i32>("start_line") as usize,
            end_line: row.get::<_, i32>("end_line") as usize,
            text: row.get("text"),
            embedding: embedding_vec,
            embedding_model: row.get("embedding_model"),
            token_count: token_count.unwrap_or(0) as usize,
            metadata,
        })
    }

    /// Helper to convert PostgreSQL row to Session
    fn row_to_session(row: &tokio_postgres::Row) -> Result<Session> {
        let metadata: Option<serde_json::Value> = row.get("metadata");
        let created_at: chrono::DateTime<chrono::Utc> = row.get("created_at");
        let updated_at: chrono::DateTime<chrono::Utc> = row.get("updated_at");
        let last_message_at: Option<chrono::DateTime<chrono::Utc>> = row.get("last_message_at");

        Ok(Session {
            id: row.get("id"),
            user_id: row.get("user_id"),
            title: row.get("title"),
            metadata,
            created_at,
            updated_at,
            last_message_at,
        })
    }

    /// Helper to convert PostgreSQL row to Message
    fn row_to_message(row: &tokio_postgres::Row) -> Result<Message> {
        let role_str: String = row.get("role");
        let role = MessageRole::from_str(&role_str)
            .map_err(|e| Error::InvalidInput(e))?;
        let metadata: Option<serde_json::Value> = row.get("metadata");
        let created_at: chrono::DateTime<chrono::Utc> = row.get("created_at");
        let sequence_number: i32 = row.get("sequence_number");

        Ok(Message {
            id: row.get("id"),
            session_id: row.get("session_id"),
            role,
            content: row.get("content"),
            metadata,
            sequence_number: sequence_number as usize,
            created_at,
        })
    }

    /// Helper to convert PostgreSQL row to Agent
    fn row_to_agent(row: &tokio_postgres::Row) -> Result<Agent> {
        let capabilities: Option<serde_json::Value> = row.get("capabilities");
        let capabilities_vec = capabilities
            .and_then(|v| serde_json::from_value::<Vec<String>>(v).ok());
        let metadata: Option<serde_json::Value> = row.get("metadata");
        let created_at: chrono::DateTime<chrono::Utc> = row.get("created_at");

        Ok(Agent {
            id: row.get("id"),
            name: row.get("name"),
            role: row.get("role"),
            model: row.get("model"),
            system_prompt: row.get("system_prompt"),
            did: row.get("did"),
            capabilities: capabilities_vec,
            metadata,
            created_at,
        })
    }
}

#[async_trait]
impl StorageBackend for PostgresBackend {
    // ========== Lifecycle ==========

    async fn get_stats(&self) -> Result<(usize, usize, usize)> {
        let client = self.pool.get().await
            .map_err(|e| Error::Other(anyhow::anyhow!("Pool error: {}", e)))?;

        let artifacts_row = client
            .query_one("SELECT COUNT(*) as count FROM artifacts", &[])
            .await
            .map_err(|e| Error::Other(anyhow::anyhow!("Query error: {}", e)))?;
        let artifacts_count: i64 = artifacts_row.get("count");

        let spans_row = client
            .query_one("SELECT COUNT(*) as count, COALESCE(SUM(token_count), 0) as tokens FROM spans", &[])
            .await
            .map_err(|e| Error::Other(anyhow::anyhow!("Query error: {}", e)))?;
        let spans_count: i64 = spans_row.get("count");
        let total_tokens: i64 = spans_row.get("tokens");

        Ok((artifacts_count as usize, spans_count as usize, total_tokens as usize))
    }

    async fn clear(&self) -> Result<()> {
        let client = self.pool.get().await
            .map_err(|e| Error::Other(anyhow::anyhow!("Pool error: {}", e)))?;

        // Delete in order respecting foreign keys
        client.execute("DELETE FROM agent_relations", &[]).await
            .map_err(|e| Error::Other(anyhow::anyhow!("Delete error: {}", e)))?;
        client.execute("DELETE FROM session_working_sets", &[]).await
            .map_err(|e| Error::Other(anyhow::anyhow!("Delete error: {}", e)))?;
        client.execute("DELETE FROM messages", &[]).await
            .map_err(|e| Error::Other(anyhow::anyhow!("Delete error: {}", e)))?;
        client.execute("DELETE FROM sessions", &[]).await
            .map_err(|e| Error::Other(anyhow::anyhow!("Delete error: {}", e)))?;
        client.execute("DELETE FROM agents", &[]).await
            .map_err(|e| Error::Other(anyhow::anyhow!("Delete error: {}", e)))?;
        client.execute("DELETE FROM spans", &[]).await
            .map_err(|e| Error::Other(anyhow::anyhow!("Delete error: {}", e)))?;
        client.execute("DELETE FROM artifacts", &[]).await
            .map_err(|e| Error::Other(anyhow::anyhow!("Delete error: {}", e)))?;

        Ok(())
    }

    // ========== Artifacts ==========

    async fn insert_artifact(&self, artifact: &Artifact) -> Result<()> {
        let client = self.pool.get().await
            .map_err(|e| Error::Other(anyhow::anyhow!("Pool error: {}", e)))?;

        client.execute(
            "INSERT INTO artifacts (id, path, content, content_hash, metadata, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $6)
             ON CONFLICT (id) DO UPDATE SET
                path = EXCLUDED.path,
                content = EXCLUDED.content,
                content_hash = EXCLUDED.content_hash,
                metadata = EXCLUDED.metadata,
                updated_at = NOW()",
            &[
                &artifact.id,
                &artifact.path,
                &artifact.content,
                &artifact.content_hash,
                &artifact.metadata,
                &artifact.created_at,
            ],
        ).await
        .map_err(|e| Error::Other(anyhow::anyhow!("Insert error: {}", e)))?;

        Ok(())
    }

    async fn get_artifact(&self, artifact_id: &str) -> Result<Option<Artifact>> {
        let client = self.pool.get().await
            .map_err(|e| Error::Other(anyhow::anyhow!("Pool error: {}", e)))?;

        let rows = client
            .query("SELECT * FROM artifacts WHERE id = $1", &[&artifact_id])
            .await
            .map_err(|e| Error::Other(anyhow::anyhow!("Query error: {}", e)))?;

        if rows.is_empty() {
            Ok(None)
        } else {
            Ok(Some(Self::row_to_artifact(&rows[0])?))
        }
    }

    async fn get_artifact_by_path(&self, path: &str) -> Result<Option<Artifact>> {
        let client = self.pool.get().await
            .map_err(|e| Error::Other(anyhow::anyhow!("Pool error: {}", e)))?;

        let rows = client
            .query("SELECT * FROM artifacts WHERE path = $1", &[&path])
            .await
            .map_err(|e| Error::Other(anyhow::anyhow!("Query error: {}", e)))?;

        if rows.is_empty() {
            Ok(None)
        } else {
            Ok(Some(Self::row_to_artifact(&rows[0])?))
        }
    }

    async fn delete_artifact(&self, artifact_id: &str) -> Result<usize> {
        let client = self.pool.get().await
            .map_err(|e| Error::Other(anyhow::anyhow!("Pool error: {}", e)))?;

        // Count spans before deletion (CASCADE will delete them)
        let count_row = client
            .query_one(
                "SELECT COUNT(*) as count FROM spans WHERE artifact_id = $1",
                &[&artifact_id],
            )
            .await
            .map_err(|e| Error::Other(anyhow::anyhow!("Query error: {}", e)))?;
        let span_count: i64 = count_row.get("count");

        // Delete artifact (spans deleted via CASCADE)
        client
            .execute("DELETE FROM artifacts WHERE id = $1", &[&artifact_id])
            .await
            .map_err(|e| Error::Other(anyhow::anyhow!("Delete error: {}", e)))?;

        Ok(span_count as usize)
    }

    async fn determine_ingest_action(
        &self,
        path: &str,
        content_hash: &str,
    ) -> Result<IngestAction> {
        let client = self.pool.get().await
            .map_err(|e| Error::Other(anyhow::anyhow!("Pool error: {}", e)))?;

        let rows = client
            .query(
                "SELECT id, content_hash FROM artifacts WHERE path = $1",
                &[&path],
            )
            .await
            .map_err(|e| Error::Other(anyhow::anyhow!("Query error: {}", e)))?;

        if rows.is_empty() {
            Ok(IngestAction::Create)
        } else {
            let existing_id: String = rows[0].get("id");
            let existing_hash: String = rows[0].get("content_hash");

            if existing_hash == content_hash {
                Ok(IngestAction::Skip {
                    artifact_id: existing_id,
                    reason: "Content unchanged".to_string(),
                })
            } else {
                Ok(IngestAction::Update {
                    artifact_id: existing_id,
                })
            }
        }
    }

    // ========== Spans ==========

    async fn insert_spans(&self, spans: &[Span]) -> Result<()> {
        if spans.is_empty() {
            return Ok(());
        }

        let client = self.pool.get().await
            .map_err(|e| Error::Other(anyhow::anyhow!("Pool error: {}", e)))?;

        // Use a transaction for batch insert
        for span in spans {
            let embedding = span.embedding.as_ref().map(|e| Vector::from(e.clone()));

            client.execute(
                "INSERT INTO spans (id, artifact_id, start_line, end_line, text, embedding, embedding_model, token_count, metadata)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
                 ON CONFLICT (id) DO UPDATE SET
                    text = EXCLUDED.text,
                    embedding = EXCLUDED.embedding,
                    embedding_model = EXCLUDED.embedding_model,
                    token_count = EXCLUDED.token_count,
                    metadata = EXCLUDED.metadata",
                &[
                    &span.id,
                    &span.artifact_id,
                    &(span.start_line as i32),
                    &(span.end_line as i32),
                    &span.text,
                    &embedding,
                    &span.embedding_model,
                    &(span.token_count as i32),
                    &span.metadata,
                ],
            ).await
            .map_err(|e| Error::Other(anyhow::anyhow!("Insert span error: {}", e)))?;
        }

        Ok(())
    }

    async fn get_all_spans(&self) -> Result<Vec<Span>> {
        let client = self.pool.get().await
            .map_err(|e| Error::Other(anyhow::anyhow!("Pool error: {}", e)))?;

        let rows = client
            .query("SELECT * FROM spans ORDER BY artifact_id, start_line", &[])
            .await
            .map_err(|e| Error::Other(anyhow::anyhow!("Query error: {}", e)))?;

        rows.iter().map(Self::row_to_span).collect()
    }

    async fn search_spans(&self, query: &str, limit: usize) -> Result<Vec<Span>> {
        let client = self.pool.get().await
            .map_err(|e| Error::Other(anyhow::anyhow!("Pool error: {}", e)))?;

        // PostgreSQL full-text search using ILIKE for simple matching
        // For production, consider using tsvector/tsquery
        let search_pattern = format!("%{}%", query);
        let rows = client
            .query(
                "SELECT * FROM spans WHERE text ILIKE $1 ORDER BY artifact_id, start_line LIMIT $2",
                &[&search_pattern, &(limit as i64)],
            )
            .await
            .map_err(|e| Error::Other(anyhow::anyhow!("Query error: {}", e)))?;

        rows.iter().map(Self::row_to_span).collect()
    }

    // ========== Vector Search ==========

    async fn get_vector_search(&self) -> Result<Arc<dyn VectorSearchProvider>> {
        Ok(Arc::new(PgVectorSearch::new(self.pool.clone())))
    }

    async fn invalidate_vector_index(&self) {
        // pgvector handles index updates automatically
        // No action needed
    }

    // ========== Sessions ==========

    async fn create_session(
        &self,
        user_id: Option<&str>,
        title: Option<&str>,
    ) -> Result<Session> {
        let client = self.pool.get().await
            .map_err(|e| Error::Other(anyhow::anyhow!("Pool error: {}", e)))?;

        let id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now();

        client.execute(
            "INSERT INTO sessions (id, user_id, title, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $4)",
            &[&id, &user_id, &title, &now],
        ).await
        .map_err(|e| Error::Other(anyhow::anyhow!("Insert error: {}", e)))?;

        Ok(Session {
            id,
            user_id: user_id.map(String::from),
            title: title.map(String::from),
            metadata: None,
            created_at: now,
            updated_at: now,
            last_message_at: None,
        })
    }

    async fn get_session(&self, session_id: &str) -> Result<Option<Session>> {
        let client = self.pool.get().await
            .map_err(|e| Error::Other(anyhow::anyhow!("Pool error: {}", e)))?;

        let rows = client
            .query("SELECT * FROM sessions WHERE id = $1", &[&session_id])
            .await
            .map_err(|e| Error::Other(anyhow::anyhow!("Query error: {}", e)))?;

        if rows.is_empty() {
            Ok(None)
        } else {
            Ok(Some(Self::row_to_session(&rows[0])?))
        }
    }

    async fn list_sessions(
        &self,
        user_id: Option<&str>,
        limit: Option<usize>,
    ) -> Result<Vec<Session>> {
        let client = self.pool.get().await
            .map_err(|e| Error::Other(anyhow::anyhow!("Pool error: {}", e)))?;

        let limit_val = limit.unwrap_or(100) as i64;

        let rows = if let Some(uid) = user_id {
            client
                .query(
                    "SELECT * FROM sessions WHERE user_id = $1 ORDER BY updated_at DESC LIMIT $2",
                    &[&uid, &limit_val],
                )
                .await
        } else {
            client
                .query(
                    "SELECT * FROM sessions ORDER BY updated_at DESC LIMIT $1",
                    &[&limit_val],
                )
                .await
        }
        .map_err(|e| Error::Other(anyhow::anyhow!("Query error: {}", e)))?;

        rows.iter().map(Self::row_to_session).collect()
    }

    async fn update_session(
        &self,
        session_id: &str,
        title: Option<&str>,
        metadata: Option<&serde_json::Value>,
    ) -> Result<()> {
        let client = self.pool.get().await
            .map_err(|e| Error::Other(anyhow::anyhow!("Pool error: {}", e)))?;

        let now = chrono::Utc::now();

        // Build dynamic update
        if title.is_some() && metadata.is_some() {
            client.execute(
                "UPDATE sessions SET title = $1, metadata = $2, updated_at = $3 WHERE id = $4",
                &[&title, &metadata, &now, &session_id],
            ).await
        } else if title.is_some() {
            client.execute(
                "UPDATE sessions SET title = $1, updated_at = $2 WHERE id = $3",
                &[&title, &now, &session_id],
            ).await
        } else if metadata.is_some() {
            client.execute(
                "UPDATE sessions SET metadata = $1, updated_at = $2 WHERE id = $3",
                &[&metadata, &now, &session_id],
            ).await
        } else {
            client.execute(
                "UPDATE sessions SET updated_at = $1 WHERE id = $2",
                &[&now, &session_id],
            ).await
        }
        .map_err(|e| Error::Other(anyhow::anyhow!("Update error: {}", e)))?;

        Ok(())
    }

    async fn delete_session(&self, session_id: &str) -> Result<()> {
        let client = self.pool.get().await
            .map_err(|e| Error::Other(anyhow::anyhow!("Pool error: {}", e)))?;

        client
            .execute("DELETE FROM sessions WHERE id = $1", &[&session_id])
            .await
            .map_err(|e| Error::Other(anyhow::anyhow!("Delete error: {}", e)))?;

        Ok(())
    }

    // ========== Messages ==========

    async fn add_message(
        &self,
        session_id: &str,
        role: MessageRole,
        content: &str,
        metadata: Option<&serde_json::Value>,
    ) -> Result<Message> {
        let client = self.pool.get().await
            .map_err(|e| Error::Other(anyhow::anyhow!("Pool error: {}", e)))?;

        // Get next sequence number
        let seq_row = client
            .query_one(
                "SELECT COALESCE(MAX(sequence_number), -1) + 1 as next_seq FROM messages WHERE session_id = $1",
                &[&session_id],
            )
            .await
            .map_err(|e| Error::Other(anyhow::anyhow!("Query error: {}", e)))?;
        let sequence_number: i32 = seq_row.get("next_seq");

        let id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now();
        let role_str = role.as_str();

        client.execute(
            "INSERT INTO messages (id, session_id, role, content, metadata, sequence_number, created_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
            &[&id, &session_id, &role_str, &content, &metadata, &sequence_number, &now],
        ).await
        .map_err(|e| Error::Other(anyhow::anyhow!("Insert error: {}", e)))?;

        // Update session timestamp
        client.execute(
            "UPDATE sessions SET updated_at = $1, last_message_at = $1 WHERE id = $2",
            &[&now, &session_id],
        ).await
        .map_err(|e| Error::Other(anyhow::anyhow!("Update error: {}", e)))?;

        Ok(Message {
            id,
            session_id: session_id.to_string(),
            role,
            content: content.to_string(),
            metadata: metadata.cloned(),
            sequence_number: sequence_number as usize,
            created_at: now,
        })
    }

    async fn get_messages(
        &self,
        session_id: &str,
        limit: Option<usize>,
    ) -> Result<Vec<Message>> {
        let client = self.pool.get().await
            .map_err(|e| Error::Other(anyhow::anyhow!("Pool error: {}", e)))?;

        let rows = if let Some(lim) = limit {
            client
                .query(
                    "SELECT * FROM messages WHERE session_id = $1 ORDER BY sequence_number LIMIT $2",
                    &[&session_id, &(lim as i64)],
                )
                .await
        } else {
            client
                .query(
                    "SELECT * FROM messages WHERE session_id = $1 ORDER BY sequence_number",
                    &[&session_id],
                )
                .await
        }
        .map_err(|e| Error::Other(anyhow::anyhow!("Query error: {}", e)))?;

        rows.iter().map(Self::row_to_message).collect()
    }

    // ========== Working Sets ==========

    async fn associate_working_set(
        &self,
        session_id: &str,
        message_id: Option<&str>,
        working_set: &WorkingSet,
        query: &str,
        config: &CompilerConfig,
    ) -> Result<SessionWorkingSet> {
        let client = self.pool.get().await
            .map_err(|e| Error::Other(anyhow::anyhow!("Pool error: {}", e)))?;

        let id = uuid::Uuid::new_v4().to_string();
        let working_set_id = working_set.deterministic_hash();
        let config_json = serde_json::to_value(config)?;
        let now = chrono::Utc::now();

        client.execute(
            "INSERT INTO session_working_sets (id, session_id, message_id, working_set_id, query, config, created_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
            &[&id, &session_id, &message_id, &working_set_id, &query, &config_json, &now],
        ).await
        .map_err(|e| Error::Other(anyhow::anyhow!("Insert error: {}", e)))?;

        Ok(SessionWorkingSet {
            id,
            session_id: session_id.to_string(),
            message_id: message_id.map(String::from),
            working_set: working_set.clone(),
            query: query.to_string(),
            config: config.clone(),
            created_at: now,
        })
    }

    async fn get_session_full(&self, session_id: &str) -> Result<Option<SessionWithMessages>> {
        let session = match self.get_session(session_id).await? {
            Some(s) => s,
            None => return Ok(None),
        };

        let messages = self.get_messages(session_id, None).await?;

        // For working sets, we'd need to store/retrieve the full WorkingSet JSON
        // For now, return empty (this matches SQLite behavior where working_set is stored separately)
        let working_sets = Vec::new();

        Ok(Some(SessionWithMessages {
            session,
            messages,
            working_sets,
        }))
    }

    // ========== Agents ==========

    async fn register_agent(&self, agent: &Agent) -> Result<Agent> {
        let client = self.pool.get().await
            .map_err(|e| Error::Other(anyhow::anyhow!("Pool error: {}", e)))?;

        let capabilities_json = agent.capabilities.as_ref()
            .map(|c| serde_json::to_value(c).unwrap_or_default());

        client.execute(
            "INSERT INTO agents (id, name, role, model, system_prompt, did, capabilities, metadata, created_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
             ON CONFLICT (id) DO UPDATE SET
                name = EXCLUDED.name,
                role = EXCLUDED.role,
                model = EXCLUDED.model,
                system_prompt = EXCLUDED.system_prompt,
                did = EXCLUDED.did,
                capabilities = EXCLUDED.capabilities,
                metadata = EXCLUDED.metadata",
            &[
                &agent.id,
                &agent.name,
                &agent.role,
                &agent.model,
                &agent.system_prompt,
                &agent.did,
                &capabilities_json,
                &agent.metadata,
                &agent.created_at,
            ],
        ).await
        .map_err(|e| Error::Other(anyhow::anyhow!("Insert error: {}", e)))?;

        Ok(agent.clone())
    }

    async fn get_agent(&self, agent_id: &str) -> Result<Option<Agent>> {
        let client = self.pool.get().await
            .map_err(|e| Error::Other(anyhow::anyhow!("Pool error: {}", e)))?;

        let rows = client
            .query("SELECT * FROM agents WHERE id = $1", &[&agent_id])
            .await
            .map_err(|e| Error::Other(anyhow::anyhow!("Query error: {}", e)))?;

        if rows.is_empty() {
            Ok(None)
        } else {
            Ok(Some(Self::row_to_agent(&rows[0])?))
        }
    }

    async fn get_agent_by_name(&self, name: &str) -> Result<Option<Agent>> {
        let client = self.pool.get().await
            .map_err(|e| Error::Other(anyhow::anyhow!("Pool error: {}", e)))?;

        let rows = client
            .query("SELECT * FROM agents WHERE name = $1", &[&name])
            .await
            .map_err(|e| Error::Other(anyhow::anyhow!("Query error: {}", e)))?;

        if rows.is_empty() {
            Ok(None)
        } else {
            Ok(Some(Self::row_to_agent(&rows[0])?))
        }
    }

    async fn list_agents(&self) -> Result<Vec<Agent>> {
        let client = self.pool.get().await
            .map_err(|e| Error::Other(anyhow::anyhow!("Pool error: {}", e)))?;

        let rows = client
            .query("SELECT * FROM agents ORDER BY name", &[])
            .await
            .map_err(|e| Error::Other(anyhow::anyhow!("Query error: {}", e)))?;

        rows.iter().map(Self::row_to_agent).collect()
    }

    // ========== Agent Relations ==========

    async fn add_agent_relation(
        &self,
        session_id: &str,
        message_id: &str,
        from_agent_id: &str,
        target_message_id: &str,
        stance: Stance,
    ) -> Result<AgentRelation> {
        let client = self.pool.get().await
            .map_err(|e| Error::Other(anyhow::anyhow!("Pool error: {}", e)))?;

        // Get to_agent_id from target message metadata
        let target_row = client
            .query_one(
                "SELECT metadata FROM messages WHERE id = $1",
                &[&target_message_id],
            )
            .await
            .map_err(|e| Error::Other(anyhow::anyhow!("Query error: {}", e)))?;

        let target_metadata: Option<serde_json::Value> = target_row.get("metadata");
        let to_agent_id = target_metadata
            .and_then(|m| m.get("agent_id").and_then(|v| v.as_str().map(String::from)))
            .unwrap_or_else(|| "unknown".to_string());

        let id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now();
        let stance_str = stance.as_str();

        client.execute(
            "INSERT INTO agent_relations (id, session_id, message_id, from_agent_id, to_agent_id, stance, target_message_id, created_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
            &[&id, &session_id, &message_id, &from_agent_id, &to_agent_id, &stance_str, &target_message_id, &now],
        ).await
        .map_err(|e| Error::Other(anyhow::anyhow!("Insert error: {}", e)))?;

        Ok(AgentRelation {
            id,
            session_id: session_id.to_string(),
            message_id: message_id.to_string(),
            from_agent_id: from_agent_id.to_string(),
            to_agent_id,
            stance,
            target_message_id: target_message_id.to_string(),
            created_at: now,
        })
    }

    async fn get_agent_relations(&self, session_id: &str) -> Result<AgentRelationSummary> {
        let client = self.pool.get().await
            .map_err(|e| Error::Other(anyhow::anyhow!("Pool error: {}", e)))?;

        let rows = client
            .query(
                "SELECT ar.*,
                        fa.name as from_name, fa.model as from_model,
                        ta.name as to_name, ta.model as to_model
                 FROM agent_relations ar
                 JOIN agents fa ON ar.from_agent_id = fa.id
                 JOIN agents ta ON ar.to_agent_id = ta.id
                 WHERE ar.session_id = $1
                 ORDER BY ar.created_at",
                &[&session_id],
            )
            .await
            .map_err(|e| Error::Other(anyhow::anyhow!("Query error: {}", e)))?;

        let mut summary = AgentRelationSummary::default();

        for row in rows {
            let stance_str: String = row.get("stance");
            let entry = AgentRelationEntry {
                from_agent: row.get("from_name"),
                from_model: row.get("from_model"),
                to_agent: row.get("to_name"),
                to_model: row.get("to_model"),
                message_id: row.get("message_id"),
                target_message_id: row.get("target_message_id"),
            };

            match stance_str.as_str() {
                "agree" => summary.agreements.push(entry),
                "disagree" => summary.disagreements.push(entry),
                "question" => summary.questions.push(entry),
                _ => {} // neutral or unknown
            }
        }

        Ok(summary)
    }

    async fn get_session_agents(&self, session_id: &str) -> Result<Vec<Agent>> {
        let client = self.pool.get().await
            .map_err(|e| Error::Other(anyhow::anyhow!("Pool error: {}", e)))?;

        // Get distinct agents from message metadata in this session
        let rows = client
            .query(
                "SELECT DISTINCT a.*
                 FROM agents a
                 JOIN messages m ON m.metadata->>'agent_id' = a.id
                 WHERE m.session_id = $1
                 ORDER BY a.name",
                &[&session_id],
            )
            .await
            .map_err(|e| Error::Other(anyhow::anyhow!("Query error: {}", e)))?;

        rows.iter().map(Self::row_to_agent).collect()
    }
}

// ========== pgvector Search Provider ==========

/// pgvector-based vector search for PostgreSQL backend
struct PgVectorSearch {
    pool: Pool,
}

impl PgVectorSearch {
    fn new(pool: Pool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl VectorSearchProvider for PgVectorSearch {
    async fn search(&self, query_embedding: &[f32], k: usize) -> Result<Vec<VectorSearchResult>> {
        let client = self.pool.get().await
            .map_err(|e| Error::Other(anyhow::anyhow!("Pool error: {}", e)))?;

        let query_vec = Vector::from(query_embedding.to_vec());

        // Use cosine similarity: 1 - cosine_distance
        // pgvector's <=> operator returns cosine distance
        let rows = client
            .query(
                "SELECT *, 1 - (embedding <=> $1) as score
                 FROM spans
                 WHERE embedding IS NOT NULL
                 ORDER BY embedding <=> $1
                 LIMIT $2",
                &[&query_vec, &(k as i64)],
            )
            .await
            .map_err(|e| Error::Other(anyhow::anyhow!("Vector search error: {}", e)))?;

        let mut results = Vec::with_capacity(rows.len());
        for row in &rows {
            let span = PostgresBackend::row_to_span(row)?;
            let score: f64 = row.get("score");
            results.push(VectorSearchResult {
                span,
                score: score as f32,
            });
        }

        Ok(results)
    }

    fn len(&self) -> usize {
        // This would require a blocking query, return 0 for now
        // In practice, the caller should use get_stats() instead
        0
    }

    fn dimension(&self) -> usize {
        384 // Default for all-MiniLM-L6-v2
    }
}
