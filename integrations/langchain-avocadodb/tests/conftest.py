"""Pytest configuration and fixtures for langchain-avocadodb tests."""

import pytest
import os
from pathlib import Path
from typing import Generator
import tempfile
import shutil


@pytest.fixture(scope="session")
def avocado_url() -> str:
    """Get AvocadoDB server URL from environment or use default."""
    return os.environ.get("AVOCADODB_URL", "http://localhost:8765")


@pytest.fixture(scope="session")
def test_data_dir() -> Path:
    """Get path to test data directory."""
    return Path(__file__).parent / "test_data"


@pytest.fixture(scope="function")
def temp_db_path() -> Generator[Path, None, None]:
    """Create a temporary database path for CLI mode tests."""
    temp_dir = Path(tempfile.mkdtemp())
    db_path = temp_dir / "test.sqlite"

    yield db_path

    # Cleanup
    if temp_dir.exists():
        shutil.rmtree(temp_dir)


@pytest.fixture(scope="session")
def sample_documents() -> list[dict[str, str]]:
    """Sample documents for testing."""
    return [
        {
            "path": "docs/auth.md",
            "content": """# Authentication System

The authentication system uses JWT tokens for secure user verification.

## Token Generation

When a user logs in, the system generates a JWT token:

```python
def generate_token(user_id: str) -> str:
    payload = {"user_id": user_id, "exp": datetime.utcnow() + timedelta(hours=24)}
    return jwt.encode(payload, SECRET_KEY, algorithm="HS256")
```

## Token Verification

The middleware verifies tokens on each request:

```python
def verify_token(token: str) -> dict:
    try:
        return jwt.decode(token, SECRET_KEY, algorithms=["HS256"])
    except jwt.ExpiredSignatureError:
        raise AuthenticationError("Token expired")
```
""",
        },
        {
            "path": "docs/database.md",
            "content": """# Database Architecture

The system uses PostgreSQL as the primary database.

## Connection Pooling

We use connection pooling for better performance:

```python
from sqlalchemy import create_engine
from sqlalchemy.pool import QueuePool

engine = create_engine(
    DATABASE_URL,
    poolclass=QueuePool,
    pool_size=10,
    max_overflow=20
)
```

## Models

User model with SQLAlchemy:

```python
class User(Base):
    __tablename__ = "users"

    id = Column(Integer, primary_key=True)
    email = Column(String, unique=True, nullable=False)
    password_hash = Column(String, nullable=False)
    created_at = Column(DateTime, default=datetime.utcnow)
```
""",
        },
        {
            "path": "docs/api.md",
            "content": """# REST API

The API follows RESTful conventions.

## User Endpoints

### Create User
POST /api/users
```json
{
  "email": "user@example.com",
  "password": "secure_password"
}
```

### Get User
GET /api/users/{id}

### Update User
PUT /api/users/{id}

### Delete User
DELETE /api/users/{id}

## Error Handling

All endpoints return standardized error responses:

```python
def handle_error(error: Exception) -> dict:
    return {
        "error": error.__class__.__name__,
        "message": str(error),
        "status_code": getattr(error, "status_code", 500)
    }
```
""",
        },
    ]


@pytest.fixture(scope="session", autouse=True)
def setup_test_environment(sample_documents) -> None:
    """Set up test environment with sample data."""
    # This would typically ingest test data into AvocadoDB
    # For now, we assume the server is running with test data
    pass


def pytest_configure(config):
    """Configure pytest with custom markers."""
    config.addinivalue_line("markers", "unit: Unit tests")
    config.addinivalue_line(
        "markers", "integration: Integration tests requiring AvocadoDB server"
    )
    config.addinivalue_line("markers", "slow: Slow tests")
    config.addinivalue_line(
        "markers", "requires_openai: Tests requiring OpenAI API key"
    )
