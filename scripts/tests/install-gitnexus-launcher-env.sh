#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

cat > "$tmp/claude.json" <<'JSON'
{
  "mcpServers": {
    "gitnexus": {
      "command": "npx",
      "args": ["gitnexus", "mcp"]
    }
  }
}
JSON

export GITNEXUS_LIBRARY_ONLY=1
export GITNEXUS_LAUNCHER="$tmp/profile-owned-gitnexus"
# shellcheck source=../install-gitnexus.sh
# shellcheck disable=SC1091
source "$repo_root/scripts/install-gitnexus.sh"

rewrite_claude_mcp_config "$tmp/claude.json"

node - "$tmp/claude.json" "$GITNEXUS_LAUNCHER" <<'NODE'
const fs = require("fs");
const config = JSON.parse(fs.readFileSync(process.argv[2], "utf8"));
const expected = process.argv[3];
if (config.mcpServers.gitnexus.command !== expected) {
  throw new Error(`launcher mismatch: ${config.mcpServers.gitnexus.command}`);
}
if (JSON.stringify(config.mcpServers.gitnexus.args) !== JSON.stringify(["mcp"])) {
  throw new Error(`args mismatch: ${JSON.stringify(config.mcpServers.gitnexus.args)}`);
}
NODE

printf '%s\n' "ok - Claude GitNexus MCP rewrite receives the launcher environment"
