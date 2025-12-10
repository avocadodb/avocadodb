-- AvocadoDB PostgreSQL Extension Schema
-- Requires: pgvector extension

CREATE SCHEMA IF NOT EXISTS avocado;

-- Artifacts table: stores ingested documents
CREATE TABLE IF NOT EXISTS avocado.artifacts (
    id TEXT PRIMARY KEY,
    path TEXT NOT NULL UNIQUE,
    content TEXT NOT NULL,
    content_hash TEXT NOT NULL,
    metadata JSONB DEFAULT '{}',
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_artifacts_path ON avocado.artifacts(path);
CREATE INDEX IF NOT EXISTS idx_artifacts_hash ON avocado.artifacts(content_hash);

-- Configuration table: tracks embedding provider settings
CREATE TABLE IF NOT EXISTS avocado.config (
    key TEXT PRIMARY KEY,
    value JSONB NOT NULL,
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- Insert default configuration
INSERT INTO avocado.config (key, value) VALUES
    ('embedding', '{"provider": "fastembed", "model": "all-MiniLM-L6-v2", "dimension": 384}'::jsonb)
ON CONFLICT (key) DO NOTHING;

-- Spans table: document chunks with embeddings
-- Note: Using 1024 dimension to support larger models like bge-m3
-- Smaller embeddings (384 for fastembed) are zero-padded automatically by pgvector
CREATE TABLE IF NOT EXISTS avocado.spans (
    id TEXT PRIMARY KEY,
    artifact_id TEXT NOT NULL REFERENCES avocado.artifacts(id) ON DELETE CASCADE,
    start_line INTEGER NOT NULL,
    end_line INTEGER NOT NULL,
    text TEXT NOT NULL,
    embedding vector(1024),  -- Max dimension for common models (bge-m3, nomic, mxbai)
    embedding_model TEXT,
    token_count INTEGER,
    metadata JSONB DEFAULT '{}',
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_spans_artifact ON avocado.spans(artifact_id);

-- HNSW index for fast approximate nearest neighbor search
CREATE INDEX IF NOT EXISTS idx_spans_embedding ON avocado.spans
    USING hnsw (embedding vector_cosine_ops)
    WITH (m = 32, ef_construction = 200);

-- Sessions table: conversation sessions
CREATE TABLE IF NOT EXISTS avocado.sessions (
    id TEXT PRIMARY KEY,
    user_id TEXT,
    title TEXT,
    metadata JSONB DEFAULT '{}',
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    last_message_at TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_sessions_user ON avocado.sessions(user_id);
CREATE INDEX IF NOT EXISTS idx_sessions_created ON avocado.sessions(created_at DESC);

-- Messages table: conversation messages
CREATE TABLE IF NOT EXISTS avocado.messages (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL REFERENCES avocado.sessions(id) ON DELETE CASCADE,
    role TEXT NOT NULL CHECK (role IN ('user', 'assistant', 'system', 'tool')),
    content TEXT NOT NULL,
    metadata JSONB DEFAULT '{}',
    sequence_number INTEGER NOT NULL,
    token_count INTEGER,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_messages_session ON avocado.messages(session_id, sequence_number);

-- Working sets table: compiled context snapshots
CREATE TABLE IF NOT EXISTS avocado.working_sets (
    id TEXT PRIMARY KEY,
    session_id TEXT REFERENCES avocado.sessions(id) ON DELETE CASCADE,
    message_id TEXT REFERENCES avocado.messages(id) ON DELETE CASCADE,
    query TEXT NOT NULL,
    context_hash TEXT NOT NULL,
    tokens_used INTEGER NOT NULL,
    span_ids TEXT[] NOT NULL,
    metadata JSONB DEFAULT '{}',
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_working_sets_session ON avocado.working_sets(session_id);
CREATE INDEX IF NOT EXISTS idx_working_sets_hash ON avocado.working_sets(context_hash);

-- Agents table: registered AI agents
CREATE TABLE IF NOT EXISTS avocado.agents (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    role TEXT NOT NULL,
    model TEXT NOT NULL,
    system_prompt TEXT,
    capabilities JSONB DEFAULT '[]',
    metadata JSONB DEFAULT '{}',
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_agents_name ON avocado.agents(name);

-- Agent relations table: tracks agreements/disagreements between agents
CREATE TABLE IF NOT EXISTS avocado.agent_relations (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL REFERENCES avocado.sessions(id) ON DELETE CASCADE,
    message_id TEXT NOT NULL REFERENCES avocado.messages(id) ON DELETE CASCADE,
    from_agent_id TEXT NOT NULL REFERENCES avocado.agents(id),
    target_message_id TEXT REFERENCES avocado.messages(id),
    stance TEXT NOT NULL CHECK (stance IN ('agree', 'disagree', 'neutral', 'question')),
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_agent_relations_session ON avocado.agent_relations(session_id);
CREATE INDEX IF NOT EXISTS idx_agent_relations_from ON avocado.agent_relations(from_agent_id);
CREATE INDEX IF NOT EXISTS idx_agent_relations_stance ON avocado.agent_relations(stance);

-- Update timestamp trigger
CREATE OR REPLACE FUNCTION avocado.update_timestamp()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Apply update trigger to artifacts
DROP TRIGGER IF EXISTS artifacts_update_timestamp ON avocado.artifacts;
CREATE TRIGGER artifacts_update_timestamp
    BEFORE UPDATE ON avocado.artifacts
    FOR EACH ROW EXECUTE FUNCTION avocado.update_timestamp();

-- Apply update trigger to sessions
DROP TRIGGER IF EXISTS sessions_update_timestamp ON avocado.sessions;
CREATE TRIGGER sessions_update_timestamp
    BEFORE UPDATE ON avocado.sessions
    FOR EACH ROW EXECUTE FUNCTION avocado.update_timestamp();

-- Update last_message_at when message is added
CREATE OR REPLACE FUNCTION avocado.update_session_last_message()
RETURNS TRIGGER AS $$
BEGIN
    UPDATE avocado.sessions
    SET last_message_at = NOW()
    WHERE id = NEW.session_id;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS messages_update_session ON avocado.messages;
CREATE TRIGGER messages_update_session
    AFTER INSERT ON avocado.messages
    FOR EACH ROW EXECUTE FUNCTION avocado.update_session_last_message();
