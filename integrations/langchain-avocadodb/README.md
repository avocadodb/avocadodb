# LangChain-AvocadoDB

Official LangChain integration for [AvocadoDB](https://github.com/avocadodb/avocadodb) - the deterministic, citation-backed context database for AI applications.

## Features

- **100% Deterministic Retrieval**: Same query always returns the same context
- **Line-Level Citations**: Every piece of context includes source file and line numbers
- **6x Faster Embeddings**: Pure Rust implementation outperforms OpenAI
- **95% Token Efficiency**: Smart chunking maximizes relevant context
- **Drop-in Replacement**: Works with existing LangChain chains and agents

## Installation

```bash
pip install langchain-avocadodb
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

### 2. Use with LangChain

```python
from langchain_avocadodb import AvocadoDBRetriever
from langchain.chains import RetrievalQA
from langchain_openai import ChatOpenAI

# Initialize retriever
retriever = AvocadoDBRetriever(
    url="http://localhost:8765",
    budget=8000,  # Token budget
    include_citations=True
)

# Create QA chain
qa_chain = RetrievalQA.from_chain_type(
    llm=ChatOpenAI(),
    retriever=retriever,
    return_source_documents=True
)

# Ask questions with deterministic context
result = qa_chain.invoke("How does authentication work?")
print(result["answer"])

# Citations included in metadata
for doc in result["source_documents"]:
    print(f"Source: {doc.metadata['source']}:{doc.metadata['start_line']}-{doc.metadata['end_line']}")
```

## Advanced Usage

### VectorStore Interface

Use AvocadoDB as a drop-in replacement for other vector stores:

```python
from langchain_avocadodb import AvocadoDBVectorStore

vectorstore = AvocadoDBVectorStore(
    url="http://localhost:8765",
    budget=8000
)

# Use as retriever
retriever = vectorstore.as_retriever(
    search_kwargs={"k": 5}
)

# Search directly
docs = vectorstore.similarity_search("query", k=4)
```

### Maximal Marginal Relevance (MMR)

Get diverse results to avoid redundancy:

```python
retriever = AvocadoDBRetriever(
    url="http://localhost:8765",
    enable_mmr=True,
    mmr_lambda=0.5  # Balance relevance and diversity
)
```

### Score Filtering

Only return high-quality matches:

```python
retriever = AvocadoDBRetriever(
    url="http://localhost:8765",
    min_score=0.7,  # Minimum relevance threshold
    include_scores=True
)
```

### Combine Adjacent Spans

Merge nearby text chunks for better context:

```python
retriever = AvocadoDBRetriever(
    url="http://localhost:8765",
    combine_spans=True  # Combines adjacent spans from same file
)
```

## Configuration

### Retriever Parameters

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `url` | str | `"http://localhost:8765"` | AvocadoDB server URL |
| `mode` | str | `"http"` | Connection mode: `"http"` or `"cli"` |
| `budget` | int | `8000` | Token budget for context |
| `semantic_weight` | float | `0.7` | Weight for semantic search (0-1) |
| `lexical_weight` | float | `0.3` | Weight for keyword search (0-1) |
| `mmr_lambda` | float | `0.5` | MMR diversity (0=diverse, 1=relevant) |
| `enable_mmr` | bool | `True` | Enable result diversification |
| `include_citations` | bool | `True` | Include citations in metadata |
| `include_scores` | bool | `True` | Include relevance scores |
| `combine_spans` | bool | `False` | Combine adjacent spans |
| `min_score` | float | `None` | Minimum score threshold |

## Examples

### Conversational Retrieval

```python
from langchain.chains import ConversationalRetrievalChain
from langchain.memory import ConversationBufferMemory

memory = ConversationBufferMemory(
    memory_key="chat_history",
    return_messages=True
)

chain = ConversationalRetrievalChain.from_llm(
    llm=ChatOpenAI(),
    retriever=retriever,
    memory=memory
)

# Have a conversation with citations
response = chain({"question": "What database does the system use?"})
```

### Agent with Tools

```python
from langchain.agents import create_retrieval_tool

tool = create_retrieval_tool(
    retriever,
    "codebase_search",
    "Search the codebase for implementation details"
)

# Use in an agent
from langchain.agents import create_openai_tools_agent
agent = create_openai_tools_agent(llm, [tool], prompt)
```

## Comparison with Traditional Vector Stores

| Feature | AvocadoDB | Pinecone | Chroma | Weaviate |
|---------|-----------|----------|--------|----------|
| Deterministic | ✅ Yes | ❌ No | ❌ No | ❌ No |
| Line Citations | ✅ Yes | ❌ No | ❌ No | ❌ No |
| Embedding Speed | 6x faster | Baseline | 1.1x | 0.9x |
| Token Efficiency | 95% | 75% | 70% | 72% |
| Self-Hosted | ✅ Yes | ❌ No | ✅ Yes | ✅ Yes |

## Troubleshooting

### Server Not Found

```bash
# Check if server is running
curl http://localhost:8765/stats

# Start server if needed
avocado-server
```

### No Documents Found

```bash
# Ingest your codebase
avocado ingest . --recursive

# Check ingestion status
avocado stats
```

### Import Errors

```bash
# Install dependencies
pip install langchain-avocadodb langchain langchain-openai
```

## API Reference

See the [full API documentation](https://docs.avocadodb.com/integrations/langchain) for detailed information on all classes and methods.

## Contributing

We welcome contributions! Please see our [Contributing Guide](CONTRIBUTING.md) for details.

## License

MIT License - see [LICENSE](LICENSE) for details.

## Support

- GitHub Issues: [langchain-avocadodb/issues](https://github.com/avocadodb/langchain-avocadodb/issues)
- Discord: [AvocadoDB Community](https://discord.gg/avocadodb)
- Documentation: [docs.avocadodb.com](https://docs.avocadodb.com)