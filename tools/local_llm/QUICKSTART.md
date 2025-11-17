# Quick Start: Day 1 POC

Get the ultra-minimal POC running in < 5 minutes!

## Prerequisites

1. **AvocadoDB server running**
   ```bash
   # In one terminal
   ./target/release/avocado-server
   ```

2. **Python dependencies**
   ```bash
   pip install transformers torch
   pip install -e sdks/python  # Or: pip install avocado
   ```

## Run the POC

```bash
python tools/local_llm/week1_poc.py
```

## What It Does

1. ✅ Indexes `avocado-core/src/compiler.rs` (just one file)
2. ✅ Loads TinyLlama-1.1B model
3. ✅ Asks: "How does the compile function work?"
4. ✅ Gets context from AvocadoDB
5. ✅ Generates answer with local model
6. ✅ Tests determinism (same query → same answer)

## Expected Output

```
🥑 AvocadoDB + Local LLM POC
============================================================

Step 1: Indexing single file...
✅ Connected to AvocadoDB server
   Indexing: avocado-core/src/compiler.rs
✅ Indexed: 45 spans created

Step 2: Loading TinyLlama model...
✅ Model loaded

Step 3: Testing query...
   Query: How does the compile function work?
✅ Context retrieved: 12 spans, 987 tokens
   Citations: 12

Step 4: Generating answer with local model...
✅ Answer generated

------------------------------------------------------------
ANSWER:
------------------------------------------------------------
The compile function in AvocadoDB is the core context compilation
engine. It takes a query and compiles a deterministic working set
by:
1. Embedding the query
2. Performing semantic and lexical search
3. Combining results with hybrid fusion
4. Applying MMR diversification
5. Packing into token budget
6. Sorting deterministically

[1] avocado-core/src/compiler.rs:35-85
------------------------------------------------------------

Step 5: Testing determinism...
✅ Deterministic: Same answer on both runs

============================================================
POC RESULTS
============================================================
✅ File indexed: avocado-core/src/compiler.rs
✅ Query answered: How does the compile function work?
✅ Citations: 12
✅ Context spans: 12
✅ Deterministic: True

🎉 POC SUCCESS!

Next steps:
  1. Review answer quality (does it make sense?)
  2. If good → run poc_10_queries.py
  3. If not → improve RAG or model selection
```

## Troubleshooting

### "Could not connect to AvocadoDB server"
Make sure the server is running:
```bash
./target/release/avocado-server
```

### "AvocadoDB not found"
```bash
pip install -e sdks/python
```

### "transformers not found"
```bash
pip install transformers torch
```

### Model download slow
First run downloads ~2GB. Subsequent runs are instant.

### Out of memory
- Use CPU: The script auto-detects GPU/CPU
- Try smaller model: Already using TinyLlama (smallest)
- Reduce tokens: Change `max_new_tokens=200` to `100`

## Next Steps

If POC succeeds:
1. Review answer quality
2. Run `poc_10_queries.py` (to be created)
3. Proceed to Week 2

If POC fails:
1. Check error messages
2. Verify AvocadoDB server is running
3. Check model download completed
4. Review determinism settings

