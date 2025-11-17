# GPU-Accelerated Evaluation Guide

Running the Week 2 evaluation on CPU takes 8-10 hours. Here are options to run it on GPU (10-20x faster, ~30-60 minutes).

## Option 1: Google Colab (Free, Easiest) ⭐ Recommended

**Pros**: Free, no API keys, simple setup
**Cons**: Manual upload, need ngrok for AvocadoDB connection

### Steps:

1. **Generate Colab notebook**:
   ```bash
   python tools/local_llm/run_colab_evaluation.py
   ```

2. **Setup ngrok** (to expose local AvocadoDB server):
   ```bash
   # Install ngrok
   brew install ngrok/ngrok/ngrok
   
   # Start tunnel
   ngrok http 8765
   # Copy the HTTPS URL (e.g., https://abc123.ngrok.io)
   ```

3. **Upload to Colab**:
   - Go to https://colab.research.google.com/
   - Upload `tools/local_llm/week2_evaluation_colab.ipynb`
   - Update `AVOCADODB_URL` with your ngrok URL
   - Run all cells (Runtime → Run all)
   - Download results when complete

**Time**: ~30-60 minutes on free T4 GPU
**Cost**: $0

---

## Option 2: RunPod API (Pay-per-second)

**Pros**: Full API control, fast spin-up/down
**Cons**: Requires API key, more complex setup

### Setup:

1. **Get RunPod API key**:
   - Sign up at https://www.runpod.io
   - Get API key from https://www.runpod.io/console/user/settings

2. **Export API key**:
   ```bash
   export RUNPOD_API_KEY="your-key-here"
   ```

3. **Run**:
   ```bash
   python tools/local_llm/run_gpu_evaluation.py
   ```

**Time**: ~30-60 minutes on RTX 3090
**Cost**: ~$0.20-0.50

---

## Option 3: Vast.ai (Cheapest)

**Pros**: Very cheap (~$0.10-0.20/hour), SSH access
**Cons**: Manual setup, less reliable

### Steps:

1. **Find GPU instance**:
   - Go to https://vast.ai
   - Search for RTX 3090 or similar
   - Rent an instance

2. **SSH into instance**:
   ```bash
   ssh root@<instance-ip>
   ```

3. **Setup and run**:
   ```bash
   # Install dependencies
   pip install transformers torch requests
   
   # Upload evaluation script (use scp)
   # scp tools/local_llm/week2_evaluation.py root@<instance-ip>:~/
   
   # Run (with AVOCADODB_URL pointing to your ngrok URL)
   AVOCADODB_URL=https://your-ngrok-url.ngrok.io python week2_evaluation.py
   ```

**Time**: ~30-60 minutes
**Cost**: ~$0.10-0.20

---

## Option 4: Lambda Labs (Easy API)

**Pros**: Simple API, good docs
**Cons**: More expensive (~$0.50/hour)

### Setup:

1. **Sign up**: https://lambdalabs.com
2. **Get API key**
3. **Use their API** to spin up instance and run script

**Time**: ~30-60 minutes
**Cost**: ~$0.50

---

## Quick Comparison

| Option | Cost | Setup Time | Speed | Recommended |
|--------|------|------------|-------|-------------|
| **Colab** | Free | 5 min | Fast | ⭐⭐⭐⭐⭐ |
| **RunPod** | $0.20-0.50 | 10 min | Fast | ⭐⭐⭐⭐ |
| **Vast.ai** | $0.10-0.20 | 15 min | Fast | ⭐⭐⭐ |
| **Lambda** | $0.50 | 10 min | Fast | ⭐⭐⭐ |

---

## Important: AvocadoDB Server Access

All options require your local AvocadoDB server to be accessible from the cloud. Use **ngrok**:

```bash
# Install
brew install ngrok/ngrok/ngrok

# Start tunnel
ngrok http 8765

# Use the HTTPS URL in your evaluation script
export AVOCADODB_URL="https://abc123.ngrok.io"
```

---

## Recommendation

**Start with Google Colab** - it's free, simple, and works great for this use case. If you need automation, use RunPod API.

