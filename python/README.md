# AvocadoDB Python SDK

Python client for AvocadoDB - the deterministic context database for AI agents.

## Installation

```bash
pip install -e .
```

## Quick Start

```python
from avocado import AvocadoDB

# Initialize client
db = AvocadoDB()

# Ingest documents
db.ingest("doc.txt", "This is my document content")

# Compile context
result = db.compile("my query", budget=8000)

print(result.text)
print(f"Tokens used: {result.tokens_used}")
print(f"Citations: {len(result.citations)}")
```

## Features

- **Deterministic**: Same query → same context, every time
- **Citation-backed**: Every piece of context has exact line numbers
- **Token efficient**: 95%+ budget utilization
- **Fast**: < 500ms for 8K token context

## API Reference

### AvocadoDB

#### `__init__(url="http://localhost:8080")`

Initialize the client.

#### `ingest(path, content=None, metadata=None)`

Ingest a single document.

#### `compile(query, budget=8000, config=None)`

Compile a context working set for a query.

#### `stats()`

Get database statistics.

#### `clear()`

Clear all data.

### WorkingSet

The compiled context result.

- `text`: Compiled context text
- `spans`: List of spans included
- `citations`: Citation information
- `tokens_used`: Total tokens used
- `compilation_time_ms`: Compilation time
