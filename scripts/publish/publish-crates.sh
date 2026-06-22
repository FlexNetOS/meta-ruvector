#!/bin/bash
set -e

# Ruvector Crates Publishing Script
# This script publishes all Ruvector crates to crates.io in the correct dependency order
#
# Prerequisites:
# - Rust and Cargo installed
# - CRATES_API_KEY set in .env file
# - All crates build successfully
# - All tests pass

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Load environment variables from .env
if [ -f .env ]; then
    export $(grep -v '^#' .env | grep CRATES_API_KEY | xargs)
else
    echo -e "${RED}Error: .env file not found${NC}"
    exit 1
fi

# Check if CRATES_API_KEY is set
if [ -z "$CRATES_API_KEY" ]; then
    echo -e "${RED}Error: CRATES_API_KEY not found in .env${NC}"
    exit 1
fi

echo -e "${BLUE}========================================${NC}"
echo -e "${BLUE}   Ruvector Crates Publishing Script${NC}"
echo -e "${BLUE}========================================${NC}"
echo ""

# Configure cargo authentication
echo -e "${YELLOW}Configuring cargo authentication...${NC}"
cargo login "$CRATES_API_KEY"
echo -e "${GREEN}✓ Authentication configured${NC}"
echo ""

# Function to publish a crate
publish_crate() {
    local crate_path=$1
    local crate_name=$(basename "$crate_path")

    echo -e "${BLUE}========================================${NC}"
    echo -e "${BLUE}Publishing: ${crate_name}${NC}"
    echo -e "${BLUE}========================================${NC}"

    cd "$crate_path"

    # Verify the package
    echo -e "${YELLOW}Verifying package...${NC}"
    if cargo package --allow-dirty; then
        echo -e "${GREEN}✓ Package verification successful${NC}"
    else
        echo -e "${RED}✗ Package verification failed${NC}"
        cd - > /dev/null
        return 1
    fi

    # Publish the package
    echo -e "${YELLOW}Publishing to crates.io...${NC}"
    if cargo publish --allow-dirty; then
        echo -e "${GREEN}✓ ${crate_name} published successfully${NC}"
    else
        echo -e "${RED}✗ Failed to publish ${crate_name}${NC}"
        cd - > /dev/null
        return 1
    fi

    cd - > /dev/null

    # Wait a bit for crates.io to index the crate
    echo -e "${YELLOW}Waiting 30 seconds for crates.io to index...${NC}"
    sleep 30

    echo ""
}

# Function to check if crate is already published
check_published() {
    local crate_name=$1
    local version=$2

    if cargo search "$crate_name" --limit 1 | grep -q "^$crate_name = \"$version\""; then
        return 0  # Already published
    else
        return 1  # Not published
    fi
}

# Get version from workspace
VERSION=$(grep '^version = ' Cargo.toml | head -1 | sed 's/version = "\(.*\)"/\1/')
echo -e "${BLUE}Publishing version: ${VERSION}${NC}"
echo ""

