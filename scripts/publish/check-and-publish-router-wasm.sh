#!/bin/bash
# Quick check and publish script for ruvector-router-wasm
# Run this manually when ruvector-router-core v0.1.1 is confirmed published

set -e

# Resolve repo root from script location (issue #359: don't hardcode paths).
REPO_ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)"

echo "Checking ruvector-router-core v0.1.1 availability..."
if cargo search ruvector-router-core 2>&1 | grep -q "ruvector-router-core.*0\.1\.1"; then
    echo "✓ ruvector-router-core v0.1.1 is available!"
    echo ""
    echo "Proceeding with ruvector-router-wasm publication..."
    echo ""

    # Load API key
    export $(grep "^CRATES_API_KEY=" "$REPO_ROOT"/.env | xargs)

    # Login
    cargo login "$CRATES_API_KEY"

    # Publish
    cd "$REPO_ROOT"/crates/ruvector-router-wasm
    cargo publish --allow-dirty

    echo ""
    echo "✓ ruvector-router-wasm v0.1.1 published successfully!"
else
    echo "✗ ruvector-router-core v0.1.1 not yet available on crates.io"
    echo "  Current version: $(cargo search ruvector-router-core 2>&1 | grep 'ruvector-router-core =' | head -1)"
    echo ""
    echo "Please wait for ruvector-router-core v0.1.1 to be published first."
    exit 1
fi
