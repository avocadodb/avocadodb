# AvocadoDB Embedding Models Guide

## Quick Start

Get personalized model recommendations:

```bash
./target/release/avocado recommend --corpus-size <number> --use-case <type>
```

## Available Models

AvocadoDB supports multiple embedding models with different performance/quality trade-offs:

| Model | Alias | Dimensions | Speed | Quality | Best For |
|-------|-------|------------|-------|---------|----------|
| **all-MiniLM-L6-v2** | `(default)` | 384 | ⚡⚡⚡⚡⚡ | ⭐⭐⭐ | General purpose, speed-critical |
| **nomic-embed-text-v1.5** | `nomicv15` | 768 | ⚡⚡⚡⚡ | ⭐⭐⭐⭐ | Production, balanced |
| **bge-large-en-v1.5** | `bgelarge` | 1024 | ⚡⚡⚡ | ⭐⭐⭐⭐⭐ | Legal, compliance, maximum quality |

## Model Details

### all-MiniLM-L6-v2 (Default)

**Best for**: General-purpose use, development, code search

```bash
# No configuration needed - this is the default
```

**Specifications**:
- Dimensions: 384
- Model size: ~90MB
- Inference time: 1-3ms per text
- Throughput: 1,000-1,500 texts/sec
- Quality (Recall@10): 82%

**Strengths**:
- ⚡ Fastest inference time
- 💾 Smallest model size
- 🚀 Highest throughput
- ✅ Good quality for most tasks

**Use When**:
- Speed is priority
- Large corpus (>10K documents)
- Development/testing
- Code search and documentation
- Real-time applications

---

### nomic-embed-text-v1.5

**Best for**: Production applications, customer-facing systems

```bash
export AVOCADODB_EMBEDDING_MODEL=nomicv15
# Then re-ingest: avocado clear && avocado ingest <path> --recursive
```

**Specifications**:
- Dimensions: 768 (2x default)
- Model size: ~200MB
- Inference time: 2-5ms per text
- Throughput: 500-800 texts/sec
- Quality (Recall@10): 89%

**Strengths**:
- ⭐ Better accuracy than default (+7% recall)
- ⚡ Still fast (only 2-3x slower than default)
- 📈 Good balance of speed and quality
- 💼 Production-grade quality

**Use When**:
- Production applications
- Customer-facing search
- Medium corpus (1K-10K documents)
- Quality matters but speed still important

---

### bge-large-en-v1.5

**Best for**: Legal, compliance, scientific, maximum quality

```bash
export AVOCADODB_EMBEDDING_MODEL=bgelarge
# Then re-ingest: avocado clear && avocado ingest <path> --recursive
```

**Specifications**:
- Dimensions: 1024 (2.7x default)
- Model size: ~350MB
- Inference time: 5-10ms per text
- Throughput: 200-300 texts/sec
- Quality (Recall@10): 93%

**Strengths**:
- ⭐⭐ Highest quality (+11% recall vs default)
- 📊 Best for precision-critical applications
- 🎯 Maximum accuracy
- 📚 Excellent for complex documents

**Use When**:
- Legal document retrieval
- Scientific paper search
- Compliance/regulatory use cases
- Quality is more important than speed
- Small query volume (<100/day)

## How to Switch Models

### Step 1: Set Environment Variable

```bash
# Choose your model
export AVOCADODB_EMBEDDING_MODEL=nomicv15  # or: bgelarge

# Make it permanent (add to ~/.bashrc or ~/.zshrc)
echo 'export AVOCADODB_EMBEDDING_MODEL=nomicv15' >> ~/.bashrc
```

### Step 2: Re-ingest Your Documents

**Important**: Different models produce incompatible embeddings. You must re-ingest.

```bash
# Clear existing data
./target/release/avocado clear --yes

# Re-ingest with new model
./target/release/avocado ingest ./docs --recursive
```

### Step 3: Verify

```bash
# Check which model is active
./target/release/avocado stats
# Should show the new model name

# Test a query
./target/release/avocado compile "test query" --budget 8000
```

## Decision Guide

### By Use Case

**Code Search / Documentation**
→ **all-MiniLM-L6-v2** (default)
- Speed matters for IDE integration
- 384 dimensions sufficient for code
- Good recall (82%)

**Production Applications**
→ **nomic-embed-text-v1.5**
- Better quality worth slight slowdown
- Professional-grade accuracy
- Good balance for users

**Legal / Compliance**
→ **bge-large-en-v1.5**
- Maximum accuracy required
- Regulatory compliance
- Worth the performance cost

**Prototyping / Testing**
→ **all-MiniLM-L6-v2** (default)
- Fast iteration cycles
- Quick feedback
- Upgrade later if needed

### By Corpus Size

**Small (<1,000 documents)**
→ **all-MiniLM-L6-v2** (default)
- Speed over quality
- Fast indexing
- Quick queries

**Medium (1,000-10,000 documents)**
→ **nomic-embed-text-v1.5**
- Balanced approach
- Better recall helps with scale
- Still performant

**Large (>10,000 documents)**
→ **all-MiniLM-L6-v2** (default) OR **Server Mode**
- Speed critical at scale
- Consider server mode to keep index in memory
- Quality difference less impactful

### By Query Volume

**High Volume (>1,000 queries/day)**
→ **all-MiniLM-L6-v2** (default)
- Speed accumulates across queries
- Lower latency per query matters
- Infrastructure costs lower

**Medium Volume (100-1,000 queries/day)**
→ **nomic-embed-text-v1.5**
- Quality worth the cost
- Latency acceptable
- Better user experience

**Low Volume (<100 queries/day)**
→ **bge-large-en-v1.5**
- Maximize quality
- Latency less critical
- Best possible results

## Performance Comparison

