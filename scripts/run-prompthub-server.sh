#!/usr/bin/env bash
# Run prompthub-server (axum REST API) for the RuVocal prompt_hub REST-bridge.
#
# RuVocal's server-side proxy (src/lib/server/prompthub/client.ts) calls this
# directly over HTTP — NO MCP. The LLM is never in the loop; prompt rendering and
# bundle generation are deterministic data ops, so MCP mediation would be pure
# token waste (ADR-262).
#
# RuVocal consumes it via PROMPTHUB_URL (default http://127.0.0.1:8080).
set -euo pipefail

HOST="${PROMPTHUB_HOST:-127.0.0.1}"
PORT="${PROMPTHUB_PORT:-8077}"  # 8080 is taken by sqld on this host; 8077 is the bridge default
HUB_DIR="${PROMPTHUB_DIR:-$(cd "$(dirname "$0")/../../prompt_hub" && pwd)}"

if [ ! -d "$HUB_DIR" ]; then
  echo "prompt_hub repo not found at $HUB_DIR (set PROMPTHUB_DIR)" >&2
  exit 1
fi

cd "$HUB_DIR"
echo "Starting prompthub-server (REST) on http://${HOST}:${PORT}  (RuVocal: PROMPTHUB_URL)"
exec cargo run --release -p prompthub-server -- --host "$HOST" --port "$PORT"
