//! SPI-based database backend for AvocadoDB extension
//!
//! Provides database operations using PostgreSQL's Server Programming Interface (SPI).

use crate::error::{AvocadoError, Result};
use pgrx::datum::JsonB;
use pgrx::prelude::*;
use serde::{Deserialize, Serialize};

/// Maximum embedding dimension (matches schema: vector(1024))
/// This supports large models like bge-m3 (1024), nomic (768), mxbai (1024)
/// Smaller embeddings (like fastembed's 384) are zero-padded
pub const MAX_EMBEDDING_DIMENSION: usize = 1024;

/// Normalize embedding to fixed dimension for database storage
/// - Pads with zeros if smaller than max dimension
/// - Truncates if larger (shouldn't happen with supported models)
fn normalize_embedding(embedding: &[f32]) -> Vec<f32> {
    let mut normalized = vec![0.0f32; MAX_EMBEDDING_DIMENSION];
    let copy_len = embedding.len().min(MAX_EMBEDDING_DIMENSION);
    normalized[..copy_len].copy_from_slice(&embedding[..copy_len]);
    normalized
}

/// Artifact metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artifact {
    pub id: String,
    pub path: String,
    pub content: String,
    pub content_hash: String,
    pub metadata: Option<serde_json::Value>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Span with text content and embedding
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Span {
    pub id: String,
    pub artifact_id: String,
    pub start_line: i32,
    pub end_line: i32,
    pub text: String,
    pub embedding: Option<Vec<f32>>,
    pub embedding_model: Option<String>,
    pub token_count: Option<i32>,
}

/// Span with similarity score
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpanWithScore {
    pub span: Span,
    pub artifact_path: String,
    pub score: f64,
}

/// Session record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub user_id: Option<String>,
    pub title: Option<String>,
    pub metadata: Option<serde_json::Value>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Message record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub session_id: String,
    pub role: String,
    pub content: String,
    pub metadata: Option<serde_json::Value>,
    pub sequence_number: i32,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Agent record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    pub id: String,
    pub name: String,
    pub role: String,
    pub model: String,
    pub system_prompt: Option<String>,
    pub capabilities: Option<serde_json::Value>,
    pub metadata: Option<serde_json::Value>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Database statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbStats {
    pub artifact_count: i64,
    pub span_count: i64,
    pub session_count: i64,
    pub message_count: i64,
    pub agent_count: i64,
    pub total_tokens: i64,
}

/// Ensure the database schema exists
pub fn ensure_schema() -> Result<()> {
    Spi::run(include_str!("../../sql/schema.sql"))
        .map_err(|e| AvocadoError::Database(format!("Failed to create schema: {}", e)))?;
    Ok(())
}

/// Get database statistics
pub fn get_stats() -> Result<DbStats> {
    let stats = Spi::connect(|client| {
        let artifact_count: i64 = client
            .select("SELECT COUNT(*) FROM avocado.artifacts", None, None)?
            .first()
            .get_one()?
            .unwrap_or(0);

        let span_count: i64 = client
            .select("SELECT COUNT(*) FROM avocado.spans", None, None)?
            .first()
            .get_one()?
            .unwrap_or(0);

        let session_count: i64 = client
            .select("SELECT COUNT(*) FROM avocado.sessions", None, None)?
            .first()
            .get_one()?
            .unwrap_or(0);

        let message_count: i64 = client
            .select("SELECT COUNT(*) FROM avocado.messages", None, None)?
            .first()
            .get_one()?
            .unwrap_or(0);

        let agent_count: i64 = client
            .select("SELECT COUNT(*) FROM avocado.agents", None, None)?
            .first()
            .get_one()?
            .unwrap_or(0);

        let total_tokens: i64 = client
            .select(
                "SELECT COALESCE(SUM(token_count), 0) FROM avocado.spans",
                None,
                None,
            )?
            .first()
            .get_one()?
            .unwrap_or(0);

        Ok(DbStats {
            artifact_count,
            span_count,
            session_count,
            message_count,
            agent_count,
            total_tokens,
        })
    })
    .map_err(|e| AvocadoError::Database(format!("Failed to get stats: {}", e)))?;

    Ok(stats)
}

