#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
codex_home="${CODEX_HOME:-${HOME}/.codex}"

cargo run -p codex-env -- --repo "${repo_root}" install-prompts --codex-home "${codex_home}"

cat <<'MSG'
Installed Codex prompt mirrors.
Restart Codex, then invoke Claude command mirrors as /prompts:<name>.
Examples: /prompts:sparc-code, /prompts:claude-flow-swarm
MSG
