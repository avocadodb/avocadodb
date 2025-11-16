# AvocadoDB + LangChain DeepAgents Integration

**Enable deterministic, citation-backed context retrieval in your DeepAgents.**

## Overview

This integration provides a tool for LangChain's DeepAgents that enables:
- **100% Deterministic** context compilation (same query → same context)
- **Citation-backed** responses with exact line numbers
- **Token efficient** retrieval (90-95% budget utilization)
- **Seamless integration** with the DeepAgents framework

## Why Use AvocadoDB with DeepAgents?

| Problem | Without AvocadoDB | With AvocadoDB |
|---------|-------------------|----------------|
| **Reproducibility** | Different results each run | Identical results every time |
| **Citations** | No source tracking | Exact file:line citations |
| **Token efficiency** | 60-70% utilization | 90-95% utilization |
| **Duplicates** | Common in results | Zero duplicates guaranteed |

## Installation

```bash
# Install both packages
pip install deepagents avocadodb

# Or from source
cd sdks/python
pip install -e .
pip install deepagents
```

## Quick Start

### 1. Start AvocadoDB Server

```bash
# Start the server
./target/release/avocado-server &

# Ingest your documentation
./target/release/avocado ingest ./docs --recursive
./target/release/avocado ingest ./src --recursive
```

### 2. Create a DeepAgent with AvocadoDB

```python
from deepagents import create_deep_agent
from avocado import avocado_compile_context

# Create agent with AvocadoDB tool
agent = create_deep_agent(
    tools=[avocado_compile_context],
    system_prompt="""You are a helpful code assistant with access
    to a deterministic knowledge base through AvocadoDB.

    Use the avocado_compile_context tool to retrieve relevant context
    from the codebase, then synthesize natural responses with citations."""
)

# Use it
result = agent.invoke({
    "messages": [{"role": "user", "content": "How does authentication work?"}]
})

print(result["messages"][-1].content)
```

### 3. See It In Action

```bash
python sdks/python/examples/deepagent_example.py
```

## Tool Reference

### `avocado_compile_context(query, ...)`

Compile deterministic, citation-backed context from your knowledge base.

**Parameters:**
- `query` (str): Search query describing what information you need
- `token_budget` (int): Maximum tokens to use (default: 8000)
- `semantic_weight` (float): Weight for semantic search 0.0-1.0 (default: 0.7)
- `lexical_weight` (float): Weight for keyword search 0.0-1.0 (default: 0.3)
- `mmr_lambda` (float): Diversity parameter 0.0-1.0 (default: 0.5)
- `enable_mmr` (bool): Enable diversification (default: True)

**Returns:**
```python
{
    "success": True,
    "context": "...compiled context text...",
    "citations": [
        {"file": "docs/auth.md", "lines": "10-25"},
        {"file": "src/auth.py", "lines": "45-78"}
    ],
    "spans": 12,
    "tokens_used": 7891,
    "compilation_time_ms": 243,
    "deterministic_hash": "e3b0c44298fc1c149afb...",
    "query": "How does authentication work?"
}
```

## Advanced Usage

### Custom System Prompt

```python
system_prompt = """You are an expert code documentation assistant.

## Available Tools

### `avocado_compile_context`
Retrieves deterministic, citation-backed context from the codebase.

**When to use:**
- Answering questions about code architecture
- Explaining API endpoints
- Finding implementation details
- Locating test documentation

**How to use:**
1. Call the tool with a specific query
2. Read the 'context' field for relevant information
3. Synthesize a natural response
4. Cite sources using the 'citations' field

**Important:** Always provide formatted responses with citations,
never show raw JSON output.
"""

agent = create_deep_agent(
    tools=[avocado_compile_context],
    system_prompt=system_prompt,
)
```

### Tuning Parameters

```python
# For more diverse results (broader topic coverage)
agent.invoke({
    "messages": [{
        "role": "user",
        "content": "Find all authentication methods",
        "tool_calls": [{
            "name": "avocado_compile_context",
            "args": {
                "query": "authentication methods",
                "mmr_lambda": 0.3,  # Lower = more diversity
            }
        }]
    }]
})

# For more focused results (similar topics)
agent.invoke({
    "messages": [{
        "role": "user",
        "content": "Explain JWT token validation",
        "tool_calls": [{
            "name": "avocado_compile_context",
            "args": {
                "query": "JWT token validation",
                "mmr_lambda": 0.8,  # Higher = more focused
            }
        }]
    }]
})

# For keyword-heavy searches (code snippets)
agent.invoke({
    "messages": [{
        "role": "user",
        "content": "Find the validateToken function",
        "tool_calls": [{
            "name": "avocado_compile_context",
            "args": {
                "query": "validateToken function",
                "lexical_weight": 0.6,  # More keyword matching
                "semantic_weight": 0.4,
            }
        }]
    }]
})
```

