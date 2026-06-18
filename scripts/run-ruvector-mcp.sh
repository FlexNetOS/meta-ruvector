#!/usr/bin/env bash
# Run the ruvector substrate as an MCP server (SSE/StreamableHTTP) for the RuVocal UI.
#
# RuVocal's MCP client (StreamableHTTP, SSE fallback) consumes this via MCP_SERVERS:
#   MCP_SERVERS=[{"name":"ruvector","url":"http://127.0.0.1:3001/mcp"}]
#
# Exposes the ruvector engine (vector_db_*, gnn_*) as chat tools — ADR-260 Pass 2.
set -euo pipefail
cd "$(dirname "$0")/.."

HOST="${RUVECTOR_MCP_HOST:-127.0.0.1}"
PORT="${RUVECTOR_MCP_PORT:-3001}"
BIN="target/release/ruvector-mcp"

if [ ! -x "$BIN" ]; then
  echo "Building ruvector-mcp (release)…"
  cargo build --release -p ruvector-cli --bin ruvector-mcp
fi

echo "Starting ruvector MCP server (SSE) on http://${HOST}:${PORT}  (endpoint: /mcp)"
exec "$BIN" -t sse --host "$HOST" --port "$PORT"
