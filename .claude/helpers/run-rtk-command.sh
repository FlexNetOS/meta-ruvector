#!/usr/bin/env sh
# Route hook payload commands through the repository's required RTK frontdoor.

set -eu

if [ "$#" -eq 0 ]; then
  printf '%s\n' 'usage: run-rtk-command.sh <command> [args...]' >&2
  exit 64
fi

exec rtk run "$*"
