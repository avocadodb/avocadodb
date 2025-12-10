//! Core storage backend trait definitions

use async_trait::async_trait;
use std::sync::Arc;

use crate::storage::vector::VectorSearchProvider;
use crate::types::{
    Agent, AgentRelation, AgentRelationSummary, Artifact, CompilerConfig, IngestAction, Message,
    MessageRole, Result, Session, SessionWithMessages, SessionWorkingSet, Span, Stance, WorkingSet,
};

/// Configuration for storage backends
#[derive(Debug, Clone)]
pub enum StorageConfig {
    /// SQLite backend (default)
    Sqlite {
        /// Path to SQLite database file
        path: String,
    },
    /// PostgreSQL backend with pgvector
    Postgres {
        /// PostgreSQL connection string
        connection_string: String,
    },
}

impl StorageConfig {
    /// Parse configuration from AVOCADO_BACKEND environment variable
    ///
    /// # Examples
    ///
    /// - `sqlite` or unset -> SQLite with default path
    /// - `sqlite:/path/to/db.sqlite` -> SQLite with specific path
    /// - `postgres://user:pass@host/db` -> PostgreSQL
    pub fn from_env(default_sqlite_path: &str) -> Self {
        match std::env::var("AVOCADO_BACKEND").ok() {
            None => StorageConfig::Sqlite {
                path: default_sqlite_path.to_string(),
            },
            Some(ref s) if s == "sqlite" || s.is_empty() => StorageConfig::Sqlite {
                path: default_sqlite_path.to_string(),
            },
            Some(ref s) if s.starts_with("sqlite:") => StorageConfig::Sqlite {
                path: s.strip_prefix("sqlite:").unwrap().to_string(),
            },
            Some(ref s) if s.starts_with("postgres://") || s.starts_with("postgresql://") => {
                StorageConfig::Postgres {
                    connection_string: s.clone(),
                }
            }
            Some(s) => {
                eprintln!(
                    "[AvocadoDB] Unknown AVOCADO_BACKEND '{}', defaulting to SQLite",
                    s
                );
                StorageConfig::Sqlite {
                    path: default_sqlite_path.to_string(),
                }
            }
        }
    }
}

/// Main storage backend trait
///
/// Implementations must be Send + Sync for use in async contexts.
/// All methods are async to support both sync (SQLite) and async (PostgreSQL) backends.
#[async_trait]
pub trait StorageBackend: Send + Sync {
    // ========== Lifecycle ==========

    /// Get database statistics
    ///
    /// # Returns
    /// Tuple of (artifacts_count, spans_count, total_tokens)
    async fn get_stats(&self) -> Result<(usize, usize, usize)>;

    /// Clear all data from the database
    async fn clear(&self) -> Result<()>;

    // ========== Artifacts ==========

    /// Insert an artifact
    async fn insert_artifact(&self, artifact: &Artifact) -> Result<()>;

    /// Get artifact by ID
    async fn get_artifact(&self, artifact_id: &str) -> Result<Option<Artifact>>;

    /// Get artifact by path
    async fn get_artifact_by_path(&self, path: &str) -> Result<Option<Artifact>>;

    /// Delete artifact and associated spans
    ///
    /// # Returns
    /// Number of spans deleted
    async fn delete_artifact(&self, artifact_id: &str) -> Result<usize>;

    /// Determine what action to take when ingesting a file
    async fn determine_ingest_action(&self, path: &str, content_hash: &str)
        -> Result<IngestAction>;

    // ========== Spans ==========

    /// Insert multiple spans in a transaction
    async fn insert_spans(&self, spans: &[Span]) -> Result<()>;

    /// Get all spans (for index building)
    async fn get_all_spans(&self) -> Result<Vec<Span>>;

    /// Search spans by text (lexical search)
    async fn search_spans(&self, query: &str, limit: usize) -> Result<Vec<Span>>;

    // ========== Vector Search ==========

    /// Get the vector search provider for this backend
    ///
    /// For SQLite, this builds/loads an HNSW index.
    /// For PostgreSQL, this uses pgvector queries.
    async fn get_vector_search(&self) -> Result<Arc<dyn VectorSearchProvider>>;

    /// Invalidate cached vector index (called after data changes)
    async fn invalidate_vector_index(&self);

    // ========== Sessions ==========

    /// Create a new session
    async fn create_session(&self, user_id: Option<&str>, title: Option<&str>) -> Result<Session>;

    /// Get session by ID
    async fn get_session(&self, session_id: &str) -> Result<Option<Session>>;

    /// List sessions with optional filtering
    async fn list_sessions(
        &self,
        user_id: Option<&str>,
        limit: Option<usize>,
    ) -> Result<Vec<Session>>;

    /// Update session metadata
    async fn update_session(
        &self,
        session_id: &str,
        title: Option<&str>,
        metadata: Option<&serde_json::Value>,
    ) -> Result<()>;

    /// Delete session and all associated data
    async fn delete_session(&self, session_id: &str) -> Result<()>;

    // ========== Messages ==========

    /// Add message to session
    async fn add_message(
        &self,
        session_id: &str,
        role: MessageRole,
        content: &str,
        metadata: Option<&serde_json::Value>,
    ) -> Result<Message>;

    /// Get messages for session
    async fn get_messages(&self, session_id: &str, limit: Option<usize>) -> Result<Vec<Message>>;

    // ========== Working Sets ==========

    /// Associate working set with session
    async fn associate_working_set(
        &self,
        session_id: &str,
        message_id: Option<&str>,
        working_set: &WorkingSet,
        query: &str,
        config: &CompilerConfig,
    ) -> Result<SessionWorkingSet>;

    /// Get full session with messages and working sets
    async fn get_session_full(&self, session_id: &str) -> Result<Option<SessionWithMessages>>;

    // ========== Agents ==========

    /// Register an agent (insert or update)
    async fn register_agent(&self, agent: &Agent) -> Result<Agent>;

    /// Get agent by ID
    async fn get_agent(&self, agent_id: &str) -> Result<Option<Agent>>;

    /// Get agent by name
    async fn get_agent_by_name(&self, name: &str) -> Result<Option<Agent>>;

    /// List all registered agents
    async fn list_agents(&self) -> Result<Vec<Agent>>;

    // ========== Agent Relations ==========

    /// Add agent relation (agreement, disagreement, etc.)
    async fn add_agent_relation(
        &self,
        session_id: &str,
        message_id: &str,
        from_agent_id: &str,
        target_message_id: &str,
        stance: Stance,
    ) -> Result<AgentRelation>;

    /// Get agent relations for session with resolved names
    async fn get_agent_relations(&self, session_id: &str) -> Result<AgentRelationSummary>;

    /// Get agents participating in session
    async fn get_session_agents(&self, session_id: &str) -> Result<Vec<Agent>>;
}
