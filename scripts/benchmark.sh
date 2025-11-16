#!/bin/bash
# Performance Benchmark Suite for AvocadoDB
# Tests compilation performance across different scenarios

set -e

AVOCADO="./target/release/avocado"
ITERATIONS=10

echo "╔══════════════════════════════════════════════════════════════╗"
echo "║         AvocadoDB Performance Benchmark Suite                ║"
echo "╚══════════════════════════════════════════════════════════════╝"
echo ""

# Function to run benchmark for a query
run_benchmark() {
    local query="$1"
    local budget="$2"
    local name="$3"

    echo "──────────────────────────────────────────────────────────────"
    echo "Benchmark: $name"
    echo "Query: \"$query\""
    echo "Budget: $budget tokens"
    echo ""

    local times=()
    local tokens_used=()
    local total_time=0

    for i in $(seq 1 $ITERATIONS); do
        # Enable debug logging and capture timing
        output=$(RUST_LOG=avocado_core=debug $AVOCADO compile "$query" --budget $budget 2>&1)

        # Extract compilation time from debug logs
        time_ms=$(echo "$output" | grep "Total compilation time:" | grep -oE '[0-9]+' | tail -1)

        if [ -z "$time_ms" ]; then
            echo "Warning: Could not extract timing for iteration $i"
            continue
        fi

        times+=($time_ms)
        total_time=$((total_time + time_ms))

        # Extract tokens used
        tokens=$(echo "$output" | grep -oE "Tokens: [0-9]+" | grep -oE "[0-9]+" | head -1)
        if [ -n "$tokens" ]; then
            tokens_used+=($tokens)
        fi

        # Progress
        printf "."
    done

    echo ""
    echo ""

    # Calculate statistics
    local count=${#times[@]}
    if [ $count -eq 0 ]; then
        echo "❌ No successful compilations"
        return
    fi

    local avg_time=$((total_time / count))
    local min_time=$(printf '%s\n' "${times[@]}" | sort -n | head -1)
    local max_time=$(printf '%s\n' "${times[@]}" | sort -n | tail -1)

    # Calculate standard deviation
    local sum_sq_diff=0
    for time in "${times[@]}"; do
        local diff=$((time - avg_time))
        sum_sq_diff=$((sum_sq_diff + diff * diff))
    done
    local variance=$((sum_sq_diff / count))
    local std_dev=$(echo "scale=2; sqrt($variance)" | bc)

    # Average tokens used
    local avg_tokens=0
    if [ ${#tokens_used[@]} -gt 0 ]; then
        local total_tokens=0
        for t in "${tokens_used[@]}"; do
            total_tokens=$((total_tokens + t))
        done
        avg_tokens=$((total_tokens / ${#tokens_used[@]}))
    fi

    local utilization=0
    if [ $budget -gt 0 ] && [ $avg_tokens -gt 0 ]; then
        utilization=$(echo "scale=1; ($avg_tokens * 100) / $budget" | bc)
    fi

    echo "Results ($ITERATIONS iterations):"
    echo "  Average time:       ${avg_time}ms"
    echo "  Min time:           ${min_time}ms"
    echo "  Max time:           ${max_time}ms"
    echo "  Std deviation:      ${std_dev}ms"
    echo "  Tokens used:        $avg_tokens / $budget (${utilization}%)"
    echo ""

    # Check performance target
    if [ $avg_time -lt 500 ]; then
        echo "  ✅ Performance target met (<500ms)"
    else
        echo "  ⚠️  Performance target missed (>500ms)"
    fi
    echo ""
}

# Show database stats
echo "Database Statistics:"
$AVOCADO stats
echo ""

echo "══════════════════════════════════════════════════════════════"
echo "  PERFORMANCE BENCHMARKS"
echo "══════════════════════════════════════════════════════════════"
echo ""

# Benchmark 1: Small budget (typical GPT-3.5 usage)
run_benchmark \
    "authentication methods and security" \
    4000 \
    "Small Budget (4K tokens)"

# Benchmark 2: Medium budget (typical GPT-4 usage)
run_benchmark \
    "How does authentication work with JWT tokens?" \
    8000 \
    "Medium Budget (8K tokens)"

# Benchmark 3: Large budget (GPT-4 Turbo)
run_benchmark \
    "authentication security best practices" \
    16000 \
    "Large Budget (16K tokens)"

# Benchmark 4: Short query
run_benchmark \
    "auth" \
    8000 \
    "Short Query"

# Benchmark 5: Long query
run_benchmark \
    "Explain the complete authentication flow including JWT token generation validation refresh mechanisms and security best practices" \
    8000 \
    "Long Query"

# Benchmark 6: Specific technical query
run_benchmark \
    "POST /api/login endpoint" \
    8000 \
    "Specific Technical Query"

echo "══════════════════════════════════════════════════════════════"
echo "  BENCHMARK COMPLETE"
echo "══════════════════════════════════════════════════════════════"
echo ""
echo "All benchmarks completed successfully."
echo ""
