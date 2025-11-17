# Session Management Technical Specification

**Phase 2 - Priority 1: Session Management**

## Overview

Session management enables AvocadoDB to track conversation history and maintain context across multiple interactions. This is the foundational layer that unlocks agent memory, debugging, and replay capabilities.

## Goals

1. **Track conversation state**: Store messages, queries, and working sets for each session
2. **Enable debugging**: Replay sessions to understand agent behavior
3. **Support memory extraction**: Provide session data as source for memory generation
4. **Maintain determinism**: Session storage should not break existing deterministic guarantees

## Architecture

### Data Model

```
SESSIONS
├── Session metadata (id, user_id, created_at, updated_at)
├── Messages (user queries, agent responses)
├── Working Sets (compiled contexts used)
└── Metadata (tags, labels, custom data)
```

### Database Schema

```sql
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
```

### Rust Types

```rust
// avocado-core/src/types.rs additions

/// A conversation session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    /// Unique identifier (UUID v4)
    pub id: String,
    /// Optional user identifier
    pub user_id: Option<String>,
    /// Optional session title
    pub title: Option<String>,
    /// Optional metadata (arbitrary JSON)
    pub metadata: Option<serde_json::Value>,
    /// When session was created
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// When session was last updated
    pub updated_at: chrono::DateTime<chrono::Utc>,
    /// When last message was added
    pub last_message_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// A message in a conversation session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// Unique identifier (UUID v4)
    pub id: String,
    /// Session this message belongs to
    pub session_id: String,
    /// Message role: 'user', 'assistant', 'system', 'tool'
    pub role: MessageRole,
    /// Message content
    pub content: String,
    /// Optional metadata (tool calls, citations, etc.)
    pub metadata: Option<serde_json::Value>,
    /// Sequence number within session (0-indexed)
    pub sequence_number: usize,
    /// When message was created
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    User,
    Assistant,
    System,
    Tool,
}

/// Association between a session and a working set
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionWorkingSet {
    /// Unique identifier (UUID v4)
    pub id: String,
    /// Session ID
    pub session_id: String,
    /// Optional message ID that triggered this compilation
    pub message_id: Option<String>,
    /// The working set (stored as JSON)
    pub working_set: WorkingSet,
    /// Query that generated this working set
    pub query: String,
    /// Configuration used for compilation
    pub config: CompilerConfig,
    /// When this was created
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Session with its messages (for retrieval)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionWithMessages {
    /// The session
    pub session: Session,
    /// Messages in chronological order
    pub messages: Vec<Message>,
    /// Associated working sets
    pub working_sets: Vec<SessionWorkingSet>,
}
```

## API Design

### Core Database Operations

```rust
// avocado-core/src/db.rs additions

impl Database {
    /// Create a new session
    pub fn create_session(&self, user_id: Option<&str>, title: Option<&str>) -> Result<Session>;
    
    /// Get a session by ID
    pub fn get_session(&self, session_id: &str) -> Result<Option<Session>>;
    
    /// List sessions for a user (or all sessions if user_id is None)
    pub fn list_sessions(&self, user_id: Option<&str>, limit: Option<usize>) -> Result<Vec<Session>>;
    
    /// Update session metadata
    pub fn update_session(&self, session_id: &str, title: Option<&str>, metadata: Option<&serde_json::Value>) -> Result<()>;
    
    /// Delete a session (cascades to messages and working sets)
    pub fn delete_session(&self, session_id: &str) -> Result<()>;
    
    /// Add a message to a session
    pub fn add_message(&self, session_id: &str, role: MessageRole, content: &str, metadata: Option<&serde_json::Value>) -> Result<Message>;
    
    /// Get messages for a session
    pub fn get_messages(&self, session_id: &str, limit: Option<usize>) -> Result<Vec<Message>>;
    
    /// Associate a working set with a session
    pub fn associate_working_set(&self, session_id: &str, message_id: Option<&str>, working_set: &WorkingSet, query: &str, config: &CompilerConfig) -> Result<SessionWorkingSet>;
    
    /// Get session with all messages and working sets
    pub fn get_session_full(&self, session_id: &str) -> Result<Option<SessionWithMessages>>;
}
```

