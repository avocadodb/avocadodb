# 🚀 START HERE: Day 1 POC

**Run this TODAY to prove the concept works!**

## Quick Start (3 steps)

### 1. Start AvocadoDB Server
```bash
# In terminal 1
./target/release/avocado-server
```

### 2. Install Dependencies
```bash
# In terminal 2
pip install transformers torch
pip install -e sdks/python
```

### 3. Run POC
```bash
python tools/local_llm/week1_poc.py
```

## What You'll See

The script will:
1. ✅ Connect to AvocadoDB server
2. ✅ Index `compiler.rs` (one file)
3. ✅ Load TinyLlama-1.1B model
4. ✅ Ask: "How does the compile function work?"
5. ✅ Get context from AvocadoDB
6. ✅ Generate answer with local model
7. ✅ Test determinism

**Expected time**: < 5 minutes (first run downloads model ~2GB)

## Success Criteria

✅ Answer makes sense (human evaluation)  
✅ Citations present  
✅ Deterministic (same query → same answer)

## If It Works

🎉 **POC SUCCESS!** Proceed to:
- Review answer quality
- Scale to 10 queries
- Continue with Week 1 plan

## If It Doesn't Work

Check:
- Is AvocadoDB server running? (`./target/release/avocado-server`)
- Are dependencies installed? (`pip install transformers torch`)
- Is model downloading? (first run takes time)

See `QUICKSTART.md` for detailed troubleshooting.

## Next Steps

After POC succeeds, see `.implementation/plan-local-llm.yaml` for full roadmap.