/// Insert an artifact
pub fn insert_artifact(artifact: &Artifact) -> Result<()> {
    Spi::connect(|mut client| {
        client.update(
            "INSERT INTO avocado.artifacts (id, path, content, content_hash, metadata, created_at)
             VALUES ($1, $2, $3, $4, $5, $6)
             ON CONFLICT (path) DO UPDATE SET
                content = EXCLUDED.content,
                content_hash = EXCLUDED.content_hash,
                metadata = EXCLUDED.metadata",
            None,
            Some(vec![
                (PgBuiltInOids::TEXTOID.oid(), artifact.id.clone().into_datum()),
                (PgBuiltInOids::TEXTOID.oid(), artifact.path.clone().into_datum()),
                (PgBuiltInOids::TEXTOID.oid(), artifact.content.clone().into_datum()),
                (PgBuiltInOids::TEXTOID.oid(), artifact.content_hash.clone().into_datum()),
                (PgBuiltInOids::JSONBOID.oid(), JsonB(artifact.metadata.clone().unwrap_or(serde_json::json!({}))).into_datum()),
                (PgBuiltInOids::TIMESTAMPTZOID.oid(), artifact.created_at.into_datum()),
            ]),
        )?;
        Ok::<_, spi::Error>(())
    })
    .map_err(|e| AvocadoError::Database(format!("Failed to insert artifact: {}", e)))?;

    Ok(())
}

/// Insert spans with embeddings
pub fn insert_spans(spans: &[Span]) -> Result<()> {
    if spans.is_empty() {
        return Ok(());
    }

    for span in spans {
        Spi::connect(|mut client| {
            // Convert embedding to pgvector format, normalizing to max dimension
            let embedding_str = span.embedding.as_ref().map(|e| {
                let normalized = normalize_embedding(e);
                format!("[{}]", normalized.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(","))
            });

            client.update(
                "INSERT INTO avocado.spans (id, artifact_id, start_line, end_line, text, embedding, embedding_model, token_count)
                 VALUES ($1, $2, $3, $4, $5, $6::vector, $7, $8)",
                None,
                Some(vec![
                    (PgBuiltInOids::TEXTOID.oid(), span.id.clone().into_datum()),
                    (PgBuiltInOids::TEXTOID.oid(), span.artifact_id.clone().into_datum()),
                    (PgBuiltInOids::INT4OID.oid(), span.start_line.into_datum()),
                    (PgBuiltInOids::INT4OID.oid(), span.end_line.into_datum()),
                    (PgBuiltInOids::TEXTOID.oid(), span.text.clone().into_datum()),
                    (PgBuiltInOids::TEXTOID.oid(), embedding_str.into_datum()),
                    (PgBuiltInOids::TEXTOID.oid(), span.embedding_model.clone().into_datum()),
                    (PgBuiltInOids::INT4OID.oid(), span.token_count.into_datum()),
                ]),
            )?;
            Ok::<_, spi::Error>(())
        })
        .map_err(|e| AvocadoError::Database(format!("Failed to insert span: {}", e)))?;
    }

    Ok(())
}

/// Delete spans for an artifact
pub fn delete_spans_for_artifact(artifact_id: &str) -> Result<i64> {
    let count = Spi::connect(|mut client| {
        let result = client.update(
            "DELETE FROM avocado.spans WHERE artifact_id = $1",
            None,
            Some(vec![(PgBuiltInOids::TEXTOID.oid(), artifact_id.into_datum())]),
        )?;
        Ok::<_, spi::Error>(result as i64)
    })
    .map_err(|e| AvocadoError::Database(format!("Failed to delete spans: {}", e)))?;

    Ok(count)
}