### Session Manager (High-Level API)

```rust
// avocado-core/src/session.rs (new file)

/// High-level session management
pub struct SessionManager {
    db: Database,
}

impl SessionManager {
    pub fn new(db: Database) -> Self {
        Self { db }
    }
    
    /// Start a new session
    pub fn start_session(&self, user_id: Option<&str>) -> Result<Session>;
    
    /// Add a user message and compile context
    pub async fn add_user_message(
        &self,
        session_id: &str,
        query: &str,
        config: CompilerConfig,
        index: &VectorIndex,
        api_key: Option<&str>,
    ) -> Result<(Message, WorkingSet)>;
    
    /// Add an assistant response
    pub fn add_assistant_message(
        &self,
        session_id: &str,
        content: &str,
        metadata: Option<&serde_json::Value>,
    ) -> Result<Message>;
    
    /// Get conversation history (formatted for LLM)
    pub fn get_conversation_history(&self, session_id: &str, max_tokens: Option<usize>) -> Result<String>;
    
    /// Replay a session (for debugging)
    pub fn replay_session(&self, session_id: &str) -> Result<SessionReplay>;
}

/// Replay data for debugging
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionReplay {
    pub session: Session,
    pub turns: Vec<SessionTurn>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionTurn {
    pub user_message: Message,
    pub working_set: Option<WorkingSet>,
    pub assistant_message: Option<Message>,
}
```

## HTTP API Extensions

```rust
// avocado-server/src/main.rs additions

// POST /sessions
// Create a new session
// Request: { user_id?: string, title?: string }
// Response: { session: Session }

// GET /sessions
// List sessions
// Query params: user_id?, limit?
// Response: { sessions: Session[] }

// GET /sessions/:id
// Get session with messages
// Response: { session: SessionWithMessages }

// POST /sessions/:id/messages
// Add a message to session
// Request: { role: "user" | "assistant" | "system" | "tool", content: string, metadata?: object }
// Response: { message: Message }

// POST /sessions/:id/compile
// Compile context for a query in this session
// Request: { query: string, config?: CompilerConfig }
// Response: { message: Message, working_set: WorkingSet }

// GET /sessions/:id/history
// Get formatted conversation history
// Query params: max_tokens?
// Response: { history: string }

// DELETE /sessions/:id
// Delete a session
// Response: { success: true }
```

## Python SDK Extensions

```python
# sdks/python/avocado/session.py (new file)

from avocado import AvocadoDB

class Session:
    """Represents a conversation session"""
    
    def __init__(self, client: AvocadoDB, session_id: str):
        self.client = client
        self.session_id = session_id
    
    def add_message(self, role: str, content: str, metadata: dict = None) -> dict:
        """Add a message to the session"""
        pass
    
    def compile(self, query: str, budget: int = 8000, **kwargs) -> dict:
        """Compile context for a query in this session"""
        pass
    
    def get_history(self, max_tokens: int = None) -> str:
        """Get formatted conversation history"""
        pass
    
    def replay(self) -> dict:
        """Get full session replay for debugging"""
        pass

class AvocadoDB:
    # ... existing methods ...
    
    def create_session(self, user_id: str = None, title: str = None) -> Session:
        """Create a new session"""
        pass
    
    def get_session(self, session_id: str) -> Session:
        """Get an existing session"""
        pass
    
    def list_sessions(self, user_id: str = None, limit: int = None) -> list:
        """List sessions"""
        pass
```

## CLI Extensions

```bash
# avocado-cli/src/main.rs additions

# Create a new session
avocado session create [--user-id <id>] [--title <title>]

# List sessions
avocado session list [--user-id <id>] [--limit <n>]

# Show session details
avocado session show <session-id>

# Add message to session
avocado session message <session-id> --role <role> --content <text>

# Compile query in session context
avocado session compile <session-id> <query> [--budget <tokens>]

# Get conversation history
avocado session history <session-id> [--max-tokens <n>]

# Replay session (for debugging)
avocado session replay <session-id>

# Delete session
avocado session delete <session-id>
```

