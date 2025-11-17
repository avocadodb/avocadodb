# LlamaIndex-AvocadoDB

Official LlamaIndex integration for [AvocadoDB](https://github.com/avocadodb/avocadodb) - the deterministic, citation-backed context database for AI applications.

## Features

- **100% Deterministic Retrieval**: Same query always returns the same documents
- **Line-Level Citations**: Every document includes source file and line numbers
- **6x Faster Embeddings**: Pure Rust implementation outperforms OpenAI
- **95% Token Efficiency**: Smart chunking maximizes relevant context
- **Native Integration**: Works seamlessly with LlamaIndex indexes and query engines

## Installation

```bash
pip install llama-index-avocadodb
```

## Quick Start

### 1. Start AvocadoDB Server

```bash
# Install AvocadoDB
curl -fsSL https://raw.githubusercontent.com/avocadodb/avocadodb/main/install.sh | sh

# Start the server
avocado-server

# Ingest your codebase
avocado ingest ./your-project --recursive
```

### 2. Use with LlamaIndex

```python
from llama_index_avocadodb import AvocadoDBReader
from llama_index.core import VectorStoreIndex

# Initialize reader
reader = AvocadoDBReader(
    url="http://localhost:8765",
    budget=8000,  # Token budget
    include_citations=True
)

# Load documents for a query
documents = reader.load_data("How does authentication work?")

# Create index and query
index = VectorStoreIndex.from_documents(documents)
query_engine = index.as_query_engine()
response = query_engine.query("Explain the JWT validation process")

print(response)

# Citations preserved in metadata
for node in response.source_nodes:
    print(f"Source: {node.metadata['file_path']}:{node.metadata['start_line']}-{node.metadata['end_line']}")
```

## Advanced Usage

### TextNode Loading

Get fine-grained control with TextNodes:

```python
reader = AvocadoDBReader(
    url="http://localhost:8765",
    create_nodes=True  # Return TextNodes instead of Documents
)

nodes = reader.load_data("API implementation")
index = VectorStoreIndex(nodes)
```

### Batch Loading

Process multiple queries efficiently:

```python
queries = [
    "database architecture",
    "authentication flow",
    "error handling"
]

all_documents = reader.load_data_batch(queries, budget=5000)

for query, docs in zip(queries, all_documents):
    print(f"Query: {query}, Found: {len(docs)} documents")
```

### Lazy Loading

Memory-efficient document streaming:

```python
reader = AvocadoDBReader(
    url="http://localhost:8765",
    min_score=0.6  # Only high-quality matches
)

# Process documents as they're loaded
for doc in reader.lazy_load_data("system architecture", budget=15000):
    print(f"Processing: {doc.metadata['file_path']}")
    # Process each document...
```

### Combine Adjacent Spans

Merge nearby text chunks for better context:

```python
reader = AvocadoDBReader(
    url="http://localhost:8765",
    combine_adjacent=True  # Combines nearby spans from same file
)

documents = reader.load_data("complete authentication flow")
```

## Configuration

### Reader Parameters

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `url` | str | `"http://localhost:8765"` | AvocadoDB server URL |
| `mode` | str | `"http"` | Connection mode: `"http"` or `"cli"` |
| `budget` | int | `8000` | Token budget for retrieval |
| `semantic_weight` | float | `0.7` | Weight for semantic search (0-1) |
| `lexical_weight` | float | `0.3` | Weight for keyword search (0-1) |
| `mmr_lambda` | float | `0.5` | MMR diversity (0=diverse, 1=relevant) |
| `enable_mmr` | bool | `True` | Enable result diversification |
| `include_citations` | bool | `True` | Include citations in metadata |
| `include_scores` | bool | `True` | Include relevance scores |
| `create_nodes` | bool | `False` | Return TextNodes vs Documents |
| `combine_adjacent` | bool | `False` | Combine adjacent spans |
| `min_score` | float | `None` | Minimum score threshold |

## Examples

### Query Engine with Citations

```python
from llama_index.core import Settings
from llama_index.llms.openai import OpenAI

# Configure LLM
Settings.llm = OpenAI(model="gpt-3.5-turbo", temperature=0)

# Load and index
documents = reader.load_data("database indexing strategies")
index = VectorStoreIndex.from_documents(documents)

# Query with different response modes
query_engine = index.as_query_engine(
    response_mode="tree_summarize",  # Summarize across documents
    verbose=True
)

response = query_engine.query("What indexing strategies are used?")
```

### Chat Engine

```python
# Load context
context_docs = reader.load_data("project overview", budget=10000)
index = VectorStoreIndex.from_documents(context_docs)

# Create chat engine
chat_engine = index.as_chat_engine(
    chat_mode="context",
    verbose=True
)

# Have a conversation
response = chat_engine.chat("What is this project about?")
print(response)

response = chat_engine.chat("How does it handle scalability?")
print(response)
```

### Custom Index Creation

```python
from llama_index.core import StorageContext, ServiceContext

# Custom configuration
service_context = ServiceContext.from_defaults(
    chunk_size=512,
    chunk_overlap=50
)

storage_context = StorageContext.from_defaults()

# Load with AvocadoDB
documents = reader.load_data("microservices architecture")

# Create custom index
index = VectorStoreIndex.from_documents(
    documents,
    service_context=service_context,
    storage_context=storage_context
)
```

## Integration Patterns

### With Agents

```python
from llama_index.core.agent import ReActAgent
from llama_index.core.tools import FunctionTool

def search_codebase(query: str) -> str:
    """Search the codebase for information."""
    docs = reader.load_data(query, budget=5000)
    if docs:
        return docs[0].text
    return "No information found."

tool = FunctionTool.from_defaults(fn=search_codebase)
agent = ReActAgent.from_tools([tool], verbose=True)

response = agent.chat("How is authentication implemented?")
```

### With Retrievers

```python
# Create retriever from index
retriever = index.as_retriever(
    similarity_top_k=5,
    verbose=True
)

# Retrieve directly
nodes = retriever.retrieve("database schema")
for node in nodes:
    print(f"Score: {node.score:.3f}")
    print(f"Text: {node.text[:100]}...")
```

## Comparison with Traditional Readers

| Feature | AvocadoDB | SimpleDirectory | DatabaseReader | WebReader |
|---------|-----------|-----------------|----------------|-----------|
| Deterministic | ✅ Yes | ❌ No | ❌ No | ❌ No |
| Line Citations | ✅ Yes | ❌ No | ❌ No | ❌ No |
| Semantic Search | ✅ Yes | ❌ No | ❌ No | ❌ No |
| Token Budget | ✅ Yes | ❌ No | ❌ No | ❌ No |
| Incremental Load | ✅ Yes | ✅ Yes | ✅ Yes | ❌ No |

## Troubleshooting

### Server Connection Issues

```python
# Check server status
import requests
response = requests.get("http://localhost:8765/stats")
print(response.json())
```

### No Documents Returned

```python
# Check if documents are ingested
from avocado import AvocadoDB
client = AvocadoDB()
stats = client.stats()
print(f"Documents: {stats['artifacts']}, Spans: {stats['spans']}")
```

### Import Errors

```bash
# Install all dependencies
pip install llama-index-avocadodb llama-index llama-index-llms-openai
```

## API Reference

See the [full API documentation](https://docs.avocadodb.com/integrations/llamaindex) for detailed information on all classes and methods.

## Contributing

We welcome contributions! Please see our [Contributing Guide](CONTRIBUTING.md) for details.

## License

MIT License - see [LICENSE](LICENSE) for details.

## Support

- GitHub Issues: [llama-index-avocadodb/issues](https://github.com/avocadodb/llama-index-avocadodb/issues)
- Discord: [AvocadoDB Community](https://discord.gg/avocadodb)
- Documentation: [docs.avocadodb.com](https://docs.avocadodb.com)