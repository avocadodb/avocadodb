# AvocadoDB Testing Guide

This document explains how to validate AvocadoDB's core guarantees: determinism and performance.

## Quick Start

```bash
# Build the project
cargo build --release

# Initialize database and ingest test documents
./target/release/avocado init
./target/release/avocado ingest test-docs/ --recursive

# Run all tests and generate report
./scripts/run-tests.sh
```

## Test Suites

### 1. Determinism Validation

**Purpose:** Verify that same query → same result, every time.

**What it tests:**
- 100 compilations of the same query
- Hash verification of context output
- Consistency across all iterations

**Run:**
```bash
./scripts/test-determinism.sh
```

**Expected output:**
```
✅ PASS: 100% Deterministic!

All 100 compilations produced identical results.
Context hash: b08193f7acf79cfc4d81f088b6f0a5b8fb4e0586a3ac7de2a9864901b542f756
```

**What this proves:**
- AvocadoDB is 100% deterministic
- No randomness in search, ranking, or selection
- Same query always produces verifiable identical context
- Hash can be used to verify results in production

### 2. Performance Benchmark

**Purpose:** Measure compilation performance across different scenarios.

**What it tests:**
- Small budget (4K tokens)
- Medium budget (8K tokens)
- Large budget (16K tokens)
- Short queries
- Long queries
- Technical specific queries

**Run:**
```bash
./scripts/benchmark.sh
```

**Expected output:**
```
Results (10 iterations):
  Average time:       301ms
  Min time:           222ms
  Max time:           434ms
  Std deviation:      70.18ms
  Tokens used:        3657 / 8000 (45.7%)

  ✅ Performance target met (<500ms)
```

**What this proves:**
- Average compilation time: 283-339ms (well under 500ms target)
- Consistent performance across different query types
- Token utilization varies by query relevance
- Performance is production-ready

### 3. Comprehensive Test Suite

**Purpose:** Run all tests and generate a markdown report.

**Run:**
```bash
./scripts/run-tests.sh
```

**Output:** Generates `test-report-YYYYMMDD-HHMMSS.md` with:
- System information
- Database statistics
- Determinism validation results
- Performance benchmark results
- Executive summary

## Test Results

### Latest Results (Database: 253 spans, 23,589 tokens)

**Determinism:**
- ✅ 100/100 iterations identical
- ✅ Hash: `b08193f7acf79cfc4d81f088b6f0a5b8fb4e0586a3ac7de2a9864901b542f756`
- ✅ 100% deterministic

**Performance (10 iterations each):**

| Query Type | Avg Time | Min | Max | Tokens | Target |
|------------|----------|-----|-----|--------|--------|
| Small budget (4K) | 332ms | 214ms | 478ms | 92.8% | ✅ |
| Medium budget (8K) | 301ms | 222ms | 434ms | 45.7% | ✅ |
| Large budget (16K) | 320ms | 208ms | 700ms | 20.9% | ✅ |
| Short query | 304ms | 211ms | 468ms | 40.9% | ✅ |
| Long query | 339ms | 221ms | 470ms | 41.3% | ✅ |
| Technical query | 283ms | 224ms | 372ms | 41.8% | ✅ |

**All performance targets met!** ✅

## Integration Tests (Rust)

The project includes Rust integration tests in the `tests/` directory:

### Run Rust Tests

```bash
# Unit tests (no API key required)
cargo test

# Integration tests (requires OPENAI_API_KEY)
OPENAI_API_KEY=sk-... cargo test --test determinism -- --ignored
```

### Test Files

- `tests/determinism.rs` - Determinism validation with different scenarios
  - `test_deterministic_compilation` - 100 iterations basic test
  - `test_determinism_with_different_instances` - Cross-instance validation
  - More tests...

## Continuous Integration

For CI/CD pipelines, use the comprehensive test suite:

```yaml
# .github/workflows/test.yml example
- name: Run AvocadoDB Tests
  env:
    OPENAI_API_KEY: ${{ secrets.OPENAI_API_KEY }}
  run: |
    cargo build --release
    ./target/release/avocado init
    ./target/release/avocado ingest test-docs/ --recursive
    ./scripts/run-tests.sh
```

