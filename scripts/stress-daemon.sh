#!/usr/bin/env bash
set -euo pipefail

BASE_URL="${BASE_URL:-http://localhost:8765}"
PROJECTS=${PROJECTS:-3}
COMPILES=${COMPILES:-50}
CONCURRENCY=${CONCURRENCY:-8}

echo "=== Stressing daemon ==="
echo "Base URL: $BASE_URL"
echo "Projects: $PROJECTS  Compiles: $COMPILES  Concurrency: $CONCURRENCY"

tmp_root="$(mktemp -d)"
trap 'rm -rf "$tmp_root"' EXIT

mkproj() {
  local p="$1"
  mkdir -p "$p"
  echo "# AvocadoDB Stress
Deterministic RAG test project.
This is a sample document $p" > "$p/sample.md"
  curl -s -X POST "$BASE_URL/ingest" -H "Content-Type: application/json" \
    -d "{\"path\":\"sample.md\",\"content\":\"$(sed 's/"/\\"/g' "$p/sample.md")\",\"project\":\"$p\"}" >/dev/null
}

echo "--- Creating projects and ingesting"
for i in $(seq 1 "$PROJECTS"); do
  mkproj "$tmp_root/proj_$i"
done

run_compile() {
  local p="$1"
  local q="$2"
  curl -s -X POST "$BASE_URL/compile" -H "Content-Type: application/json" \
    -d "{\"query\":\"$q\",\"token_budget\":8000,\"project\":\"$p\"}" >/dev/null
}

echo "--- Running compiles"
queries=("What is AvocadoDB?" "Explain hybrid search" "What is MMR?" "How does determinism work?")

export -f run_compile

jobs=()
for i in $(seq 1 "$COMPILES"); do
  proj="$tmp_root/proj_$(( (i % PROJECTS) + 1 ))"
  q="${queries[$((i % ${#queries[@]}))]}"
  (run_compile "$proj" "$q") &
  jobs+=($!)
  # throttle
  if (( ${#jobs[@]} >= CONCURRENCY )); then
    wait -n
    jobs=("${jobs[@]:1}")
  fi
done
wait

echo "✅ Stress run complete"

