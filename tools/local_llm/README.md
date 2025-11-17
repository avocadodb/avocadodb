# Local LLM Integration Tools

Tools for fine-tuning and running local LLMs with AvocadoDB for codebase Q&A.

## Quick Start

### Day 1: Ultra-Minimal POC

Test if the approach works with just 1 file, 1 query, 1 model:

```bash
# 1. Start AvocadoDB server
./target/release/avocado-server &

# 2. Install dependencies
pip install transformers torch
pip install -e sdks/python  # Or: pip install avocado

# 3. Run POC
python tools/local_llm/week1_poc.py
```

**Expected time**: < 5 minutes (first run may take longer to download model)

**Success criteria**:
- ✅ Answer makes sense
- ✅ Citations present
- ✅ Deterministic (same query → same answer)

### Week 1: Scale to 10 Queries

After Day 1 POC succeeds:

```bash
python tools/local_llm/poc_10_queries.py
```

## Requirements

- Python 3.8+
- AvocadoDB server running (port 8765)
- GPU recommended (but not required for TinyLlama)
- ~2GB RAM for TinyLlama-1.1B

## Model Options

| Model | Size | Speed | GPU Required |
|-------|------|-------|--------------|
| TinyLlama-1.1B | 1.1B | Fastest | No (CPU works) |
| Phi-2 | 2.7B | Fast | Recommended |
| StableLM-1.6B | 1.6B | Fast | Recommended |
| Phi-3-mini | 3.8B | Medium | Yes |

## Troubleshooting

### "AvocadoDB not found"
```bash
pip install -e sdks/python
```

### "transformers not found"
```bash
pip install transformers torch
```

### "Could not connect to AvocadoDB server"
```bash
# Start the server
./target/release/avocado-server
```

### Model download slow
First run downloads the model (~2GB for TinyLlama). Subsequent runs are instant.

### Out of memory
- Use CPU instead of GPU
- Try smaller model (TinyLlama)
- Reduce `max_new_tokens`

## Next Steps

See `.implementation/plan-local-llm.yaml` for full roadmap.

