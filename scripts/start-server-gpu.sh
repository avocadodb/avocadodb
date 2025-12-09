#!/usr/bin/env bash
set -euo pipefail

# Start AvocadoDB server configured to use a remote GPU embedding endpoint (e.g., Modal).
#
# Usage:
#   EMBED_URL="https://.../embed" ./scripts/start-server-gpu.sh
#   # optional overrides:
#   # MODEL="BAAI/bge-large-en-v1.5" DIM=1024 ./scripts/start-server-gpu.sh
#
# This script:
# - exports the required env vars for remote embeddings
# - starts avocado-server in the background
# - waits for /health
# - pre-warms the remote endpoint to avoid first-call latency

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SERVER_BIN="${ROOT_DIR}/target/release/avocado-server"
LOG_FILE="/tmp/avocado-server.log"

EMBED_URL="${EMBED_URL:-}"
MODEL="${MODEL:-BAAI/bge-large-en-v1.5}"
DIM="${DIM:-1024}"
URL="${URL:-http://127.0.0.1:8765}"

if [[ -z "${EMBED_URL}" ]]; then
  echo "ERROR: EMBED_URL is required (e.g., your Modal /embed endpoint)" >&2
  exit 1
fi

export AVOCADODB_EMBEDDING_PROVIDER=remote
export AVOCADODB_EMBEDDING_MODEL="${MODEL}"
export AVOCADODB_EMBEDDING_DIM="${DIM}"
export AVOCADODB_EMBEDDING_URL="${EMBED_URL}"
export AVOCADODB_FORBID_FALLBACKS=1

# Kill any existing server and start a clean one
pkill -f avocado-server >/dev/null 2>&1 || true

echo "Starting avocado-server with remote embeddings (${MODEL}, dim=${DIM})..."
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

echo "Pre-warming remote embed endpoint to avoid first-call latency..."
curl -s -X POST "${EMBED_URL}" -H "Content-Type: application/json" \
  -d '{"inputs":["warmup 1","warmup 2","warmup 3","warmup 4"]}' >/dev/null || true

echo "✓ Avocado server ready at ${URL} (remote embeddings: ${MODEL}, dim=${DIM})"
echo "Logs: ${LOG_FILE} | PID: ${SERVER_PID}"


