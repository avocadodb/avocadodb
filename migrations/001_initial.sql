-- AvocadoDB Initial Schema
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
