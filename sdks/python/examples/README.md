# AvocadoDB Python SDK Examples

Comprehensive examples demonstrating AvocadoDB features and real-world usage patterns.

## Prerequisites

```bash
# Install the SDK
pip install -e ..

# Start the AvocadoDB server (required for some examples)
cargo run --bin avocado-server
```

## Examples Overview

### Basic Examples

#### 1. `example.py` - Basic Usage
The simplest introduction to AvocadoDB.

**Features demonstrated:**
- Connecting to AvocadoDB
- Ingesting documents
- Compiling context
- Basic querying

**Run:**
```bash
python example.py
```

#### 2. `cli_mode_example.py` - CLI Mode
Using AvocadoDB without a server.

**Features demonstrated:**
- Direct database access
- CLI mode operations
- Ingest and compile without HTTP

**Run:**
```bash
python cli_mode_example.py
```

#### 3. `ask_example.py` - Natural Language Q&A
Using the `ask()` method for direct answers.

**Features demonstrated:**
- Natural language queries
- LLM integration
- Getting direct answers (not just context)

**Run:**
```bash
python ask_example.py
```

### Session Management Examples

#### 4. `session_example.py` - Basic Sessions
Introduction to session management.

**Features demonstrated:**
- Creating sessions
- Multi-turn conversations
- Adding messages
- Getting conversation history

**Use case:** Building a simple chatbot

**Run:**
```bash
python session_example.py
```

#### 5. `session_replay_example.py` - Session Replay
Debugging agent behavior with session replay.

**Features demonstrated:**
- Session replay for debugging
- Analyzing context quality
- Tracking token usage
- Exporting session data

**Use case:** Debugging why an agent gave a specific answer

**Run:**
```bash
python session_replay_example.py
```

#### 6. `session_agent_memory.py` - Agent with Memory
Building an AI agent that maintains conversation context.

**Features demonstrated:**
- Agent conversation loops
- Context-aware responses
- Session persistence across restarts
- Multi-user session management
- Token-limited history

**Use case:** Production chatbot with conversation memory

**Run:**
```bash
python session_agent_memory.py
```

**Interactive demos:**
1. Multi-turn conversation
2. Token-limited history
3. Session persistence
4. Multiple sessions

#### 7. `session_debugging.py` - Advanced Debugging
Advanced debugging and analysis tools.

**Features demonstrated:**
- Session replay analysis
- Context quality scoring
- Token usage tracking
- Citation analysis
- Export for offline analysis
- Token budget comparison

**Use case:** Optimizing agent performance and context quality

**Run:**
```bash
python session_debugging.py
```

**Available tools:**
1. Session Replay Analysis
2. Context Quality Analysis
3. Token Usage Analysis
4. Export Session Data
5. Compare Token Budgets
6. Debug Empty Results

#### 8. `session_batch_processing.py` - Batch Operations
Batch processing and analytics for multiple sessions.

**Features demonstrated:**
- Batch session analysis
- Parallel processing
- Bulk export
- Session cleanup
- User reports
- Performance optimization

**Use case:** Analytics dashboards, session maintenance, data migration

**Run:**
```bash
python session_batch_processing.py
```

**Available operations:**
1. Create Sample Sessions
2. Batch Analysis
3. Batch Export
4. User Report
5. Cleanup (Dry Run)
6. Full Demo

### Advanced Examples

#### 9. `deepagent_example.py` - DeepAgent Integration
Integration with agent frameworks (legacy).

**Features demonstrated:**
- Agent framework integration
- Complex workflows
- Multi-step processing

**Run:**
```bash
python deepagent_example.py
```

#### 10. `deepagents_cli_integration.py` - CLI Integration
Using AvocadoDB with CLI tools.

**Features demonstrated:**
- CLI integration patterns
- Automation scripts
- Pipeline integration

**Run:**
```bash
python deepagents_cli_integration.py
```

## Quick Start Guide

### 1. First Steps

```python
from avocado import AvocadoDB

# Connect to AvocadoDB
db = AvocadoDB(mode="http")

# Ingest your codebase
db.ingest("/path/to/project", recursive=True)

# Compile context for a query
result = db.compile("How does authentication work?", budget=8000)
print(result.text)
```

### 2. Session Management

```python
# Create a session
session = db.create_session(user_id="alice", title="Project Q&A")

# Multi-turn conversation
result = session.compile("What is AvocadoDB?", budget=8000)
session.add_message("assistant", "AvocadoDB is a deterministic context database...")

result2 = session.compile("How does the compiler work?")
session.add_message("assistant", "The compiler uses hybrid search...")

# Get conversation history
history = session.get_history()
print(history)
```

### 3. Debugging with Replay