## Test Data

The `test-docs/` directory contains test documents:

- `authentication.md` - 204 lines, authentication documentation
- `api-reference.md` - 218 lines, API reference
- `cesm.md` - 4,433 lines, comprehensive technical document

Total: **253 spans, 23,589 tokens**

This provides a realistic corpus for testing with:
- Various document sizes
- Technical content
- Code examples
- Markdown formatting

## Interpreting Results

### Determinism Test

**PASS Criteria:**
- Unique hashes: 1
- All iterations produce identical context

**If FAIL:**
- Check for randomness in code (there shouldn't be any!)
- Verify no external state changes between runs
- Report as a critical bug

### Performance Test

**PASS Criteria:**
- Average compilation time < 500ms for 8K token budget
- Min time shows best-case performance
- Max time shows worst-case (usually OpenAI API latency)

**If performance degrades:**
- Check OpenAI API latency (primary bottleneck)
- Profile with `RUST_LOG=avocado_core=debug`
- Review algorithm changes for O(n²) issues

## Performance Profiling

Enable detailed timing logs:

```bash
RUST_LOG=avocado_core=debug ./target/release/avocado compile "query" --budget 8000
```

**Output shows timing breakdown:**
```
Embed query: 234ms
Semantic search: 1ms
Lexical search: 1ms
Hybrid fusion: 1ms
MMR diversification: 8ms
Token packing: 1ms
Deterministic sort: 1ms
Build context: 1ms
Count tokens: 35ms
━━━━━━━━━━━━━━━━━━━━━━
TOTAL: 283ms
```

**Bottleneck analysis:**
- If `Embed query` > 500ms: OpenAI API is slow (network latency)
- If `Semantic search` > 50ms: Index too large (>10K spans)
- If `MMR diversification` > 100ms: Too many candidates
- If `Count tokens` > 100ms: Tiktoken initialization issue

## Adding New Tests

### Shell Script Tests

Create a new script in `scripts/`:

```bash
#!/bin/bash
# scripts/my-test.sh
set -e

AVOCADO="./target/release/avocado"

# Your test logic here
$AVOCADO compile "query" --budget 8000

# Assert expected behavior
if [ $? -eq 0 ]; then
    echo "✅ PASS"
else
    echo "❌ FAIL"
    exit 1
fi
```

### Rust Integration Tests

Create a new file in `tests/`:

```rust
// tests/my_test.rs
use avocado_core::{Database, compiler};

#[tokio::test]
#[ignore]
async fn test_my_scenario() {
    // Test implementation
    assert!(true);
}
```

Run with: `cargo test --test my_test -- --ignored`

## Troubleshooting

### "Failed to open database"

Make sure you've initialized:
```bash
./target/release/avocado init
```

### "No spans found"

Ingest documents first:
```bash
./target/release/avocado ingest test-docs/ --recursive
```

### "OpenAI API Error"

Set your API key:
```bash
export OPENAI_API_KEY="sk-..."
```

### Test timeout

Increase timeout in script or use fewer iterations:
```bash
ITERATIONS=10 ./scripts/benchmark.sh
```

## Benchmarking Different Corpora

Test with your own documents:

```bash
# Clear existing data
./target/release/avocado clear

# Ingest your documents
./target/release/avocado ingest /path/to/your/docs --recursive

# Run benchmarks
./scripts/benchmark.sh
```

Expect performance to scale based on corpus size:
- < 1K spans: <300ms average
- 1K-10K spans: <500ms average
- > 10K spans: Consider Phase 2 HNSW optimization

## Reporting Issues

If tests fail, include:
1. Full test output
2. Database stats (`avocado stats`)
3. System info (OS, Rust version)
4. Test report if generated

```bash
# Generate comprehensive report
./scripts/run-tests.sh

# Report will be saved to test-report-YYYYMMDD-HHMMSS.md
```

---

**Test suite validates AvocadoDB's core guarantees: deterministic context compilation with production-ready performance.**
