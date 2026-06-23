#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
codex_home="${repo_root}/.codex"

cargo run -p codex-env -- --repo "${repo_root}" install --codex-home "${codex_home}"

cat <<'MSG'
Installed Codex mirror surface and verified repo-local prompt commands.
Restart Codex from this repo, then invoke Claude command mirrors as /prompts:<name>.
Examples: /prompts:sparc-code, /prompts:sparc:code, /prompts:claude-flow-swarm
MSG
