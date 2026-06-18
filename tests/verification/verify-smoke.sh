#!/bin/bash
# Smoke test for .claude/helpers/verify.sh — proves the verification gate works.
set -uo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
VERIFY="$REPO_ROOT/.claude/helpers/verify.sh"

pass=0; fail=0
ok()  { echo "  ✓ $1"; pass=$((pass + 1)); }
bad() { echo "  ✗ $1"; fail=$((fail + 1)); }

echo "== verify.sh smoke =="

# 1. Default run gates PASS on a healthy harness (exit 0) and emits a score line.
OUT="$(bash "$VERIFY" --quiet 2>/dev/null)"; rc=$?
echo "$OUT" | grep -q "truth score:" && ok "emits a truth-score line" || bad "emits a truth-score line"
[ "$rc" -eq 0 ] && ok "exit 0 when score >= threshold" || bad "exit 0 when score >= threshold"

# 2. The score is a float in [0,1].
SCORE="$(echo "$OUT" | grep -oE 'truth score: [0-9.]+' | grep -oE '[0-9.]+$')"
awk -v s="$SCORE" 'BEGIN { exit !(s >= 0 && s <= 1) }' && ok "score in [0,1] ($SCORE)" || bad "score in [0,1] ($SCORE)"

# 3. An impossible threshold forces FAIL (exit 1) — the gate actually gates.
VERIFY_THRESHOLD=1.01 bash "$VERIFY" --quiet >/dev/null 2>&1 && bad "gate rejects impossible threshold" || ok "gate rejects impossible threshold (exit 1)"

echo "== result: $pass passed, $fail failed =="
[ "$fail" -eq 0 ]