/// Search for similar spans using pgvector
pub fn search_similar_spans(embedding: &[f32], limit: i32) -> Result<Vec<SpanWithScore>> {
    // Normalize query embedding to match stored dimension
    let normalized = normalize_embedding(embedding);
    let embedding_str = format!(
        "[{}]",
        normalized.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(",")
    );

    let results = Spi::connect(|client| {
        let table_result = client.select(
            "SELECT s.id, s.artifact_id, s.start_line, s.end_line, s.text,
                    s.embedding_model, s.token_count, a.path as artifact_path,
                    1 - (s.embedding <=> $1::vector) as score
             FROM avocado.spans s
             JOIN avocado.artifacts a ON s.artifact_id = a.id
             WHERE s.embedding IS NOT NULL
             ORDER BY s.embedding <=> $1::vector
             LIMIT $2",
            None,
            Some(vec![
                (PgBuiltInOids::TEXTOID.oid(), embedding_str.clone().into_datum()),
                (PgBuiltInOids::INT4OID.oid(), limit.into_datum()),
            ]),
        )?;

        let mut spans = Vec::new();
        for row in table_result {
            let span = SpanWithScore {
                span: Span {
                    id: row.get_by_name("id")?.unwrap_or_default(),
                    artifact_id: row.get_by_name("artifact_id")?.unwrap_or_default(),
                    start_line: row.get_by_name("start_line")?.unwrap_or(0),
                    end_line: row.get_by_name("end_line")?.unwrap_or(0),
                    text: row.get_by_name("text")?.unwrap_or_default(),
                    embedding: None, // Don't fetch full embedding
                    embedding_model: row.get_by_name("embedding_model")?,
                    token_count: row.get_by_name("token_count")?,
                },
                artifact_path: row.get_by_name("artifact_path")?.unwrap_or_default(),
                score: row.get_by_name::<f64, &str>("score")?.unwrap_or(0.0),
            };
            spans.push(span);
        }
        Ok::<_, spi::Error>(spans)
    })
    .map_err(|e| AvocadoError::Database(format!("Failed to search spans: {}", e)))?;

    Ok(results)
}

/// Create a new session
pub fn create_session(user_id: Option<&str>, title: Option<&str>) -> Result<Session> {
    let session_id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now();

    Spi::connect(|mut client| {
        client.update(
            "INSERT INTO avocado.sessions (id, user_id, title, created_at)
             VALUES ($1, $2, $3, $4)",
            None,
            Some(vec![
                (PgBuiltInOids::TEXTOID.oid(), session_id.clone().into_datum()),
                (PgBuiltInOids::TEXTOID.oid(), user_id.map(|s| s.to_string()).into_datum()),
                (PgBuiltInOids::TEXTOID.oid(), title.map(|s| s.to_string()).into_datum()),
                (PgBuiltInOids::TIMESTAMPTZOID.oid(), now.into_datum()),
            ]),
        )?;
        Ok::<_, spi::Error>(())
    })
    .map_err(|e| AvocadoError::Database(format!("Failed to create session: {}", e)))?;

    Ok(Session {
        id: session_id,
        user_id: user_id.map(String::from),
        title: title.map(String::from),
        metadata: None,
        created_at: now,
    })
}

/// Add a message to a session
pub fn add_message(
    session_id: &str,
    role: &str,
    content: &str,
    metadata: Option<serde_json::Value>,
) -> Result<Message> {
    let message_id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now();

    // Get next sequence number
    let seq_num = Spi::connect(|client| {
        let result: Option<i32> = client
            .select(
                "SELECT COALESCE(MAX(sequence_number), 0) + 1 FROM avocado.messages WHERE session_id = $1",
                None,
                Some(vec![(PgBuiltInOids::TEXTOID.oid(), session_id.into_datum())]),
            )?
            .first()
            .get_one()?;
        Ok::<_, spi::Error>(result.unwrap_or(1))
    })
    .map_err(|e| AvocadoError::Database(format!("Failed to get sequence number: {}", e)))?;

    Spi::connect(|mut client| {
        client.update(
            "INSERT INTO avocado.messages (id, session_id, role, content, metadata, sequence_number, created_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
            None,
            Some(vec![
                (PgBuiltInOids::TEXTOID.oid(), message_id.clone().into_datum()),
                (PgBuiltInOids::TEXTOID.oid(), session_id.into_datum()),
                (PgBuiltInOids::TEXTOID.oid(), role.into_datum()),
                (PgBuiltInOids::TEXTOID.oid(), content.into_datum()),
                (PgBuiltInOids::JSONBOID.oid(), JsonB(metadata.clone().unwrap_or(serde_json::json!({}))).into_datum()),
                (PgBuiltInOids::INT4OID.oid(), seq_num.into_datum()),
                (PgBuiltInOids::TIMESTAMPTZOID.oid(), now.into_datum()),
            ]),
        )?;
        Ok::<_, spi::Error>(())
    })
    .map_err(|e| AvocadoError::Database(format!("Failed to add message: {}", e)))?;

    Ok(Message {
        id: message_id,
        session_id: session_id.to_string(),
        role: role.to_string(),
        content: content.to_string(),
        metadata,
        sequence_number: seq_num,
        created_at: now,
    })
}

