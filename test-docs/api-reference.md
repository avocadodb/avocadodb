# API Reference

## Base URL

All API endpoints are relative to: `https://api.example.com/v1`

## Authentication

All endpoints require authentication unless otherwise specified.
Include your JWT token in the Authorization header:

```
Authorization: Bearer YOUR_TOKEN_HERE
```

## User Management

### List Users

Get a paginated list of all users.

**Endpoint:** `GET /users`

**Query Parameters:**
- `page` (integer): Page number (default: 1)
- `limit` (integer): Items per page (default: 20, max: 100)
- `sort` (string): Sort field (default: created_at)
- `order` (string): Sort order - asc or desc (default: desc)

**Example Request:**
```bash
curl -X GET "https://api.example.com/v1/users?page=1&limit=20" \
  -H "Authorization: Bearer YOUR_TOKEN"
```

**Example Response:**
```json
{
  "users": [
    {
      "id": "550e8400-e29b-41d4-a716-446655440000",
      "email": "user@example.com",
      "name": "John Doe",
      "created_at": "2024-01-01T00:00:00Z",
      "last_login": "2024-01-15T10:30:00Z"
    }
  ],
  "pagination": {
    "page": 1,
    "limit": 20,
    "total": 150,
    "total_pages": 8
  }
}
```

### Create User

Create a new user account.

**Endpoint:** `POST /users`

**Request Body:**
```json
{
  "email": "newuser@example.com",
  "password": "SecurePass123!",
  "name": "Jane Smith",
  "role": "user"
}
```

**Response (201 Created):**
```json
{
  "id": "660e8400-e29b-41d4-a716-446655440000",
  "email": "newuser@example.com",
  "name": "Jane Smith",
  "role": "user",
  "created_at": "2024-01-15T12:00:00Z"
}
```

### Get User

Retrieve details for a specific user.

**Endpoint:** `GET /users/{id}`

**Response (200 OK):**
```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "email": "user@example.com",
  "name": "John Doe",
  "role": "admin",
  "created_at": "2024-01-01T00:00:00Z",
  "last_login": "2024-01-15T10:30:00Z",
  "profile": {
    "avatar": "https://example.com/avatars/user.jpg",
    "bio": "Software developer"
  }
}
```

### Update User

Update user information.

**Endpoint:** `PUT /users/{id}`

**Request Body:**
```json
{
  "name": "John Updated",
  "profile": {
    "bio": "Senior software developer"
  }
}
```

**Response (200 OK):**
```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "email": "user@example.com",
  "name": "John Updated",
  "updated_at": "2024-01-15T12:30:00Z"
}
```

### Delete User

Permanently delete a user account.

**Endpoint:** `DELETE /users/{id}`

**Response (204 No Content)**

## Error Responses

All errors follow a consistent format:

```json
{
  "error": {
    "code": "VALIDATION_ERROR",
    "message": "Invalid request parameters",
    "details": [
      {
        "field": "email",
        "message": "Email is required"
      }
    ]
  }
}
```

### Error Codes

- `400` Bad Request - Invalid request parameters
- `401` Unauthorized - Missing or invalid authentication
- `403` Forbidden - Insufficient permissions
- `404` Not Found - Resource does not exist
- `429` Too Many Requests - Rate limit exceeded
- `500` Internal Server Error - Server-side error

## Rate Limiting

API requests are rate limited to prevent abuse:
- 1000 requests per hour for authenticated users
- 100 requests per hour for unauthenticated endpoints

Rate limit information is included in response headers:
```
X-RateLimit-Limit: 1000
X-RateLimit-Remaining: 950
X-RateLimit-Reset: 1705329600
```

## Webhooks

Subscribe to events via webhooks.

### Register Webhook

**Endpoint:** `POST /webhooks`

**Request Body:**
```json
{
  "url": "https://your-app.com/webhook",
  "events": ["user.created", "user.updated", "user.deleted"],
  "secret": "your-webhook-secret"
}
```

### Webhook Payload

```json
{
  "event": "user.created",
  "timestamp": "2024-01-15T12:00:00Z",
  "data": {
    "id": "660e8400-e29b-41d4-a716-446655440000",
    "email": "newuser@example.com"
  }
}
```

## SDKs and Libraries

Official SDKs available for:
- Python: `pip install example-api`
- JavaScript: `npm install example-api-js`
- Ruby: `gem install example-api`
- Go: `go get github.com/example/api-go`
