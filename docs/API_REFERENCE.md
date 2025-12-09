# AvocadoDB API Reference

**Version:** 0.1.0
**Base URL:** `http://localhost:8765`

## Table of Contents

- [Introduction](#introduction)
- [Authentication](#authentication)
- [Base URL and Versioning](#base-url-and-versioning)
- [API Documentation](#api-documentation)
- [Common Patterns](#common-patterns)
- [Error Handling](#error-handling)
- [Endpoints](#endpoints)
  - [Health](#health-endpoints)
  - [Context Compilation](#context-compilation-endpoints)
  - [Document Ingestion](#document-ingestion-endpoints)
  - [Session Management](#session-management-endpoints)
  - [Statistics](#statistics-endpoints)
- [Request/Response Examples](#requestresponse-examples)
- [Rate Limiting](#rate-limiting)
- [Best Practices](#best-practices)
- [Changelog](#changelog)

## Introduction

The AvocadoDB HTTP API provides programmatic access to the deterministic context compilation engine. Use it to:

- Ingest documents and build searchable knowledge bases
- Compile deterministic, citation-backed context for AI agents
- Manage multi-turn conversation sessions
- Track database statistics and health

All responses are JSON-formatted. The API follows REST principles and uses standard HTTP status codes.

## Authentication

**Current Version:** No authentication required.

The current version of AvocadoDB does not require authentication. This is suitable for local development and trusted internal deployments.

**Future Versions:** Authentication will be added in future releases using:
- API keys (header: `Authorization: Bearer <token>`)
- JWT tokens for session-based authentication
- OAuth 2.0 for third-party integrations

## Base URL and Versioning

**Base URL:** `http://localhost:8765`

**Custom Port:** Set via environment variable:
```bash
PORT=8080 avocado-server
```

**Versioning Strategy:** See [API_VERSIONING.md](./API_VERSIONING.md) for details.

Currently, all endpoints are unversioned. When versioning is introduced:
- URL-based: `/v1/compile`, `/v2/compile`
- Backward compatibility maintained for 6 months
- Deprecation warnings in response headers

## API Documentation

**Interactive Documentation:**
- Swagger UI: `http://localhost:8765/api-docs`
- OpenAPI Spec (JSON): `http://localhost:8765/api-docs/openapi.json`
- OpenAPI Spec (YAML): `./openapi.yaml` in repository

The Swagger UI provides:
- Interactive API explorer
- Request/response examples
- Schema documentation
- Try-it-out functionality

## Common Patterns

### Project Isolation

AvocadoDB supports multi-project isolation. Each project has its own database and vector index.

**Query Parameter:**
```
?project=/path/to/project
```

**Request Body:**
```json
{
  "project": "/path/to/project",
  ...
}
```

**Default:** Current working directory (`.`)

**Examples:**
```bash
# Use current directory
curl -X POST http://localhost:8765/compile \
  -H "Content-Type: application/json" \
  -d '{"query": "authentication"}'

# Specify project path
curl -X POST http://localhost:8765/compile \
  -H "Content-Type: application/json" \
  -d '{"query": "authentication", "project": "/home/user/my-project"}'
```

### Pagination

Session listing supports pagination:

```bash
# Limit results
GET /sessions?limit=20

# Filter by user
GET /sessions?user_id=alice&limit=50
```

## Error Handling

All errors return a JSON response with:

```json
{
  "error": "Human-readable error message",
  "code": "MACHINE_READABLE_CODE",
  "details": {
    "field": "Additional context"
  }
}
```

### HTTP Status Codes

| Code | Meaning | Description |
|------|---------|-------------|
| `200` | OK | Request succeeded |
| `400` | Bad Request | Invalid request parameters |
| `404` | Not Found | Resource not found |
| `500` | Internal Server Error | Server-side error |
| `503` | Service Unavailable | Server degraded (health check) |

### Error Codes

| Code | HTTP Status | Description |
|------|-------------|-------------|
| `INTERNAL_ERROR` | 500 | Generic internal server error |
| `NOT_FOUND` | 404 | Resource not found |
| `INVALID_ROLE` | 400 | Invalid message role |
| `COMPILATION_ERROR` | 500 | Context compilation failed |
| `INGESTION_ERROR` | 500 | Document ingestion failed |

### Example Error Response

```json
{
  "error": "Invalid role: moderator. Must be one of: user, assistant, system, tool",
  "code": "INVALID_ROLE"
}
```

## Endpoints

### Health Endpoints

#### GET /health

Check server health and status.

**Response:**
```json
{
  "status": "ok",
  "service": "avocadodb-daemon",
  "version": "0.1.0",
  "uptime_seconds": 3600,
  "database_status": "ok",
  "projects_loaded": 3,
  "max_projects_in_memory": 10
}
```

**Status Codes:**
- `200 OK`: Server healthy
- `503 Service Unavailable`: Server degraded

**Fields:**
- `status`: Overall health (`ok` | `degraded` | `error`)
- `service`: Service name
- `version`: AvocadoDB version
- `uptime_seconds`: Server uptime in seconds
- `database_status`: Database connection status
- `projects_loaded`: Number of projects currently in memory
- `max_projects_in_memory`: Maximum projects cached

**Example:**
```bash
curl http://localhost:8765/health
```

### Context Compilation Endpoints

#### POST /compile

Compile deterministic context for a query.

**Request Body:**
```json
{
  "query": "How does authentication work?",
  "token_budget": 8000,
  "project": "/path/to/project",
  "config": {
    "semantic_weight": 0.7,
    "lexical_weight": 0.3,
    "mmr_lambda": 0.5,
    "enable_mmr": true
  }
}
```

**Parameters:**
- `query` (required, string): Search query
- `token_budget` (optional, integer): Max tokens (default: 8000)
- `project` (optional, string): Project path (default: current directory)
- `config` (optional, object): Compiler configuration
  - `semantic_weight` (float, 0.0-1.0): Semantic search weight (default: 0.7)
  - `lexical_weight` (float, 0.0-1.0): Lexical search weight (default: 0.3)
  - `mmr_lambda` (float, 0.0-1.0): Diversity parameter (default: 0.5)
  - `enable_mmr` (boolean): Enable MMR diversification (default: true)

**Response:**
```json
{
  "working_set": {
    "text": "[1] docs/auth.md\nLines 1-23\n\n# Authentication\n...",
    "spans": [
      {
        "id": "uuid",
        "artifact_id": "uuid",
        "artifact_path": "docs/auth.md",
        "start_line": 1,
        "end_line": 23,
        "text": "# Authentication...",
        "token_count": 127,
        "embedding_model": "text-embedding-3-small",
        "metadata": null,
        "score": 0.95
      }
    ],
    "citations": [
      {
        "span_id": "uuid",
        "artifact_id": "uuid",
        "artifact_path": "docs/auth.md",
        "start_line": 1,
        "end_line": 23,
        "score": 0.95
      }
    ],
    "tokens_used": 2232,
    "query": "How does authentication work?",
    "compilation_time_ms": 43
  }
}
```

**Example:**
```bash
curl -X POST http://localhost:8765/compile \
  -H "Content-Type: application/json" \
  -d '{
    "query": "How does authentication work?",
    "token_budget": 8000
  }'
```

### Document Ingestion Endpoints

#### POST /ingest

Ingest a single document into the database.

**Request Body:**
```json
{
  "path": "docs/authentication.md",
  "content": "# Authentication\n\nOur system uses JWT tokens...",
  "project": "/path/to/project",
  "metadata": {
    "author": "alice",
    "tags": ["auth", "security"]
  }
}
```

**Parameters:**
- `path` (required, string): Document file path
- `content` (required, string): Document content
- `project` (optional, string): Project path
- `metadata` (optional, object): Custom metadata

**Response:**
```json
{
  "artifact_id": "uuid",
  "spans_created": 12,
  "tokens_indexed": 2456
}
```

**Example:**
```bash
curl -X POST http://localhost:8765/ingest \
  -H "Content-Type: application/json" \
  -d '{
    "path": "docs/api.md",
    "content": "# API Documentation\n\n...",
    "metadata": {"version": "1.0"}
  }'
```

#### POST /ingest/batch

Ingest multiple documents in one request.

**Request Body:**
```json
{
  "documents": [
    {
      "path": "docs/auth.md",
      "content": "# Authentication...",
      "project": "/path/to/project"
    },
    {
      "path": "docs/api.md",
      "content": "# API Reference...",
      "project": "/path/to/project"
    }
  ]
}
```

**Response:**
```json
{
  "results": [
    {
      "artifact_id": "uuid-1",
      "spans_created": 12,
      "status": "success",
      "error": null
    },
    {
      "artifact_id": "uuid-2",
      "spans_created": 8,
      "status": "success",
      "error": null
    }
  ]
}
```

**Example:**
```bash
curl -X POST http://localhost:8765/ingest/batch \
  -H "Content-Type: application/json" \
  -d '{
    "documents": [
      {"path": "doc1.md", "content": "..."},
      {"path": "doc2.md", "content": "..."}
    ]
  }'
```

### Session Management Endpoints

#### POST /sessions

Create a new conversation session.

**Request Body:**
```json
{
  "user_id": "alice",
  "title": "Project Q&A",
  "project": "/path/to/project"
}
```

**Response:**
```json
{
  "session": {
    "id": "uuid",
    "user_id": "alice",
    "title": "Project Q&A",
    "metadata": null,
    "created_at": "2025-01-15T10:30:00Z",
    "updated_at": "2025-01-15T10:30:00Z"
  }
}
```

**Example:**
```bash
curl -X POST http://localhost:8765/sessions \
  -H "Content-Type: application/json" \
  -d '{"user_id": "alice", "title": "My Session"}'
```

#### GET /sessions

List all sessions.

**Query Parameters:**
- `project` (optional): Filter by project
- `user_id` (optional): Filter by user ID
- `limit` (optional): Maximum sessions to return (1-1000)

**Response:**
```json
{
  "sessions": [
    {
      "id": "uuid",
      "user_id": "alice",
      "title": "Project Q&A",
      "metadata": null,
      "created_at": "2025-01-15T10:30:00Z",
      "updated_at": "2025-01-15T10:35:00Z"
    }
  ]
}
```

**Example:**
```bash
# List all sessions
curl http://localhost:8765/sessions

# Filter by user
curl http://localhost:8765/sessions?user_id=alice

# Limit results
curl http://localhost:8765/sessions?limit=10
```

#### GET /sessions/{id}

Get session details with all messages.

**Path Parameters:**
- `id`: Session UUID

**Query Parameters:**
- `project` (optional): Project path

**Response:**
```json
{
  "session": {
    "id": "uuid",
    "user_id": "alice",
    "title": "Project Q&A",
    "created_at": "2025-01-15T10:30:00Z",
    "updated_at": "2025-01-15T10:35:00Z"
  },
  "messages": [
    {
      "id": "msg-uuid-1",
      "session_id": "uuid",
      "role": "user",
      "content": "What is AvocadoDB?",
      "metadata": null,
      "created_at": "2025-01-15T10:30:00Z"
    },
    {
      "id": "msg-uuid-2",
      "session_id": "uuid",
      "role": "assistant",
      "content": "AvocadoDB is a deterministic context database...",
      "metadata": null,
      "created_at": "2025-01-15T10:30:05Z"
    }
  ]
}
```

**Example:**
```bash
curl http://localhost:8765/sessions/{session-id}
```

#### DELETE /sessions/{id}

Delete a session and all its messages.

**Path Parameters:**
- `id`: Session UUID

**Response:**
```json
{
  "success": true
}
```

**Example:**
```bash
curl -X DELETE http://localhost:8765/sessions/{session-id}
```

#### POST /sessions/{id}/messages

Add a message to a session.

**Path Parameters:**
- `id`: Session UUID

**Request Body:**
```json
{
  "role": "user",
  "content": "How does the compiler work?",
  "project": "/path/to/project",
  "metadata": {
    "source": "web-ui"
  }
}
```

**Parameters:**
- `role` (required): Message role (`user` | `assistant` | `system` | `tool`)
- `content` (required): Message content
- `project` (optional): Project path
- `metadata` (optional): Custom metadata

**Response:**
```json
{
  "message": {
    "id": "msg-uuid",
    "session_id": "uuid",
    "role": "user",
    "content": "How does the compiler work?",
    "metadata": {"source": "web-ui"},
    "created_at": "2025-01-15T10:35:00Z"
  }
}
```

**Example:**
```bash
curl -X POST http://localhost:8765/sessions/{session-id}/messages \
  -H "Content-Type: application/json" \
  -d '{"role": "user", "content": "Hello!"}'
```

#### POST /sessions/{id}/compile

Add user message and compile context in one operation.

**Path Parameters:**
- `id`: Session UUID

**Request Body:**
```json
{
  "query": "How does the compiler work?",
  "token_budget": 8000,
  "project": "/path/to/project",
  "config": {
    "semantic_weight": 0.7,
    "lexical_weight": 0.3
  }
}
```

**Response:**
```json
{
  "message": {
    "id": "msg-uuid",
    "session_id": "uuid",
    "role": "user",
    "content": "How does the compiler work?",
    "created_at": "2025-01-15T10:35:00Z"
  },
  "working_set": {
    "text": "[1] docs/compiler.md\n...",
    "spans": [...],
    "citations": [...],
    "tokens_used": 3456,
    "query": "How does the compiler work?",
    "compilation_time_ms": 45
  }
}
```

**Example:**
```bash
curl -X POST http://localhost:8765/sessions/{session-id}/compile \
  -H "Content-Type: application/json" \
  -d '{
    "query": "How does it work?",
    "token_budget": 8000
  }'
```

#### GET /sessions/{id}/history

Get formatted conversation history.

**Path Parameters:**
- `id`: Session UUID

**Query Parameters:**
- `project` (optional): Project path
- `max_tokens` (optional): Maximum tokens in history

**Response:**
```json
{
  "history": "user: What is AvocadoDB?\nassistant: AvocadoDB is...\nuser: How does it work?\nassistant: It works by..."
}
```

**Example:**
```bash
curl http://localhost:8765/sessions/{session-id}/history?max_tokens=2000
```

#### GET /sessions/{id}/replay

Get complete session replay for debugging.

**Path Parameters:**
- `id`: Session UUID

**Query Parameters:**
- `project` (optional): Project path

**Response:**
```json
{
  "session_id": "uuid",
  "turns": [
    {
      "message_id": "msg-uuid-1",
      "query": "What is AvocadoDB?",
      "working_set": {
        "text": "...",
        "spans": [...],
        "tokens_used": 2000
      },
      "timestamp": "2025-01-15T10:30:00Z"
    }
  ]
}
```

**Example:**
```bash
curl http://localhost:8765/sessions/{session-id}/replay
```

### Statistics Endpoints

#### GET /stats

Get database statistics.

**Query Parameters:**
- `project` (optional): Project path

**Response:**
```json
{
  "artifacts_count": 42,
  "spans_count": 387,
  "total_tokens": 125431
}
```

**Example:**
```bash
curl http://localhost:8765/stats
```

#### DELETE /clear

Clear all data from the database.

**Warning:** This permanently deletes all artifacts and spans!

**Query Parameters:**
- `project` (optional): Project path

**Response:**
- `200 OK` (empty body)

**Example:**
```bash
curl -X DELETE http://localhost:8765/clear
```

## Request/Response Examples

### Complete Workflow Example

```bash
# 1. Check server health
curl http://localhost:8765/health

# 2. Ingest documents
curl -X POST http://localhost:8765/ingest \
  -H "Content-Type: application/json" \
  -d '{
    "path": "docs/auth.md",
    "content": "# Authentication\n\nJWT tokens..."
  }'

# 3. Create a session
SESSION_ID=$(curl -X POST http://localhost:8765/sessions \
  -H "Content-Type: application/json" \
  -d '{"user_id": "alice", "title": "Learning Session"}' \
  | jq -r '.session.id')

# 4. Compile context and add to session
curl -X POST http://localhost:8765/sessions/$SESSION_ID/compile \
  -H "Content-Type: application/json" \
  -d '{
    "query": "How does authentication work?",
    "token_budget": 8000
  }'

# 5. Add assistant response
curl -X POST http://localhost:8765/sessions/$SESSION_ID/messages \
  -H "Content-Type: application/json" \
  -d '{
    "role": "assistant",
    "content": "Based on the documentation, authentication uses JWT tokens..."
  }'

# 6. Get conversation history
curl http://localhost:8765/sessions/$SESSION_ID/history

# 7. View statistics
curl http://localhost:8765/stats
```

### Python SDK Example

```python
import requests

BASE_URL = "http://localhost:8765"

# Ingest document
response = requests.post(f"{BASE_URL}/ingest", json={
    "path": "docs/api.md",
    "content": "# API Documentation\n..."
})
print(f"Ingested: {response.json()['artifact_id']}")

# Compile context
response = requests.post(f"{BASE_URL}/compile", json={
    "query": "API endpoints",
    "token_budget": 8000
})
working_set = response.json()["working_set"]
print(f"Compiled {len(working_set['spans'])} spans")
print(f"Context: {working_set['text'][:200]}...")
```

### JavaScript/Node.js Example

```javascript
const axios = require('axios');

const BASE_URL = 'http://localhost:8765';

async function main() {
  // Create session
  const sessionResponse = await axios.post(`${BASE_URL}/sessions`, {
    user_id: 'alice',
    title: 'My Session'
  });
  const sessionId = sessionResponse.data.session.id;

  // Compile context
  const compileResponse = await axios.post(
    `${BASE_URL}/sessions/${sessionId}/compile`,
    {
      query: 'How does it work?',
      token_budget: 8000
    }
  );

  console.log('Compiled context:', compileResponse.data.working_set.text);
}

main();
```

## Rate Limiting

**Current Version:** No rate limiting.

Future versions may implement rate limiting:
- Rate limit headers: `X-RateLimit-Limit`, `X-RateLimit-Remaining`
- 429 Too Many Requests status code
- Configurable limits per API key

## Best Practices

### 1. Project Organization

Use separate project paths for different applications:

```bash
# Development
curl -X POST http://localhost:8765/compile \
  -d '{"query": "...", "project": "./dev-project"}'

# Production
curl -X POST http://localhost:8765/compile \
  -d '{"query": "...", "project": "./prod-project"}'
```

### 2. Token Budget Tuning

Choose token budgets based on your LLM's context window:

| LLM | Context Window | Recommended Budget |
|-----|----------------|-------------------|
| GPT-3.5 | 4K | 2000-3000 |
| GPT-4 | 8K | 4000-6000 |
| GPT-4 Turbo | 128K | 8000-16000 |
| Claude 3 | 200K | 8000-32000 |

### 3. Batch Ingestion

Use `/ingest/batch` for multiple documents to reduce network overhead:

```bash
curl -X POST http://localhost:8765/ingest/batch \
  -H "Content-Type: application/json" \
  -d '{
    "documents": [
      {"path": "doc1.md", "content": "..."},
      {"path": "doc2.md", "content": "..."},
      {"path": "doc3.md", "content": "..."}
    ]
  }'
```

### 4. Session Management

- Use sessions for multi-turn conversations
- Add metadata to messages for debugging
- Clean up old sessions periodically

### 5. Error Handling

Always check response status codes and handle errors gracefully:

```python
response = requests.post(url, json=data)
if response.status_code == 200:
    result = response.json()
elif response.status_code == 400:
    error = response.json()
    print(f"Bad request: {error['error']}")
elif response.status_code == 500:
    error = response.json()
    print(f"Server error: {error['error']}")
```

### 6. Determinism Verification

Verify determinism by compiling the same query multiple times:

```bash
# Run multiple times and compare hashes
for i in {1..5}; do
  curl -X POST http://localhost:8765/compile \
    -H "Content-Type: application/json" \
    -d '{"query": "authentication", "token_budget": 8000}' \
    | jq -r '.working_set.text' \
    | sha256sum
done
# All hashes should be identical
```

### 7. Performance Monitoring

Monitor compilation times and adjust configurations:

```bash
curl -X POST http://localhost:8765/compile \
  -H "Content-Type: application/json" \
  -d '{"query": "...", "token_budget": 8000}' \
  | jq '.working_set.compilation_time_ms'
```

Target: < 100ms for most queries

### 8. CORS Configuration

For production deployments, configure CORS properly:

```bash
# Development (permissive)
CORS_PERMISSIVE=1 avocado-server

# Production (restricted)
CORS_ALLOWED_ORIGINS="https://app.example.com,https://admin.example.com" avocado-server
```

## Changelog

### v0.1.0 (Current)

**Added:**
- Initial API release
- Context compilation endpoint
- Document ingestion (single and batch)
- Session management
- Health check endpoint
- Statistics endpoint
- Swagger UI documentation
- Production-ready CORS configuration

**Changed:**
- Enhanced health endpoint with detailed status
- Standardized error responses with error codes

**Known Limitations:**
- No authentication
- No rate limiting
- No API versioning
- Local deployment only

### Upcoming (v0.2.0)

**Planned:**
- API key authentication
- Rate limiting
- Metrics endpoint (Prometheus format)
- WebSocket support for streaming
- API versioning (v1 prefix)
- Multi-tenant support

---

## Support

- **GitHub Issues:** https://github.com/avocadodb/avocadodb/issues
- **Documentation:** https://github.com/avocadodb/avocadodb/docs
- **OpenAPI Spec:** http://localhost:8765/api-docs/openapi.json

## Related Documentation

- [OpenAPI Specification](../openapi.yaml)
- [API Versioning Strategy](./API_VERSIONING.md)
- [Session Management Guide](./SESSION_MANAGEMENT.md)
- [Performance Guide](./performance.md)
