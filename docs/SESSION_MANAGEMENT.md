# Session Management

**Status**: ✅ Phase 4 Complete (SDK & CLI Support)

Session management enables AvocadoDB to track conversation history and maintain context across multiple interactions. This is the foundational layer for agent memory, debugging, and replay capabilities.

## Features

- **Multi-turn conversations**: Track user queries and agent responses
- **Context compilation**: Automatically compile context for each query
- **Conversation history**: Retrieve formatted conversation history with token limiting
- **Session replay**: Debug agent behavior by replaying entire sessions
- **Session metadata**: Tag sessions with user IDs and titles
- **Database persistence**: Sessions stored in SQLite with full ACID guarantees

## Architecture

### Data Model

```
SESSIONS
├── Session metadata (id, user_id, created_at, updated_at)
├── Messages (user queries, agent responses)
├── Working Sets (compiled contexts used)
└── Metadata (tags, labels, custom data)
```

### Components

1. **Core Layer** (`avocado-core/src/session.rs`)
   - `SessionManager`: High-level session management API
   - Database operations (CRUD for sessions and messages)
   - Token-based history limiting
   - Session replay logic

2. **HTTP API** (`avocado-server/src/main.rs`)
   - RESTful endpoints for session management
   - Integrated with project management
   - Full session lifecycle support

3. **Python SDK** (`sdks/python/avocado/session.py`)
   - `Session` class with Pythonic API
   - Auto-managed HTTP requests
   - Rich examples and documentation

4. **CLI** (`avocado-cli/src/commands/session.rs`)
   - Full command suite for session management
   - Beautiful terminal output with colors
   - Interactive confirmations

## Usage

### Python SDK

#### Basic Usage

```python
from avocado import AvocadoDB

# Initialize client (HTTP mode required for sessions)
db = AvocadoDB(mode="http")

# Create a new session
session = db.create_session(
    user_id="alice",
    title="Learning about Rust"
)

# First query - compile context and add user message
result = session.compile("What is Rust?", budget=8000)
print(result['working_set']['text'])

# Add assistant response
session.add_message(
    "assistant",
    "Rust is a systems programming language..."
)

# Continue conversation
result2 = session.compile("Tell me about ownership")

# Get conversation history
history = session.get_history()
print(history)
```

#### Session Replay (Debugging)

```python
# Replay session to understand agent behavior
replay = session.replay()

for turn in replay['turns']:
    print(f"User: {turn['user_message']['content']}")

    if turn.get('working_set'):
        ws = turn['working_set']
        print(f"Context: {ws['tokens_used']} tokens")

    if turn.get('assistant_message'):
        print(f"Assistant: {turn['assistant_message']['content']}")
```

#### Managing Sessions

```python
# List all sessions
sessions = db.list_sessions(user_id="alice", limit=10)

# Get existing session
session = db.get_session("session-id")

# Delete session
session.delete()
```

### CLI

#### Create a Session

```bash
avocado session create --user-id alice --title "Q&A about Rust"
# Output:
# ✓ Created session: 476b63b0-88ea-4ae0-a512-a98565dd6e5b
#   User: alice
#   Title: Q&A about Rust
#   Created: 2025-11-17 19:28:12
```

#### List Sessions

```bash
avocado session list --user-id alice
# Output:
# Found 1 sessions
# ────────────────────────────────────────────────────────────────────────────────
#
# Session: 476b63b0-88ea-4ae0-a512-a98565dd6e5b
#   User: alice
#   Title: Q&A about Rust
#   Created: 2025-11-17 19:28:12
```

#### Add Messages

```bash
# Add user message
avocado session message <session-id> --role user --content "What is Rust?"

# Add assistant response
avocado session message <session-id> --role assistant --content "Rust is..."
```

#### Compile in Session Context

```bash
avocado session compile <session-id> "What is ownership?" --budget 8000
# Output:
# 🥑 Compiling context for session <session-id>
# Query: What is ownership?
#
# ✓ Compilation complete
#   Message ID: abc123
#   Tokens: 7856 / 8000
#   Spans: 12
#   Time: 45ms
#
# Context Preview:
# ────────────────────────────────────────────────────────────────────────────────
# [context text...]
```

