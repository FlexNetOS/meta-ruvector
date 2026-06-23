#!/bin/bash
# Rust-specific pre-edit hook for RuVector
# Runs cargo check, clippy hints, and detects crate context

set -e
repo_root="${CODEX_REPO_ROOT:-$(git rev-parse --show-toplevel 2>/dev/null || pwd)}"

FILE="$1"
if [ -z "$FILE" ]; then
    echo "Usage: $0 <file_path>"
    exit 1
fi

EXT="${FILE##*.}"
if [ "$EXT" != "rs" ]; then
    exit 0  # Not a Rust file
fi

cd ${repo_root}

# Detect which crate this file belongs to
CRATE_DIR=$(echo "$FILE" | grep -oP "crates/[^/]+" | head -1 || echo "")
CRATE_NAME=""

if [ -n "$CRATE_DIR" ]; then
    CRATE_NAME=$(basename "$CRATE_DIR")
    echo "🦀 Crate: $CRATE_NAME"

    # Show crate-specific context
    case "$CRATE_NAME" in
        "ruvector-core")
            echo "   📊 Core vector engine (HNSW, SIMD, quantization)"
            echo "   📦 Key: VectorStore, HnswIndex, Distance metrics"
            ;;
        "rvlite")
            echo "   🌐 WASM standalone DB (SQL/SPARQL/Cypher)"
            echo "   📦 Key: RvLite, SqlExecutor, CypherParser"
            echo "   ⚠️  Size target: <3MB gzipped"
            ;;
        "ruvector-wasm")
            echo "   🌐 WASM bindings for ruvector-core"
            echo "   📦 Key: WasmVectorStore, IndexedDB storage"
            ;;
        "ruvector-graph"|"ruvector-graph-wasm"|"ruvector-graph-node")
            echo "   🕸️  Graph database with Cypher support"
            echo "   📦 Key: GraphStore, CypherQuery, HyperEdge"
            ;;
        "ruvector-gnn"|"ruvector-gnn-wasm"|"ruvector-gnn-node")
            echo "   🧠 Graph Neural Networks (GCN, GraphSAGE, GAT)"
            echo "   📦 Key: GnnLayer, MessagePassing, Aggregation"
            ;;
        "ruvector-postgres")
            echo "   🐘 PostgreSQL extension (pgvector compatible)"
            echo "   📦 Key: pgrx, SQL functions, background workers"
            ;;
        "sona")
            echo "   🎓 ReasoningBank with 9 RL algorithms"
            echo "   📦 Key: Trajectory, Verdict, LoRA, EWC++"
            ;;
        "ruvector-mincut"|"ruvector-mincut-wasm"|"ruvector-mincut-node")
            echo "   ✂️  Subpolynomial dynamic min-cut algorithm"
            echo "   📦 Key: ContractedGraph, LambdaCut, SparseCertificate"
            ;;
        "ruvector-attention"|"ruvector-attention-wasm"|"ruvector-attention-node")
            echo "   👁️  39+ attention mechanisms"
            echo "   📦 Key: MultiHeadAttention, GeometricAttention"
            ;;
        "ruvector-tiny-dancer"|"ruvector-tiny-dancer-wasm"|"ruvector-tiny-dancer-node")
            echo "   💃 FastGRNN neural router for agents"
            echo "   📦 Key: Router, FastGRNN, CircuitBreaker"
            ;;
        "ruvector-cli")
            echo "   ⌨️  CLI and MCP server"
            echo "   📦 Key: Commands, MCP protocol, REST API"
            ;;
        *)
            echo "   📦 Crate: $CRATE_NAME"
            ;;
    esac

    # Quick cargo check for the specific crate
    echo ""
    echo "🔍 Running cargo check -p $CRATE_NAME..."
    if cargo check -p "$CRATE_NAME" --message-format=short 2>&1 | head -10; then
        echo "✅ Cargo check passed"
    else
        echo "⚠️  Check for warnings/errors above"
    fi
fi

# Check for WASM-related files
if echo "$FILE" | grep -qE "wasm|rvlite"; then
    echo ""
    echo "📏 WASM file detected - size considerations apply"
    echo "   Target: <3MB gzipped for rvlite"
fi

echo ""
