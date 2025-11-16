# AvocadoDB

**The first deterministic context database for AI agents**

Fix your RAG in 5 minutes - same query, same context, every time.

## What is AvocadoDB?

AvocadoDB is a span-based context compiler that replaces traditional vector databases' chaotic "top-k" retrieval with deterministic, citation-backed context generation.

### The Problem with RAG

Current RAG systems are fundamentally broken:

- ❌ Same query → different results each time
- ❌ Token budgets wasted on duplicates (60-70% utilization)
- ❌ No citations or verifiability
- ❌ Hallucinations from inconsistent context

### The AvocadoDB Solution

- ✅ **100% Deterministic**: Same query → same context, every time
- ✅ **Citation-Backed**: Every span has exact line number citations
- ✅ **Token Efficient**: 95%+ budget utilization
- ✅ **Fast**: < 500ms for 8K token context
- ✅ **Drop-in Replacement**: Works with any LLM

## Quick Start

### Installation

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Clone and build
git clone https://github.com/avocadodb/avocadodb
cd avocadodb
cargo build --release

# Set OpenAI API key
export OPENAI_API_KEY="sk-..."
```

### CLI Usage

```bash
# Initialize database
./target/release/avocado init

# Ingest documents
./target/release/avocado ingest ./docs --recursive

# Compile context
./target/release/avocado compile "How does authentication work?" --budget 8000
```

### Python SDK

```bash
cd python
pip install -e .
```

```python
from avocado import AvocadoDB

db = AvocadoDB()
db.ingest("./docs", recursive=True)

result = db.compile("my query", budget=8000)
print(result.text)  # Deterministic every time
```

### HTTP Server

```bash
# Start server
./target/release/avocado-server

# Use the API
curl -X POST http://localhost:8080/compile \
  -H "Content-Type: application/json" \
  -d '{"query": "authentication", "token_budget": 8000}'
```

## How It Works

### Architecture

```
Query → Embed → [Semantic Search + Lexical Search] → Hybrid Fusion
      → MMR Diversification → Token Packing → Deterministic Sort → WorkingSet
```

### Key Innovations

1. **Span-Based Indexing**: Documents are split into spans (20-50 lines) with precise line numbers
2. **Hybrid Retrieval**: Combines semantic (vector) and lexical (keyword) search
3. **Deterministic Ordering**: Results sorted by `(artifact_id, start_line)` for reproducibility
4. **Greedy Token Packing**: Maximizes token budget utilization without duplicates

## Why Determinism Matters

When RAG systems return different context for the same query:
- LLMs produce inconsistent answers
- Users can't verify results
- Debugging is impossible
- Trust is broken

AvocadoDB fixes this with deterministic compilation - same query, same context, every time.

## Development

### Project Structure

```
avocadodb/
├── avocado-core/      # Core engine (Rust)
├── avocado-cli/       # Command-line tool
├── avocado-server/    # HTTP server
├── python/            # Python SDK
├── migrations/        # Database schema
├── tests/             # Integration tests
└── docs/              # Documentation
```

### Running Tests

```bash
# Unit tests
cargo test

# Integration tests (requires OPENAI_API_KEY)
cargo test --test determinism -- --ignored
cargo test --test performance -- --ignored
cargo test --test correctness -- --ignored
```

### Building

```bash
# Development build
cargo build

# Release build
cargo build --release

# Run CLI
cargo run --bin avocado -- --help

# Run server
cargo run --bin avocado-server
```

## Roadmap

### Phase 1 (Current) - Drop-in RAG Replacement
- [x] Core span extraction
- [x] Embedding generation
- [x] Hybrid search (semantic + lexical)
- [x] Deterministic compilation
- [x] CLI tool
- [x] HTTP server
- [x] Python SDK
- [ ] Performance optimization
- [ ] Documentation

### Phase 2 - Advanced Features
- [ ] Multi-modal support (images, code)
- [ ] Advanced retrieval (BM25, learned rankers)
- [ ] PostgreSQL support
- [ ] Framework integrations (LangChain, LlamaIndex)

### Phase 3 - Agent Memory
- [ ] Session management
- [ ] Working set versioning
- [ ] Collaborative features
- [ ] Memory systems

## Contributing

We welcome contributions! See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## License

MIT License - see [LICENSE](LICENSE) for details.

## Learn More

- [Technical Documentation](docs/guide.md)
- [Implementation Plan](docs/plan.md)
- [Vision Document](docs/vision.md)

---

**Built by the AvocadoDB Team** | Making retrieval deterministic, one context at a time.
