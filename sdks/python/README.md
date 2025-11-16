# AvocadoDB Python SDK

**Simple HTTP client for AvocadoDB - the deterministic context database.**

## Installation

```bash
pip install avocadodb
```

Or from source:

```bash
cd python
pip install -e .
```

## Quick Start

### 1. Start Server & Ingest Data

```bash
# Start the server
./target/release/avocado-server &

# Ingest documents
./target/release/avocado ingest test-docs/ --recursive
```

### 2. Use Python SDK

```python
from avocado import AvocadoDB

# Connect to server
db = AvocadoDB("http://localhost:8080")

# Compile context
result = db.compile("How does authentication work?", budget=8000)

print(f"Compiled {len(result.spans)} spans")
print(f"Used {result.tokens_used} tokens")
print(f"Hash: {result.deterministic_hash()}")
print(result.text)
```

## Features

- ✅ **Deterministic**: Same query → same context, every time
- ✅ **Citation-backed**: Every span has exact line numbers
- ✅ **Token efficient**: 90-95% budget utilization
- ✅ **Fast**: < 500ms for 8K token context
- ✅ **Simple**: Just HTTP requests, no complex dependencies

## API Reference

### `AvocadoDB(url="http://localhost:8080")`

Create client connection.

### `db.compile(query, **options)`

Compile deterministic context.

**Parameters:**
- `query` (str): Search query
- `budget` (int): Token budget (default: 8000)
- `semantic_weight` (float): Semantic weight (default: 0.7)
- `lexical_weight` (float): Lexical weight (default: 0.3)
- `mmr_lambda` (float): Diversity 0.0-1.0 (default: 0.5)
- `enable_mmr` (bool): Enable MMR (default: True)

**Returns:** `WorkingSet`

### `db.ingest(path, content=None)`

Ingest document.

**Parameters:**
- `path` (str): Document path
- `content` (str): Content (reads from file if None)

**Returns:** Dict with `artifact_id` and span count

### `db.stats()`

Get database statistics.

**Returns:** Dict with `artifacts`, `spans`, `tokens`

## WorkingSet Object

**Attributes:**
- `text` (str): Compiled context
- `spans` (List[Span]): Included spans
- `citations` (List[Citation]): Citations
- `tokens_used` (int): Tokens used
- `query` (str): Original query
- `compilation_time_ms` (int): Compilation time

**Methods:**
- `deterministic_hash()`: SHA-256 hash of context

## Examples

### Verify Determinism

```python
from avocado import AvocadoDB

db = AvocadoDB()

# Run same query 3 times
hashes = []
for i in range(3):
    result = db.compile("authentication")
    hashes.append(result.deterministic_hash())

# All hashes identical
assert hashes[0] == hashes[1] == hashes[2]
print("✅ Deterministic!")
```

### Tune Parameters

```python
# More diverse results
result = db.compile("query", mmr_lambda=0.3)

# More keyword matching
result = db.compile("query", lexical_weight=0.5)

# Large context
result = db.compile("query", budget=16000)
```

See `example.py` for full demonstration.

## Requirements

- Python 3.8+
- `requests` library
- Running AvocadoDB server

## License

MIT License
