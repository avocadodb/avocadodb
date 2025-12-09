# Session Management API - Quick Reference

## Endpoints Summary

| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/sessions` | Create new session |
| GET | `/sessions` | List sessions |
| GET | `/sessions/:id` | Get session with messages |
| POST | `/sessions/:id/messages` | Add message to session |
| POST | `/sessions/:id/compile` | Compile query in session context |
| GET | `/sessions/:id/history` | Get formatted conversation history |
| GET | `/sessions/:id/replay` | Get session replay (debugging) |
| DELETE | `/sessions/:id` | Delete session |

## Quick Examples

### Create Session
```bash
curl -X POST http://localhost:8765/sessions \
  -H "Content-Type: application/json" \
  -d '{"user_id": "alice", "title": "My Session"}'
```

### List Sessions
```bash
curl http://localhost:8765/sessions?user_id=alice&limit=10
```

### Add Message
```bash
curl -X POST http://localhost:8765/sessions/$SESSION_ID/messages \
  -H "Content-Type: application/json" \
  -d '{"role": "user", "content": "Hello!"}'
```

### Compile Query
```bash
curl -X POST http://localhost:8765/sessions/$SESSION_ID/compile \
  -H "Content-Type: application/json" \
  -d '{"query": "What is Rust?", "token_budget": 4000}'
```

### Get History
```bash
curl http://localhost:8765/sessions/$SESSION_ID/history?max_tokens=500
```

### Delete Session
```bash
curl -X DELETE http://localhost:8765/sessions/$SESSION_ID
```

## Message Roles

- `user` - User messages
- `assistant` - LLM/assistant responses
- `system` - System messages
- `tool` - Tool/function call results

## Response Codes

- `200` - Success
- `400` - Bad request (invalid role, etc.)
- `404` - Session not found
- `500` - Server error

## Complete Workflow

```bash
# 1. Create session
SESSION_ID=$(curl -s -X POST http://localhost:8765/sessions \
  -H "Content-Type: application/json" \
  -d '{"user_id": "alice"}' | jq -r '.session.id')

# 2. Compile query (creates user message + working set)
curl -X POST http://localhost:8765/sessions/$SESSION_ID/compile \
  -H "Content-Type: application/json" \
  -d '{"query": "What is Rust?"}'

# 3. Add assistant response
curl -X POST http://localhost:8765/sessions/$SESSION_ID/messages \
  -H "Content-Type: application/json" \
  -d '{"role": "assistant", "content": "Rust is..."}'

# 4. Get conversation history
curl http://localhost:8765/sessions/$SESSION_ID/history

# 5. Clean up
curl -X DELETE http://localhost:8765/sessions/$SESSION_ID
```

## Notes

- All endpoints support optional `project` parameter
- Session IDs are UUIDs
- Messages are ordered by sequence_number
- History formatting: "User: ...\n\nAssistant: ...\n\n..."
- Token limiting keeps most recent messages