## Implementation Plan

### Week 1: Database Schema & Core Types
- [ ] Create migration `002_sessions.sql`
- [ ] Add `Session`, `Message`, `SessionWorkingSet` types to `types.rs`
- [ ] Add `MessageRole` enum
- [ ] Add `SessionWithMessages` type
- [ ] Update `Database::new()` to run new migration

### Week 2: Database Operations
- [ ] Implement `Database::create_session()`
- [ ] Implement `Database::get_session()`
- [ ] Implement `Database::list_sessions()`
- [ ] Implement `Database::update_session()`
- [ ] Implement `Database::delete_session()`
- [ ] Implement `Database::add_message()`
- [ ] Implement `Database::get_messages()`
- [ ] Implement `Database::associate_working_set()`
- [ ] Implement `Database::get_session_full()`
- [ ] Write unit tests for all operations

### Week 3: Session Manager
- [ ] Create `session.rs` module
- [ ] Implement `SessionManager::new()`
- [ ] Implement `SessionManager::start_session()`
- [ ] Implement `SessionManager::add_user_message()` (integrates with compiler)
- [ ] Implement `SessionManager::add_assistant_message()`
- [ ] Implement `SessionManager::get_conversation_history()` (with token limiting)
- [ ] Implement `SessionManager::replay_session()`
- [ ] Write integration tests

### Week 4: HTTP API & SDKs
- [ ] Add session endpoints to HTTP server
- [ ] Update Python SDK with session support
- [ ] Add CLI commands for session management
- [ ] Write example scripts
- [ ] Update documentation

## Design Decisions

### 1. Working Set Storage
**Decision**: Store working sets as JSON in `session_working_sets` table
**Rationale**: 
- Simpler than creating separate tables for all working set components
- Working sets are immutable once created
- JSON allows flexible querying and retrieval
- Can optimize later if needed

### 2. Message Sequence Numbers
**Decision**: Use explicit `sequence_number` field instead of relying on timestamps
**Rationale**:
- Ensures deterministic ordering
- Handles clock skew issues
- Makes replay logic simpler

### 3. Session Isolation
**Decision**: Sessions are independent - no cross-session queries by default
**Rationale**:
- Simpler mental model
- Better for debugging
- Can add cross-session features later if needed

### 4. Token Limiting in History
**Decision**: Implement token-based truncation in `get_conversation_history()`
**Rationale**:
- Prevents context window overflow
- Keeps most recent messages (most relevant)
- Can be configured per call

### 5. Optional User IDs
**Decision**: `user_id` is optional in sessions
**Rationale**:
- Supports anonymous sessions
- Allows multi-user scenarios
- Can add authentication layer later

## Testing Strategy

### Unit Tests
- Database operations (CRUD for sessions, messages, working sets)
- Session manager logic
- Token limiting in conversation history

### Integration Tests
- Full session lifecycle (create → add messages → compile → replay)
- Session replay accuracy
- Determinism preservation (sessions don't break existing guarantees)

### Performance Tests
- Session creation/retrieval latency
- Message insertion performance
- History retrieval with large sessions

## Success Criteria

1. ✅ Can create and manage sessions via CLI, HTTP API, and SDKs
2. ✅ Messages are stored and retrieved in correct order
3. ✅ Working sets are associated with sessions and messages
4. ✅ Conversation history can be retrieved with token limits
5. ✅ Session replay works for debugging
6. ✅ Existing deterministic compilation still works
7. ✅ Performance: < 10ms for session operations, < 50ms for history retrieval

## Future Enhancements (Not in Phase 2)

- Session summarization (auto-generate titles)
- Cross-session search
- Session templates
- Session sharing/collaboration
- Advanced filtering and search
- Session analytics

## Migration Path

1. Existing users: No breaking changes - sessions are opt-in
2. Migration script: `avocado migrate` command to add session tables
3. Backward compatibility: All existing APIs continue to work without sessions

---

**Next Steps After Session Management**: Memory Extraction Pipeline (Phase 2, Priority 2)


