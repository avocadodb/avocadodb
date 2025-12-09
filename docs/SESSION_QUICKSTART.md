# Session Management - Quick Start Guide

Get started with AvocadoDB session management in 5 minutes.

## Installation & Setup

```bash
# Make sure server is running
cargo run --bin avocado-server

# Or use CLI mode directly (no server needed)
cargo build --release
```

## Python SDK - 30 Second Tutorial

```python
from avocado import AvocadoDB

# 1. Connect (HTTP mode for sessions)
db = AvocadoDB(mode="http")

# 2. Create a session
session = db.create_session(user_id="alice", title="My Chat")

# 3. Have a conversation
result = session.compile("What is Rust?")
print(result['working_set']['text'])  # Context retrieved

session.add_message("assistant", "Rust is a systems programming language...")

# 4. Continue chatting
result2 = session.compile("Tell me about ownership")
session.add_message("assistant", "Ownership is Rust's unique feature...")

# 5. Review conversation
print(session.get_history())

# 6. Debug (if needed)
replay = session.replay()
for turn in replay['turns']:
    print(f"Q: {turn['user_message']['content']}")
    if turn.get('assistant_message'):
        print(f"A: {turn['assistant_message']['content']}")
```

## CLI - Common Commands

```bash
# Create session
SESSION_ID=$(avocado session create --user-id alice --title "My Chat" | grep "Created session:" | cut -d' ' -f4)

# Add messages
avocado session message $SESSION_ID --role user --content "What is Rust?"
avocado session message $SESSION_ID --role assistant --content "Rust is..."

# View session
avocado session show $SESSION_ID

# Get history
avocado session history $SESSION_ID

# Replay for debugging
avocado session replay $SESSION_ID

# List all sessions
avocado session list --user-id alice

# Delete when done
avocado session delete $SESSION_ID --yes
```

## Common Patterns

### Pattern 1: Q&A Bot

```python
session = db.create_session(user_id="user123", title="Q&A Bot")

while True:
    query = input("Ask: ")
    if query.lower() == 'quit':
        break

    # Compile context
    result = session.compile(query, budget=8000)

    # Generate answer (your LLM here)
    answer = your_llm(result['working_set']['text'], query)

    # Store answer
    session.add_message("assistant", answer)

    print(f"Answer: {answer}\n")

# Review conversation
print("\n=== Full Conversation ===")
print(session.get_history())
```

### Pattern 2: Debug Agent Behavior

```python
# Run your agent...
session = db.create_session(user_id="agent", title="Debug Session")
# ... agent interacts with session ...

# Later, analyze what happened
replay = session.replay()

for i, turn in enumerate(replay['turns'], 1):
    print(f"\n=== Turn {i} ===")
    print(f"User: {turn['user_message']['content']}")

    if turn.get('working_set'):
        ws = turn['working_set']
        print(f"Context: {ws['tokens_used']} tokens")
        print(f"Top files: {[c['artifact_path'] for c in ws['citations'][:3]]}")

    if turn.get('assistant_message'):
        print(f"Agent: {turn['assistant_message']['content'][:100]}...")
```

### Pattern 3: Multi-User Chat

```python
# Create sessions for different users
alice_session = db.create_session(user_id="alice", title="Alice's Chat")
bob_session = db.create_session(user_id="bob", title="Bob's Chat")

# Each user has isolated conversation
alice_session.compile("How does auth work?")
bob_session.compile("What about security?")

# List sessions by user
alice_sessions = db.list_sessions(user_id="alice")
bob_sessions = db.list_sessions(user_id="bob")
```

## HTTP API - cURL Examples

```bash
# Create session
curl -X POST http://localhost:8765/sessions \
  -H "Content-Type: application/json" \
  -d '{"user_id":"alice","title":"API Test"}'

# Response: {"session":{"id":"<session-id>",...}}

# Compile in session
curl -X POST http://localhost:8765/sessions/<session-id>/compile \
  -H "Content-Type: application/json" \
  -d '{"query":"What is Rust?","token_budget":8000}'

# Get history
curl http://localhost:8765/sessions/<session-id>/history
```

## Troubleshooting

### "Session management not available in CLI mode"

**Solution**: Use HTTP mode

```python
# ❌ Wrong
db = AvocadoDB(mode="cli")
session = db.create_session()  # Error!

# ✅ Correct
db = AvocadoDB(mode="http")  # Requires server
session = db.create_session()
```

### "Connection refused"

**Solution**: Start the server first

```bash
# Terminal 1
cargo run --bin avocado-server

# Terminal 2
python your_script.py
```

### Session not found

**Solution**: Use correct database path

```bash
# List sessions to verify
avocado session list --db-path .avocado/db.sqlite
```

## Best Practices

1. **Use meaningful titles**:
   ```python
   session = db.create_session(
       user_id="alice",
       title="Debugging auth issue #123"  # Descriptive!
   )
   ```

2. **Limit history for large sessions**:
   ```python
   # Only get last ~2000 tokens
   recent_history = session.get_history(max_tokens=2000)
   ```

3. **Clean up old sessions**:
   ```python
   # Delete sessions older than 30 days
   for session_data in db.list_sessions():
       if is_old(session_data['created_at']):
           session = db.get_session(session_data['id'])
           session.delete()
   ```

4. **Use replay for debugging**:
   ```python
   # When something goes wrong
   replay = session.replay()

   # Analyze context quality
   for turn in replay['turns']:
       if turn.get('working_set'):
           ws = turn['working_set']
           if ws['tokens_used'] < ws['token_budget'] * 0.5:
               print(f"⚠️  Low context utilization: {ws['tokens_used']} tokens")
   ```

## What's Next?

- Read [Full Documentation](./SESSION_MANAGEMENT.md)
- Check [Examples](../sdks/python/examples/)
- See [API Reference](./session-management-spec.md)

---

**Quick Links**:
- [Session Management Docs](./SESSION_MANAGEMENT.md)
- [Python Examples](../sdks/python/examples/)
- [Spec](./session-management-spec.md)