### Multiple Knowledge Bases

```python
from avocado.deepagents_tool import get_avocadodb_client

# Create custom tools for different knowledge bases
def frontend_context(query: str, **kwargs):
    """Retrieve context from frontend codebase."""
    client = get_avocadodb_client("http://localhost:8080")
    result = client.compile(query, **kwargs)
    # ... format response

def backend_context(query: str, **kwargs):
    """Retrieve context from backend codebase."""
    client = get_avocadodb_client("http://localhost:8081")
    result = client.compile(query, **kwargs)
    # ... format response

agent = create_deep_agent(
    tools=[frontend_context, backend_context],
    system_prompt="You have access to both frontend and backend knowledge bases..."
)
```

## Integration with DeepAgents CLI

To add AvocadoDB to the DeepAgents CLI, copy the integration file:

```bash
# Copy the AvocadoDB integration
cp sdks/python/avocado/deepagents_tool.py \
   ~/orion/deepagents/libs/deepagents-cli/deepagents_cli/integrations/avocadodb.py
```

Then import and use in the CLI's `tools.py`:

```python
from deepagents_cli.integrations.avocadodb import avocado_compile_context

# The tool is now available in the CLI
```

## Complete Example

See `sdks/python/examples/deepagent_example.py` for a complete working example.

```python
#!/usr/bin/env python3
from deepagents import create_deep_agent
from avocado import avocado_compile_context

# Create documentation assistant
agent = create_deep_agent(
    tools=[avocado_compile_context],
    system_prompt="""You are an expert code documentation assistant.
    Use avocado_compile_context to retrieve deterministic, citation-backed
    context from the knowledge base."""
)

# Query the agent
result = agent.invoke({
    "messages": [{"role": "user", "content": "How does authentication work?"}]
})

# Print response
print(result["messages"][-1].content)
```

## Benefits

### Determinism

```python
# Same query, same context, every time
for i in range(3):
    result = agent.invoke({"messages": [{"role": "user", "content": "auth flow"}]})
    # All 3 responses will use identical context with same hash
```

### Citations

```python
# Every response includes exact source locations
{
    "context": "Our auth system uses JWT tokens...",
    "citations": [
        {"file": "docs/auth.md", "lines": "10-25"},
        {"file": "src/middleware/auth.ts", "lines": "45-78"}
    ]
}
```

### Token Efficiency

```python
# 90-95% budget utilization (vs 60-70% with traditional RAG)
{
    "tokens_used": 7891,  # Asked for 8000
    "token_budget": 8000,
    "utilization": "98.6%"
}
```

## Troubleshooting

### Server Not Reachable

```python
# Check if server is running
from avocado import AvocadoDB
try:
    db = AvocadoDB()
    stats = db.stats()
    print(f"✅ Server running: {stats['spans']} spans")
except Exception as e:
    print("❌ Server not reachable")
    print("Start with: ./target/release/avocado-server")
```

### No Context Returned

```bash
# Make sure documents are ingested
./target/release/avocado stats

# If empty, ingest documents
./target/release/avocado ingest ./docs --recursive
```

### Environment Variables

```bash
# Set AvocadoDB server URL (optional)
export AVOCADODB_URL="http://localhost:8080"

# Set API keys for DeepAgents
export ANTHROPIC_API_KEY="sk-ant-..."
export OPENAI_API_KEY="sk-..."
```

## Next Steps

- See [examples/deepagent_example.py](../sdks/python/examples/deepagent_example.py) for complete code
- Read [DeepAgents documentation](https://docs.langchain.com/oss/python/deepagents/overview)
- Explore [AvocadoDB examples](examples.md) for more use cases

## Support

- Issues: https://github.com/avocadodb/avocadodb/issues
- DeepAgents: https://github.com/langchain-ai/deepagents
- Docs: https://docs.langchain.com/oss/python/deepagents/overview
