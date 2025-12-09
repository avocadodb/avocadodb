# Session Management HTTP API Guide

**Phase 3 Implementation - COMPLETE**

This guide documents the session management HTTP API endpoints for AvocadoDB. All endpoints follow REST conventions and return JSON responses.

## Overview

AvocadoDB now supports session management, enabling:
- Multi-turn conversations with context tracking
- Message persistence and retrieval
- Conversation history formatting for LLM consumption
- Session replay for debugging
- Working set association with user queries

## Base URL

```
http://localhost:8765
```

## Endpoints

### 1. Create Session

Create a new conversation session.

**Endpoint:** `POST /sessions`

**Request Body:**
```json
{
  "user_id": "alice",          // Optional: user identifier
  "title": "Learning Rust",    // Optional: session title
  "project": "/path/to/project" // Optional: project path
}
```

**Response:**
```json
{
  "session": {
    "id": "209e4078-a87f-4d96-8e3a-efe170bc7b8c",
    "user_id": "alice",
    "title": "Learning Rust",
    "metadata": null,
    "created_at": "2025-11-17T19:28:34.235182Z",
    "updated_at": "2025-11-17T19:28:34.235320Z",
    "last_message_at": null
  }
}
```

**Example:**
```bash
curl -X POST http://localhost:8765/sessions \
  -H "Content-Type: application/json" \
  -d '{"user_id": "alice", "title": "Test Session"}'
```

---

### 2. List Sessions

Retrieve all sessions, optionally filtered by user ID.

**Endpoint:** `GET /sessions`

**Query Parameters:**
- `user_id` (optional): Filter by user ID
- `limit` (optional): Maximum number of sessions to return
- `project` (optional): Project path

**Response:**
```json
{
  "sessions": [
    {
      "id": "209e4078-a87f-4d96-8e3a-efe170bc7b8c",
      "user_id": "alice",
      "title": "Learning Rust",
      "metadata": null,
      "created_at": "2025-11-17T19:28:34.235182Z",
      "updated_at": "2025-11-17T19:28:34.235320Z",
      "last_message_at": "2025-11-17T19:30:15.123456Z"
    }
  ]
}
```

**Examples:**
```bash
# List all sessions
curl http://localhost:8765/sessions

# List sessions for a specific user
curl "http://localhost:8765/sessions?user_id=alice"

# List with limit
curl "http://localhost:8765/sessions?limit=10"
```

---

### 3. Get Session

Retrieve a specific session with all its messages.

**Endpoint:** `GET /sessions/:id`

**Query Parameters:**
- `project` (optional): Project path

**Response:**
```json
{
  "session": {
    "id": "209e4078-a87f-4d96-8e3a-efe170bc7b8c",
    "user_id": "alice",
    "title": "Learning Rust",
    "metadata": null,
    "created_at": "2025-11-17T19:28:34.235182Z",
    "updated_at": "2025-11-17T19:28:34.235320Z",
    "last_message_at": "2025-11-17T19:30:15.123456Z"
  },
  "messages": [
    {
      "id": "112ec33b-7604-4c6f-a501-9cbd2f40c800",
      "session_id": "209e4078-a87f-4d96-8e3a-efe170bc7b8c",
      "role": "user",
      "content": "What is Rust?",
      "metadata": null,
      "sequence_number": 0,
      "created_at": "2025-11-17T19:28:41.140241Z"
    }
  ]
}
```

**Error Response (404):**
```json
{
  "error": "Session not found: 209e4078-a87f-4d96-8e3a-efe170bc7b8c"
}
```

**Example:**
```bash
curl http://localhost:8765/sessions/209e4078-a87f-4d96-8e3a-efe170bc7b8c
```

---

### 4. Add Message

Add a message to a session (user, assistant, system, or tool).

**Endpoint:** `POST /sessions/:id/messages`

**Request Body:**
```json
{
  "role": "user",                      // Required: "user", "assistant", "system", or "tool"
  "content": "What is Rust?",          // Required: message content
  "metadata": {"key": "value"},        // Optional: arbitrary metadata
  "project": "/path/to/project"        // Optional: project path
}
```

**Response:**
```json
{
  "message": {
    "id": "112ec33b-7604-4c6f-a501-9cbd2f40c800",
    "session_id": "209e4078-a87f-4d96-8e3a-efe170bc7b8c",
    "role": "user",
    "content": "What is Rust?",
    "metadata": null,
    "sequence_number": 0,
    "created_at": "2025-11-17T19:28:41.140241Z"
  }
}
```

