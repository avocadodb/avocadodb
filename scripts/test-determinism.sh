#!/bin/bash
# Comprehensive Determinism Validation Test
# Runs the same query multiple times and verifies identical results

set -e

AVOCADO="./target/release/avocado"
ITERATIONS=100
QUERY="How does authentication work with JWT tokens?"
BUDGET=8000

echo "╔══════════════════════════════════════════════════════════════╗"
echo "║         AvocadoDB Determinism Validation Test               ║"
echo "╚══════════════════════════════════════════════════════════════╝"
echo ""
echo "Query: \"$QUERY\""
echo "Budget: $BUDGET tokens"
echo "Iterations: $ITERATIONS"
echo ""

# Create temp directory for results
TMP_DIR=$(mktemp -d)
echo "Temp directory: $TMP_DIR"
echo ""

# Run compilations and collect hashes
echo "Running $ITERATIONS compilations..."
for i in $(seq 1 $ITERATIONS); do
    # Compile and extract just the context text (not timing info)
    RESULT=$($AVOCADO compile "$QUERY" --budget $BUDGET 2>/dev/null | head -100)

    # Hash the result
    HASH=$(echo "$RESULT" | sha256sum | cut -d' ' -f1)
    echo "$HASH" >> "$TMP_DIR/hashes.txt"

    # Store full result for first iteration
    if [ $i -eq 1 ]; then
        echo "$RESULT" > "$TMP_DIR/first_result.txt"
        echo "$HASH" > "$TMP_DIR/expected_hash.txt"
    fi

    # Progress indicator
    if [ $((i % 10)) -eq 0 ]; then
        echo "  ✓ Completed $i iterations"
    fi
done

echo ""
echo "═══════════════════════════════════════════════════════════════"
echo "  RESULTS"
echo "═══════════════════════════════════════════════════════════════"
echo ""

# Count unique hashes
UNIQUE_HASHES=$(sort "$TMP_DIR/hashes.txt" | uniq | wc -l | tr -d ' ')
EXPECTED_HASH=$(cat "$TMP_DIR/expected_hash.txt")

echo "Total iterations: $ITERATIONS"
echo "Unique hashes:    $UNIQUE_HASHES"
echo ""

if [ "$UNIQUE_HASHES" -eq 1 ]; then
    echo "✅ PASS: 100% Deterministic!"
    echo ""
    echo "All $ITERATIONS compilations produced identical results."
    echo "Context hash: $EXPECTED_HASH"
    echo ""
    RESULT=0
else
    echo "❌ FAIL: Non-deterministic results detected!"
    echo ""
    echo "Found $UNIQUE_HASHES different hashes across $ITERATIONS iterations."
    echo "Expected hash: $EXPECTED_HASH"
    echo ""
    echo "Unique hashes found:"
    sort "$TMP_DIR/hashes.txt" | uniq -c
    echo ""
    RESULT=1
fi

# Show first result
echo "═══════════════════════════════════════════════════════════════"
echo "  SAMPLE OUTPUT"
echo "═══════════════════════════════════════════════════════════════"
echo ""
cat "$TMP_DIR/first_result.txt"
echo ""

# Cleanup
rm -rf "$TMP_DIR"

exit $RESULT
