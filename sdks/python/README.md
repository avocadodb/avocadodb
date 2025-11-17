# AvocadoDB Python SDK

**Framework-agnostic SDK for AvocadoDB** - the deterministic context database for AI agents.

## 🚀 What's New in v2.0

The SDK has been completely refactored to be **framework-agnostic**, moving all intelligence from framework-specific code into reusable SDK primitives.

### New Features

- ✅ **Local LLM Support**: Optional TinyLlama integration for natural language answers
- ✅ **Server Lifecycle Management**: Auto-start, health checks, daemon mode
- ✅ **Background File Monitoring**: Auto-detect and re-ingest changed files
- ✅ **Smart Auto-Ingest**: Project type detection (Python/Node/Rust/Go/etc.)
- ✅ **Framework Integrations**: LangChain, AutoGen, CrewAI support
- ✅ **Utilities**: Token counting, citation formatting, prompt generation
- ✅ **100% Backward Compatible**: All v1.0 APIs still work

## 📦 Installation

```bash
# Basic installation (RAG only)
pip install avocadodb

# With LLM support (TinyLlama)
pip install avocadodb[llm]
```

Or from source:
```bash
cd sdks/python
pip install -e .          # Basic
pip install -e .[llm]     # With LLM support
```

## 🎯 Quick Start

### Basic Usage (HTTP Client)

```python
from avocado import AvocadoDB

# Connect to server
db = AvocadoDB("http://localhost:8765")

# Compile context
result = db.compile("How does authentication work?")

# Use the context
print(f"Context ({result.tokens_used} tokens):")
print(result.text)

# Show citations
for citation in result.citations:
    print(f"  - {citation.artifact_path}:{citation.start_line}-{citation.end_line}")
```

### Ask Questions (v2.0 - New!)

Get natural language answers using TinyLlama:

```python
from avocado import AvocadoDB

db = AvocadoDB("http://localhost:8765")

# Ask a question - uses TinyLlama if available
answer = db.ask("How does authentication work?")
print(answer)

# Options:
# - llm="auto" (default): Try TinyLlama, fallback to context
# - llm="local": Require TinyLlama (raises error if not available)
# - llm="none": Just return context (same as compile())
```

### With Auto-Management

```python
from avocado import get_manager, AutoIngest

# Auto-start server (daemon mode)
manager = get_manager()
manager.ensure_running()

# Auto-ingest project (detects Python/Node/Rust/etc.)
ingester = AutoIngest()
result = ingester.ingest_project(".", max_files=100)
print(f"Ingested {result['ingested']} files ({result['project_type']} project)")

# Compile context
from avocado import AvocadoDB
db = AvocadoDB()
context = db.compile("How does the authentication module work?")
```

### Background File Monitoring

```python
from avocado import FileMonitor

# Monitor files and auto-re-ingest on changes
monitor = FileMonitor(interval_seconds=30)
monitor.start_monitoring([
    "docs/**/*.md",
    "src/**/*.py",
    "README.md"
])

# Optional: callback for change events
def on_change(files):
    print(f"Re-ingested {len(files)} changed files")

monitor.on_change(on_change)

# Monitor runs in background thread
# Stop when done: monitor.stop_monitoring()
```

## 🔧 Framework Integrations

### LangChain / DeepAgents

```python
from avocado.integrations.langchain import avocado_compile_context, AvocadoDBMiddleware
from deepagents import create_deep_agent

agent = create_deep_agent(
    model="claude-3-5-sonnet-20241022",
    tools=[avocado_compile_context],  # Auto-start, auto-ingest built-in!
    middleware=[AvocadoDBMiddleware()]  # Blocks sequential read tools
)

# The agent will automatically:
# 1. Start AvocadoDB server (daemon mode on port 8765)
# 2. Auto-ingest current directory on first query
# 3. Use AvocadoDB exclusively for codebase questions
```

### AutoGen

```python
from autogen import AssistantAgent, UserProxyAgent
from avocado.integrations.autogen import avocado_compile_context

assistant = AssistantAgent(
    name="assistant",
    llm_config={"model": "gpt-4"},
    functions=[avocado_compile_context]  # Auto-start built-in!
)

user_proxy = UserProxyAgent(name="user")
user_proxy.initiate_chat(assistant, message="How does authentication work?")
```

### CrewAI

```python
from crewai import Agent, Task, Crew
from avocado.integrations.crewai import AvocadoDBTool

agent = Agent(
    role="Research Assistant",
    goal="Answer questions about the codebase",
    tools=[AvocadoDBTool()]  # Auto-start built-in!
)

task = Task(
    description="Explain how the authentication module works",
    agent=agent
)

crew = Crew(agents=[agent], tasks=[task])
result = crew.kickoff()
```

## 📚 API Reference

### Core Client

#### `AvocadoDB(url: str = "http://localhost:8080")`

