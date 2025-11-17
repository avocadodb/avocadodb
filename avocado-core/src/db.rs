//! Database operations using SQLite
//!
//! This module handles all database interactions using rusqlite.
//! SQLite is sufficient for Phase 1 (can handle 10K+ documents easily).

use crate::types::{Artifact, Result, Span};
use crate::index::VectorIndex;
use rusqlite::{params, Connection, OptionalExtension};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use sha2::{Digest, Sha256};

/// Database connection wrapper with thread-safe access
#[derive(Clone)]
pub struct Database {
    conn: Arc<Mutex<Connection>>,
    // Cached vector index to avoid rebuilding on every compile request
    vector_index: Arc<RwLock<Option<Arc<VectorIndex>>>>,
    // Flag to track if index needs rebuilding (invalidated on ingest)
    index_dirty: Arc<AtomicBool>,
    // Path to database file (for index cache location)
    db_path: PathBuf,
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
        let db_path = path.as_ref().to_path_buf();
        let conn = Connection::open(&db_path)?;

        // Run migrations (without PRAGMA statements)
        let schema = include_str!("../../migrations/001_initial.sql");
        
        // Execute the schema without PRAGMAs
        let schema_without_pragma = schema
            .lines()
            .filter(|line| {
                let trimmed = line.trim();
                !trimmed.starts_with("PRAGMA") && !trimmed.starts_with("-- Enable WAL")
            })
            .collect::<Vec<_>>()
            .join("\n");

        conn.execute_batch(&schema_without_pragma)?;

