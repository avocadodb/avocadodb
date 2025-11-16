# 5-Minute Quick Start

Get AvocadoDB running in 5 minutes or less.

## Prerequisites

- Rust toolchain (1.70+)
- OpenAI API key
- Some documents to index (markdown, text, code, etc.)

## Step 1: Install Rust (if needed)

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

## Step 2: Clone and Build

```bash
git clone https://github.com/yourusername/avacadodb.git
cd avacadodb
cargo build --release
```

**Expected time:** 2-3 minutes for first build

## Step 3: Set API Key

```bash
export OPENAI_API_KEY="sk-your-key-here"
```

**Tip:** Add this to your `~/.bashrc` or `~/.zshrc` to make it permanent.

## Step 4: Initialize Database

```bash
./target/release/avocado init
```

**Output:**
```
Initializing AvocadoDB...
Created database at .avocado/db.sqlite
Ready to ingest documents!
```

## Step 5: Ingest Documents

```bash
# Create a test document
cat > test-doc.md << 'EOF'
# Authentication System

Our system uses JWT tokens for authentication.

## How It Works

1. User logs in with credentials
2. Server validates and returns JWT token
3. Client includes token in subsequent requests
4. Server validates token on each request

## Token Structure

```json
{
  "sub": "user-id",
  "exp": 1234567890,
  "iat": 1234567890
}
```

## Refresh Tokens

Refresh tokens have a longer lifetime and can be used
to obtain new access tokens without re-authentication.
EOF

# Ingest it
./target/release/avocado ingest test-doc.md
```

**Output:**
```
Ingesting: test-doc.md
Generating embeddings... (using OpenAI text-embedding-ada-002)
✓ Extracted 1 span
✓ Generated embeddings
✓ Stored in database

Ingested 1 file → 1 span
```

## Step 6: Compile Context

```bash
./target/release/avocado compile "How does authentication work?" --budget 8000
```

**Output:**
```
Compiling context for: "How does authentication work?"
Token budget: 8000

[1] test-doc.md
Lines 1-20

# Authentication System

Our system uses JWT tokens for authentication.

## How It Works

1. User logs in with credentials
2. Server validates and returns JWT token
3. Client includes token in subsequent requests
4. Server validates token on each request

## Token Structure

```json
{
  "sub": "user-id",
  "exp": 1234567890,
  "iat": 1234567890
}
```

---

Compiled 1 span using 127 tokens (1.6% utilization)
Compilation time: 245ms
Context hash: a8f3c2d1e9b7f6... (deterministic ✓)
```

## Step 7: Verify Determinism

```bash
# Run the same query twice and compare hashes
./target/release/avocado compile "authentication" | head -50 | sha256sum
./target/release/avocado compile "authentication" | head -50 | sha256sum
```

**Output:**
```
e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855
e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855
```

**✅ Same hash every time!** This is the core guarantee of AvocadoDB.

## Step 8: Ingest Real Documents

```bash
# Ingest your own documentation
./target/release/avocado ingest ./docs --recursive

# Or your source code
./target/release/avocado ingest ./src --recursive

# Check what was indexed
./target/release/avocado stats
```

## Next Steps

Now that you have AvocadoDB running, you can:

### Use Different Token Budgets

```bash
# Small context (GPT-3.5 Turbo)
avocado compile "your query" --budget 4000

# Large context (GPT-4 Turbo)
avocado compile "your query" --budget 16000

# Huge context (Claude 3)
avocado compile "your query" --budget 100000
```

### Tune Search Parameters

```bash
# More diverse results (lower MMR lambda)
avocado compile "your query" --mmr-lambda 0.3

# More relevant results (higher MMR lambda)
avocado compile "your query" --mmr-lambda 0.8

# Balance semantic vs lexical search
avocado compile "your query" --semantic-weight 0.5 --lexical-weight 0.5
```

### Integrate with Your Application

See the [Library Usage](README.md#library-usage-rust) section in the main README for how to use AvocadoDB as a library.

### Run the HTTP Server

```bash
./target/release/avocado-server
```

Then use the REST API:

```bash
curl -X POST http://localhost:8080/compile \
  -H "Content-Type: application/json" \
  -d '{
    "query": "authentication",
    "token_budget": 8000
  }'
```

## Troubleshooting

### "Failed to initialize tiktoken"

Make sure you're using Rust 1.70 or newer:

```bash
rustc --version
```

If needed, update:

```bash
rustup update
```

### "OpenAI API Error: 401 Unauthorized"

Check that your API key is set correctly:

```bash
echo $OPENAI_API_KEY
```

Make sure it starts with `sk-` and has no extra quotes or spaces.

### "No spans found for query"

This usually means:
1. You haven't ingested any documents yet
2. Your query doesn't match any content in the database
3. The documents weren't embedded successfully

Try:

```bash
# Check database stats
avocado stats

# Re-ingest with logging
RUST_LOG=avocado_core=debug avocado ingest ./docs --recursive
```

### Compilation is slow (>1 second)

This is usually due to OpenAI API latency. The embedding API call typically takes 200-300ms but can spike to 1000ms+ during high load.

To diagnose:

```bash
RUST_LOG=avocado_core=debug avocado compile "your query"
```

Look for the "Embed query" timing in the output.

## Performance Tips

1. **Batch ingest**: Ingest directories instead of individual files
2. **Right-size budgets**: Don't use 100K tokens if you only need 8K
3. **Cache results**: If using the same query repeatedly, cache the compiled context
4. **Local development**: Consider Phase 2 local embeddings (no API calls)

## What's Next?

- Read the [full README](README.md) for complete documentation
- Check out [EXAMPLES.md](EXAMPLES.md) for real-world usage patterns
- Review [docs/performance.md](docs/performance.md) for optimization tips
- Explore the [Phase 2 roadmap](README.md#roadmap) for upcoming features

---

**You're now ready to use AvocadoDB!** 🥑

Same query → Same context, every time. That's the guarantee.
