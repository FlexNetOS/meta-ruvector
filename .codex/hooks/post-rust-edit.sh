#!/bin/bash
# Post-edit hook for Rust files in RuVector
# Runs format check, clippy, and optional benchmarks

set -e
repo_root="${CODEX_REPO_ROOT:-$(git rev-parse --show-toplevel 2>/dev/null || pwd)}"

FILE="$1"
RUN_BENCH="${2:-false}"

if [ -z "$FILE" ]; then
    echo "Usage: $0 <file_path> [run_bench]"
    exit 1
fi

EXT="${FILE##*.}"
if [ "$EXT" != "rs" ]; then
    exit 0  # Not a Rust file
fi

cd ${repo_root}

# Detect crate
CRATE_DIR=$(echo "$FILE" | grep -oP "crates/[^/]+" | head -1 || echo "")
CRATE_NAME=""

if [ -n "$CRATE_DIR" ]; then
    CRATE_NAME=$(basename "$CRATE_DIR")
fi

echo "🦀 Post-edit checks for: $FILE"

# 1. Format check (don't auto-fix, just report)
echo ""
echo "📐 Checking format..."
if cargo fmt --check -- "$FILE" 2>/dev/null; then
    echo "   ✅ Format OK"
else
    echo "   ⚠️  Format issues detected (run: cargo fmt)"
fi

# 2. Quick clippy check for the crate
if [ -n "$CRATE_NAME" ]; then
    echo ""
    echo "📎 Running clippy for $CRATE_NAME..."
    CLIPPY_OUT=$(cargo clippy -p "$CRATE_NAME" --message-format=short 2>&1 | grep -E "^(warning|error)" | head -5)
    if [ -z "$CLIPPY_OUT" ]; then
        echo "   ✅ No clippy warnings"
    else
        echo "$CLIPPY_OUT"
    fi
fi

# 3. Check for test file and suggest tests
TEST_FILE="${FILE%.rs}_test.rs"
if [ -f "$TEST_FILE" ]; then
    echo ""
    echo "🧪 Test file exists: $TEST_FILE"
fi

# 4. WASM size check for wasm crates
if echo "$FILE" | grep -qE "wasm|rvlite"; then
    echo ""
    echo "📏 WASM crate modified - consider running:"
    echo "   cd crates/rvlite && wasm-pack build --release"
    echo "   ls -lh pkg/*.wasm"
fi

# 5. Optional benchmark for performance-critical crates
if [ "$RUN_BENCH" = "true" ]; then
    case "$CRATE_NAME" in
        "ruvector-core"|"ruvector-bench")
            echo ""
            echo "📊 Running benchmarks..."
            cargo bench -p ruvector-bench -- --noplot 2>&1 | tail -20
            ;;
        "ruvector-mincut")
            echo ""
            echo "📊 Running mincut benchmarks..."
            cargo bench -p ruvector-mincut -- --noplot 2>&1 | tail -20
            ;;
        "ruvector-attention")
            echo ""
            echo "📊 Running attention benchmarks..."
            cargo bench -p ruvector-attention -- --noplot 2>&1 | tail -20
            ;;
    esac
fi

# Store metrics
METRICS_DIR="${repo_root}/.claude-flow/metrics"
mkdir -p "$METRICS_DIR"

# Record edit in metrics
echo "{\"file\": \"$FILE\", \"crate\": \"$CRATE_NAME\", \"timestamp\": \"$(date -Iseconds)\"}" >> "$METRICS_DIR/edit-log.jsonl"

echo ""
echo "✅ Post-edit checks complete"
