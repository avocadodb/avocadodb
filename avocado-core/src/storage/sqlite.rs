//! SQLite storage backend implementation
//!
//! Wraps the existing Database struct with async interface using spawn_blocking.

use async_trait::async_trait;
use std::path::Path;
use std::sync::Arc;
use tokio::task;

use crate::db::Database;
use crate::index::VectorIndex;
use crate::storage::traits::StorageBackend;
use crate::storage::vector::{VectorSearchProvider, VectorSearchResult};
use crate::types::*;

/// SQLite storage backend
///
/// Wraps the existing Database implementation with async interface.
/// Uses `tokio::task::spawn_blocking` for rusqlite operations.
pub struct SqliteBackend {
    db: Database,
}

impl SqliteBackend {
    /// Create new SQLite backend
    ///
    /// # Arguments
    /// * `path` - Path to SQLite database file
    pub async fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        let db = task::spawn_blocking(move || Database::new(&path))
            .await
            .map_err(|e| Error::Other(anyhow::anyhow!("Task join error: {}", e)))??;

        Ok(Self { db })
    }

    /// Get reference to underlying Database (for backward compatibility)
    pub fn database(&self) -> &Database {
        &self.db
    }
}

#[async_trait]
impl StorageBackend for SqliteBackend {
    // ========== Lifecycle ==========

    async fn get_stats(&self) -> Result<(usize, usize, usize)> {
        let db = self.db.clone();
        task::spawn_blocking(move || db.get_stats())
            .await
            .map_err(|e| Error::Other(anyhow::anyhow!("Task join error: {}", e)))?
    }

    async fn clear(&self) -> Result<()> {
        let db = self.db.clone();
        task::spawn_blocking(move || db.clear())
            .await
            .map_err(|e| Error::Other(anyhow::anyhow!("Task join error: {}", e)))?
    }

    // ========== Artifacts ==========

    async fn insert_artifact(&self, artifact: &Artifact) -> Result<()> {
        let db = self.db.clone();
        let artifact = artifact.clone();
        task::spawn_blocking(move || db.insert_artifact(&artifact))
            .await
            .map_err(|e| Error::Other(anyhow::anyhow!("Task join error: {}", e)))?
    }

    async fn get_artifact(&self, artifact_id: &str) -> Result<Option<Artifact>> {
        let db = self.db.clone();
        let id = artifact_id.to_string();
        task::spawn_blocking(move || db.get_artifact(&id))
            .await
            .map_err(|e| Error::Other(anyhow::anyhow!("Task join error: {}", e)))?
    }

    async fn get_artifact_by_path(&self, path: &str) -> Result<Option<Artifact>> {
        let db = self.db.clone();
        let p = path.to_string();
        task::spawn_blocking(move || db.get_artifact_by_path(&p))
            .await
            .map_err(|e| Error::Other(anyhow::anyhow!("Task join error: {}", e)))?
    }

    async fn delete_artifact(&self, artifact_id: &str) -> Result<usize> {
        let db = self.db.clone();
        let id = artifact_id.to_string();
        task::spawn_blocking(move || db.delete_artifact(&id))
            .await
            .map_err(|e| Error::Other(anyhow::anyhow!("Task join error: {}", e)))?
    }

    async fn determine_ingest_action(
        &self,
        path: &str,
        content_hash: &str,
    ) -> Result<IngestAction> {
        let db = self.db.clone();
        let p = path.to_string();
        let h = content_hash.to_string();
        task::spawn_blocking(move || db.determine_ingest_action(&p, &h))
            .await
            .map_err(|e| Error::Other(anyhow::anyhow!("Task join error: {}", e)))?
    }

    // ========== Spans ==========

    async fn insert_spans(&self, spans: &[Span]) -> Result<()> {
        let db = self.db.clone();
        let spans = spans.to_vec();
        task::spawn_blocking(move || db.insert_spans(&spans))
            .await
            .map_err(|e| Error::Other(anyhow::anyhow!("Task join error: {}", e)))?
    }

    async fn get_all_spans(&self) -> Result<Vec<Span>> {
        let db = self.db.clone();
        task::spawn_blocking(move || db.get_all_spans())
            .await
            .map_err(|e| Error::Other(anyhow::anyhow!("Task join error: {}", e)))?
    }

