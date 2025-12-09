# LangChain-AvocadoDB Examples

This directory contains comprehensive examples demonstrating how to use AvocadoDB with LangChain for various RAG (Retrieval-Augmented Generation) patterns.

## Prerequisites

Before running these examples, ensure you have:

1. **AvocadoDB server running**:
   ```bash
   avocado-server
   ```

2. **Documents ingested**:
   ```bash
   avocado ingest . --recursive
   ```

3. **Dependencies installed**:
   ```bash
   pip install langchain-avocadodb langchain langchain-openai
   ```

4. **OpenAI API key set**:
   ```bash
   export OPENAI_API_KEY="your-key-here"
   ```

## Examples Overview

### 1. Basic RAG (`basic_rag.py`)

**What it demonstrates:**
- Setting up AvocadoDBRetriever with custom configuration
- Using RetrievalQA chain for question answering
- Getting deterministic, citation-backed answers
- Different retrieval strategies (MMR, filtering, span combination)

**Key concepts:**
- Deterministic retrieval (same query = same results)
- Line-level citations with source tracking
- Hybrid semantic + lexical search
- Maximal Marginal Relevance (MMR) for diverse results

**Run it:**
```bash
python basic_rag.py
```

**Best for:** Getting started with AvocadoDB and LangChain, understanding basic RAG patterns.

---

### 2. Conversational RAG (`conversational_rag.py`)

**What it demonstrates:**
- Building a conversational interface with memory
- Multi-turn conversations with context retention
- Follow-up questions that reference previous answers
- Combining chat history with retrieval

**Key concepts:**
- ConversationBufferMemory for chat history
- Context-aware question reformulation
- Persistent conversation state
- Interactive CLI interface

**Run it:**
```bash
# Interactive mode
python conversational_rag.py

# Demo mode (programmatic examples)
python conversational_rag.py --demo
```

**Example interaction:**
```
You: How does authentication work?
Assistant: The authentication system uses JWT tokens...

You: What algorithm does it use?  # References previous context
Assistant: It uses HS256 algorithm...
```

**Best for:** Building chatbots, interactive documentation assistants, customer support tools.

---

### 3. Agent with Memory (`agent_with_memory.py`)

**What it demonstrates:**
- Creating an agent that can use AvocadoDB as a search tool
- Combining retrieval with other tools
- Agent reasoning and tool selection
- Multi-step problem solving

**Key concepts:**
- LangChain agents and tool use
- create_retriever_tool for AvocadoDB
- Agent reasoning with multiple tools
- Verbose mode to see agent thinking

**Run it:**
```bash
# Interactive agent
python agent_with_memory.py

# Pre-defined task examples
python agent_with_memory.py --demo

# With custom tools
python agent_with_memory.py --custom
```

**Agent capabilities:**
- Decides when to search the codebase
- Can break down complex questions
- Uses multiple tools in sequence
- Synthesizes information from multiple sources

**Best for:** Complex queries requiring reasoning, multi-step analysis, combining retrieval with computation.

---

### 4. Advanced QA Chains (`qa_chain.py`)

**What it demonstrates:**
- Different chain types (stuff, map_reduce, refine)
- Custom prompts for better answers
- Source attribution and citation formatting
- Batch processing multiple questions
- Exporting results to JSON

**Key concepts:**
- Chain type selection based on context size
- Custom prompt templates
- Score filtering for quality control
- Batch processing for efficiency

**Run it:**
```bash
python qa_chain.py
```

**Chain types explained:**
- **Stuff**: All context in one prompt (default, fastest)
- **Map-Reduce**: Process docs separately, combine results (for large contexts)
- **Refine**: Iteratively refine answer (highest quality)

**Best for:** Documentation generation, FAQ creation, systematic code analysis, large codebases.

---

## Common Patterns

### Deterministic Retrieval

All examples leverage AvocadoDB's determinism:

```python
# Same query always returns same context
result1 = retriever.get_relevant_documents("auth")
result2 = retriever.get_relevant_documents("auth")

# Verify determinism
hash1 = result1[0].metadata["deterministic_hash"]
hash2 = result2[0].metadata["deterministic_hash"]
assert hash1 == hash2  # Always true!
```

### Citation Tracking

Every retrieved document includes precise citations:

```python
for doc in result["source_documents"]:
    print(f"Source: {doc.metadata['source']}")
    print(f"Lines: {doc.metadata['start_line']}-{doc.metadata['end_line']}")
    print(f"Score: {doc.metadata['score']}")
```

### Hybrid Search

AvocadoDB combines semantic and lexical search:

```python
retriever = AvocadoDBRetriever(
    semantic_weight=0.7,  # Vector similarity
    lexical_weight=0.3,   # Keyword matching
)
```

### MMR for Diversity

Get diverse results to avoid redundancy:

```python
retriever = AvocadoDBRetriever(
    enable_mmr=True,
    mmr_lambda=0.5,  # 0=diverse, 1=relevant
)
```

## Configuration Guide

### Budget (Token Limit)

Controls how much context to retrieve:

```python
retriever = AvocadoDBRetriever(
    budget=8000,  # Default: 8K tokens
)
```

**Guidelines:**
- Small queries (3-5K): Fast, focused answers
- Medium queries (8-10K): Balanced coverage
- Large queries (15K+): Comprehensive analysis

### Score Filtering

Only retrieve high-quality matches:

```python
retriever = AvocadoDBRetriever(
    min_score=0.7,  # Only scores >= 0.7
    include_scores=True,
)
```

**Guidelines:**
- 0.9+: Highly specific queries
- 0.7-0.9: General queries
- <0.7: Exploratory queries

### Span Combination

Merge adjacent code spans:

```python
retriever = AvocadoDBRetriever(
    combine_spans=True,  # Merge nearby spans
)
```

**Use when:**
- You need larger context windows
- Related code is split across spans
- You want fewer, larger documents

## Troubleshooting

### Server Not Found

```bash
# Check if server is running
curl http://localhost:8765/stats

# Start if needed
avocado-server
```

### No Documents Retrieved

```bash
# Check ingestion
avocado stats

# Ingest if needed
avocado ingest . --recursive
```

### Import Errors

```bash
# Install all dependencies
pip install langchain-avocadodb langchain langchain-openai langchain-community
```

### OpenAI API Errors

```bash
# Set API key
export OPENAI_API_KEY="sk-..."

# Or use in code
import os
os.environ["OPENAI_API_KEY"] = "sk-..."
```

## Performance Tips

1. **Start with smaller budgets** (3-5K tokens) and increase if needed
2. **Enable MMR** for diverse results unless you need redundancy
3. **Use score filtering** (min_score=0.7) for focused queries
4. **Combine spans** for better context windows
5. **Use GPT-3.5-turbo** for development, GPT-4 for production
6. **Set temperature=0** for deterministic answers

## Next Steps

1. **Customize prompts** to match your use case
2. **Add more tools** to the agent (calculator, APIs, etc.)
3. **Integrate with your app** using the patterns shown
4. **Build evaluation** using deterministic hashes
5. **Monitor performance** and optimize budgets

## Additional Resources

- [AvocadoDB Documentation](https://docs.avocadodb.com)
- [LangChain Documentation](https://python.langchain.com)
- [Integration Guide](../docs/LANGCHAIN_INTEGRATION.md)
- [API Reference](https://docs.avocadodb.com/api)

## Contributing

Found a bug or have an example to share? Open an issue or PR on GitHub!

---

**Happy Building!** 🥑