**Error Response (400):**
```json
{
  "error": "Invalid role: invalid. Must be one of: user, assistant, system, tool"
}
```

**Examples:**
```bash
# Add user message
curl -X POST http://localhost:8765/sessions/$SESSION_ID/messages \
  -H "Content-Type: application/json" \
  -d '{"role": "user", "content": "What is Rust?"}'

# Add assistant message with metadata
curl -X POST http://localhost:8765/sessions/$SESSION_ID/messages \
  -H "Content-Type: application/json" \
  -d '{
    "role": "assistant",
    "content": "Rust is a systems programming language.",
    "metadata": {"model": "gpt-4", "tokens": 150}
  }'
```

---

### 5. Compile Query in Session

Compile a query in the session context. This automatically:
1. Creates a user message
2. Compiles a working set using the vector index
3. Associates the working set with the session
4. Returns both the message and working set

**Endpoint:** `POST /sessions/:id/compile`

**Request Body:**
```json
{
  "query": "What is Rust?",            // Required: query to compile
  "token_budget": 8000,                // Optional: token budget (default: 8000)
  "config": {},                        // Optional: CompilerConfig
  "project": "/path/to/project"        // Optional: project path
}
```

**Response:**
```json
{
  "message": {
    "id": "c5ad5958-4815-43af-8af0-1496b6628020",
    "session_id": "209e4078-a87f-4d96-8e3a-efe170bc7b8c",
    "role": "user",
    "content": "What is Rust?",
    "metadata": null,
    "sequence_number": 0,
    "created_at": "2025-11-17T19:29:15.234567Z"
  },
  "working_set": {
    "text": "# Compiled Context\n\nRust is a systems programming language...",
    "citations": [...],
    "spans": [...],
    "tokens_used": 1523,
    "query": "What is Rust?",
    "compilation_time_ms": 42
  }
}
```

**Error Response (404):**
```json
{
  "error": "Session not found: 209e4078-a87f-4d96-8e3a-efe170bc7b8c"
}
```

**Example:**
```bash
curl -X POST http://localhost:8765/sessions/$SESSION_ID/compile \
  -H "Content-Type: application/json" \
  -d '{"query": "What is Rust?", "token_budget": 4000}'
```

---

### 6. Get Conversation History

Retrieve formatted conversation history, suitable for including in LLM prompts.

**Endpoint:** `GET /sessions/:id/history`

**Query Parameters:**
- `max_tokens` (optional): Maximum tokens to include (keeps most recent messages)
- `project` (optional): Project path

**Response:**
```json
{
  "history": "User: What is Rust?\n\nAssistant: Rust is a systems programming language that focuses on safety, speed, and concurrency.\n\nUser: Tell me about ownership\n\nAssistant: Ownership is Rust's most unique feature that enables memory safety without a garbage collector."
}
```

**Example:**
```bash
# Get full history
curl http://localhost:8765/sessions/$SESSION_ID/history

# Get history with token limit (keeps most recent)
curl "http://localhost:8765/sessions/$SESSION_ID/history?max_tokens=500"
```

---

### 7. Session Replay

Get session replay data for debugging. Groups messages into turns (user + assistant pairs) and includes working sets.

**Endpoint:** `GET /sessions/:id/replay`

**Query Parameters:**
- `project` (optional): Project path

**Response:**
```json
{
  "session": {
    "id": "6c4819d8-6712-4e69-9a75-9a88e4978357",
    "user_id": "bob",
    "title": "Learning Rust",
    ...
  },
  "turns": [
    {
      "user_message": {
        "id": "msg-1",
        "content": "What is Rust?",
        "role": "user",
        ...
      },
      "working_set": {
        "text": "...",
        "citations": [...],
        ...
      },
      "assistant_message": {
        "id": "msg-2",
        "content": "Rust is a systems programming language...",
        "role": "assistant",
        ...
      }
    }
  ]
}
```

**Example:**
```bash
curl http://localhost:8765/sessions/$SESSION_ID/replay
```

---

### 8. Delete Session

Delete a session and all associated messages and working sets.

**Endpoint:** `DELETE /sessions/:id`

**Query Parameters:**
- `project` (optional): Project path

**Response:**
```json
{
  "success": true
}
```