    async fn search_spans(&self, query: &str, limit: usize) -> Result<Vec<Span>> {
        let db = self.db.clone();
        let q = query.to_string();
        task::spawn_blocking(move || db.search_spans(&q, limit))
            .await
            .map_err(|e| Error::Other(anyhow::anyhow!("Task join error: {}", e)))?
    }

    // ========== Vector Search ==========

    async fn get_vector_search(&self) -> Result<Arc<dyn VectorSearchProvider>> {
        let db = self.db.clone();
        let index = task::spawn_blocking(move || db.get_vector_index())
            .await
            .map_err(|e| Error::Other(anyhow::anyhow!("Task join error: {}", e)))??;

        Ok(Arc::new(HnswVectorSearch::new(index)))
    }

    async fn invalidate_vector_index(&self) {
        // The existing Database handles this internally via index_dirty flag
        // This is called after data changes to force index rebuild
    }

    // ========== Sessions ==========

    async fn create_session(
        &self,
        user_id: Option<&str>,
        title: Option<&str>,
    ) -> Result<Session> {
        let db = self.db.clone();
        let uid = user_id.map(|s| s.to_string());
        let t = title.map(|s| s.to_string());
        task::spawn_blocking(move || db.create_session(uid.as_deref(), t.as_deref()))
            .await
            .map_err(|e| Error::Other(anyhow::anyhow!("Task join error: {}", e)))?
    }

    async fn get_session(&self, session_id: &str) -> Result<Option<Session>> {
        let db = self.db.clone();
        let id = session_id.to_string();
        task::spawn_blocking(move || db.get_session(&id))
            .await
            .map_err(|e| Error::Other(anyhow::anyhow!("Task join error: {}", e)))?
    }

    async fn list_sessions(
        &self,
        user_id: Option<&str>,
        limit: Option<usize>,
    ) -> Result<Vec<Session>> {
        let db = self.db.clone();
        let uid = user_id.map(|s| s.to_string());
        task::spawn_blocking(move || db.list_sessions(uid.as_deref(), limit))
            .await
            .map_err(|e| Error::Other(anyhow::anyhow!("Task join error: {}", e)))?
    }

    async fn update_session(
        &self,
        session_id: &str,
        title: Option<&str>,
        metadata: Option<&serde_json::Value>,
    ) -> Result<()> {
        let db = self.db.clone();
        let id = session_id.to_string();
        let t = title.map(|s| s.to_string());
        let m = metadata.cloned();
        task::spawn_blocking(move || db.update_session(&id, t.as_deref(), m.as_ref()))
            .await
            .map_err(|e| Error::Other(anyhow::anyhow!("Task join error: {}", e)))?
    }

    async fn delete_session(&self, session_id: &str) -> Result<()> {
        let db = self.db.clone();
        let id = session_id.to_string();
        task::spawn_blocking(move || db.delete_session(&id))
            .await
            .map_err(|e| Error::Other(anyhow::anyhow!("Task join error: {}", e)))?
    }

    // ========== Messages ==========

    async fn add_message(
        &self,
        session_id: &str,
        role: MessageRole,
        content: &str,
        metadata: Option<&serde_json::Value>,
    ) -> Result<Message> {
        let db = self.db.clone();
        let sid = session_id.to_string();
        let c = content.to_string();
        let m = metadata.cloned();
        task::spawn_blocking(move || db.add_message(&sid, role, &c, m.as_ref()))
            .await
            .map_err(|e| Error::Other(anyhow::anyhow!("Task join error: {}", e)))?
    }

    async fn get_messages(
        &self,
        session_id: &str,
        limit: Option<usize>,
    ) -> Result<Vec<Message>> {
        let db = self.db.clone();
        let sid = session_id.to_string();
        task::spawn_blocking(move || db.get_messages(&sid, limit))
            .await
            .map_err(|e| Error::Other(anyhow::anyhow!("Task join error: {}", e)))?
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
        let db = self.db.clone();
        let sid = session_id.to_string();
        let mid = message_id.map(|s| s.to_string());
        let ws = working_set.clone();
        let q = query.to_string();
        let cfg = config.clone();
        task::spawn_blocking(move || {
            db.associate_working_set(&sid, mid.as_deref(), &ws, &q, &cfg)
        })
        .await
        .map_err(|e| Error::Other(anyhow::anyhow!("Task join error: {}", e)))?
    }

