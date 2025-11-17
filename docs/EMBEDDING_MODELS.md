# Embedding Model Configuration

## How to Increase Dimensionality

AvocadoDB supports multiple embedding models with different dimensions. To increase dimensionality, set the `AVOCADODB_EMBEDDING_MODEL` environment variable.

## Available Models

| Model | Dimensions | Quality | Speed | Use Case |
|-------|------------|---------|-------|----------|
| **AllMiniLML6V2** (default) | 384 | Good | Fastest | General purpose, fast |
| AllMiniLML12V2 | 384 | Good+ | Fast | Slightly better quality |
| BGESmallENV15 | 384 | Good | Fast | Optimized for English |
| **NomicEmbedTextV15** | 768 | Better | Medium | Good balance |
| NomicEmbedTextV1 | 768 | Better | Medium | Good balance |
| **BGELargeENV15** | 1024 | Best | Slower | Maximum quality |

## Usage

### Default (384 dimensions)
```bash
# No configuration needed - uses AllMiniLML6V2
avocado compile "your query"
```

### 768 dimensions (Nomic)
```bash
export AVOCADODB_EMBEDDING_MODEL=nomic
avocado compile "your query"
```

### 1024 dimensions (BGE Large)
```bash
export AVOCADODB_EMBEDDING_MODEL=bgelarge
avocado compile "your query"
```

## Model Aliases

You can use any of these aliases for each model:

**AllMiniLML6V2** (384 dims):
- `allminilml6v2`
- `all-minilm-l6-v2`
- `minilm6`

**AllMiniLML12V2** (384 dims):
- `allminilml12v2`
- `all-minilm-l12-v2`
- `minilm12`

**BGESmallENV15** (384 dims):
- `bgesmallen`
- `bge-small-en-v1.5`
- `bgesmall`

**BGELargeENV15** (1024 dims):
- `bgelargeen`
- `bge-large-en-v1.5`
- `bgelarge`

**NomicEmbedTextV1** (768 dims):
- `nomicv1`
- `nomic-embed-text-v1`

**NomicEmbedTextV15** (768 dims):
- `nomicv15`
- `nomic-embed-text-v1.5`
- `nomic`

## Performance Impact

Higher dimensions = better quality but:
- ⚠️ **Slower inference** (more computation)
- ⚠️ **More memory** (larger vectors)
- ⚠️ **Larger model files** (more disk space)

### Performance Comparison

| Model | Dimensions | Inference Time | Model Size |
|-------|------------|----------------|------------|
| AllMiniLML6V2 | 384 | ~1-3ms | ~90MB |
| NomicEmbedTextV15 | 768 | ~2-5ms | ~150MB |
| BGELargeENV15 | 1024 | ~5-10ms | ~400MB |

## Recommendations

**Use 384 dimensions (default) when:**
- ✅ Speed is important
- ✅ Storage is limited
- ✅ General-purpose semantic search
- ✅ Most code/documentation queries

**Use 768 dimensions (Nomic) when:**
- ✅ You need better quality
- ✅ You can accept slightly slower inference
- ✅ Complex semantic relationships matter

**Use 1024 dimensions (BGE Large) when:**
- ✅ Maximum quality is required
- ✅ You have sufficient CPU/memory
- ✅ Quality > speed

## Important Notes

1. **Model Download**: Models are downloaded automatically on first use and cached locally
2. **Database Compatibility**: Changing models requires re-ingesting your data (different dimensions)
3. **Mixed Dimensions**: You cannot mix different dimension models in the same database
4. **Migration**: To switch models, clear your database and re-ingest:
   ```bash
   avocado clear
   export AVOCADODB_EMBEDDING_MODEL=nomic
   avocado ingest ./your-codebase --recursive
   ```

## Example: Switching to Higher Dimensions

```bash
# 1. Clear existing database (if you have one)
avocado clear

# 2. Set higher-dimensional model
export AVOCADODB_EMBEDDING_MODEL=nomic  # 768 dimensions

# 3. Re-ingest your codebase
avocado ingest ./your-codebase --recursive

# 4. Use as normal
avocado compile "your query"
```