```python
# Replay a session to analyze agent behavior
replay = session.replay()

for turn in replay['turns']:
    print(f"User: {turn['user_message']['content']}")

    if turn.get('working_set'):
        ws = turn['working_set']
        print(f"Context: {ws['tokens_used']} tokens, {len(ws['spans'])} spans")

    if turn.get('assistant_message'):
        print(f"Assistant: {turn['assistant_message']['content']}")
```

## Example Selection Guide

**I want to...**

- **Learn the basics** → Start with `example.py`
- **Build a chatbot** → Use `session_example.py` and `session_agent_memory.py`
- **Debug agent issues** → Use `session_replay_example.py` and `session_debugging.py`
- **Process multiple sessions** → Use `session_batch_processing.py`
- **Analyze performance** → Use `session_debugging.py` (Token Usage Analysis)
- **Export session data** → Use `session_debugging.py` or `session_batch_processing.py`
- **Work without a server** → Use `cli_mode_example.py`

## Common Patterns

### Pattern 1: Agent Loop

```python
def agent_loop(session, system_prompt):
    while True:
        user_input = input("User: ")
        if user_input == "exit":
            break

        # Get context
        result = session.compile(user_input, budget=8000)
        context = result['working_set']['text']

        # Get conversation history
        history = session.get_history(max_tokens=2000)

        # Call your LLM (OpenAI, Claude, etc.)
        response = call_llm(system_prompt, context, history, user_input)

        # Add response to session
        session.add_message("assistant", response)

        print(f"Assistant: {response}")
```

### Pattern 2: Session Analysis

```python
def analyze_session(session):
    replay = session.replay()

    total_tokens = 0
    total_spans = 0

    for turn in replay['turns']:
        if turn.get('working_set'):
            ws = turn['working_set']
            total_tokens += ws['tokens_used']
            total_spans += len(ws['spans'])

    print(f"Total tokens: {total_tokens}")
    print(f"Total spans: {total_spans}")
    print(f"Avg tokens/turn: {total_tokens / len(replay['turns'])}")
```

### Pattern 3: Batch Processing

```python
def process_all_sessions(db, user_id):
    sessions = db.list_sessions(user_id=user_id)

    for session_info in sessions:
        session = db.get_session(session_info.id)
        replay = session.replay()

        # Process session
        analyze_session(session)
```

## Error Handling

```python
from avocado import AvocadoDB

try:
    db = AvocadoDB(mode="http")
    session = db.create_session(user_id="alice")

    result = session.compile("query", budget=8000)

except Exception as e:
    print(f"Error: {e}")

    # Check if server is running
    # Verify session exists
    # Check data is ingested
```

## Performance Tips

1. **Token Budgets**
   - Use lower budgets (2000-4000) for quick queries
   - Use higher budgets (8000-16000) for comprehensive context

2. **History Limiting**
   ```python
   # Limit history to prevent context overflow
   history = session.get_history(max_tokens=2000)
   ```

3. **Batch Operations**
   - Use parallel processing for multiple sessions
   - See `session_batch_processing.py` for examples

4. **Caching**
   - AvocadoDB caches compiled contexts
   - Reuse sessions for related queries

## Troubleshooting

### Server Not Running

```bash
# Start the server
cargo run --bin avocado-server

# Or check if it's running
curl http://localhost:8765/health
```

### Import Errors

```bash
# Reinstall SDK
pip uninstall avocadodb
pip install -e ..
```

### Session Not Found

```python
# List sessions to verify
sessions = db.list_sessions(user_id="alice")
print([s.id for s in sessions])
```

### No Context Retrieved

```bash
# Check if data is ingested
cargo run --bin avocado -- stats

# Ingest data
cargo run --bin avocado -- ingest /path/to/docs
```

## Next Steps

1. **Read the Documentation**
   - [Session Management Guide](../../../docs/SESSION_MANAGEMENT.md)
   - [Python SDK Reference](../README.md)

2. **Try the Examples**
   - Start with basic examples
   - Move to session management
   - Explore advanced patterns

3. **Build Your Application**
   - Use examples as templates
   - Customize for your use case
   - Share your patterns!

## Contributing Examples

Have a great example? We'd love to include it!

1. Create your example following the existing pattern
2. Add documentation and comments
3. Test thoroughly
4. Submit a pull request

See [CONTRIBUTING.md](../../../CONTRIBUTING.md) for guidelines.

## Support

- **Documentation**: [docs/](../../../docs/)
- **Issues**: [GitHub Issues](https://github.com/avocadodb/avocadodb/issues)
- **Discord**: [Community Discord](#)

## License

MIT License - see [LICENSE](../../../LICENSE) for details.
