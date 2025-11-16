//! Database operations using SQLite
//!
//! This module handles all database interactions using rusqlite.
//! SQLite is sufficient for Phase 1 (can handle 10K+ documents easily).

use crate::types::{Artifact, Result, Span};
use parking_lot::RwLock;
use rusqlite::{params, Connection};
use std::path::Path;
use std::sync::Arc;

/// Database connection wrapper with thread-safe access
#[derive(Clone)]
pub struct Database {
    conn: Arc<RwLock<Connection>>,
}

impl Database {
    /// Create a new database connection and run migrations
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the SQLite database file
    ///
    /// # Returns
    ///
    /// A new Database instance
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let conn = Connection::open(path)?;

        // Enable foreign keys and WAL mode
        conn.execute("PRAGMA foreign_keys = ON", [])?;
        conn.execute("PRAGMA journal_mode = WAL", [])?;

        // Run migrations
        conn.execute_batch(include_str!("../../migrations/001_initial.sql"))?;

        Ok(Self {
            conn: Arc::new(RwLock::new(conn)),
        })
    }

    /// Insert an artifact into the database
    ///
    /// # Arguments
    ///
    /// * `artifact` - The artifact to insert
    ///
    /// # Returns
    ///
    /// Ok(()) if successful
    pub fn insert_artifact(&self, artifact: &Artifact) -> Result<()> {
        let conn = self.conn.write();
        conn.execute(
            "INSERT INTO artifacts (id, path, content, content_hash, metadata, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                artifact.id,
                artifact.path,
                artifact.content,
                artifact.content_hash,
                artifact.metadata.as_ref().map(|m| m.to_string()),
                artifact.created_at.to_rfc3339(),
            ],
        )?;
        Ok(())
    }

    /// Insert multiple spans in a transaction
    ///
    /// # Arguments
    ///
    /// * `spans` - Vector of spans to insert
    ///
    /// # Returns
    ///
    /// Ok(()) if successful
    pub fn insert_spans(&self, spans: &[Span]) -> Result<()> {
        let mut conn = self.conn.write();
        let tx = conn.transaction()?;

        for span in spans {
            tx.execute(
                "INSERT INTO spans (
                    id, artifact_id, start_line, end_line, text,
                    embedding, embedding_model, token_count, metadata
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    span.id,
                    span.artifact_id,
                    span.start_line as i64,
                    span.end_line as i64,
                    span.text,
                    span.embedding.as_ref().map(serialize_embedding),
                    span.embedding_model,
                    span.token_count as i64,
                    span.metadata.as_ref().map(|m| m.to_string()),
                ],
            )?;
        }

        tx.commit()?;
        Ok(())
    }

    /// Get all spans from the database
    ///
    /// # Returns
    ///
    /// Vector of all spans
    pub fn get_all_spans(&self) -> Result<Vec<Span>> {
        let conn = self.conn.read();
        let mut stmt = conn.prepare(
            "SELECT id, artifact_id, start_line, end_line, text,
                    embedding, embedding_model, token_count, metadata
             FROM spans",
        )?;

        let spans = stmt
            .query_map([], |row| {
                Ok(Span {
                    id: row.get(0)?,
                    artifact_id: row.get(1)?,
                    start_line: row.get::<_, i64>(2)? as usize,
                    end_line: row.get::<_, i64>(3)? as usize,
                    text: row.get(4)?,
                    embedding: row
                        .get::<_, Option<Vec<u8>>>(5)?
                        .map(|bytes| deserialize_embedding(&bytes)),
                    embedding_model: row.get(6)?,
                    token_count: row.get::<_, i64>(7)? as usize,
                    metadata: row
                        .get::<_, Option<String>>(8)?
                        .and_then(|s| serde_json::from_str(&s).ok()),
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(spans)
    }

    /// Get artifact by ID
    ///
    /// # Arguments
    ///
    /// * `artifact_id` - The artifact ID to look up
    ///
    /// # Returns
    ///
    /// The artifact if found
    pub fn get_artifact(&self, artifact_id: &str) -> Result<Option<Artifact>> {
        let conn = self.conn.read();
        let mut stmt = conn.prepare(
            "SELECT id, path, content, content_hash, metadata, created_at
             FROM artifacts WHERE id = ?1",
        )?;

        let artifact = stmt
            .query_row(params![artifact_id], |row| {
                Ok(Artifact {
                    id: row.get(0)?,
                    path: row.get(1)?,
                    content: row.get(2)?,
                    content_hash: row.get(3)?,
                    metadata: row
                        .get::<_, Option<String>>(4)?
                        .and_then(|s| serde_json::from_str(&s).ok()),
                    created_at: row
                        .get::<_, String>(5)?
                        .parse()
                        .unwrap_or_else(|_| chrono::Utc::now()),
                })
            })
            .optional()?;

        Ok(artifact)
    }

    /// Search spans by text content (simple keyword matching)
    ///
    /// # Arguments
    ///
    /// * `query` - The search query
    /// * `limit` - Maximum number of results
    ///
    /// # Returns
    ///
    /// Vector of matching spans
    pub fn search_spans(&self, query: &str, limit: usize) -> Result<Vec<Span>> {
        let conn = self.conn.read();
        let mut stmt = conn.prepare(
            "SELECT id, artifact_id, start_line, end_line, text,
                    embedding, embedding_model, token_count, metadata
             FROM spans
             WHERE text LIKE ?1
             LIMIT ?2",
        )?;

        let pattern = format!("%{}%", query);
        let spans = stmt
            .query_map(params![pattern, limit as i64], |row| {
                Ok(Span {
                    id: row.get(0)?,
                    artifact_id: row.get(1)?,
                    start_line: row.get::<_, i64>(2)? as usize,
                    end_line: row.get::<_, i64>(3)? as usize,
                    text: row.get(4)?,
                    embedding: row
                        .get::<_, Option<Vec<u8>>>(5)?
                        .map(|bytes| deserialize_embedding(&bytes)),
                    embedding_model: row.get(6)?,
                    token_count: row.get::<_, i64>(7)? as usize,
                    metadata: row
                        .get::<_, Option<String>>(8)?
                        .and_then(|s| serde_json::from_str(&s).ok()),
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(spans)
    }

    /// Get database statistics
    ///
    /// # Returns
    ///
    /// (artifacts_count, spans_count, total_tokens)
    pub fn get_stats(&self) -> Result<(usize, usize, usize)> {
        let conn = self.conn.read();

        let artifacts_count: i64 = conn.query_row("SELECT COUNT(*) FROM artifacts", [], |row| {
            row.get(0)
        })?;

        let spans_count: i64 = conn.query_row("SELECT COUNT(*) FROM spans", [], |row| row.get(0))?;

        let total_tokens: i64 = conn
            .query_row("SELECT COALESCE(SUM(token_count), 0) FROM spans", [], |row| {
                row.get(0)
            })?;

        Ok((
            artifacts_count as usize,
            spans_count as usize,
            total_tokens as usize,
        ))
    }

    /// Clear all data from the database
    pub fn clear(&self) -> Result<()> {
        let conn = self.conn.write();
        conn.execute("DELETE FROM spans", [])?;
        conn.execute("DELETE FROM artifacts", [])?;
        Ok(())
    }
}

/// Serialize embedding vector to bytes for storage
fn serialize_embedding(embedding: &[f32]) -> Vec<u8> {
    embedding.iter().flat_map(|f| f.to_le_bytes()).collect()
}

/// Deserialize embedding vector from bytes
fn deserialize_embedding(bytes: &[u8]) -> Vec<f32> {
    bytes
        .chunks_exact(4)
        .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_database_creation() {
        let db = Database::new(":memory:").unwrap();
        let (artifacts, spans, tokens) = db.get_stats().unwrap();
        assert_eq!(artifacts, 0);
        assert_eq!(spans, 0);
        assert_eq!(tokens, 0);
    }

    #[test]
    fn test_insert_artifact() {
        let db = Database::new(":memory:").unwrap();

        let artifact = Artifact {
            id: Uuid::new_v4().to_string(),
            path: "test.txt".to_string(),
            content: "Test content".to_string(),
            content_hash: "hash123".to_string(),
            metadata: None,
            created_at: chrono::Utc::now(),
        };

        db.insert_artifact(&artifact).unwrap();

        let (count, _, _) = db.get_stats().unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_embedding_serialization() {
        let original = vec![1.0, 2.5, -3.14, 0.0];
        let bytes = serialize_embedding(&original);
        let restored = deserialize_embedding(&bytes);

        assert_eq!(original.len(), restored.len());
        for (a, b) in original.iter().zip(restored.iter()) {
            assert!((a - b).abs() < 0.0001);
        }
    }
}