/// Get conversation history as formatted text
pub fn get_conversation_history(session_id: &str, max_tokens: Option<i32>) -> Result<String> {
    let messages = Spi::connect(|client| {
        let table_result = client.select(
            "SELECT role, content, token_count FROM avocado.messages
             WHERE session_id = $1
             ORDER BY sequence_number ASC",
            None,
            Some(vec![(PgBuiltInOids::TEXTOID.oid(), session_id.into_datum())]),
        )?;

        let mut messages = Vec::new();
        for row in table_result {
            let role: String = row.get_by_name("role")?.unwrap_or_default();
            let content: String = row.get_by_name("content")?.unwrap_or_default();
            messages.push((role, content));
        }
        Ok::<_, spi::Error>(messages)
    })
    .map_err(|e| AvocadoError::Database(format!("Failed to get history: {}", e)))?;

    // Format as conversation
    let mut history = String::new();
    for (role, content) in messages {
        history.push_str(&format!("{}: {}\n\n", role.to_uppercase(), content));
    }

    // TODO: Implement token truncation if max_tokens is specified
    Ok(history.trim().to_string())
}

/// Register a new agent
pub fn register_agent(
    name: &str,
    role: &str,
    model: &str,
    system_prompt: Option<&str>,
    capabilities: Option<serde_json::Value>,
) -> Result<Agent> {
    let agent_id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now();

    Spi::connect(|mut client| {
        client.update(
            "INSERT INTO avocado.agents (id, name, role, model, system_prompt, capabilities, created_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7)
             ON CONFLICT (name) DO UPDATE SET
                role = EXCLUDED.role,
                model = EXCLUDED.model,
                system_prompt = EXCLUDED.system_prompt,
                capabilities = EXCLUDED.capabilities",
            None,
            Some(vec![
                (PgBuiltInOids::TEXTOID.oid(), agent_id.clone().into_datum()),
                (PgBuiltInOids::TEXTOID.oid(), name.into_datum()),
                (PgBuiltInOids::TEXTOID.oid(), role.into_datum()),
                (PgBuiltInOids::TEXTOID.oid(), model.into_datum()),
                (PgBuiltInOids::TEXTOID.oid(), system_prompt.map(|s| s.to_string()).into_datum()),
                (PgBuiltInOids::JSONBOID.oid(), JsonB(capabilities.clone().unwrap_or(serde_json::json!({}))).into_datum()),
                (PgBuiltInOids::TIMESTAMPTZOID.oid(), now.into_datum()),
            ]),
        )?;
        Ok::<_, spi::Error>(())
    })
    .map_err(|e| AvocadoError::Database(format!("Failed to register agent: {}", e)))?;

    Ok(Agent {
        id: agent_id,
        name: name.to_string(),
        role: role.to_string(),
        model: model.to_string(),
        system_prompt: system_prompt.map(String::from),
        capabilities,
        metadata: None,
        created_at: now,
    })
}

/// List all agents
pub fn list_agents() -> Result<Vec<Agent>> {
    let agents = Spi::connect(|client| {
        let table_result = client.select(
            "SELECT id, name, role, model, system_prompt, capabilities, metadata, created_at
             FROM avocado.agents
             ORDER BY created_at DESC",
            None,
            None,
        )?;

        let mut agents = Vec::new();
        for row in table_result {
            let agent = Agent {
                id: row.get_by_name("id")?.unwrap_or_default(),
                name: row.get_by_name("name")?.unwrap_or_default(),
                role: row.get_by_name("role")?.unwrap_or_default(),
                model: row.get_by_name("model")?.unwrap_or_default(),
                system_prompt: row.get_by_name("system_prompt")?,
                capabilities: row.get_by_name::<JsonB, &str>("capabilities")?.map(|j| j.0),
                metadata: row.get_by_name::<JsonB, &str>("metadata")?.map(|j| j.0),
                created_at: row.get_by_name("created_at")?.unwrap_or_else(chrono::Utc::now),
            };
            agents.push(agent);
        }
        Ok::<_, spi::Error>(agents)
    })
    .map_err(|e| AvocadoError::Database(format!("Failed to list agents: {}", e)))?;

    Ok(agents)
}
