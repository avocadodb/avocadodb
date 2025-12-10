//! SQLite schema migrations
//!
//! These are the same schemas used in db.rs, extracted for clarity.

/// Phase 1 schema: artifacts and spans
pub const SCHEMA_001_ARTIFACTS_SPANS: &str = r#"
-- Artifacts table: stores document metadata
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
    embedding BLOB,                           -- Serialized f32 vector (384 dims for MiniLM)
    embedding_model TEXT,                     -- e.g., "all-MiniLM-L6-v2"
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
"#;

/// Phase 2 schema: sessions, messages, working sets, agents
pub const SCHEMA_002_SESSIONS_AGENTS: &str = r#"
-- Sessions table: stores conversation sessions
CREATE TABLE IF NOT EXISTS sessions (
    id TEXT PRIMARY KEY,                      -- UUID v4
    user_id TEXT,                             -- Optional user identifier
    title TEXT,                               -- Optional session title
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

-- Working sets table: stores compiled context
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

-- Agents table: stores registered agents for multi-agent orchestration
CREATE TABLE IF NOT EXISTS agents (
    id TEXT PRIMARY KEY,                      -- UUID v4
    name TEXT NOT NULL,                       -- Human-readable name (e.g., "moderator")
    role TEXT NOT NULL,                       -- Agent's role/persona description
    model TEXT NOT NULL,                      -- LLM model identifier
    system_prompt TEXT,                       -- Optional system prompt / personality
    did TEXT,                                 -- Optional DID for decentralized identity
    capabilities TEXT,                        -- JSON array of capabilities
    metadata TEXT,                            -- JSON string with arbitrary metadata
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Agent relations table: tracks agreements/disagreements between agents
CREATE TABLE IF NOT EXISTS agent_relations (
    id TEXT PRIMARY KEY,                      -- UUID v4
    session_id TEXT NOT NULL,                 -- Session where this occurred
    message_id TEXT NOT NULL,                 -- Message that created this relation
    from_agent_id TEXT NOT NULL,              -- Agent who expressed the stance
    to_agent_id TEXT NOT NULL,                -- Agent being referenced
    stance TEXT NOT NULL,                     -- 'agree', 'disagree', 'neutral', 'question'
    target_message_id TEXT NOT NULL,          -- Message being referenced
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE,
    FOREIGN KEY (message_id) REFERENCES messages(id) ON DELETE CASCADE,
    FOREIGN KEY (from_agent_id) REFERENCES agents(id) ON DELETE CASCADE,
    FOREIGN KEY (to_agent_id) REFERENCES agents(id) ON DELETE CASCADE
);

-- Indexes for performance
CREATE INDEX IF NOT EXISTS idx_sessions_user ON sessions(user_id);
CREATE INDEX IF NOT EXISTS idx_sessions_updated ON sessions(updated_at DESC);
CREATE INDEX IF NOT EXISTS idx_messages_session ON messages(session_id, sequence_number);
CREATE INDEX IF NOT EXISTS idx_working_sets_session ON session_working_sets(session_id);
CREATE INDEX IF NOT EXISTS idx_agents_name ON agents(name);
CREATE INDEX IF NOT EXISTS idx_agent_relations_session ON agent_relations(session_id);
CREATE INDEX IF NOT EXISTS idx_agent_relations_from ON agent_relations(from_agent_id);
CREATE INDEX IF NOT EXISTS idx_agent_relations_to ON agent_relations(to_agent_id);
"#;
