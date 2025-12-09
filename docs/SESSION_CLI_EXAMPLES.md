# Session Management CLI Examples

Comprehensive guide to using AvocadoDB sessions via the CLI for common workflows, debugging, and automation.

## Table of Contents

1. [Basic Workflow](#basic-workflow)
2. [Multi-Turn Conversations](#multi-turn-conversations)
3. [Debugging Agent Behavior](#debugging-agent-behavior)
4. [Session Management](#session-management)
5. [Automation Scripts](#automation-scripts)
6. [Performance Analysis](#performance-analysis)
7. [Common Patterns](#common-patterns)

## Basic Workflow

### Creating and Using a Session

```bash
# 1. Start the server (if not already running)
cargo run --bin avocado-server &

# 2. Ingest your codebase
avocado ingest /path/to/project

# 3. Create a new session
avocado session create --user-id alice --title "Project Q&A"
# Output: Session ID (save this for later use)
SESSION_ID="abc123..."

# 4. Ask your first question
avocado session compile $SESSION_ID "How does authentication work?" --budget 8000

# 5. Add the assistant's response
avocado session message $SESSION_ID \
  --role assistant \
  --content "Authentication is handled by..."

# 6. Continue the conversation
avocado session compile $SESSION_ID "Can you show me an example?"

# 7. View the full conversation
avocado session history $SESSION_ID
```

## Multi-Turn Conversations

### Interactive Q&A Session

```bash
#!/bin/bash
# interactive_qa.sh - Interactive session script

# Create session
SESSION_ID=$(avocado session create \
  --user-id $USER \
  --title "Interactive Q&A" \
  | grep "Created session:" | awk '{print $3}')

echo "Session ID: $SESSION_ID"

# Interactive loop
while true; do
  echo ""
  read -p "Your question (or 'quit' to exit): " QUESTION

  if [[ "$QUESTION" == "quit" ]]; then
    echo "Session saved: $SESSION_ID"
    break
  fi

  # Compile context
  echo "Compiling context..."
  avocado session compile $SESSION_ID "$QUESTION" --budget 8000

  # Get assistant response (in real app, this would be LLM call)
  echo ""
  read -p "Assistant response: " RESPONSE

  # Add response to session
  avocado session message $SESSION_ID \
    --role assistant \
    --content "$RESPONSE"
done
```

### Resuming a Previous Conversation

```bash
# List your recent sessions
avocado session list --user-id alice | head -n 20

# Get specific session
SESSION_ID="previous-session-id"

# View what was discussed
avocado session history $SESSION_ID

# Continue the conversation
avocado session compile $SESSION_ID "Following up on our earlier discussion..."
```

## Debugging Agent Behavior

### Analyzing a Problematic Session

```bash
# Replay the session to see exactly what happened
avocado session replay $SESSION_ID

# This shows:
# - Each user query
# - Context compiled for each query (tokens, spans, citations)
# - Assistant responses
# - Timestamps and metadata
```

### Investigating Context Quality

```bash
# Compile with different budgets to compare
for BUDGET in 2000 4000 8000 16000; do
  echo "=== Budget: $BUDGET ==="
  avocado session compile $SESSION_ID \
    "Test query" \
    --budget $BUDGET \
    | grep "Tokens:"
done
```

### Checking Citation Quality

```bash
# Replay session and examine citations
avocado session replay $SESSION_ID | grep -A 5 "Top citations"

# Example output:
# Top citations:
#   • src/auth.rs:45-67 (score: 0.89)
#   • docs/authentication.md:10-30 (score: 0.82)
#   • config/auth.yaml:1-20 (score: 0.75)
```

## Session Management

### Listing and Filtering Sessions

```bash
# List all sessions
avocado session list

# Filter by user
avocado session list --user-id alice

# Limit results
avocado session list --limit 10

# Show detailed information
avocado session list --user-id alice | while read SESSION_ID; do
  avocado session show $SESSION_ID
done
```

### Session Cleanup

```bash
#!/bin/bash
# cleanup_old_sessions.sh - Remove empty or old sessions

# List all sessions for a user
SESSION_IDS=$(avocado session list --user-id alice | grep "Session:" | awk '{print $2}')

for SESSION_ID in $SESSION_IDS; do
  # Get session details
  DETAILS=$(avocado session show $SESSION_ID)

  # Check if session has no messages (adjust logic as needed)
  MESSAGE_COUNT=$(echo "$DETAILS" | grep -c "Message:")

  if [[ $MESSAGE_COUNT -eq 0 ]]; then
    echo "Deleting empty session: $SESSION_ID"
    avocado session delete $SESSION_ID --yes
  fi
done
```

### Exporting Sessions

```bash
#!/bin/bash
# export_sessions.sh - Export all sessions to JSON

mkdir -p session_exports

SESSION_IDS=$(avocado session list --user-id alice | grep "Session:" | awk '{print $2}')

for SESSION_ID in $SESSION_IDS; do
  echo "Exporting $SESSION_ID..."

  avocado session replay $SESSION_ID --format json > \
    "session_exports/${SESSION_ID}.json"
done

echo "Exported $(ls session_exports | wc -l) sessions"
```

## Automation Scripts

### Batch Question Processing

```bash
#!/bin/bash
# batch_qa.sh - Process multiple questions in a session

SESSION_ID=$(avocado session create \
  --user-id batch_user \
  --title "Batch Q&A" \
  | grep "Created session:" | awk '{print $3}')

# Read questions from file
QUESTIONS_FILE="questions.txt"

while IFS= read -r QUESTION; do
  echo "Processing: $QUESTION"

  # Compile context
  avocado session compile $SESSION_ID "$QUESTION" --budget 8000

  # Here you would typically call an LLM API
  # For now, just add a placeholder response
  avocado session message $SESSION_ID \
    --role assistant \
    --content "[Response to: $QUESTION]"

  echo "---"
done < "$QUESTIONS_FILE"

# Generate final report
echo "Final conversation:"
avocado session history $SESSION_ID
```

### Continuous Monitoring

```bash
#!/bin/bash
# monitor_sessions.sh - Monitor active sessions

watch -n 30 '
  echo "Active Sessions:"
  avocado session list --limit 20

  echo ""
  echo "Recent Activity:"
  avocado session list --limit 5 | while read SESSION_ID; do
    LAST_MSG=$(avocado session show $SESSION_ID | tail -n 5)
    echo "$SESSION_ID: $LAST_MSG"
  done
'
```

## Performance Analysis

### Measuring Session Performance

```bash
#!/bin/bash
# perf_test.sh - Test session performance

SESSION_ID=$(avocado session create \
  --user-id perf_test \
  --title "Performance Test" \
  | grep "Created session:" | awk '{print $3}')

# Measure compilation time
echo "Testing compilation performance..."

for i in {1..10}; do
  START=$(date +%s%N)

  avocado session compile $SESSION_ID \
    "Test query $i" \
    --budget 8000 > /dev/null

  END=$(date +%s%N)
  ELAPSED=$(( (END - START) / 1000000 ))

  echo "Query $i: ${ELAPSED}ms"
done

# Measure history retrieval
echo ""
echo "Testing history retrieval..."

START=$(date +%s%N)
avocado session history $SESSION_ID > /dev/null
END=$(date +%s%N)
ELAPSED=$(( (END - START) / 1000000 ))

echo "History retrieval: ${ELAPSED}ms"
```

### Token Usage Analysis

```bash
#!/bin/bash
# token_analysis.sh - Analyze token usage patterns

SESSION_ID=$1

if [[ -z "$SESSION_ID" ]]; then
  echo "Usage: $0 <session-id>"
  exit 1
fi

echo "Token Usage Analysis for Session: $SESSION_ID"
echo "================================================"

# Extract token usage from replay
avocado session replay $SESSION_ID | \
  grep "Context:" | \
  awk '{print $2}' | \
  while read TOKENS; do
    echo $TOKENS
  done | \
  awk '{
    sum += $1;
    count++;
    if (NR == 1 || $1 < min) min = $1;
    if (NR == 1 || $1 > max) max = $1;
  }
  END {
    print "Total queries:", count;
    print "Total tokens:", sum;
    print "Average tokens:", sum/count;
    print "Min tokens:", min;
    print "Max tokens:", max;
  }'
```

## Common Patterns

### 1. Agent Loop with Context

```bash
#!/bin/bash
# agent_loop.sh - Agent conversation loop

SESSION_ID=$(avocado session create \
  --user-id agent \
  --title "Agent Conversation" \
  | grep "Created session:" | awk '{print $3}')

# Agent system prompt
SYSTEM_PROMPT="You are a helpful assistant with access to project documentation."

while true; do
  read -p "User: " USER_INPUT

  if [[ "$USER_INPUT" == "exit" ]]; then
    break
  fi

  # Get context
  CONTEXT=$(avocado session compile $SESSION_ID "$USER_INPUT" --budget 8000)

  # Get conversation history
  HISTORY=$(avocado session history $SESSION_ID --max-tokens 2000)

  # Call LLM (example with OpenAI API)
  RESPONSE=$(curl -s https://api.openai.com/v1/chat/completions \
    -H "Authorization: Bearer $OPENAI_API_KEY" \
    -H "Content-Type: application/json" \
    -d "{
      \"model\": \"gpt-4\",
      \"messages\": [
        {\"role\": \"system\", \"content\": \"$SYSTEM_PROMPT\"},
        {\"role\": \"system\", \"content\": \"Context: $CONTEXT\"},
        {\"role\": \"user\", \"content\": \"$HISTORY\n\nUser: $USER_INPUT\"}
      ]
    }" | jq -r '.choices[0].message.content')

  # Add response to session
  avocado session message $SESSION_ID \
    --role assistant \
    --content "$RESPONSE"

  echo "Assistant: $RESPONSE"
  echo ""
done

echo "Session saved: $SESSION_ID"
```

### 2. Context Quality Check

```bash
#!/bin/bash
# check_context_quality.sh - Verify context quality

SESSION_ID=$1
QUERY=$2

if [[ -z "$SESSION_ID" ]] || [[ -z "$QUERY" ]]; then
  echo "Usage: $0 <session-id> <query>"
  exit 1
fi

echo "Checking context quality for query: $QUERY"
echo "================================================"

# Compile and capture output
OUTPUT=$(avocado session compile $SESSION_ID "$QUERY" --budget 8000)

# Extract metrics
TOKENS=$(echo "$OUTPUT" | grep "Tokens:" | awk '{print $2}')
SPANS=$(echo "$OUTPUT" | grep "Spans:" | awk '{print $2}')

echo "Tokens: $TOKENS"
echo "Spans: $SPANS"

# Check for warnings
if [[ $SPANS -lt 5 ]]; then
  echo "⚠️  WARNING: Low span count - may indicate poor results"
fi

if [[ $TOKENS -lt 1000 ]]; then
  echo "⚠️  WARNING: Low token count - context may be insufficient"
fi

echo ""
echo "Top citations:"
echo "$OUTPUT" | grep -A 10 "Top citations"
```

### 3. Session Comparison

```bash
#!/bin/bash
# compare_sessions.sh - Compare two sessions

SESSION_1=$1
SESSION_2=$2

if [[ -z "$SESSION_1" ]] || [[ -z "$SESSION_2" ]]; then
  echo "Usage: $0 <session-id-1> <session-id-2>"
  exit 1
fi

echo "Comparing sessions:"
echo "  Session 1: $SESSION_1"
echo "  Session 2: $SESSION_2"
echo "================================================"

for SESSION in $SESSION_1 $SESSION_2; do
  echo ""
  echo "Session: $SESSION"
  echo "---"

  avocado session show $SESSION | grep -E "(User:|Title:|Created:|Messages:)"

  TURNS=$(avocado session replay $SESSION | grep -c "Turn")
  echo "Turns: $TURNS"
done
```

## Tips and Best Practices

### 1. Session Organization

```bash
# Use descriptive titles
avocado session create \
  --user-id $USER \
  --title "Bug #123: Login issue investigation"

# Tag sessions with metadata (via title or user_id)
avocado session create \
  --user-id "project:frontend" \
  --title "[PROD] User auth flow"
```

### 2. Error Handling

```bash
#!/bin/bash
# robust_session_script.sh

set -e  # Exit on error

# Check if server is running
if ! curl -s http://localhost:8765/health > /dev/null; then
  echo "Error: AvocadoDB server is not running"
  echo "Start it with: cargo run --bin avocado-server"
  exit 1
fi

# Create session with error handling
SESSION_ID=$(avocado session create \
  --user-id $USER \
  --title "Test Session" 2>&1 | \
  grep "Created session:" | \
  awk '{print $3}')

if [[ -z "$SESSION_ID" ]]; then
  echo "Error: Failed to create session"
  exit 1
fi

echo "Session created: $SESSION_ID"
```

### 3. Token Budget Guidelines

```bash
# Small queries (quick lookups)
avocado session compile $SESSION_ID "What is X?" --budget 2000

# Medium queries (detailed explanations)
avocado session compile $SESSION_ID "Explain how X works" --budget 8000

# Large queries (comprehensive analysis)
avocado session compile $SESSION_ID "Analyze the entire auth system" --budget 16000
```

## Troubleshooting

### Session Not Found

```bash
# Verify session exists
avocado session list | grep $SESSION_ID

# If not found, list recent sessions
avocado session list --limit 20
```

### No Context Retrieved

```bash
# Check if data is ingested
avocado stats

# Try broader query
avocado session compile $SESSION_ID "general query" --budget 8000

# Check database
avocado session show $SESSION_ID
```

### Performance Issues

```bash
# Reduce token budget
avocado session compile $SESSION_ID "query" --budget 4000

# Limit history
avocado session history $SESSION_ID --max-tokens 1000

# Check server logs
tail -f ~/.avocadodb/server.log
```

## Advanced Use Cases

### Session Migration

```bash
#!/bin/bash
# migrate_sessions.sh - Migrate sessions to new project

OLD_PROJECT="/path/to/old/project"
NEW_PROJECT="/path/to/new/project"

# Export from old project
cd $OLD_PROJECT
avocado session list | grep "Session:" | awk '{print $2}' | \
  while read SESSION_ID; do
    avocado session replay $SESSION_ID --format json > \
      "/tmp/session_${SESSION_ID}.json"
  done

# Import to new project (manual process - adapt as needed)
cd $NEW_PROJECT
# ... import logic ...
```

### CI/CD Integration

```bash
#!/bin/bash
# ci_session_test.sh - Test session functionality in CI

set -e

# Start server
cargo run --bin avocado-server &
SERVER_PID=$!

sleep 5  # Wait for server to start

# Run session tests
SESSION_ID=$(avocado session create \
  --user-id ci_test \
  --title "CI Test" | \
  grep "Created session:" | \
  awk '{print $3}')

avocado session compile $SESSION_ID "test query" --budget 4000
avocado session message $SESSION_ID --role assistant --content "test response"
avocado session history $SESSION_ID

# Cleanup
avocado session delete $SESSION_ID --yes
kill $SERVER_PID

echo "✓ Session tests passed"
```

## See Also

- [Session Management Documentation](./SESSION_MANAGEMENT.md)
- [HTTP API Reference](../avocado-server/README.md)
- [Python SDK Examples](../sdks/python/examples/)
- [CLI Reference](../avocado-cli/README.md)