**Example:**
```bash
curl -X DELETE http://localhost:8765/sessions/$SESSION_ID
```

---

## Complete Workflow Example

Here's a complete example of using the session management API:

```bash
#!/bin/bash

# 1. Ingest some documents
curl -X POST http://localhost:8765/ingest \
  -H "Content-Type: application/json" \
  -d '{
    "path": "rust_basics.md",
    "content": "Rust is a systems programming language that runs blazingly fast, prevents segfaults, and guarantees thread safety."
  }'

# 2. Create a session
SESSION_ID=$(curl -s -X POST http://localhost:8765/sessions \
  -H "Content-Type: application/json" \
  -d '{"user_id": "alice", "title": "Learning Rust"}' | jq -r '.session.id')

echo "Created session: $SESSION_ID"

# 3. First turn: User asks a question
curl -X POST http://localhost:8765/sessions/$SESSION_ID/compile \
  -H "Content-Type: application/json" \
  -d '{"query": "What is Rust?"}'

# Add assistant response
curl -X POST http://localhost:8765/sessions/$SESSION_ID/messages \
  -H "Content-Type: application/json" \
  -d '{
    "role": "assistant",
    "content": "Rust is a systems programming language that focuses on safety, speed, and concurrency."
  }'

# 4. Second turn: Follow-up question
curl -X POST http://localhost:8765/sessions/$SESSION_ID/compile \
  -H "Content-Type: application/json" \
  -d '{"query": "Tell me about ownership"}'

curl -X POST http://localhost:8765/sessions/$SESSION_ID/messages \
  -H "Content-Type: application/json" \
  -d '{
    "role": "assistant",
    "content": "Ownership is Rust'"'"'s most unique feature that enables memory safety without a garbage collector."
  }'

# 5. Get conversation history
curl http://localhost:8765/sessions/$SESSION_ID/history

# 6. Get session details
curl http://localhost:8765/sessions/$SESSION_ID

# 7. Replay session for debugging
curl http://localhost:8765/sessions/$SESSION_ID/replay

# 8. Delete session when done
curl -X DELETE http://localhost:8765/sessions/$SESSION_ID
```

## Error Handling

All endpoints return appropriate HTTP status codes:

- `200 OK` - Success
- `400 Bad Request` - Invalid input (e.g., invalid role)
- `404 Not Found` - Session not found
- `500 Internal Server Error` - Server error

Error responses include a descriptive message:

```json
{
  "error": "Descriptive error message"
}
```

## Multi-Project Support

All endpoints support an optional `project` parameter to specify which project's database to use:

```bash
# Using query parameter
curl "http://localhost:8765/sessions?project=/path/to/project"

# Using request body
curl -X POST http://localhost:8765/sessions \
  -H "Content-Type: application/json" \
  -d '{"user_id": "alice", "project": "/path/to/project"}'
```

If no project is specified, the current working directory is used.

## Integration with SDK

These HTTP endpoints are designed to be consumed by the Python/TypeScript SDKs. Example Python SDK usage:

```python
from avocado import AvocadoDB

db = AvocadoDB()

# Create session
session = db.create_session(user_id="alice", title="Learning Rust")

# Add query and compile context
message, working_set = session.compile("What is Rust?")

# Add assistant response
session.add_message("assistant", "Rust is a systems programming language...")

# Get conversation history
history = session.get_history()

# Replay session
replay = session.replay()

# Delete session
session.delete()
```

## Testing

Integration tests are available in `avocado-server/tests/session_api_tests.rs`.

Run tests with:
```bash
# Start server first
cargo run --bin avocado-server --release

# Run tests in another terminal
cargo test --test session_api_tests -- --ignored
```

## Next Steps

With Phase 3 complete, session management is ready for SDK integration:

1. **Python SDK** - Add session management methods
2. **TypeScript SDK** - Add session management methods
3. **CLI Commands** - Add `avocado session` commands
4. **Documentation** - Update SDK documentation with examples

## Implementation Notes

- Sessions are stored in the same SQLite database as artifacts and spans
- Session IDs are UUIDs (v4)
- Messages are ordered by sequence_number for deterministic ordering
- Working sets are associated with the user message that triggered compilation
- Session deletion cascades to all messages and working set associations
- All timestamps are in UTC

---

**Status:** ✅ Phase 3 Complete - All 7 endpoints implemented, tested, and working correctly.