#### Get Conversation History

```bash
avocado session history <session-id>
# Output:
# Conversation History (session: <session-id>)
# ════════════════════════════════════════════════════════════════════════════════
# User: What is Rust?
#
# Assistant: Rust is a systems programming language...
#
# User: What is ownership?
# ════════════════════════════════════════════════════════════════════════════════

# With token limit
avocado session history <session-id> --max-tokens 1000
```

#### Replay Session (Debugging)

```bash
avocado session replay <session-id>
# Output:
# Session Replay 476b63b0-88ea-4ae0-a512-a98565dd6e5b
# ════════════════════════════════════════════════════════════════════════════════
# Title: Q&A about Rust
# User: alice
# Created: 2025-11-17 19:28:12
# Turns: 2
#
# Conversation:
# ────────────────────────────────────────────────────────────────────────────────
#
# Turn 1 ──────────────────────────────────────────────────────────────────────
#
# User: What is Rust?
#
# Context: 7856 tokens, 12 spans, 45ms
#   Top citations:
#     • docs/rust_intro.md:1-10
#     • docs/rust_features.md:5-15
#
# Assistant: Rust is a systems programming language...
#
# Turn 2 ──────────────────────────────────────────────────────────────────────
# [...]
```

#### Show Session Details

```bash
avocado session show <session-id>
```

#### Delete Session

```bash
avocado session delete <session-id>
# Confirmation prompt:
# Delete session <session-id> and all messages? (y/N): y
# ✓ Deleted session <session-id>

# Skip confirmation
avocado session delete <session-id> --yes
```

### HTTP API

#### Create Session

```bash
POST /sessions
Content-Type: application/json

{
  "user_id": "alice",
  "title": "Q&A about Rust",
  "project": "/path/to/project"
}

# Response:
{
  "session": {
    "id": "476b63b0-88ea-4ae0-a512-a98565dd6e5b",
    "user_id": "alice",
    "title": "Q&A about Rust",
    "created_at": "2025-11-17T19:28:12Z",
    "updated_at": "2025-11-17T19:28:12Z"
  }
}
```

#### List Sessions

```bash
GET /sessions?user_id=alice&limit=10&project=/path/to/project

# Response:
{
  "sessions": [
    {
      "id": "476b63b0-88ea-4ae0-a512-a98565dd6e5b",
      "user_id": "alice",
      "title": "Q&A about Rust",
      "created_at": "2025-11-17T19:28:12Z",
      "updated_at": "2025-11-17T19:28:12Z"
    }
  ]
}
```

#### Get Session

```bash
GET /sessions/:id?project=/path/to/project

# Response:
{
  "session": { /* session data */ },
  "messages": [
    {
      "id": "msg-1",
      "session_id": "476b63b0-88ea-4ae0-a512-a98565dd6e5b",
      "role": "user",
      "content": "What is Rust?",
      "sequence_number": 0,
      "created_at": "2025-11-17T19:28:15Z"
    }
  ]
}
```

#### Add Message

```bash
POST /sessions/:id/messages
Content-Type: application/json

{
  "role": "user",
  "content": "What is Rust?",
  "project": "/path/to/project"
}

# Response:
{
  "message": {
    "id": "msg-1",
    "session_id": "476b63b0-88ea-4ae0-a512-a98565dd6e5b",
    "role": "user",
    "content": "What is Rust?",
    "sequence_number": 0,
    "created_at": "2025-11-17T19:28:15Z"
  }
}
```

#### Compile in Session

```bash
POST /sessions/:id/compile
Content-Type: application/json

{
  "query": "What is ownership?",
  "token_budget": 8000,
  "project": "/path/to/project"
}

# Response:
{
  "message": { /* user message */ },
  "working_set": { /* compiled context */ }
}
```

#### Get Conversation History

