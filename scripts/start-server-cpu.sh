#!/usr/bin/env bash
set -euo pipefail

# Start AvocadoDB server configured to use local CPU embeddings (fastembed via Rust).
#
# Usage:
#   ./scripts/start-server-cpu.sh
#   # optional:
#   # URL="http://127.0.0.1:8765" ./scripts/start-server-cpu.sh
#
# This script:
# - exports env vars for local embeddings only
# - starts avocado-server in the background
# - waits for /health

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SERVER_BIN="${ROOT_DIR}/target/release/avocado-server"
LOG_FILE="/tmp/avocado-server.log"
URL="${URL:-http://127.0.0.1:8765}"

export AVOCADODB_EMBEDDING_PROVIDER=local
export AVOCADODB_FORBID_FALLBACKS=1

pkill -f avocado-server >/dev/null 2>&1 || true

echo "Starting avocado-server with local CPU embeddings..."
RUST_LOG=info "${SERVER_BIN}" > "${LOG_FILE}" 2>&1 &
SERVER_PID=$!

echo "Waiting for server health at ${URL}/health ..."
until curl -sf "${URL}/health" >/dev/null; do
  if ! kill -0 "${SERVER_PID}" 2>/dev/null; then
    echo "Server crashed. Last 100 log lines:" >&2
    tail -n 100 "${LOG_FILE}" >&2 || true
    exit 1
  fi
  sleep 1
done

echo "✓ Avocado server ready at ${URL} (local CPU embeddings)"
echo "Logs: ${LOG_FILE} | PID: ${SERVER_PID}"


