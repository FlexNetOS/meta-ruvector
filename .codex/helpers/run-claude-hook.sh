#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 1 ]]; then
  echo "usage: run-claude-hook.sh <helper-file> [args...]" >&2
  exit 64
fi

repo_root="$(git rev-parse --show-toplevel)"
helper="$1"
shift

case "${helper}" in
  hook-handler.cjs|auto-memory-hook.mjs) ;;
  *)
    echo "unsupported Claude helper: ${helper}" >&2
    exit 64
    ;;
esac

export CLAUDE_PROJECT_DIR="${repo_root}"
exec node "${repo_root}/.claude/helpers/${helper}" "$@"
