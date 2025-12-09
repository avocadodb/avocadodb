# LangChain Integration Guide

Comprehensive guide for integrating AvocadoDB with LangChain for deterministic, citation-backed RAG applications.

## Table of Contents

- [Overview](#overview)
- [Installation](#installation)
- [Quick Start](#quick-start)
- [Core Concepts](#core-concepts)
- [Configuration](#configuration)
- [Usage Patterns](#usage-patterns)
- [Advanced Features](#advanced-features)
- [Performance Optimization](#performance-optimization)
- [Migration Guide](#migration-guide)
- [Troubleshooting](#troubleshooting)
- [FAQ](#faq)

## Overview

The AvocadoDB-LangChain integration provides:

- **100% Deterministic Retrieval**: Same query always returns identical context
- **Line-Level Citations**: Every span includes source file and line numbers
- **6x Faster Embeddings**: Pure Rust implementation outperforms OpenAI
- **95% Token Efficiency**: Smart chunking maximizes relevant context
- **Drop-in Replacement**: Compatible with existing LangChain chains and agents

### Why AvocadoDB?

Traditional vector stores have inherent non-determinism due to:
- Approximate nearest neighbor search (HNSW, IVF)
- Non-deterministic tie-breaking
- Index updates affecting results

AvocadoDB solves this with:
- Exact search with deterministic ranking
- Hybrid semantic + lexical scoring
- Immutable context compilation
- Reproducible SHA-256 hashes

## Installation

### Prerequisites

- Python 3.9+
- AvocadoDB server running
- Documents ingested into AvocadoDB

### Install Package

```bash
pip install langchain-avocadodb
```

### Install with Optional Dependencies

```bash
# With LangChain chains
pip install langchain-avocadodb langchain

# With OpenAI models
pip install langchain-avocadodb langchain-openai

# Complete installation
pip install langchain-avocadodb langchain langchain-openai langchain-community
```

### Verify Installation

```python
from langchain_avocadodb import AvocadoDBRetriever, AvocadoDBVectorStore
print("Installation successful!")
```

## Quick Start

### 1. Start AvocadoDB Server

```bash
# Start the daemon server
avocado-server

# Verify it's running
curl http://localhost:8765/stats
```

### 2. Ingest Your Codebase

```bash
# Ingest current directory
avocado ingest . --recursive

# Check ingestion
avocado stats
```

### 3. Use with LangChain

```python
from langchain_avocadodb import AvocadoDBRetriever
from langchain.chains import RetrievalQA
from langchain_openai import ChatOpenAI

# Initialize retriever
retriever = AvocadoDBRetriever(
    url="http://localhost:8765",
    budget=8000,
    include_citations=True
)

# Create QA chain
qa_chain = RetrievalQA.from_chain_type(
    llm=ChatOpenAI(),
    retriever=retriever,
    return_source_documents=True
)

# Ask questions
result = qa_chain.invoke({"query": "How does authentication work?"})
print(result["answer"])

# View citations
for doc in result["source_documents"]:
    print(f"{doc.metadata['source']}:{doc.metadata['start_line']}")
```

## Core Concepts

### Retriever vs VectorStore

AvocadoDB provides two interfaces:

**AvocadoDBRetriever** (Recommended)
- Direct retriever interface
- Full access to AvocadoDB features
- Better performance
- More configuration options

```python
retriever = AvocadoDBRetriever(url="http://localhost:8765")
docs = retriever.get_relevant_documents("query")
```

**AvocadoDBVectorStore** (Compatibility)
- VectorStore-compatible interface
- Drop-in replacement for other stores
- Works with existing code
- Wraps AvocadoDBRetriever

```python
vectorstore = AvocadoDBVectorStore(url="http://localhost:8765")
retriever = vectorstore.as_retriever()
```

### Deterministic Context

Every retrieval produces a deterministic hash:

```python
result1 = retriever.get_relevant_documents("auth")
result2 = retriever.get_relevant_documents("auth")

hash1 = result1[0].metadata["deterministic_hash"]
hash2 = result2[0].metadata["deterministic_hash"]

assert hash1 == hash2  # Always true!
```

This enables:
- **Reproducible Results**: Same query = same context
- **Testing & Validation**: Assert on exact results
- **Versioning**: Track context changes over time
- **Debugging**: Reproduce issues exactly

### Citation System

Every document includes precise citations:

```python
{
    "source": "src/auth.py",
    "start_line": 45,
    "end_line": 67,
    "token_count": 156,
    "score": 0.92,
    "span_id": "abc123",
    "deterministic_hash": "def456...",
    "query": "authentication"
}
```

## Configuration

### Retriever Parameters

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `url` | str | `"http://localhost:8765"` | Server URL |
| `mode` | str | `"http"` | Connection mode: `"http"` or `"cli"` |
| `budget` | int | `8000` | Token budget for context |
| `semantic_weight` | float | `0.7` | Semantic search weight (0-1) |
| `lexical_weight` | float | `0.3` | Lexical search weight (0-1) |
| `mmr_lambda` | float | `0.5` | MMR diversity (0=diverse, 1=relevant) |
| `enable_mmr` | bool | `True` | Enable MMR diversification |
| `include_citations` | bool | `True` | Include citation metadata |
| `include_scores` | bool | `True` | Include relevance scores |
| `combine_spans` | bool | `False` | Combine adjacent spans |
| `min_score` | float | `None` | Minimum score threshold |

### Connection Modes

**HTTP Mode** (Default)
```python
retriever = AvocadoDBRetriever(
    url="http://localhost:8765",
    mode="http"
)
```

- Connects to daemon server
- Multi-project support
- Persistent indexes in memory
- Best for production

**CLI Mode**
```python
retriever = AvocadoDBRetriever(
    mode="cli",
    db_path=".avocado/db.sqlite"
)
```

- Direct binary calls
- No server needed
- Per-directory databases
- Best for development

### Budget Guidelines

Token budget controls context size:

```python
# Small queries (fast, focused)
retriever = AvocadoDBRetriever(budget=3000)

# Medium queries (balanced)
retriever = AvocadoDBRetriever(budget=8000)  # Default

# Large queries (comprehensive)
retriever = AvocadoDBRetriever(budget=15000)
```

**Choosing a budget:**
- **3-5K**: Specific questions, fast responses
- **8-10K**: General questions, good coverage
- **15K+**: Complex questions, comprehensive analysis

### Search Configuration

**Hybrid Search Weights**
```python
# More semantic (better for concepts)
retriever = AvocadoDBRetriever(
    semantic_weight=0.9,
    lexical_weight=0.1
)

# More lexical (better for exact matches)
retriever = AvocadoDBRetriever(
    semantic_weight=0.5,
    lexical_weight=0.5
)
```

**MMR Diversity**
```python
# High diversity (avoid redundancy)
retriever = AvocadoDBRetriever(
    enable_mmr=True,
    mmr_lambda=0.3  # More diverse
)

# High relevance (allow redundancy)
retriever = AvocadoDBRetriever(
    enable_mmr=True,
    mmr_lambda=0.9  # More relevant
)
```

**Score Filtering**
```python
# Only high-quality matches
retriever = AvocadoDBRetriever(
    min_score=0.8,
    include_scores=True
)
```

## Usage Patterns

### Pattern 1: Simple QA

```python
from langchain_avocadodb import AvocadoDBRetriever
from langchain.chains import RetrievalQA
from langchain_openai import ChatOpenAI

retriever = AvocadoDBRetriever()
llm = ChatOpenAI(temperature=0)

qa_chain = RetrievalQA.from_chain_type(
    llm=llm,
    retriever=retriever,
    return_source_documents=True
)

result = qa_chain.invoke({"query": "How does X work?"})
```

### Pattern 2: Conversational RAG

```python
from langchain.chains import ConversationalRetrievalChain
from langchain.memory import ConversationBufferMemory

memory = ConversationBufferMemory(
    memory_key="chat_history",
    return_messages=True
)

chain = ConversationalRetrievalChain.from_llm(
    llm=llm,
    retriever=retriever,
    memory=memory
)

# Multi-turn conversation
chain({"question": "What database does the system use?"})
chain({"question": "How is it configured?"})  # References previous context
```

### Pattern 3: Agent with Tools

```python
from langchain.agents import create_openai_tools_agent, AgentExecutor
from langchain.tools.retriever import create_retriever_tool

# Create retriever tool
tool = create_retriever_tool(
    retriever,
    name="codebase_search",
    description="Search the codebase for information"
)

# Create agent
agent = create_openai_tools_agent(llm, [tool], prompt)
agent_executor = AgentExecutor(agent=agent, tools=[tool])

# Agent decides when to search
result = agent_executor.invoke({"input": "Find and explain auth code"})
```

### Pattern 4: Custom Prompts

```python
from langchain.prompts import PromptTemplate

template = """Use the following code context to answer the question.

Context:
{context}

Question: {question}

Provide a detailed technical answer with citations."""

prompt = PromptTemplate(
    template=template,
    input_variables=["context", "question"]
)

qa_chain = RetrievalQA.from_chain_type(
    llm=llm,
    retriever=retriever,
    chain_type_kwargs={"prompt": prompt}
)
```

### Pattern 5: Batch Processing

```python
questions = [
    "How is authentication implemented?",
    "What database is used?",
    "How are errors handled?"
]

results = []
for question in questions:
    result = qa_chain.invoke({"query": question})
    results.append(result)
```

## Advanced Features

### Map-Reduce for Large Contexts

For queries requiring lots of context:

```python
qa_chain = RetrievalQA.from_chain_type(
    llm=llm,
    chain_type="map_reduce",  # Process docs separately
    retriever=AvocadoDBRetriever(budget=20000)
)
```

### Combining Adjacent Spans

Merge nearby code for better context:

```python
retriever = AvocadoDBRetriever(
    combine_spans=True  # Merge spans within 5 lines
)
```

### VectorStore as Retriever

Use VectorStore interface for compatibility:

```python
vectorstore = AvocadoDBVectorStore()

# Similarity search
docs = vectorstore.similarity_search("query", k=5)

# With scores
results = vectorstore.similarity_search_with_score("query")

# MMR search
docs = vectorstore.max_marginal_relevance_search("query", k=5)

# As retriever
retriever = vectorstore.as_retriever(
    search_type="mmr",
    search_kwargs={"k": 5, "lambda_mult": 0.5}
)
```

### Async Support

```python
# Async retrieval
docs = await retriever.aget_relevant_documents("query")

# Async chains
result = await qa_chain.ainvoke({"query": "question"})
```

## Performance Optimization

### 1. Budget Optimization

Start small and increase if needed:

```python
# Development
retriever = AvocadoDBRetriever(budget=5000)

# Production (after testing)
retriever = AvocadoDBRetriever(budget=10000)
```

### 2. Enable MMR

Reduces redundancy and improves quality:

```python
retriever = AvocadoDBRetriever(
    enable_mmr=True,
    mmr_lambda=0.5
)
```

### 3. Score Filtering

Focus on high-quality matches:

```python
retriever = AvocadoDBRetriever(
    min_score=0.7,
    include_scores=True
)
```

### 4. Model Selection

```python
# Development: Fast and cheap
llm = ChatOpenAI(model="gpt-3.5-turbo", temperature=0)

# Production: High quality
llm = ChatOpenAI(model="gpt-4", temperature=0)
```

### 5. Caching

AvocadoDB automatically caches embeddings. For additional caching:

```python
from langchain.cache import InMemoryCache
from langchain.globals import set_llm_cache

set_llm_cache(InMemoryCache())
```

## Migration Guide

### From Pinecone

```python
# Before (Pinecone)
from langchain_pinecone import PineconeVectorStore
vectorstore = PineconeVectorStore.from_existing_index(index_name)

# After (AvocadoDB)
from langchain_avocadodb import AvocadoDBVectorStore
vectorstore = AvocadoDBVectorStore(url="http://localhost:8765")
```

### From Chroma

```python
# Before (Chroma)
from langchain_chroma import Chroma
vectorstore = Chroma(persist_directory="./chroma_db")

# After (AvocadoDB)
from langchain_avocadodb import AvocadoDBVectorStore
vectorstore = AvocadoDBVectorStore(url="http://localhost:8765")
```

### From FAISS

```python
# Before (FAISS)
from langchain_community.vectorstores import FAISS
vectorstore = FAISS.load_local("faiss_index", embeddings)

# After (AvocadoDB)
from langchain_avocadodb import AvocadoDBVectorStore
vectorstore = AvocadoDBVectorStore(url="http://localhost:8765")
# Note: AvocadoDB uses its own embeddings (6x faster!)
```

## Troubleshooting

### Server Connection Issues

**Problem**: `Connection refused` or `Server not found`

**Solution**:
```bash
# Check if server is running
curl http://localhost:8765/stats

# Start server
avocado-server

# Check port
netstat -an | grep 8765
```

### No Documents Retrieved

**Problem**: Empty results or no matches

**Solution**:
```bash
# Check if documents are ingested
avocado stats

# Ingest documents
avocado ingest . --recursive

# Verify ingestion
avocado compile "test query"
```

### Import Errors

**Problem**: `ModuleNotFoundError`

**Solution**:
```bash
# Install package
pip install langchain-avocadodb

# Install dependencies
pip install langchain langchain-openai

# Verify
python -c "from langchain_avocadodb import AvocadoDBRetriever"
```

### Poor Results Quality

**Problem**: Irrelevant or low-quality results

**Solutions**:
1. Increase budget: `budget=12000`
2. Adjust weights: `semantic_weight=0.8, lexical_weight=0.2`
3. Enable filtering: `min_score=0.7`
4. Enable MMR: `enable_mmr=True`
5. Try combining spans: `combine_spans=True`

### Slow Performance

**Problem**: Queries taking too long

**Solutions**:
1. Reduce budget: `budget=5000`
2. Use HTTP mode (faster): `mode="http"`
3. Disable citations if not needed: `include_citations=False`
4. Use smaller LLM model: `ChatOpenAI(model="gpt-3.5-turbo")`

## FAQ

**Q: Is AvocadoDB truly deterministic?**

A: Yes! Same query always returns identical context with same SHA-256 hash. This is verified by our test suite.

**Q: Do I need to provide embeddings?**

A: No! AvocadoDB uses its own Rust-based embeddings (6x faster than OpenAI) built into the binary.

**Q: Can I use AvocadoDB with other LLMs?**

A: Yes! Works with any LangChain-compatible LLM (Anthropic, Cohere, local models, etc.)

**Q: How does citation tracking work?**

A: Every retrieved span includes source file, start line, and end line in metadata. Citations are automatically included.

**Q: What's the difference between HTTP and CLI mode?**

A: HTTP mode connects to a daemon server (best for production). CLI mode calls the binary directly (best for development).

**Q: Can I use this with streaming responses?**

A: Yes! Retrieval is not streamed, but LLM responses can stream normally.

**Q: How do I update my documents?**

A: Re-ingest with `avocado ingest <path>`. AvocadoDB handles deduplication automatically.

**Q: Is there a query length limit?**

A: No hard limit, but optimal queries are 5-50 words. Very long queries may be less effective.

**Q: Can I use AvocadoDB for multiple projects?**

A: Yes! HTTP mode auto-detects projects by directory. CLI mode uses per-directory databases.

**Q: How much does it cost?**

A: AvocadoDB is open source (MIT license). Only costs are for LLM API calls (OpenAI, etc.)

**Q: What happens if I query before ingesting documents?**

A: You'll get empty results. Always ingest first: `avocado ingest . --recursive`

**Q: Can I customize the prompt template?**

A: Yes! Use `chain_type_kwargs={"prompt": your_prompt}` when creating chains.

**Q: How do I handle very large codebases?**

A: Use `map_reduce` chain type and higher budgets (15K+). AvocadoDB is optimized for large codebases.

**Q: Can I use AvocadoDB with LangGraph?**

A: Yes! AvocadoDBRetriever works with any LangChain-compatible framework including LangGraph.

**Q: How do I contribute or report issues?**

A: Open issues on GitHub: https://github.com/avocadodb/langchain-avocadodb

---

## Additional Resources

- [Examples](../integrations/langchain-avocadodb/examples/)
- [API Reference](https://docs.avocadodb.com/api)
- [AvocadoDB Documentation](https://docs.avocadodb.com)
- [LangChain Documentation](https://python.langchain.com)
- [GitHub Repository](https://github.com/avocadodb/langchain-avocadodb)

## Support

- GitHub Issues: https://github.com/avocadodb/langchain-avocadodb/issues
- Discord Community: https://discord.gg/avocadodb
- Email: support@avocadodb.com

---

**Built with love by the AvocadoDB team** 🥑