    async fn get_session_full(&self, session_id: &str) -> Result<Option<SessionWithMessages>> {
        let db = self.db.clone();
        let sid = session_id.to_string();
        task::spawn_blocking(move || db.get_session_full(&sid))
            .await
            .map_err(|e| Error::Other(anyhow::anyhow!("Task join error: {}", e)))?
    }

    // ========== Agents ==========

    async fn register_agent(&self, agent: &Agent) -> Result<Agent> {
        let db = self.db.clone();
        let a = agent.clone();
        task::spawn_blocking(move || db.register_agent(&a))
            .await
            .map_err(|e| Error::Other(anyhow::anyhow!("Task join error: {}", e)))?
    }

    async fn get_agent(&self, agent_id: &str) -> Result<Option<Agent>> {
        let db = self.db.clone();
        let id = agent_id.to_string();
        task::spawn_blocking(move || db.get_agent(&id))
            .await
            .map_err(|e| Error::Other(anyhow::anyhow!("Task join error: {}", e)))?
    }

    async fn get_agent_by_name(&self, name: &str) -> Result<Option<Agent>> {
        let db = self.db.clone();
        let n = name.to_string();
        task::spawn_blocking(move || db.get_agent_by_name(&n))
            .await
            .map_err(|e| Error::Other(anyhow::anyhow!("Task join error: {}", e)))?
    }

    async fn list_agents(&self) -> Result<Vec<Agent>> {
        let db = self.db.clone();
        task::spawn_blocking(move || db.list_agents())
            .await
            .map_err(|e| Error::Other(anyhow::anyhow!("Task join error: {}", e)))?
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
        let db = self.db.clone();
        let sid = session_id.to_string();
        let mid = message_id.to_string();
        let fid = from_agent_id.to_string();
        let tmid = target_message_id.to_string();
        task::spawn_blocking(move || db.add_agent_relation(&sid, &mid, &fid, &tmid, stance))
            .await
            .map_err(|e| Error::Other(anyhow::anyhow!("Task join error: {}", e)))?
    }

    async fn get_agent_relations(&self, session_id: &str) -> Result<AgentRelationSummary> {
        let db = self.db.clone();
        let sid = session_id.to_string();
        task::spawn_blocking(move || db.get_agent_relations(&sid))
            .await
            .map_err(|e| Error::Other(anyhow::anyhow!("Task join error: {}", e)))?
    }

    async fn get_session_agents(&self, session_id: &str) -> Result<Vec<Agent>> {
        let db = self.db.clone();
        let sid = session_id.to_string();
        task::spawn_blocking(move || db.get_session_agents(&sid))
            .await
            .map_err(|e| Error::Other(anyhow::anyhow!("Task join error: {}", e)))?
    }
}

// ========== HNSW Vector Search Provider ==========

/// HNSW-based vector search for SQLite backend
struct HnswVectorSearch {
    index: Arc<VectorIndex>,
}

impl HnswVectorSearch {
    fn new(index: Arc<VectorIndex>) -> Self {
        Self { index }
    }
}

#[async_trait]
impl VectorSearchProvider for HnswVectorSearch {
    async fn search(&self, query_embedding: &[f32], k: usize) -> Result<Vec<VectorSearchResult>> {
        let index = self.index.clone();
        let query = query_embedding.to_vec();

        let results = task::spawn_blocking(move || index.search(&query, k))
            .await
            .map_err(|e| Error::Other(anyhow::anyhow!("Task join error: {}", e)))??;

        Ok(results.into_iter().map(VectorSearchResult::from).collect())
    }

    fn len(&self) -> usize {
        self.index.len()
    }

    fn dimension(&self) -> usize {
        self.index
            .spans()
            .first()
            .and_then(|s| s.embedding.as_ref().map(|e| e.len()))
            .unwrap_or(384) // Default for all-MiniLM-L6-v2
    }
}
