# LlamaIndex Integration Guide

Complete guide to integrating AvocadoDB with LlamaIndex for deterministic, citation-backed RAG applications.

## Table of Contents

1. [Introduction](#introduction)
2. [Installation](#installation)
3. [Quick Start](#quick-start)
4. [Core Concepts](#core-concepts)
5. [Configuration](#configuration)
6. [Usage Patterns](#usage-patterns)
7. [Advanced Features](#advanced-features)
8. [Performance Optimization](#performance-optimization)
9. [Migration Guide](#migration-guide)
10. [Troubleshooting](#troubleshooting)
11. [FAQ](#faq)

---

## Introduction

The AvocadoDB-LlamaIndex integration provides a production-ready data connector that brings deterministic retrieval and line-level citations to LlamaIndex applications.

### Why Use AvocadoDB with LlamaIndex?

**Traditional LlamaIndex Data Loaders**:
- Non-deterministic retrieval
- No source line tracking
- Limited citation capabilities
- Variable performance

**AvocadoDB Integration**:
- ✅ **100% Deterministic**: Same query = same results, always
- ✅ **Line-Level Citations**: Track exact source lines
- ✅ **6x Faster Embeddings**: Pure Rust implementation
- ✅ **95% Token Efficiency**: Smart chunking maximizes context
- ✅ **Native LlamaIndex**: Seamless BaseReader integration

### Key Benefits

1. **Reproducibility**: Debug and test with confidence
2. **Provenance**: Know exactly where information came from
3. **Performance**: Faster embeddings and retrieval
4. **Efficiency**: Better use of token budgets
5. **Integration**: Drop-in replacement for other readers

---

## Installation

### Prerequisites

- Python 3.9+
- AvocadoDB server running
- OpenAI API key (for embeddings/LLM)

### Install the Integration

```bash
pip install llama-index-avocadodb
```

This installs:
- `llama-index-avocadodb` (the integration)
- `llama-index-core` (LlamaIndex core)
- `avocadodb` (Python SDK)

### Install LlamaIndex Components

For complete functionality:

```bash
# For OpenAI models
pip install llama-index-llms-openai llama-index-embeddings-openai

# Or for the full LlamaIndex package
pip install llama-index
```

### Setup AvocadoDB

```bash
# Install AvocadoDB CLI
curl -fsSL https://raw.githubusercontent.com/avocadodb/avocadodb/main/install.sh | sh

# Start the server
avocado-server

# Ingest your codebase
avocado ingest /path/to/your/project --recursive
```

### Verify Installation

```python
from llama_index_avocadodb import AvocadoDBReader
from llama_index.core import VectorStoreIndex

print("✓ Installation successful")
```

---

## Quick Start

### 5-Line RAG Application

```python
from llama_index_avocadodb import AvocadoDBReader
from llama_index.core import VectorStoreIndex

reader = AvocadoDBReader(url="http://localhost:8765")
documents = reader.load_data("How does authentication work?")
response = VectorStoreIndex.from_documents(documents).as_query_engine().query("Explain JWT validation")
print(response)
```

### With Citations

```python
from llama_index_avocadodb import AvocadoDBReader
from llama_index.core import VectorStoreIndex

# Enable citations
reader = AvocadoDBReader(include_citations=True)
documents = reader.load_data("database architecture")

index = VectorStoreIndex.from_documents(documents)
response = index.as_query_engine().query("How is data stored?")

# Show answer with sources
print(f"Answer: {response}\n")
print("Sources:")
for node in response.source_nodes:
    metadata = node.metadata
    print(f"  {metadata['file_path']}:{metadata['start_line']}-{metadata['end_line']}")
```

---

## Core Concepts

### The AvocadoDBReader

The `AvocadoDBReader` is a LlamaIndex `BaseReader` that loads documents from AvocadoDB:

```python
from llama_index_avocadodb import AvocadoDBReader

reader = AvocadoDBReader(
    url="http://localhost:8765",  # Server URL
    budget=8000,                   # Token budget
    include_citations=True         # Enable citations
)
```

### Loading Documents

Documents are loaded based on semantic queries:

```python
# Load documents relevant to a query
documents = reader.load_data("authentication implementation")

# Each document contains:
# - text: The actual code/content
# - metadata: Citations, scores, file info
# - id_: Unique span identifier
```

### Document Metadata

Every document includes rich metadata:

```python
{
    "file_path": "auth/jwt.py",
    "start_line": 45,
    "end_line": 52,
    "token_count": 85,
    "score": 0.87,
    "query": "authentication implementation",
    "deterministic_hash": "abc123...",
    "citations": [
        {
            "file": "auth/utils.py",
            "start_line": 10,
            "end_line": 15,
            "score": 0.82
        }
    ]
}
```

### Integration with LlamaIndex

The reader works with all LlamaIndex components:

```python
# Indexes
from llama_index.core import VectorStoreIndex, SummaryIndex, TreeIndex

vector_index = VectorStoreIndex.from_documents(documents)
summary_index = SummaryIndex.from_documents(documents)
tree_index = TreeIndex.from_documents(documents)

# Query Engines
query_engine = vector_index.as_query_engine()
chat_engine = vector_index.as_chat_engine()

# Retrievers
retriever = vector_index.as_retriever(similarity_top_k=5)
```

---

## Configuration

### Reader Parameters

#### Connection Settings

```python
reader = AvocadoDBReader(
    url="http://localhost:8765",  # Server URL
    mode="http"                    # "http" or "cli"
)
```

#### Retrieval Settings

```python
reader = AvocadoDBReader(
    budget=8000,            # Token budget for context
    semantic_weight=0.7,    # Weight for semantic search (0-1)
    lexical_weight=0.3,     # Weight for keyword search (0-1)
    mmr_lambda=0.5,         # Diversity parameter (0=diverse, 1=relevant)
    enable_mmr=True,        # Enable Maximal Marginal Relevance
    min_score=0.6           # Minimum relevance threshold
)
```

#### Output Settings

```python
reader = AvocadoDBReader(
    include_citations=True,  # Include source citations
    include_scores=True,     # Include relevance scores
    create_nodes=False,      # Return TextNodes instead of Documents
    combine_adjacent=False   # Combine nearby spans from same file
)
```

### Environment Variables

```bash
# AvocadoDB server URL
export AVOCADO_URL=http://localhost:8765

# OpenAI API key (for LlamaIndex)
export OPENAI_API_KEY=your-key-here
```

### LlamaIndex Global Settings

```python
from llama_index.core import Settings
from llama_index.llms.openai import OpenAI
from llama_index.embeddings.openai import OpenAIEmbedding

# Configure LLM
Settings.llm = OpenAI(
    model="gpt-4",
    temperature=0
)

# Configure embeddings
Settings.embed_model = OpenAIEmbedding(
    model="text-embedding-3-small"
)

# Configure chunk size (AvocadoDB handles chunking, but affects index)
Settings.chunk_size = 512
Settings.chunk_overlap = 50
```

---

## Usage Patterns

### Pattern 1: Simple Q&A

```python
reader = AvocadoDBReader()
documents = reader.load_data("error handling")

index = VectorStoreIndex.from_documents(documents)
query_engine = index.as_query_engine()

response = query_engine.query("How are errors caught?")
print(response)
```

### Pattern 2: Chat Interface

```python
from llama_index.core.memory import ChatMemoryBuffer

reader = AvocadoDBReader(budget=10000)
documents = reader.load_data("API documentation")

index = VectorStoreIndex.from_documents(documents)
chat_engine = index.as_chat_engine(
    chat_mode="context",
    memory=ChatMemoryBuffer.from_defaults(token_limit=3000)
)

# Multi-turn conversation
chat_engine.chat("What APIs are available?")
chat_engine.chat("How do I authenticate?")
chat_engine.chat("Show me an example")
```

### Pattern 3: Batch Processing

```python
reader = AvocadoDBReader()

queries = [
    "user authentication",
    "data validation",
    "error handling"
]

# Load all at once
all_docs = reader.load_data_batch(queries, budget=5000)

# Process each set
for query, docs in zip(queries, all_docs):
    index = VectorStoreIndex.from_documents(docs)
    response = index.as_query_engine().query(f"Explain {query}")
    print(f"{query}: {response}")
```

### Pattern 4: Streaming Responses

```python
reader = AvocadoDBReader()
documents = reader.load_data("system architecture")

index = VectorStoreIndex.from_documents(documents)
query_engine = index.as_query_engine(streaming=True)

response = query_engine.query("Explain the architecture")

# Stream the response
for text in response.response_gen:
    print(text, end="", flush=True)
```

### Pattern 5: Custom Retriever

```python
from llama_index.core.retrievers import VectorIndexRetriever
from llama_index.core.postprocessor import SimilarityPostprocessor
from llama_index.core.query_engine import RetrieverQueryEngine

reader = AvocadoDBReader()
documents = reader.load_data("database queries")

index = VectorStoreIndex.from_documents(documents)

# Custom retriever with filtering
retriever = VectorIndexRetriever(
    index=index,
    similarity_top_k=10
)

# Post-process to filter results
query_engine = RetrieverQueryEngine(
    retriever=retriever,
    node_postprocessors=[
        SimilarityPostprocessor(similarity_cutoff=0.7)
    ]
)

response = query_engine.query("SQL optimization")
```

---

## Advanced Features

### Citation Tracking

AvocadoDB provides line-level citations:

```python
reader = AvocadoDBReader(include_citations=True)
documents = reader.load_data("authentication")

index = VectorStoreIndex.from_documents(documents)
response = index.as_query_engine().query("How does auth work?")

# Extract all citations
for node in response.source_nodes:
    # Primary source
    print(f"Source: {node.metadata['file_path']}")
    print(f"  Lines: {node.metadata['start_line']}-{node.metadata['end_line']}")

    # Referenced citations
    if 'citations' in node.metadata:
        for citation in node.metadata['citations']:
            print(f"  References: {citation['file']}:{citation['start_line']}")
```

### Deterministic Retrieval

Verify determinism:

```python
reader = AvocadoDBReader()

# Run same query multiple times
hashes = []
for i in range(3):
    documents = reader.load_data("test query")
    hash_val = documents[0].metadata['deterministic_hash']
    hashes.append(hash_val)

# All hashes should be identical
assert len(set(hashes)) == 1
print(f"✓ Deterministic: {hashes[0]}")
```

### Combining Adjacent Spans

Get more complete context:

```python
reader = AvocadoDBReader(
    combine_adjacent=True  # Combine nearby code from same file
)

documents = reader.load_data("complete authentication flow")

# Documents now contain combined spans
for doc in documents:
    if doc.metadata.get('span_count', 1) > 1:
        print(f"Combined {doc.metadata['span_count']} spans")
        print(f"Lines: {doc.metadata['start_line']}-{doc.metadata['end_line']}")
```

### TextNode Conversion

For fine-grained control:

```python
reader = AvocadoDBReader(create_nodes=True)
nodes = reader.load_data("API implementation")

# Nodes have more capabilities than documents
index = VectorStoreIndex(nodes)

# Nodes support relationships, metadata, etc.
for node in nodes:
    print(f"Node ID: {node.id_}")
    print(f"Score: {node.metadata.get('score')}")
```

### Lazy Loading

Memory-efficient processing:

```python
reader = AvocadoDBReader(min_score=0.7)

# Generator - doesn't load all at once
for document in reader.lazy_load_data("large query", budget=20000):
    # Process each document as it's loaded
    process_document(document)

    # Can break early if needed
    if should_stop():
        break
```

---

## Performance Optimization

### Token Budget Tuning

```python
# Small budget: faster, less context
reader = AvocadoDBReader(budget=3000)  # ~1-2 files

# Medium budget: balanced
reader = AvocadoDBReader(budget=8000)  # ~3-5 files

# Large budget: comprehensive context
reader = AvocadoDBReader(budget=15000)  # ~8-12 files
```

### Search Weight Optimization

```python
# Favor semantic search (better for conceptual queries)
reader = AvocadoDBReader(
    semantic_weight=0.9,
    lexical_weight=0.1
)

# Favor keyword search (better for specific terms)
reader = AvocadoDBReader(
    semantic_weight=0.3,
    lexical_weight=0.7
)

# Balanced (recommended)
reader = AvocadoDBReader(
    semantic_weight=0.7,
    lexical_weight=0.3
)
```

### MMR for Diversity

```python
# Enable MMR to reduce redundancy
reader = AvocadoDBReader(
    enable_mmr=True,
    mmr_lambda=0.5  # Balance relevance and diversity
)

# Higher lambda = more relevant, less diverse
# Lower lambda = more diverse, less relevant
```

### Score Filtering

```python
# Only high-quality matches
reader = AvocadoDBReader(min_score=0.7)

documents = reader.load_data("complex query")
# Only documents with score >= 0.7 returned
```

### Batch Processing

```python
# More efficient than individual queries
queries = ["query1", "query2", "query3"]
all_docs = reader.load_data_batch(queries, budget=5000)

# Processes all queries in one pass
```

### Caching

```python
# AvocadoDB automatically caches:
# - Embeddings (persistent)
# - Index structures (in-memory)
# - Compilation results (short-lived)

# No additional configuration needed
```

---

## Migration Guide

### From SimpleDirectoryReader

**Before (SimpleDirectoryReader)**:
```python
from llama_index.core import SimpleDirectoryReader

reader = SimpleDirectoryReader("./code")
documents = reader.load_data()
```

**After (AvocadoDBReader)**:
```python
from llama_index_avocadodb import AvocadoDBReader

# One-time: ingest directory
# avocado ingest ./code --recursive

reader = AvocadoDBReader()
documents = reader.load_data("your query here")
```

**Benefits**:
- Query-based loading (only relevant files)
- Deterministic results
- Line-level citations
- Faster embeddings

### From DatabaseReader

**Before**:
```python
from llama_index.readers.database import DatabaseReader

reader = DatabaseReader(
    sql_database=db,
    query="SELECT * FROM docs"
)
documents = reader.load_data()
```

**After**:
```python
from llama_index_avocadodb import AvocadoDBReader

# Ingest database schema/docs first
reader = AvocadoDBReader()
documents = reader.load_data("database schema and operations")
```

### From Web Scrapers

**Before**:
```python
from llama_index.readers.web import SimpleWebPageReader

reader = SimpleWebPageReader()
documents = reader.load_data(["https://..."])
```

**After**:
```python
# Ingest downloaded docs with AvocadoDB
# avocado ingest ./docs --recursive

from llama_index_avocadodb import AvocadoDBReader

reader = AvocadoDBReader()
documents = reader.load_data("API documentation")
```

---

## Troubleshooting

### Server Connection Issues

**Problem**: `Connection refused` or timeout errors

**Solutions**:
```bash
# 1. Check server is running
curl http://localhost:8765/stats

# 2. Start server if needed
avocado-server

# 3. Check port
avocado-server --port 8765

# 4. Use correct URL in code
reader = AvocadoDBReader(url="http://localhost:8765")
```

### No Documents Returned

**Problem**: `load_data()` returns empty list

**Solutions**:
```bash
# 1. Verify documents are ingested
avocado stats

# 2. Check query matches content
# Try broader queries

# 3. Lower min_score threshold
reader = AvocadoDBReader(min_score=0.3)

# 4. Increase budget
documents = reader.load_data("query", budget=15000)
```

### Import Errors

**Problem**: `ModuleNotFoundError: No module named 'llama_index_avocadodb'`

**Solutions**:
```bash
# 1. Install the package
pip install llama-index-avocadodb

# 2. Install LlamaIndex
pip install llama-index

# 3. Check Python version
python --version  # Should be 3.9+
```

### Low Quality Results

**Problem**: Irrelevant documents returned

**Solutions**:
```python
# 1. Increase semantic weight
reader = AvocadoDBReader(semantic_weight=0.9, lexical_weight=0.1)

# 2. Use more specific queries
documents = reader.load_data("JWT token validation in auth module")

# 3. Filter by score
reader = AvocadoDBReader(min_score=0.7)

# 4. Enable MMR
reader = AvocadoDBReader(enable_mmr=True, mmr_lambda=0.6)
```

### Memory Issues

**Problem**: Out of memory with large result sets

**Solutions**:
```python
# 1. Use lazy loading
for doc in reader.lazy_load_data("query"):
    process(doc)

# 2. Reduce budget
reader = AvocadoDBReader(budget=5000)

# 3. Filter results
reader = AvocadoDBReader(min_score=0.8)
```

---

## FAQ

### Q: How is AvocadoDB different from vector databases?

**A**: AvocadoDB is a deterministic context database, not a vector database. Key differences:
- Deterministic retrieval (same query = same results)
- Line-level citations
- Query-based loading vs similarity search
- Optimized for code and structured content

### Q: Can I use AvocadoDB with other LlamaIndex components?

**A**: Yes! AvocadoDBReader is a standard `BaseReader`. Works with:
- All index types (Vector, Summary, Tree, Knowledge Graph)
- All query engines
- All chat engines
- Agents and tools
- Storage contexts

### Q: Do I need to re-embed documents?

**A**: No! AvocadoDB handles embeddings internally with its fast Rust implementation. LlamaIndex will embed the retrieved chunks for its indexes, but the initial retrieval is already optimized.

### Q: How do I handle updates to my codebase?

**A**: Re-ingest the updated files:
```bash
# Re-ingest specific files
avocado ingest path/to/changed/files --recursive

# Or re-ingest everything
avocado ingest . --recursive --force
```

### Q: Can I use AvocadoDB without LlamaIndex?

**A**: Yes! AvocadoDB has a standalone Python SDK:
```python
from avocado import AvocadoDB

client = AvocadoDB()
working_set = client.compile("your query", budget=8000)
```

### Q: What's the recommended token budget?

**A**: Depends on use case:
- Quick queries: 3000-5000 tokens
- Standard RAG: 8000-10000 tokens
- Comprehensive analysis: 15000-20000 tokens

### Q: How do citations work?

**A**: Every span includes:
1. Primary source (the actual code location)
2. Referenced citations (cross-file references)
3. Line-level precision (exact line numbers)

Access via `document.metadata['citations']`.

### Q: Is this production-ready?

**A**: Yes! The integration includes:
- Comprehensive error handling
- Type hints throughout
- Extensive test coverage
- Performance optimizations
- Production PyPI package

### Q: Can I customize the retrieval?

**A**: Yes! Use LlamaIndex post-processors:
```python
from llama_index.core.postprocessor import (
    SimilarityPostprocessor,
    KeywordNodePostprocessor
)

# Custom filtering pipeline
query_engine = RetrieverQueryEngine(
    retriever=retriever,
    node_postprocessors=[
        SimilarityPostprocessor(similarity_cutoff=0.7),
        KeywordNodePostprocessor(required_keywords=["important"])
    ]
)
```

### Q: How do I monitor performance?

**A**: Check document metadata:
```python
documents = reader.load_data("query")

for doc in documents:
    print(f"Tokens: {doc.metadata['token_count']}")
    print(f"Score: {doc.metadata['score']}")
    print(f"Compile time: {doc.metadata['compilation_time_ms']}ms")
```

---

## Additional Resources

- [GitHub Repository](https://github.com/avocadodb/llama-index-avocadodb)
- [AvocadoDB Documentation](https://docs.avocadodb.com)
- [LlamaIndex Documentation](https://docs.llamaindex.ai)
- [API Reference](https://docs.avocadodb.com/integrations/llamaindex)
- [Examples](../integrations/llama-index-avocadodb/examples)

## Support

- **GitHub Issues**: [Report bugs](https://github.com/avocadodb/llama-index-avocadodb/issues)
- **Discord**: [Join community](https://discord.gg/avocadodb)
- **Documentation**: [docs.avocadodb.com](https://docs.avocadodb.com)

## Contributing

We welcome contributions! See [CONTRIBUTING.md](../CONTRIBUTING.md) for guidelines.

## License

MIT License - see [LICENSE](../LICENSE) for details.