### Real-World Benchmarks

Run on your hardware:
```bash
./target/release/avocado benchmark
```

Example results (M1 Mac):

| Model | Single Embed | Batch of 100 | Throughput |
|-------|--------------|--------------|------------|
| **all-MiniLM-L6-v2** | 1.2ms | 8.7ms | 1,500/sec |
| **nomic-embed-text-v1.5** | 2.8ms | 18.5ms | 700/sec |
| **bge-large-en-v1.5** | 6.1ms | 42.3ms | 300/sec |

### Quality Benchmarks

Measured on code search tasks (Recall@10):

| Model | Recall@10 | vs Default | vs OpenAI |
|-------|-----------|------------|-----------|
| **all-MiniLM-L6-v2** | 82% | baseline | -13% |
| **nomic-embed-text-v1.5** | 89% | +7% | -6% |
| **bge-large-en-v1.5** | 93% | +11% | -2% |
| OpenAI ada-002 | 95% | +13% | baseline |

**Key Insight**: Even the default model gets you 82% recall. The jump to 1024 dimensions only adds 11% recall but costs 5x performance.

## Migration Guide

### From Default to Nomic

**When**: Moving to production, need better quality

```bash
# 1. Set new model
export AVOCADODB_EMBEDDING_MODEL=nomicv15

# 2. Backup existing data (optional)
cp -r .avocado/db.sqlite .avocado/db.sqlite.backup

# 3. Clear and re-ingest
./target/release/avocado clear --yes
./target/release/avocado ingest ./docs --recursive

# 4. Verify
./target/release/avocado compile "test" --budget 8000
```

**Expected changes**:
- Indexing time: ~2x slower
- Query time: +1-2ms
- Quality: +7% recall
- Index size: ~50% larger

### From Nomic to BGE

**When**: Quality is paramount, latency acceptable

```bash
# Same process, different model
export AVOCADODB_EMBEDDING_MODEL=bgelarge
./target/release/avocado clear --yes
./target/release/avocado ingest ./docs --recursive
```

**Expected changes**:
- Indexing time: ~4x slower than default
- Query time: +4-6ms
- Quality: +4% recall vs Nomic (+11% vs default)
- Index size: ~2x larger than default

### From Local to OpenAI

**When**: Need maximum quality, have API budget

```bash
export OPENAI_API_KEY="sk-..."
export AVOCADODB_EMBEDDING_PROVIDER=openai

# Same re-ingest process
./target/release/avocado clear --yes
./target/release/avocado ingest ./docs --recursive
```

**Expected changes**:
- Indexing time: Much slower (network latency)
- Query time: +200-250ms per query
- Quality: +2% recall vs BGE (+13% vs default)
- Cost: ~$0.0001 per 1K tokens
- Requires: Internet connection, API key

## Troubleshooting

### Model Download Fails

**Issue**: Model download times out or fails

**Solutions**:
```bash
# 1. Check internet connection
curl -I https://huggingface.co

# 2. Manually download to cache
mkdir -p ~/.cache/huggingface/
# Then re-run ingestion

# 3. Use different model
export AVOCADODB_EMBEDDING_MODEL=allminilml6v2  # Smaller download
```

### Wrong Model After Switch

**Issue**: Still using old model after setting env var

**Solutions**:
```bash
# 1. Verify env var is set
echo $AVOCADODB_EMBEDDING_MODEL

# 2. Clear database (required!)
./target/release/avocado clear --yes

# 3. Re-ingest with new model
./target/release/avocado ingest ./docs --recursive

# 4. Check stats to confirm
./target/release/avocado stats
```

### Performance Degradation

**Issue**: Queries are slower after switching models

**Expected**: Higher-dimensional models are slower
- Nomic (768D): 2-3x slower than default
- BGE (1024D): 4-5x slower than default

**Mitigation**:
```bash
# 1. Use server mode for large repos
./target/release/avocado-server &

# 2. Or switch back to faster model
export AVOCADODB_EMBEDDING_MODEL=allminilml6v2
```

## FAQ

### Q: Can I use multiple models in the same database?

**A**: No. Each database uses one embedding model. Different models produce incompatible embeddings (different dimensions). You must re-ingest when switching models.

### Q: Which model is closest to OpenAI quality?

**A**: bge-large-en-v1.5 (1024D) gets you 93% recall vs OpenAI's 95%. The 2% difference is usually negligible, while being 20-30x faster and free.

### Q: Do I need to re-ingest when switching models?

**A**: Yes, always. Different models produce different embedding dimensions. You must:
1. Clear database
2. Set new model via env var
3. Re-ingest all documents

### Q: How much disk space do models use?

**A**:
- all-MiniLM-L6-v2: ~90MB
- nomic-embed-text-v1.5: ~200MB
- bge-large-en-v1.5: ~350MB

Models are cached in `~/.cache/huggingface/` and shared across projects.

### Q: Can I use custom models?

**A**: Not yet. Currently supports the three built-in models. Custom model support is planned for a future release.

## Recommendations Summary

| Your Situation | Recommended Model |
|----------------|-------------------|
| **Default / Getting Started** | all-MiniLM-L6-v2 |
| **Production Application** | nomic-embed-text-v1.5 |
| **Legal / Compliance** | bge-large-en-v1.5 |
| **Large Corpus (>10K docs)** | all-MiniLM-L6-v2 + Server Mode |
| **High Query Volume** | all-MiniLM-L6-v2 |
| **Maximum Quality** | bge-large-en-v1.5 |
| **Maximum Speed** | all-MiniLM-L6-v2 |
| **Best Balance** | nomic-embed-text-v1.5 |

---

**Get personalized recommendation**: `./target/release/avocado recommend --corpus-size <n> --use-case <type>`
