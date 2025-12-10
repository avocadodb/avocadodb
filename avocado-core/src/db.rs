//! Database operations using SQLite
//!
//! This module handles all database interactions using rusqlite.
//! SQLite is sufficient for Phase 1 (can handle 10K+ documents easily).

use crate::types::{Artifact, Result, Span, Session, Message, MessageRole, SessionWorkingSet, SessionWithMessages, WorkingSet, CompilerConfig};
use crate::index::VectorIndex;
use rusqlite::{params, Connection, OptionalExtension};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use sha2::{Digest, Sha256};
use serde::{Serialize, Deserialize};

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
    // Serialize builds for this Database to avoid concurrent heavy index builds
    build_lock: Arc<Mutex<()>>,
}

/// How the ANN index was obtained
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum IndexLoadKind {
    /// Loaded from on-disk cache (ANN persistence)
    LoadedFromCache,
    /// Built from spans in the database
    BuiltFromSpans,
    /// Returned from in-memory cache (no disk or build)
    CachedInMemory,
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

        // Run initial migration (without PRAGMA statements)
        let schema_001 = r#"-- AvocadoDB Initial Schema
-- Phase 1: Simple SQLite-compatible schema for deterministic context compilation

-- Artifacts table: stores ingested documents
CREATE TABLE IF NOT EXISTS artifacts (
    id TEXT PRIMARY KEY,                      -- UUID v4
    path TEXT NOT NULL UNIQUE,                -- File path or identifier
    content TEXT NOT NULL,                    -- Full document text
    content_hash TEXT NOT NULL,               -- SHA256 of content
    metadata TEXT,                            -- JSON string with arbitrary metadata
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Spans table: stores document fragments with embeddings
CREATE TABLE IF NOT EXISTS spans (
    id TEXT PRIMARY KEY,                      -- UUID v4
    artifact_id TEXT NOT NULL,                -- Foreign key to artifacts
    start_line INTEGER NOT NULL,              -- Starting line number (1-indexed)
    end_line INTEGER NOT NULL,                -- Ending line number (inclusive)
    text TEXT NOT NULL,                       -- Actual span text
    embedding BLOB,                           -- Serialized f32 vector (1536 dims for ada-002)
    embedding_model TEXT,                     -- e.g., "text-embedding-ada-002"
    token_count INTEGER,                      -- Estimated token count
    metadata TEXT,                            -- JSON string
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (artifact_id) REFERENCES artifacts(id) ON DELETE CASCADE
);

-- Indexes for performance
CREATE INDEX IF NOT EXISTS idx_spans_artifact ON spans(artifact_id);
CREATE INDEX IF NOT EXISTS idx_spans_lines ON spans(artifact_id, start_line, end_line);
CREATE INDEX IF NOT EXISTS idx_artifacts_path ON artifacts(path);
CREATE INDEX IF NOT EXISTS idx_artifacts_hash ON artifacts(content_hash);

-- Enable WAL mode for better concurrency
PRAGMA journal_mode = WAL;
PRAGMA foreign_keys = ON;
"#;

        // Execute the schema without PRAGMAs
        let schema_without_pragma = schema_001
            .lines()
            .filter(|line| {
                let trimmed = line.trim();
                !trimmed.starts_with("PRAGMA") && !trimmed.starts_with("-- Enable WAL")
            })
            .collect::<Vec<_>>()
            .join("\n");

        conn.execute_batch(&schema_without_pragma)?;

        // Run session management migration
        let schema_002 = r#"-- AvocadoDB Session Management Schema
-- Phase 2, Priority 1: Session tracking for conversation history and agent memory

-- Sessions table: tracks conversation sessions
CREATE TABLE IF NOT EXISTS sessions (
    id TEXT PRIMARY KEY,                      -- UUID v4
    user_id TEXT,                             -- Optional user identifier
    title TEXT,                               -- Optional session title (auto-generated or user-provided)
    metadata TEXT,                            -- JSON string with arbitrary metadata
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    last_message_at TIMESTAMP                 -- For sorting/filtering
);

-- Messages table: stores individual conversation turns
CREATE TABLE IF NOT EXISTS messages (
    id TEXT PRIMARY KEY,                      -- UUID v4
    session_id TEXT NOT NULL,                 -- Foreign key to sessions
    role TEXT NOT NULL,                       -- 'user', 'assistant', 'system', 'tool'
    content TEXT NOT NULL,                    -- Message content
    metadata TEXT,                            -- JSON string (tool calls, citations, etc.)
    sequence_number INTEGER NOT NULL,         -- Order within session (0-indexed)
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE
);

-- Working set associations: links compiled contexts to sessions
CREATE TABLE IF NOT EXISTS session_working_sets (
    id TEXT PRIMARY KEY,                      -- UUID v4
    session_id TEXT NOT NULL,                 -- Foreign key to sessions
    message_id TEXT,                          -- Optional: which message triggered this compilation
    working_set_id TEXT NOT NULL,             -- Reference to working set (stored as JSON for now)
    query TEXT NOT NULL,                      -- Query that generated this working set
    config TEXT,                              -- JSON string of CompilerConfig used
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE,
    FOREIGN KEY (message_id) REFERENCES messages(id) ON DELETE SET NULL
);

-- Indexes for performance
CREATE INDEX IF NOT EXISTS idx_sessions_user ON sessions(user_id);
CREATE INDEX IF NOT EXISTS idx_sessions_updated ON sessions(updated_at DESC);
CREATE INDEX IF NOT EXISTS idx_messages_session ON messages(session_id, sequence_number);
CREATE INDEX IF NOT EXISTS idx_working_sets_session ON session_working_sets(session_id);
"#;
        conn.execute_batch(schema_002)?;

        // Execute PRAGMAs separately (they return results)
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "foreign_keys", true)?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
            vector_index: Arc::new(RwLock::new(None)),
            index_dirty: Arc::new(AtomicBool::new(true)),
            db_path,
            build_lock: Arc::new(Mutex::new(())),
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
        Ok(self.get_vector_index_with_kind()?.0)
    }

    /// Get or build the cached vector index and return how it was obtained
    ///
    /// Returns the index and an indicator of whether it was loaded from cache
    /// or freshly built from spans.
    pub fn get_vector_index_with_kind(&self) -> Result<(Arc<VectorIndex>, IndexLoadKind)> {
        // Check if index needs rebuilding
        if self.index_dirty.load(Ordering::Acquire) {
            // Ensure only one thread builds/loads at a time for this database
            let _guard = self.build_lock.lock()
                .map_err(|e| crate::types::Error::Other(anyhow::anyhow!("Build lock poisoned: {}", e)))?;
            // Re-check after acquiring lock in case another thread already built it
            if !self.index_dirty.load(Ordering::Acquire) {
                let cached = self.vector_index.read()
                    .map_err(|e| crate::types::Error::Other(anyhow::anyhow!("Index lock poisoned: {}", e)))?;
                let idx = cached.as_ref()
                    .cloned()
                    .ok_or_else(|| crate::types::Error::Other(anyhow::anyhow!("Index cache empty after build")))?
                    ;
                return Ok((idx, IndexLoadKind::CachedInMemory));
            }
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
                return Ok((index, IndexLoadKind::LoadedFromCache));
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
            
            Ok((index, IndexLoadKind::BuiltFromSpans))
        } else {
            // Return cached index
            let cached = self.vector_index.read()
                .map_err(|e| crate::types::Error::Other(anyhow::anyhow!("Index lock poisoned: {}", e)))?;
            let idx = cached.as_ref()
                .cloned()
                .ok_or_else(|| crate::types::Error::Other(anyhow::anyhow!("Index cache is None but not dirty - this should not happen")))?;
            Ok((idx, IndexLoadKind::CachedInMemory))
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

    /// Get artifact by path
    ///
    /// Returns the artifact row matching the unique path, if present.
    pub fn get_artifact_by_path(&self, path: &str) -> Result<Option<Artifact>> {
        let conn = self.conn.lock()
            .map_err(|e| crate::types::Error::Other(anyhow::anyhow!("Database lock poisoned: {}", e)))?;
        let mut stmt = conn.prepare(
            "SELECT id, path, content, content_hash, metadata, created_at
             FROM artifacts WHERE path = ?1",
        )?;

        let artifact = stmt
            .query_row(params![path], |row| {
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

    /// Determine what action to take when ingesting a document
    ///
    /// Compares content hash to detect if document needs update or can be skipped.
    ///
    /// # Arguments
    ///
    /// * `path` - The document path
    /// * `content_hash` - SHA256 hash of the new content
    ///
    /// # Returns
    ///
    /// - `IngestAction::Skip` if document exists with same content hash
    /// - `IngestAction::Update` if document exists but content changed
    /// - `IngestAction::Create` if document doesn't exist
    pub fn determine_ingest_action(&self, path: &str, content_hash: &str) -> Result<crate::types::IngestAction> {
        match self.get_artifact_by_path(path)? {
            Some(existing) => {
                if existing.content_hash == content_hash {
                    Ok(crate::types::IngestAction::Skip {
                        artifact_id: existing.id,
                        reason: "Content unchanged (same hash)".to_string(),
                    })
                } else {
                    Ok(crate::types::IngestAction::Update {
                        artifact_id: existing.id,
                    })
                }
            }
            None => Ok(crate::types::IngestAction::Create),
        }
    }

    /// Delete an artifact and its spans
    ///
    /// # Arguments
    ///
    /// * `artifact_id` - The artifact ID to delete
    ///
    /// # Returns
    ///
    /// Number of spans deleted
    pub fn delete_artifact(&self, artifact_id: &str) -> Result<usize> {
        let conn = self.conn.lock()
            .map_err(|e| crate::types::Error::Other(anyhow::anyhow!("Database lock poisoned: {}", e)))?;

        // Delete spans first (FK constraint allows CASCADE but let's be explicit)
        let spans_deleted = conn.execute(
            "DELETE FROM spans WHERE artifact_id = ?1",
            params![artifact_id],
        )?;

        // Delete artifact
        conn.execute(
            "DELETE FROM artifacts WHERE id = ?1",
            params![artifact_id],
        )?;

        // Mark index as dirty
        self.index_dirty.store(true, std::sync::atomic::Ordering::Release);

        // Invalidate disk cache
        let cache_dir = self.db_path.with_extension("sqlite.idx");
        if cache_dir.exists() {
            let _ = std::fs::remove_dir_all(&cache_dir);
        }

        Ok(spans_deleted)
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

    // ========== Session Management Operations ==========

    /// Create a new session
    ///
    /// # Arguments
    ///
    /// * `user_id` - Optional user identifier
    /// * `title` - Optional session title
    ///
    /// # Returns
    ///
    /// The newly created session
    pub fn create_session(&self, user_id: Option<&str>, title: Option<&str>) -> Result<Session> {
        let conn = self.conn.lock()
            .map_err(|e| crate::types::Error::Other(anyhow::anyhow!("Database lock poisoned: {}", e)))?;

        let id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now();

        conn.execute(
            "INSERT INTO sessions (id, user_id, title, metadata, created_at, updated_at, last_message_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                id,
                user_id,
                title,
                None::<String>, // metadata
                now.to_rfc3339(),
                now.to_rfc3339(),
                None::<String>, // last_message_at
            ],
        )?;

        Ok(Session {
            id,
            user_id: user_id.map(|s| s.to_string()),
            title: title.map(|s| s.to_string()),
            metadata: None,
            created_at: now,
            updated_at: now,
            last_message_at: None,
        })
    }

    /// Get a session by ID
    ///
    /// # Arguments
    ///
    /// * `session_id` - The session ID to look up
    ///
    /// # Returns
    ///
    /// The session if found
    pub fn get_session(&self, session_id: &str) -> Result<Option<Session>> {
        let conn = self.conn.lock()
            .map_err(|e| crate::types::Error::Other(anyhow::anyhow!("Database lock poisoned: {}", e)))?;

        let mut stmt = conn.prepare(
            "SELECT id, user_id, title, metadata, created_at, updated_at, last_message_at
             FROM sessions WHERE id = ?1",
        )?;

        let session = stmt.query_row(params![session_id], |row| {
            Ok(Session {
                id: row.get(0)?,
                user_id: row.get(1)?,
                title: row.get(2)?,
                metadata: row.get::<_, Option<String>>(3)?
                    .and_then(|s| serde_json::from_str(&s).ok()),
                created_at: row.get::<_, String>(4)?
                    .parse()
                    .unwrap_or_else(|_| chrono::Utc::now()),
                updated_at: row.get::<_, String>(5)?
                    .parse()
                    .unwrap_or_else(|_| chrono::Utc::now()),
                last_message_at: row.get::<_, Option<String>>(6)?
                    .and_then(|s| s.parse().ok()),
            })
        }).optional()?;

        Ok(session)
    }

    /// List sessions for a user (or all sessions if user_id is None)
    ///
    /// # Arguments
    ///
    /// * `user_id` - Optional user ID to filter by
    /// * `limit` - Maximum number of sessions to return
    ///
    /// # Returns
    ///
    /// Vector of sessions, sorted by updated_at descending
    pub fn list_sessions(&self, user_id: Option<&str>, limit: Option<usize>) -> Result<Vec<Session>> {
        let conn = self.conn.lock()
            .map_err(|e| crate::types::Error::Other(anyhow::anyhow!("Database lock poisoned: {}", e)))?;

        let limit_val = limit.unwrap_or(100) as i64;

        let mut sessions = Vec::new();

        if let Some(uid) = user_id {
            let mut stmt = conn.prepare(
                "SELECT id, user_id, title, metadata, created_at, updated_at, last_message_at
                 FROM sessions WHERE user_id = ?1
                 ORDER BY updated_at DESC
                 LIMIT ?2"
            )?;

            let rows = stmt.query_map(params![uid, limit_val], |row| {
                Ok(Session {
                    id: row.get(0)?,
                    user_id: row.get(1)?,
                    title: row.get(2)?,
                    metadata: row.get::<_, Option<String>>(3)?
                        .and_then(|s| serde_json::from_str(&s).ok()),
                    created_at: row.get::<_, String>(4)?
                        .parse()
                        .unwrap_or_else(|_| chrono::Utc::now()),
                    updated_at: row.get::<_, String>(5)?
                        .parse()
                        .unwrap_or_else(|_| chrono::Utc::now()),
                    last_message_at: row.get::<_, Option<String>>(6)?
                        .and_then(|s| s.parse().ok()),
                })
            })?;

            for row in rows {
                sessions.push(row?);
            }
        } else {
            let mut stmt = conn.prepare(
                "SELECT id, user_id, title, metadata, created_at, updated_at, last_message_at
                 FROM sessions
                 ORDER BY updated_at DESC
                 LIMIT ?1"
            )?;

            let rows = stmt.query_map(params![limit_val], |row| {
                Ok(Session {
                    id: row.get(0)?,
                    user_id: row.get(1)?,
                    title: row.get(2)?,
                    metadata: row.get::<_, Option<String>>(3)?
                        .and_then(|s| serde_json::from_str(&s).ok()),
                    created_at: row.get::<_, String>(4)?
                        .parse()
                        .unwrap_or_else(|_| chrono::Utc::now()),
                    updated_at: row.get::<_, String>(5)?
                        .parse()
                        .unwrap_or_else(|_| chrono::Utc::now()),
                    last_message_at: row.get::<_, Option<String>>(6)?
                        .and_then(|s| s.parse().ok()),
                })
            })?;

            for row in rows {
                sessions.push(row?);
            }
        }

        Ok(sessions)
    }

    /// Update session metadata
    ///
    /// # Arguments
    ///
    /// * `session_id` - The session ID to update
    /// * `title` - Optional new title
    /// * `metadata` - Optional new metadata
    ///
    /// # Returns
    ///
    /// Ok(()) if successful
    pub fn update_session(
        &self,
        session_id: &str,
        title: Option<&str>,
        metadata: Option<&serde_json::Value>,
    ) -> Result<()> {
        let conn = self.conn.lock()
            .map_err(|e| crate::types::Error::Other(anyhow::anyhow!("Database lock poisoned: {}", e)))?;

        let now = chrono::Utc::now();

        conn.execute(
            "UPDATE sessions
             SET title = COALESCE(?1, title),
                 metadata = COALESCE(?2, metadata),
                 updated_at = ?3
             WHERE id = ?4",
            params![
                title,
                metadata.map(|m| m.to_string()),
                now.to_rfc3339(),
                session_id,
            ],
        )?;

        Ok(())
    }

    /// Delete a session (cascades to messages and working sets)
    ///
    /// # Arguments
    ///
    /// * `session_id` - The session ID to delete
    ///
    /// # Returns
    ///
    /// Ok(()) if successful
    pub fn delete_session(&self, session_id: &str) -> Result<()> {
        let conn = self.conn.lock()
            .map_err(|e| crate::types::Error::Other(anyhow::anyhow!("Database lock poisoned: {}", e)))?;

        conn.execute("DELETE FROM sessions WHERE id = ?1", params![session_id])?;

        Ok(())
    }

    /// Add a message to a session
    ///
    /// # Arguments
    ///
    /// * `session_id` - The session to add the message to
    /// * `role` - Message role (user, assistant, system, tool)
    /// * `content` - Message content
    /// * `metadata` - Optional metadata
    ///
    /// # Returns
    ///
    /// The newly created message
    pub fn add_message(
        &self,
        session_id: &str,
        role: MessageRole,
        content: &str,
        metadata: Option<&serde_json::Value>,
    ) -> Result<Message> {
        let mut conn = self.conn.lock()
            .map_err(|e| crate::types::Error::Other(anyhow::anyhow!("Database lock poisoned: {}", e)))?;

        let tx = conn.transaction()?;

        // Get the next sequence number for this session
        let sequence_number: i64 = tx.query_row(
            "SELECT COALESCE(MAX(sequence_number), -1) + 1 FROM messages WHERE session_id = ?1",
            params![session_id],
            |row| row.get(0),
        )?;

        let id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now();

        tx.execute(
            "INSERT INTO messages (id, session_id, role, content, metadata, sequence_number, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                id,
                session_id,
                role.as_str(),
                content,
                metadata.map(|m| m.to_string()),
                sequence_number,
                now.to_rfc3339(),
            ],
        )?;

        // Update session's last_message_at and updated_at
        tx.execute(
            "UPDATE sessions
             SET last_message_at = ?1, updated_at = ?1
             WHERE id = ?2",
            params![now.to_rfc3339(), session_id],
        )?;

        tx.commit()?;

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

    /// Get messages for a session
    ///
    /// # Arguments
    ///
    /// * `session_id` - The session ID
    /// * `limit` - Optional limit on number of messages (most recent first)
    ///
    /// # Returns
    ///
    /// Vector of messages in chronological order
    pub fn get_messages(&self, session_id: &str, limit: Option<usize>) -> Result<Vec<Message>> {
        let conn = self.conn.lock()
            .map_err(|e| crate::types::Error::Other(anyhow::anyhow!("Database lock poisoned: {}", e)))?;

        let mut messages = Vec::new();

        if let Some(lim) = limit {
            let mut stmt = conn.prepare(
                "SELECT id, session_id, role, content, metadata, sequence_number, created_at
                 FROM messages
                 WHERE session_id = ?1
                 ORDER BY sequence_number ASC
                 LIMIT ?2"
            )?;

            let rows = stmt.query_map(params![session_id, lim as i64], |row| {
                let role_str: String = row.get(2)?;
                let role = MessageRole::from_str(&role_str)
                    .unwrap_or(MessageRole::User);

                Ok(Message {
                    id: row.get(0)?,
                    session_id: row.get(1)?,
                    role,
                    content: row.get(3)?,
                    metadata: row.get::<_, Option<String>>(4)?
                        .and_then(|s| serde_json::from_str(&s).ok()),
                    sequence_number: row.get::<_, i64>(5)? as usize,
                    created_at: row.get::<_, String>(6)?
                        .parse()
                        .unwrap_or_else(|_| chrono::Utc::now()),
                })
            })?;

            for row in rows {
                messages.push(row?);
            }
        } else {
            let mut stmt = conn.prepare(
                "SELECT id, session_id, role, content, metadata, sequence_number, created_at
                 FROM messages
                 WHERE session_id = ?1
                 ORDER BY sequence_number ASC"
            )?;

            let rows = stmt.query_map(params![session_id], |row| {
                let role_str: String = row.get(2)?;
                let role = MessageRole::from_str(&role_str)
                    .unwrap_or(MessageRole::User);

                Ok(Message {
                    id: row.get(0)?,
                    session_id: row.get(1)?,
                    role,
                    content: row.get(3)?,
                    metadata: row.get::<_, Option<String>>(4)?
                        .and_then(|s| serde_json::from_str(&s).ok()),
                    sequence_number: row.get::<_, i64>(5)? as usize,
                    created_at: row.get::<_, String>(6)?
                        .parse()
                        .unwrap_or_else(|_| chrono::Utc::now()),
                })
            })?;

            for row in rows {
                messages.push(row?);
            }
        }

        Ok(messages)
    }

    /// Associate a working set with a session
    ///
    /// # Arguments
    ///
    /// * `session_id` - The session ID
    /// * `message_id` - Optional message ID that triggered this compilation
    /// * `working_set` - The working set to associate
    /// * `query` - Query that generated this working set
    /// * `config` - Configuration used for compilation
    ///
    /// # Returns
    ///
    /// The newly created SessionWorkingSet
    pub fn associate_working_set(
        &self,
        session_id: &str,
        message_id: Option<&str>,
        working_set: &WorkingSet,
        query: &str,
        config: &CompilerConfig,
    ) -> Result<SessionWorkingSet> {
        let conn = self.conn.lock()
            .map_err(|e| crate::types::Error::Other(anyhow::anyhow!("Database lock poisoned: {}", e)))?;

        let id = uuid::Uuid::new_v4().to_string();
        let working_set_id = uuid::Uuid::new_v4().to_string(); // Generate a unique ID for this working set
        let now = chrono::Utc::now();

        conn.execute(
            "INSERT INTO session_working_sets (id, session_id, message_id, working_set_id, query, config, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                id,
                session_id,
                message_id,
                working_set_id,
                query,
                serde_json::to_string(config)?,
                now.to_rfc3339(),
            ],
        )?;

        Ok(SessionWorkingSet {
            id,
            session_id: session_id.to_string(),
            message_id: message_id.map(|s| s.to_string()),
            working_set: working_set.clone(),
            query: query.to_string(),
            config: config.clone(),
            created_at: now,
        })
    }

    /// Get session with all messages and working sets
    ///
    /// # Arguments
    ///
    /// * `session_id` - The session ID
    ///
    /// # Returns
    ///
    /// SessionWithMessages if found
    pub fn get_session_full(&self, session_id: &str) -> Result<Option<SessionWithMessages>> {
        let session = self.get_session(session_id)?;

        if session.is_none() {
            return Ok(None);
        }

        let session = session.unwrap();
        let messages = self.get_messages(session_id, None)?;

        // Get working sets for this session
        let conn = self.conn.lock()
            .map_err(|e| crate::types::Error::Other(anyhow::anyhow!("Database lock poisoned: {}", e)))?;

        let mut stmt = conn.prepare(
            "SELECT id, session_id, message_id, working_set_id, query, config, created_at
             FROM session_working_sets
             WHERE session_id = ?1
             ORDER BY created_at ASC",
        )?;

        let working_sets = stmt.query_map(params![session_id], |row| {
            let config_str: String = row.get(5)?;
            let config: CompilerConfig = serde_json::from_str(&config_str)
                .unwrap_or_default();

            // Note: We can't reconstruct the full WorkingSet from storage without additional data
            // For now, we'll create a placeholder. In a real implementation, you'd store the
            // working set data as JSON and deserialize it here.
            let working_set = WorkingSet {
                text: String::new(),
                spans: Vec::new(),
                citations: Vec::new(),
                tokens_used: 0,
                query: row.get::<_, String>(4)?,
                compilation_time_ms: 0,
                manifest: None,
                explain: None,
            };

            Ok(SessionWorkingSet {
                id: row.get(0)?,
                session_id: row.get(1)?,
                message_id: row.get(2)?,
                working_set,
                query: row.get(4)?,
                config,
                created_at: row.get::<_, String>(6)?
                    .parse()
                    .unwrap_or_else(|_| chrono::Utc::now()),
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(Some(SessionWithMessages {
            session,
            messages,
            working_sets,
        }))
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

    // ========== Session Management Tests ==========

    #[test]
    fn test_create_session() {
        let db = Database::new(":memory:").unwrap();

        let session = db.create_session(Some("user123"), Some("Test Session")).unwrap();

        assert!(!session.id.is_empty());
        assert_eq!(session.user_id, Some("user123".to_string()));
        assert_eq!(session.title, Some("Test Session".to_string()));
        assert!(session.metadata.is_none());
        assert!(session.last_message_at.is_none());
    }

    #[test]
    fn test_get_session() {
        let db = Database::new(":memory:").unwrap();

        let created = db.create_session(Some("user456"), Some("Another Session")).unwrap();
        let retrieved = db.get_session(&created.id).unwrap();

        assert!(retrieved.is_some());
        let session = retrieved.unwrap();
        assert_eq!(session.id, created.id);
        assert_eq!(session.user_id, created.user_id);
        assert_eq!(session.title, created.title);
    }

    #[test]
    fn test_get_nonexistent_session() {
        let db = Database::new(":memory:").unwrap();

        let result = db.get_session("nonexistent-id").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_list_sessions() {
        let db = Database::new(":memory:").unwrap();

        // Create multiple sessions
        db.create_session(Some("user1"), Some("Session 1")).unwrap();
        db.create_session(Some("user1"), Some("Session 2")).unwrap();
        db.create_session(Some("user2"), Some("Session 3")).unwrap();

        // List all sessions
        let all_sessions = db.list_sessions(None, None).unwrap();
        assert_eq!(all_sessions.len(), 3);

        // List sessions for user1
        let user1_sessions = db.list_sessions(Some("user1"), None).unwrap();
        assert_eq!(user1_sessions.len(), 2);

        // List sessions for user2
        let user2_sessions = db.list_sessions(Some("user2"), None).unwrap();
        assert_eq!(user2_sessions.len(), 1);

        // Test limit
        let limited = db.list_sessions(None, Some(2)).unwrap();
        assert_eq!(limited.len(), 2);
    }

    #[test]
    fn test_update_session() {
        let db = Database::new(":memory:").unwrap();

        let session = db.create_session(Some("user1"), Some("Original Title")).unwrap();

        // Update title
        db.update_session(&session.id, Some("Updated Title"), None).unwrap();

        let updated = db.get_session(&session.id).unwrap().unwrap();
        assert_eq!(updated.title, Some("Updated Title".to_string()));

        // Update metadata
        let metadata = serde_json::json!({"key": "value"});
        db.update_session(&session.id, None, Some(&metadata)).unwrap();

        let updated2 = db.get_session(&session.id).unwrap().unwrap();
        assert!(updated2.metadata.is_some());
        assert_eq!(updated2.metadata.unwrap()["key"], "value");
    }

    #[test]
    fn test_delete_session() {
        let db = Database::new(":memory:").unwrap();

        let session = db.create_session(Some("user1"), Some("To Delete")).unwrap();

        // Verify session exists
        assert!(db.get_session(&session.id).unwrap().is_some());

        // Delete session
        db.delete_session(&session.id).unwrap();

        // Verify session is gone
        assert!(db.get_session(&session.id).unwrap().is_none());
    }

    #[test]
    fn test_add_message() {
        let db = Database::new(":memory:").unwrap();

        let session = db.create_session(Some("user1"), Some("Chat Session")).unwrap();

        // Add first message
        let msg1 = db.add_message(&session.id, MessageRole::User, "Hello", None).unwrap();
        assert_eq!(msg1.sequence_number, 0);
        assert_eq!(msg1.content, "Hello");
        assert_eq!(msg1.role.as_str(), "user");

        // Add second message
        let msg2 = db.add_message(&session.id, MessageRole::Assistant, "Hi there!", None).unwrap();
        assert_eq!(msg2.sequence_number, 1);
        assert_eq!(msg2.content, "Hi there!");
        assert_eq!(msg2.role.as_str(), "assistant");

        // Verify session was updated
        let updated_session = db.get_session(&session.id).unwrap().unwrap();
        assert!(updated_session.last_message_at.is_some());
    }

    #[test]
    fn test_add_message_with_metadata() {
        let db = Database::new(":memory:").unwrap();

        let session = db.create_session(Some("user1"), Some("Chat Session")).unwrap();

        let metadata = serde_json::json!({"tool": "search", "query": "test"});
        let msg = db.add_message(&session.id, MessageRole::Tool, "Result", Some(&metadata)).unwrap();

        assert!(msg.metadata.is_some());
        assert_eq!(msg.metadata.unwrap()["tool"], "search");
    }

    #[test]
    fn test_get_messages() {
        let db = Database::new(":memory:").unwrap();

        let session = db.create_session(Some("user1"), Some("Chat Session")).unwrap();

        // Add multiple messages
        db.add_message(&session.id, MessageRole::User, "Message 1", None).unwrap();
        db.add_message(&session.id, MessageRole::Assistant, "Message 2", None).unwrap();
        db.add_message(&session.id, MessageRole::User, "Message 3", None).unwrap();

        // Get all messages
        let messages = db.get_messages(&session.id, None).unwrap();
        assert_eq!(messages.len(), 3);
        assert_eq!(messages[0].sequence_number, 0);
        assert_eq!(messages[1].sequence_number, 1);
        assert_eq!(messages[2].sequence_number, 2);

        // Test limit
        let limited = db.get_messages(&session.id, Some(2)).unwrap();
        assert_eq!(limited.len(), 2);
    }

    #[test]
    fn test_message_ordering() {
        let db = Database::new(":memory:").unwrap();

        let session = db.create_session(Some("user1"), Some("Chat Session")).unwrap();

        // Add messages
        db.add_message(&session.id, MessageRole::User, "First", None).unwrap();
        db.add_message(&session.id, MessageRole::Assistant, "Second", None).unwrap();
        db.add_message(&session.id, MessageRole::User, "Third", None).unwrap();

        let messages = db.get_messages(&session.id, None).unwrap();

        // Verify chronological order
        assert_eq!(messages[0].content, "First");
        assert_eq!(messages[1].content, "Second");
        assert_eq!(messages[2].content, "Third");

        // Verify sequence numbers are consecutive
        for (i, msg) in messages.iter().enumerate() {
            assert_eq!(msg.sequence_number, i);
        }
    }

    #[test]
    fn test_associate_working_set() {
        let db = Database::new(":memory:").unwrap();

        let session = db.create_session(Some("user1"), Some("Chat Session")).unwrap();
        let message = db.add_message(&session.id, MessageRole::User, "Query", None).unwrap();

        // Create a working set
        let working_set = WorkingSet {
            text: "Test context".to_string(),
            spans: Vec::new(),
            citations: Vec::new(),
            tokens_used: 100,
            query: "test query".to_string(),
            compilation_time_ms: 50,
            manifest: None,
            explain: None,
        };

        let config = CompilerConfig::default();

        let sws = db.associate_working_set(
            &session.id,
            Some(&message.id),
            &working_set,
            "test query",
            &config,
        ).unwrap();

        assert_eq!(sws.session_id, session.id);
        assert_eq!(sws.message_id, Some(message.id));
        assert_eq!(sws.query, "test query");
        assert_eq!(sws.working_set.text, "Test context");
    }

    #[test]
    fn test_get_session_full() {
        let db = Database::new(":memory:").unwrap();

        let session = db.create_session(Some("user1"), Some("Full Session")).unwrap();

        // Add messages
        let msg1 = db.add_message(&session.id, MessageRole::User, "Hello", None).unwrap();
        db.add_message(&session.id, MessageRole::Assistant, "Hi!", None).unwrap();

        // Add working set
        let working_set = WorkingSet {
            text: "Context".to_string(),
            spans: Vec::new(),
            citations: Vec::new(),
            tokens_used: 50,
            query: "test".to_string(),
            compilation_time_ms: 25,
            manifest: None,
            explain: None,
        };

        db.associate_working_set(
            &session.id,
            Some(&msg1.id),
            &working_set,
            "test",
            &CompilerConfig::default(),
        ).unwrap();

        // Get full session
        let full = db.get_session_full(&session.id).unwrap();
        assert!(full.is_some());

        let swm = full.unwrap();
        assert_eq!(swm.session.id, session.id);
        assert_eq!(swm.messages.len(), 2);
        assert_eq!(swm.working_sets.len(), 1);
    }

    #[test]
    fn test_delete_session_cascade() {
        let db = Database::new(":memory:").unwrap();

        let session = db.create_session(Some("user1"), Some("To Delete")).unwrap();

        // Add messages
        db.add_message(&session.id, MessageRole::User, "Message 1", None).unwrap();
        db.add_message(&session.id, MessageRole::Assistant, "Message 2", None).unwrap();

        // Verify messages exist
        let messages_before = db.get_messages(&session.id, None).unwrap();
        assert_eq!(messages_before.len(), 2);

        // Delete session
        db.delete_session(&session.id).unwrap();

        // Verify messages are gone (cascade delete)
        let messages_after = db.get_messages(&session.id, None).unwrap();
        assert_eq!(messages_after.len(), 0);
    }

    #[test]
    fn test_message_role_conversion() {
        assert_eq!(MessageRole::User.as_str(), "user");
        assert_eq!(MessageRole::Assistant.as_str(), "assistant");
        assert_eq!(MessageRole::System.as_str(), "system");
        assert_eq!(MessageRole::Tool.as_str(), "tool");

        assert!(matches!(MessageRole::from_str("user").unwrap(), MessageRole::User));
        assert!(matches!(MessageRole::from_str("assistant").unwrap(), MessageRole::Assistant));
        assert!(matches!(MessageRole::from_str("system").unwrap(), MessageRole::System));
        assert!(matches!(MessageRole::from_str("tool").unwrap(), MessageRole::Tool));

        assert!(MessageRole::from_str("invalid").is_err());
    }
}
