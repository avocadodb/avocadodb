-- AvocadoDB Session Management Schema
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
