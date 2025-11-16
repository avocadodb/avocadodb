# Authentication System Documentation

## Overview

Our authentication system provides secure access control for all API endpoints.
Users must authenticate with valid credentials to access protected resources.
The system uses industry-standard JWT tokens for session management.

## Authentication Flow

### 1. Login Process

Users authenticate by sending credentials to the `/api/login` endpoint:

```http
POST /api/login
Content-Type: application/json

{
  "username": "user@example.com",
  "password": "secure_password"
}
```

The server validates credentials against the user database.
On successful authentication, the server issues a JWT token.

### 2. Token Structure

JWT tokens contain three parts:
- Header: Algorithm and token type
- Payload: User claims and metadata
- Signature: Cryptographic signature for verification

Tokens are valid for 24 hours by default.
After expiration, users must re-authenticate.

### 3. Using Tokens

Include the token in the Authorization header for all API requests:

```http
GET /api/users
Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...
```

The server validates the token on each request.
Invalid or expired tokens result in 401 Unauthorized responses.

## Security Best Practices

### Password Requirements

- Minimum length: 12 characters
- Must include uppercase and lowercase letters
- Must include at least one number
- Must include at least one special character
- Cannot be a commonly used password

### Token Security

1. **Never share tokens**: Tokens grant full access to user accounts
2. **Store securely**: Use secure storage mechanisms (keychain, encrypted storage)
3. **Transmit over HTTPS**: Always use encrypted connections in production
4. **Rotate secrets**: Change JWT signing secrets regularly
5. **Implement rate limiting**: Prevent brute force attacks

### Session Management

Sessions automatically expire after 24 hours of inactivity.
Users can manually logout to invalidate their tokens.
The server maintains a revocation list for compromised tokens.

## Implementation Details

### Database Schema

```sql
CREATE TABLE users (
    id UUID PRIMARY KEY,
    email VARCHAR(255) UNIQUE NOT NULL,
    password_hash VARCHAR(255) NOT NULL,
    created_at TIMESTAMP DEFAULT NOW(),
    last_login TIMESTAMP
);

CREATE TABLE sessions (
    id UUID PRIMARY KEY,
    user_id UUID REFERENCES users(id),
    token_hash VARCHAR(255) NOT NULL,
    created_at TIMESTAMP DEFAULT NOW(),
    expires_at TIMESTAMP NOT NULL,
    revoked_at TIMESTAMP
);
```

### Password Hashing

We use bcrypt with a cost factor of 12 for password hashing.
This provides strong protection against rainbow table attacks.
Passwords are never stored in plaintext.

### Token Generation

```python
def generate_token(user_id, secret_key):
    payload = {
        'user_id': user_id,
        'exp': datetime.utcnow() + timedelta(hours=24),
        'iat': datetime.utcnow()
    }
    return jwt.encode(payload, secret_key, algorithm='HS256')
```

## API Endpoints

### POST /api/login

Authenticate user and receive JWT token.

**Request:**
```json
{
  "username": "string",
  "password": "string"
}
```

**Response (200 OK):**
```json
{
  "token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
  "expires_at": "2024-01-15T10:30:00Z",
  "user": {
    "id": "uuid",
    "email": "user@example.com"
  }
}
```

**Error (401 Unauthorized):**
```json
{
  "error": "Invalid credentials"
}
```

### POST /api/logout

Invalidate current session token.

**Headers:**
- `Authorization: Bearer <token>`

**Response (200 OK):**
```json
{
  "message": "Successfully logged out"
}
```

### POST /api/refresh

Refresh an expiring token.

**Headers:**
- `Authorization: Bearer <token>`

**Response (200 OK):**
```json
{
  "token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
  "expires_at": "2024-01-15T10:30:00Z"
}
```

## Troubleshooting

### Common Issues

**401 Unauthorized**
- Check that token is included in Authorization header
- Verify token has not expired
- Ensure token format is correct (Bearer prefix)

**403 Forbidden**
- User lacks required permissions
- Token is valid but insufficient privileges

**429 Too Many Requests**
- Rate limit exceeded
- Wait before retrying
- Consider implementing exponential backoff

## Monitoring and Logging

The system logs all authentication events:
- Successful logins
- Failed login attempts
- Token refreshes
- Logout events

Failed authentication attempts trigger security alerts after 5 consecutive failures.
Account lockout occurs after 10 failed attempts within 15 minutes.