```bash
GET /sessions/:id/history?max_tokens=1000&project=/path/to/project

# Response:
{
  "history": "User: What is Rust?\n\nAssistant: Rust is..."
}
```

#### Replay Session

```bash
GET /sessions/:id/replay?project=/path/to/project

# Response:
{
  "session": { /* session data */ },
  "turns": [
    {
      "user_message": { /* message */ },
      "working_set": { /* compiled context */ },
      "assistant_message": { /* message */ }
    }
  ]
}
```

#### Delete Session

```bash
DELETE /sessions/:id?project=/path/to/project

# Response:
{
  "success": true
}
```

## Database Schema

See [session-management-spec.md](./session-management-spec.md) for complete schema details.

Key tables:
- `sessions`: Session metadata
- `messages`: Conversation messages
- `session_working_sets`: Compiled contexts

## Examples

### Python Examples

1. **Basic Session Usage** (`sdks/python/examples/session_example.py`)
   - Creating sessions
   - Multi-turn conversations
   - History retrieval
   - Session management

2. **Session Replay** (`sdks/python/examples/session_replay_example.py`)
   - Debugging agent behavior
   - Analyzing context retrieval
   - JSON export for analysis

### CLI Examples

All CLI commands have built-in help:

```bash
avocado session --help
avocado session create --help
avocado session compile --help
```

## Design Decisions

### 1. Token Limiting in History

Conversation history can be limited by tokens to prevent context window overflow. The algorithm keeps the most recent messages (which are typically most relevant).

```python
# Get last ~1000 tokens of conversation
history = session.get_history(max_tokens=1000)
```

### 2. Message Sequence Numbers

Messages have explicit `sequence_number` fields (0-indexed) to ensure deterministic ordering, independent of timestamps or clock skew.

### 3. Working Set Association

Working sets are associated with both sessions and the specific message that triggered compilation. This enables:
- Replay of exact context used for each query
- Debugging why certain results were returned
- Analysis of context quality over time

### 4. HTTP Mode Only (Python SDK)

Sessions require HTTP mode because:
- Session state must persist across CLI calls
- Server manages sessions in database
- CLI mode is stateless by design

## Performance

- **Session creation**: < 10ms
- **Message insertion**: < 5ms
- **History retrieval**: < 50ms (even with large sessions)
- **Replay**: < 100ms (depends on session size)

## Testing

The implementation includes comprehensive tests in:
- `avocado-core/src/session.rs`: Unit and integration tests
- `tests/correctness.rs`: Session correctness tests
- Manual testing via CLI and Python SDK

All tests pass successfully.

## Future Enhancements

Potential improvements (not in current phase):

- Session summarization (auto-generate titles)
- Cross-session search
- Session templates
- Session sharing/collaboration
- Advanced filtering and analytics
- Session export/import

## Migration

Existing AvocadoDB users:
- **No breaking changes**: Sessions are opt-in
- **Backward compatible**: All existing APIs work without sessions
- **Database migration**: Automatic on first use (adds session tables)

## Troubleshooting

### "Session management not available in CLI mode"

Sessions require HTTP server mode. Start the server:

```bash
# Terminal 1: Start server
cargo run --bin avocado-server

# Terminal 2: Use Python SDK in HTTP mode
python your_script.py
```

Or use the CLI with a database path for direct session access.

### "Session not found"

Verify the session exists:

```bash
avocado session list --db-path .avocado/db.sqlite
```

### Performance Issues

For large sessions (>1000 messages):
- Use token limiting in `get_history()`
- Consider session archiving
- Monitor database size

## Contributing

To extend session management:

1. Core functionality: `avocado-core/src/session.rs`
2. Database operations: `avocado-core/src/db.rs`
3. HTTP API: `avocado-server/src/main.rs`
4. Python SDK: `sdks/python/avocado/session.py`
5. CLI: `avocado-cli/src/commands/session.rs`

## References

- [Session Management Specification](./session-management-spec.md)
- [Python SDK Documentation](../sdks/python/README.md)
- [CLI Documentation](../avocado-cli/README.md)
- [HTTP API Documentation](../avocado-server/README.md)