HTTP client for AvocadoDB server.

**Methods:**
- `compile(query, budget=8000, ...)` → `WorkingSet`
- `ingest(path, content=None)` → `dict`
- `stats()` → `dict`

**Example:**
```python
from avocado import AvocadoDB

db = AvocadoDB("http://localhost:8765")
result = db.compile("authentication", budget=8000)
print(f"Compiled {len(result.spans)} spans")
```

### Server Management

#### `AvocadoDBManager(auto_start=True, port=8765)`

Manages server lifecycle (auto-start, health checks, daemon mode).

**Methods:**
- `ensure_running()` → `bool` - Start server if not running
- `is_running()` → `bool` - Check if server is reachable
- `start_server()` → `bool` - Start server as daemon
- `stop_server()` - Stop server subprocess
- `get_stats()` → `dict` - Get database statistics
- `health_check()` → `dict` - Comprehensive health check

**Example:**
```python
from avocado import AvocadoDBManager

manager = AvocadoDBManager(auto_start=True, port=8765)
if manager.ensure_running():
    stats = manager.get_stats()
    print(f"Indexed: {stats['artifacts_count']} docs")
```

#### `get_manager()` → `AvocadoDBManager`

Get global manager instance (singleton pattern).

### Auto-Ingest

#### `AutoIngest(ingest_binary=None)`

Smart auto-ingestion with project type detection.

**Methods:**
- `detect_project_type(path)` → `str` - Detect Python/Node/Rust/etc.
- `get_patterns_for_project(project_type)` → `list[str]` - Get file patterns
- `ingest_project(path, max_files=100, ...)` → `dict` - Ingest entire project
- `ingest_file(path)` → `bool` - Ingest single file

**Example:**
```python
from avocado import AutoIngest

ingester = AutoIngest()

# Auto-detect and ingest
result = ingester.ingest_project(".", max_files=100)
print(f"Ingested {result['ingested']} files")
print(f"Project type: {result['project_type']}")
```

### File Monitoring

#### `FileMonitor(interval_seconds=30, ingest_binary=None)`

Background file watcher for automatic re-ingestion.

**Methods:**
- `start_monitoring(patterns)` - Start background monitoring
- `stop_monitoring()` - Stop monitoring thread
- `on_change(callback)` - Register change event handler
- `is_monitoring()` → `bool` - Check if monitoring is active

**Example:**
```python
from avocado import FileMonitor

monitor = FileMonitor(interval_seconds=30)
monitor.start_monitoring(["docs/**/*.md", "src/**/*.py"])

def on_change(files):
    print(f"Changed: {[str(f) for f in files]}")

monitor.on_change(on_change)
```

### Utilities

#### `count_tokens(text, model="gpt-4")` → `int`

Exact token count using tiktoken.

#### `format_citations(citations, style="compact")` → `str`

Format citations for display.

**Styles:** `compact`, `verbose`, `markdown`

#### `create_system_prompt(framework="generic", enforce_avocado_only=True)` → `str`

Generate AvocadoDB-first system prompt.

#### `format_working_set(working_set, include_context=False)` → `str`

Format WorkingSet for human-readable display.

## 🔄 Migration from v1.0

### What Changed

**v1.0** (Thin HTTP wrapper):
```python
from avocado import AvocadoDB
db = AvocadoDB()
# Manual server management required
```

**v2.0** (Full-featured SDK):
```python
from avocado import get_manager, AvocadoDB

# Auto-start server
manager = get_manager()
manager.ensure_running()

# Use client
db = AvocadoDB()
```

### Backward Compatibility

**All v1.0 APIs still work!** The new features are additive:

```python
# v1.0 code (still works in v2.0)
from avocado import AvocadoDB
db = AvocadoDB()
result = db.compile("query")

# v2.0 code (new features)
from avocado import get_manager, AutoIngest, FileMonitor
manager = get_manager()  # Auto-start
ingester = AutoIngest()  # Auto-ingest
monitor = FileMonitor()  # Auto-monitor
```

## 🌐 Environment Variables

- `AVOCADODB_URL` - Server URL (default: `http://localhost:8765`)
- `AVOCADODB_AUTO_START` - Enable auto-start (default: `true`)

## 📖 Examples

See `examples/` directory for:
- Basic HTTP client usage
- Auto-management with lifecycle
- Background monitoring
- Framework integrations (LangChain, AutoGen, CrewAI)

## 🤝 Contributing

Contributions welcome! See [CONTRIBUTING.md](../../CONTRIBUTING.md)

## 📄 License

MIT License - see [LICENSE](../../LICENSE)

## 🔗 Links

- [GitHub](https://github.com/servesys-labs/avacadodb)
- [Documentation](https://github.com/servesys-labs/avacadodb/tree/main/docs)
- [DeepAgents Integration](https://github.com/servesys-labs/deepagents-avocado)
