#!/usr/bin/env bash
#
# Attractor node 5: Distill (ruvector brain side).
#
# Promote the validated trajectory back into ReasoningBank so the next
# iteration starts from a richer memory. This is the closing of the
# self-learning loop.
#
# Full implementation: ReasoningBank::add_trajectory (re-exported by
# `sona`) + a SHAKE-256 witness anchor recorded by
# `crates/prime-radiant/src/execution/gate.rs`.
#
# Stub: writes a single record into .attractor/runs/<stamp>.distill.jsonl
# so the run history is auditable even before the real distillation
# pipeline is wired.

set -euo pipefail

readonly ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
readonly RUNS_DIR="$ROOT/.attractor/runs"
mkdir -p "$RUNS_DIR"

stamp="$(date -u +%Y%m%dT%H%M%SZ)"
out="$RUNS_DIR/${stamp}.distill.jsonl"
printf '{"distilled":true,"stub":true,"stamp":"%s"}\n' "$stamp" > "$out"

# JSON-escape the output path. If $ROOT contains a quote or backslash
# (rare but possible on dev hosts) the unescaped form would emit invalid
# JSON and break the JSONL parser in the runner.
escaped_out="${out//\\/\\\\}"
escaped_out="${escaped_out//\"/\\\"}"
printf '{"distilled":true,"stub":true,"output":"%s"}\n' "$escaped_out"