        // Execute PRAGMAs separately (they return results)
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "foreign_keys", true)?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
            vector_index: Arc::new(RwLock::new(None)),
            index_dirty: Arc::new(AtomicBool::new(true)),
            db_path,
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
        let conn = self.conn.lock()
            .map_err(|e| crate::types::Error::Other(anyhow::anyhow!("Database lock poisoned: {}", e)))?;
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
        // Invalidate cached index since we added a new artifact
        self.index_dirty.store(true, Ordering::Release);
        // Delete index cache directory since it's now stale
        let _ = std::fs::remove_dir_all(self.get_index_cache_dir());
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
        let mut conn = self.conn.lock()
            .map_err(|e| crate::types::Error::Other(anyhow::anyhow!("Database lock poisoned: {}", e)))?;
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
                    span.embedding.as_ref().map(|e| serialize_embedding(e)),
                    span.embedding_model,
                    span.token_count as i64,
                    span.metadata.as_ref().map(|m| m.to_string()),
                ],
            )?;
        }

        tx.commit()?;
        // Invalidate cached index since we added new spans
        self.index_dirty.store(true, Ordering::Release);
        // Delete index cache directory since it's now stale
        let _ = std::fs::remove_dir_all(self.get_index_cache_dir());
        Ok(())
    }

    /// Get or build the cached vector index
    ///
    /// The index is cached and only rebuilt when data changes (on ingest).
    /// Phase 2.1: Tries to load from disk first, then builds if needed.
    ///
    /// # Returns
    ///
    /// A reference-counted vector index
    pub fn get_vector_index(&self) -> Result<Arc<VectorIndex>> {
        // Check if index needs rebuilding
        if self.index_dirty.load(Ordering::Acquire) {
            // Try to load from disk first (Phase 2.1 persistent index)
            let cache_dir = self.get_index_cache_dir();
            if let Ok(index) = self.load_index_from_disk(&cache_dir) {
                // Index loaded successfully from cache
                // Note: We still rebuild HNSW from cached spans due to lifetime constraints in hnsw_rs
                // This is faster than loading from SQLite, but not as fast as loading HNSW structure directly
                let mut cached = self.vector_index.write()
                    .map_err(|e| crate::types::Error::Other(anyhow::anyhow!("Index lock poisoned: {}", e)))?;
                *cached = Some(index.clone());
                self.index_dirty.store(false, Ordering::Release);
                return Ok(index);
            }
            
            // Build index from spans (load from SQLite)
            // For large repos, this can take 1-2 minutes
            let spans = self.get_all_spans()?;
            let index = Arc::new(VectorIndex::build(spans));
            
            // Save to disk for next time (Phase 2.1)
            // This saves both HNSW dump files and spans cache
            // Note: HNSW structure can't be directly loaded due to lifetime constraints,
            // but caching spans still provides significant speedup (avoids SQLite queries)
            let _ = self.save_index_to_disk(&cache_dir, &index);
            
            // Update cache
            let mut cached = self.vector_index.write()
                .map_err(|e| crate::types::Error::Other(anyhow::anyhow!("Index lock poisoned: {}", e)))?;
            *cached = Some(index.clone());
            
            // Mark as clean
            self.index_dirty.store(false, Ordering::Release);
            
            Ok(index)
        } else {
            // Return cached index
            let cached = self.vector_index.read()
                .map_err(|e| crate::types::Error::Other(anyhow::anyhow!("Index lock poisoned: {}", e)))?;
            cached.as_ref()
                .cloned()
                .ok_or_else(|| crate::types::Error::Other(anyhow::anyhow!("Index cache is None but not dirty - this should not happen")))
        }
    }
    
    /// Get the path to the index cache directory
    fn get_index_cache_dir(&self) -> PathBuf {
        // Store index cache in a directory next to database: db.sqlite -> db.sqlite.idx/
        let mut cache_dir = self.db_path.clone();
        cache_dir.set_extension("sqlite.idx");
        cache_dir
    }
    
    /// Calculate a hash of all spans to detect changes
    fn calculate_spans_hash(&self) -> Result<String> {
        let spans = self.get_all_spans()?;
        let mut hasher = Sha256::new();
        for span in &spans {
            hasher.update(span.id.as_bytes());
            if let Some(emb) = &span.embedding {
                hasher.update(&emb.len().to_le_bytes());
            }
        }
        Ok(format!("{:x}", hasher.finalize()))
    }
    
    /// Load index from disk if valid
    fn load_index_from_disk(&self, cache_dir: &Path) -> Result<Arc<VectorIndex>> {
        // Try to load using VectorIndex::load_from_disk
        match VectorIndex::load_from_disk(cache_dir) {
            Ok(Some(index)) => {
                // Verify hash matches current spans (double-check)
                let current_hash = self.calculate_spans_hash()?;
                let cached_spans = index.spans();
                let mut hasher = Sha256::new();
                for span in cached_spans {
                    hasher.update(span.id.as_bytes());
                    if let Some(emb) = &span.embedding {
                        hasher.update(&emb.len().to_le_bytes());
                    }
                }
                let cached_hash = format!("{:x}", hasher.finalize());
                
                if cached_hash == current_hash {
                    Ok(Arc::new(index))
                } else {
                    Err(crate::types::Error::NotFound("Index cache is stale".to_string()))
                }
            }
            Ok(None) => Err(crate::types::Error::NotFound("Index cache not found".to_string())),
            Err(e) => Err(e),
        }
    }
    
    /// Save index to disk for persistence
    fn save_index_to_disk(&self, cache_dir: &Path, index: &VectorIndex) -> Result<()> {
        // Use VectorIndex::save_to_disk which saves both HNSW dump and spans
        index.save_to_disk(cache_dir)
            .map_err(|e| crate::types::Error::Other(anyhow::anyhow!("Failed to save index to disk: {}", e)))?;
        Ok(())
    }

    /// Get all spans from the database
    ///
    /// # Returns
    ///
    /// Vector of all spans
    pub fn get_all_spans(&self) -> Result<Vec<Span>> {
        let conn = self.conn.lock()
            .map_err(|e| crate::types::Error::Other(anyhow::anyhow!("Database lock poisoned: {}", e)))?;
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
        let conn = self.conn.lock()
            .map_err(|e| crate::types::Error::Other(anyhow::anyhow!("Database lock poisoned: {}", e)))?;
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
        let conn = self.conn.lock()
            .map_err(|e| crate::types::Error::Other(anyhow::anyhow!("Database lock poisoned: {}", e)))?;
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
        let conn = self.conn.lock()
            .map_err(|e| crate::types::Error::Other(anyhow::anyhow!("Database lock poisoned: {}", e)))?;

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
        let conn = self.conn.lock()
            .map_err(|e| crate::types::Error::Other(anyhow::anyhow!("Database lock poisoned: {}", e)))?;
        conn.execute("DELETE FROM spans", [])?;
        conn.execute("DELETE FROM artifacts", [])?;
        // Clear cached index
        let mut cached = self.vector_index.write()
            .map_err(|e| crate::types::Error::Other(anyhow::anyhow!("Index lock poisoned: {}", e)))?;
        *cached = None;
        self.index_dirty.store(true, Ordering::Release);
        // Delete index cache directory
        let _ = std::fs::remove_dir_all(self.get_index_cache_dir());
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
