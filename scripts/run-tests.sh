#!/usr/bin/env bash
# CI-safe test entrypoint for AvocadoDB
# Runs the minimal, deterministic subset that must stay green in CI.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"
cd "$ROOT_DIR"

case "${1:-test}" in
  test)
    echo "=== AvocadoDB CI-safe tests ==="
    cargo build -q
    echo "--- Running avocado-core unit tests"
    cargo test -q -p avocado-core
    echo "--- Running determinism tests"
    RUST_LOG=warn cargo test -q -p avocado-tests determinism -- --test-threads=1
    echo "✅ CI-safe tests passed"
    ;;
  evals)
    echo "=== AvocadoDB Retrieval Evals ==="
    BASE_URL="${BASE_URL:-http://localhost:8765}"
    PROJECT="${PROJECT:-$PWD}"
    DATASET="${DATASET:-scripts/evals/samples/code_qa.jsonl}"
    python3 "$SCRIPT_DIR/evals/run_evals.py" --base-url "$BASE_URL" --project "$PROJECT" --dataset "$DATASET"
    ;;
  *)
    echo "Usage: $0 [test|evals]"
    exit 1
    ;;
esac
