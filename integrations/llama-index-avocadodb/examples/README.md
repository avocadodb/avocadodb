# AvocadoDB + LlamaIndex Examples

Comprehensive examples demonstrating the AvocadoDB integration with LlamaIndex.

## Prerequisites

Before running these examples, ensure you have:

1. **AvocadoDB server running**:
   ```bash
   avocado-server
   ```

2. **Documents ingested**:
   ```bash
   avocado ingest /path/to/your/codebase --recursive
   ```

3. **OpenAI API key configured**:
   ```bash
   export OPENAI_API_KEY=your-key-here
   ```

4. **Dependencies installed**:
   ```bash
   pip install llama-index-avocadodb llama-index llama-index-llms-openai llama-index-embeddings-openai
   ```

## Examples Overview

### 1. Basic RAG (`basic_rag.py`)

**Difficulty**: Beginner
**Time to run**: 2-3 minutes
**What it demonstrates**:
- Basic AvocadoDBReader setup
- Document loading from queries
- Creating VectorStoreIndex
- Query engines with citations
- TextNode conversion
- Batch loading
- Lazy loading
- Combined documents
- Chat engines

**Run it**:
```bash
python examples/basic_rag.py
```

**Key concepts**:
- How to initialize the reader
- Loading documents for queries
- Citation tracking
- Different loading patterns

**When to use this pattern**:
- Simple RAG applications
- One-off queries
- Quick prototyping

---

### 2. Conversational Index (`conversational_index.py`)

**Difficulty**: Intermediate
**Time to run**: 3-5 minutes
**What it demonstrates**:
- Session-based conversations
- Context evolution over time
- Memory management
- Dynamic context expansion
- Multi-session handling
- Session persistence

**Run it**:
```bash
python examples/conversational_index.py
```

**Key concepts**:
- Building conversational agents
- Session management
- Progressive context loading
- Conversation history tracking
- Saving/loading sessions

**When to use this pattern**:
- Chatbots and assistants
- Interactive code exploration
- Debugging sessions
- Technical Q&A systems

**Example output**:
```
Session: tech-qa-001
User: What authentication methods are supported?
Assistant: The system supports JWT tokens and OAuth2...
  → Expanding context based on: 'JWT validation'
  → Added 5 new documents to context
```

---

### 3. Advanced Query Engine (`query_engine_advanced.py`)

**Difficulty**: Advanced
**Time to run**: 5-7 minutes
**What it demonstrates**:
- Multiple response modes (compact, tree_summarize, etc.)
- Custom retrievers with post-processing
- Citation verification
- Multi-hop reasoning
- Streaming responses
- Batch query processing
- Deterministic retrieval verification
- Performance benchmarking

**Run it**:
```bash
python examples/query_engine_advanced.py
```

**Key concepts**:
- Advanced query engine configuration
- Post-processors for filtering
- Citation tracking and display
- Complex reasoning patterns
- Performance optimization
- Deterministic behavior

**When to use this pattern**:
- Production RAG systems
- Complex question answering
- Systems requiring provenance
- Performance-critical applications

**Example output**:
```
COMPACT
  Description: Combine text chunks into larger context
  Response (0.15s): JWT validation is implemented using...
  Sources: 3 nodes

TREE_SUMMARIZE
  Description: Build summary tree from chunks
  Response (0.89s): The authentication system validates...
  Sources: 5 nodes
```

---

### 4. Chat Engine (`chat_engine.py`)

**Difficulty**: Intermediate
**Time to run**: 4-6 minutes
**What it demonstrates**:
- Context chat mode
- Condense chat mode
- Memory persistence across sessions
- Multi-index conversations
- Citation tracking in chat
- Smart context loading

**Run it**:
```bash
python examples/chat_engine.py
```

**Key concepts**:
- Different chat modes and when to use them
- Persisting conversation state
- Switching between knowledge bases
- Dynamic context management
- Source attribution in chat

**When to use this pattern**:
- Conversational interfaces
- Multi-turn interactions
- Context-aware assistants
- Customer support bots

**Example output**:
```
User: What authentication methods are available?
Assistant: The system supports JWT and OAuth2...

Sources:
  - auth/jwt.py:15
  - auth/oauth.py:23

User: How is JWT token validation handled?
Assistant: Based on our previous discussion, JWT validation...
```

---

## Running All Examples

To run all examples sequentially:

```bash
for example in examples/*.py; do
    echo "Running $example..."
    python "$example"
    echo "---"
done
```

## Common Patterns

### Pattern 1: Simple Query