# Publishing order (dependencies first)
CRATES=(
    # Auto-generated from the workspace dependency graph (cargo metadata).
    # Publishable crates only (publish != false); 47 publish=false crates
    # (examples, benches, hardware demos) are intentionally excluded.
    # Ordered by dependency depth: a crate is published after its deps.
    # --- dependency layer 0 ---
    "crates/emergent-time"
    "crates/hailort-sys"
    "crates/photonlayer-core"
    "crates/ruvector-acorn"
    "crates/ruvector-attn-mincut"
    "crates/ruvector-cnn"
    "crates/ruvector-cognitive-container"
    "crates/ruvector-coherence"
    "crates/ruvector-coherence-hnsw"
    "crates/ruvector-core"
    "crates/ruvector-dag-wasm"
    "crates/ruvector-delta-core"
    "crates/ruvector-diskann"
    "crates/ruvector-dither"
    "crates/ruvector-domain-expansion"
    "crates/ruvector-economy-wasm"
    "crates/ruvector-exotic-wasm"
    "crates/ruvector-fpga-transformer"
    "crates/ruvector-gnn-rerank"
    "crates/ruvector-graph-transformer-node"
    "crates/ruvector-graph-transformer-wasm"
    "crates/ruvector-hybrid"
    "crates/ruvector-learning-wasm"
    "crates/ruvector-math"
    "crates/ruvector-metrics"
    "crates/ruvector-mincut-gated-transformer"
    "crates/ruvector-nervous-system"
    "crates/ruvector-nervous-system-wasm"
    "crates/ruvector-profiler"
    "crates/ruvector-proof-gate"
    "crates/ruvector-rabitq"
    "crates/ruvector-rairs"
    "crates/ruvector-router-core"
    "crates/ruvector-solver"
    "crates/ruvector-sparse-inference"
    "crates/ruvector-sparsifier"
    "crates/ruvector-temporal-coherence"
    "crates/ruvector-temporal-tensor"
    "crates/ruvector-tiny-dancer-core"
    "crates/ruvix/crates/types"
    "crates/ruvllm-wasm"
    "crates/ruvllm_sparse_attention"
    "crates/rvAgent/rvagent-core"
    "crates/rvAgent/rvagent-wasm"
    "crates/sona"
    "crates/thermorust"
    "examples/scipix"
    # --- dependency layer 1 ---
    "crates/photonlayer-bench"
    "crates/photonlayer-ruvector"
    "crates/photonlayer-wasm"
    "crates/ruvector-acorn-wasm"
    "crates/ruvector-attention"
    "crates/ruvector-cluster"
    "crates/ruvector-cnn-wasm"
    "crates/ruvector-collections"
    "crates/ruvector-dag"
    "crates/ruvector-delta-consensus"
    "crates/ruvector-delta-graph"
    "crates/ruvector-delta-index"
    "crates/ruvector-delta-wasm"
    "crates/ruvector-diskann-node"
    "crates/ruvector-domain-expansion-wasm"
    "crates/ruvector-filter"
    "crates/ruvector-fpga-transformer-wasm"
    "crates/ruvector-gnn"
    "crates/ruvector-math-wasm"
    "crates/ruvector-mincut-gated-transformer-wasm"
    "crates/ruvector-rabitq-wasm"
    "crates/ruvector-raft"
    "crates/ruvector-replication"
    "crates/ruvector-robotics"
    "crates/ruvector-router-cli"
    "crates/ruvector-router-ffi"
    "crates/ruvector-router-wasm"
    "crates/ruvector-rulake"
    "crates/ruvector-server"
    "crates/ruvector-snapshot"
    "crates/ruvector-solver-node"
    "crates/ruvector-solver-wasm"
    "crates/ruvector-sparsifier-wasm"
    "crates/ruvector-tiny-dancer-node"
    "crates/ruvector-tiny-dancer-wasm"
    "crates/ruvector-verified"
    "crates/ruvix/crates/cap"
    "crates/ruvix/crates/hal"
    "crates/ruvix/crates/region"
    "crates/ruvix/crates/shell"
    "crates/ruvllm_retrieval_diffusion"
    "crates/rvAgent/rvagent-backends"
    "crates/rvlite"
    # --- dependency layer 2 ---
    "crates/photonlayer-cli"
    "crates/ruvector-attention-node"
    "crates/ruvector-attention-unified-wasm"
    "crates/ruvector-attention-wasm"
    "crates/ruvector-gnn-node"
    "crates/ruvector-gnn-wasm"
    "crates/ruvector-graph"
    "crates/ruvector-node"
    "crates/ruvector-verified-wasm"
    "crates/ruvector-wasm"
    "crates/ruvix/crates/aarch64"
    "crates/ruvix/crates/drivers"
    "crates/ruvix/crates/proof"
    "crates/ruvix/crates/queue"
    "crates/ruvix/crates/sched"
    "crates/ruvix/crates/vecgraph"
    "crates/rvAgent/rvagent-middleware"
    "crates/rvAgent/rvagent-tools"
    # --- dependency layer 3 ---
    "crates/ruvector-cli"
    "crates/ruvector-graph-node"
    "crates/ruvector-graph-wasm"
    "crates/ruvector-mincut"
    "crates/ruvix/crates/boot"
    "crates/ruvix/crates/nucleus"
    "crates/ruvllm"
    "crates/rvAgent/rvagent-a2a"
    "crates/rvAgent/rvagent-mcp"
    "crates/rvAgent/rvagent-subagents"
    "examples/google-cloud"
    # --- dependency layer 4 ---
    "crates/cognitum-gate-kernel"
    "crates/cognitum-gate-tilezero"
    "crates/ruvector-consciousness"
    "crates/ruvector-crv"
    "crates/ruvector-decompiler"
    "crates/ruvector-graph-condense"
    "crates/ruvector-graph-transformer"
    "crates/ruvector-mincut-node"
    "crates/ruvector-mincut-wasm"
    "crates/ruvector-perception"
    "crates/ruvix/examples/cognitive_demo"
    "crates/ruvllm-cli"
    "crates/rvAgent/rvagent-acp"
    "crates/rvAgent/rvagent-cli"
    # --- dependency layer 5 ---
    "crates/mcp-gate"
    "crates/prime-radiant"
    "crates/ruvector-consciousness-wasm"
    "crates/ruvector-decompiler-wasm"
    "crates/ruvector-graph-condense-wasm"
)

# Track success/failure
SUCCESS_COUNT=0
FAILED_CRATES=()

# Publish each crate
for crate in "${CRATES[@]}"; do
    if [ ! -d "$crate" ]; then
        echo -e "${YELLOW}Warning: $crate directory not found, skipping${NC}"
        continue
    fi

    crate_name=$(basename "$crate")

    # Check if already published
    if check_published "$crate_name" "$VERSION"; then
        echo -e "${YELLOW}$crate_name v$VERSION already published, skipping${NC}"
        ((SUCCESS_COUNT++))
        echo ""
        continue
    fi

    if publish_crate "$crate"; then
        ((SUCCESS_COUNT++))
    else
        FAILED_CRATES+=("$crate_name")
    fi
done

# Summary
echo -e "${BLUE}========================================${NC}"
echo -e "${BLUE}   Publishing Summary${NC}"
echo -e "${BLUE}========================================${NC}"
echo -e "${GREEN}Successfully published: ${SUCCESS_COUNT}/${#CRATES[@]}${NC}"

if [ ${#FAILED_CRATES[@]} -gt 0 ]; then
    echo -e "${RED}Failed to publish:${NC}"
    for crate in "${FAILED_CRATES[@]}"; do
        echo -e "${RED}  - $crate${NC}"
    done
    exit 1
else
    echo -e "${GREEN}All crates published successfully! 🎉${NC}"
fi

echo ""
echo -e "${BLUE}View your crates at: https://crates.io/users/ruvector${NC}"