```python
from llama_index_avocadodb import AvocadoDBReader
from llama_index.core import VectorStoreIndex

reader = AvocadoDBReader(url="http://localhost:8765")
documents = reader.load_data("your query here")
index = VectorStoreIndex.from_documents(documents)
response = index.as_query_engine().query("your question")
print(response)
```

### Pattern 2: With Citations

```python
reader = AvocadoDBReader(include_citations=True)
documents = reader.load_data("your query")
index = VectorStoreIndex.from_documents(documents)
response = index.as_query_engine().query("question")

# Show sources
for node in response.source_nodes:
    print(f"{node.metadata['file_path']}:{node.metadata['start_line']}")
```

### Pattern 3: Chat with Memory

```python
from llama_index.core.memory import ChatMemoryBuffer

reader = AvocadoDBReader()
documents = reader.load_data("context query")
index = VectorStoreIndex.from_documents(documents)

chat_engine = index.as_chat_engine(
    chat_mode="context",
    memory=ChatMemoryBuffer.from_defaults(token_limit=3000)
)

response = chat_engine.chat("your message")
```

### Pattern 4: Batch Processing

```python
reader = AvocadoDBReader()
queries = ["query1", "query2", "query3"]
all_docs = reader.load_data_batch(queries, budget=5000)

for query, docs in zip(queries, all_docs):
    print(f"{query}: {len(docs)} documents")
```

## Configuration Options

### Reader Configuration

```python
reader = AvocadoDBReader(
    url="http://localhost:8765",     # Server URL
    budget=8000,                      # Token budget
    semantic_weight=0.7,              # Semantic search weight
    lexical_weight=0.3,               # Keyword search weight
    mmr_lambda=0.5,                   # Diversity parameter
    enable_mmr=True,                  # Enable diversification
    include_citations=True,           # Include source citations
    include_scores=True,              # Include relevance scores
    create_nodes=False,               # Return TextNodes vs Documents
    combine_adjacent=False,           # Combine nearby spans
    min_score=None                    # Minimum score threshold
)
```

### Query Engine Configuration

```python
query_engine = index.as_query_engine(
    response_mode="compact",          # Response synthesis mode
    similarity_top_k=5,               # Number of results
    verbose=True,                     # Enable logging
    streaming=False                   # Stream responses
)
```

## Performance Tips

1. **Use appropriate budgets**: Start with 5000-10000 tokens
2. **Enable MMR for diversity**: Reduces redundant results
3. **Combine adjacent spans**: Better context for related code
4. **Set min_score threshold**: Filter low-quality matches
5. **Use batch loading**: More efficient for multiple queries
6. **Lazy loading**: Memory-efficient for large result sets

## Troubleshooting

### Server Connection Error

```bash
# Check if server is running
curl http://localhost:8765/stats

# Start server if needed
avocado-server
```

### No Documents Returned

```bash
# Check ingestion status
avocado stats

# Re-ingest if needed
avocado ingest /path/to/code --recursive
```

### Import Errors

```bash
# Install all required packages
pip install llama-index-avocadodb llama-index llama-index-llms-openai llama-index-embeddings-openai
```

### OpenAI API Errors

```bash
# Verify API key is set
echo $OPENAI_API_KEY

# Set it if needed
export OPENAI_API_KEY=your-key-here
```

## Example Modifications

### Changing the Query

Find this line in any example:
```python
documents = reader.load_data("authentication implementation")
```

Change it to your query:
```python
documents = reader.load_data("your custom query here")
```

### Using Different Models

```python
from llama_index.core import Settings
from llama_index.llms.openai import OpenAI

# Use GPT-4
Settings.llm = OpenAI(model="gpt-4", temperature=0)

# Use GPT-3.5 Turbo (faster, cheaper)
Settings.llm = OpenAI(model="gpt-3.5-turbo", temperature=0)
```

### Adjusting Token Budget

```python
# Lower budget = faster, less context
documents = reader.load_data("query", budget=3000)

# Higher budget = slower, more context
documents = reader.load_data("query", budget=15000)
```

## Next Steps

1. **Start with `basic_rag.py`** to understand fundamentals
2. **Try `conversational_index.py`** for interactive use cases
3. **Explore `query_engine_advanced.py`** for production patterns
4. **Use `chat_engine.py`** for conversational applications

## Additional Resources

- [AvocadoDB Documentation](https://docs.avocadodb.com)
- [LlamaIndex Documentation](https://docs.llamaindex.ai)
- [Integration Guide](../docs/LLAMAINDEX_INTEGRATION.md)
- [API Reference](https://docs.avocadodb.com/integrations/llamaindex)

## Contributing

Found a bug or have an example to add? Please open an issue or PR!

## License

MIT License - see LICENSE for details
