#!/bin/bash
# verify.sh — .claude harness verification & quality gate
#
# Applies the verification-quality skill to the real .claude harness: it runs the
# syntax, config, and test checks across the harness, computes a TRUTH SCORE
# (checks passed / total, 0.0–1.0), and gates at a threshold (default 0.95).
# Exit 0 if score >= threshold, else exit 1 — usable as a CI / pre-commit gate.
#
#   verify.sh            run all checks, print the score, gate at the threshold
#   verify.sh --quiet    only print the final score line
#   VERIFY_THRESHOLD=1.0 verify.sh   require a perfect score
set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
CLAUDE_DIR="$PROJECT_ROOT/.claude"
THRESHOLD="${VERIFY_THRESHOLD:-0.95}"
QUIET=0; [ "${1:-}" = "--quiet" ] && QUIET=1

pass=0; fail=0; failed_checks=""
say() { [ "$QUIET" -eq 1 ] || echo "$@"; }

# run <name> <command...> : run a check, record pass/fail.
run() {
  local name="$1"; shift
  if "$@" >/dev/null 2>&1; then
    pass=$((pass + 1)); say "  ✓ $name"
  else
    fail=$((fail + 1)); failed_checks="$failed_checks\n    - $name"; say "  ✗ $name"
  fi
}

say "== .claude harness verification =="

# 1. Bash syntax across every shell helper.
say "[syntax] bash helpers"
for f in "$CLAUDE_DIR"/helpers/*.sh; do
  [ -e "$f" ] || continue
  run "bash -n $(basename "$f")" bash -n "$f"
done

# 2. JS/ESM syntax across every node helper + the intelligence modules.
say "[syntax] node modules"
while IFS= read -r f; do
  run "node --check $(echo "$f" | sed "s#$CLAUDE_DIR/##")" node --check "$f"
done < <(find "$CLAUDE_DIR/helpers" "$CLAUDE_DIR/intelligence" -maxdepth 2 \
           \( -name '*.js' -o -name '*.mjs' -o -name '*.cjs' \) \
           -not -path '*/data/*' -not -path '*/node_modules/*' 2>/dev/null)

# 3. Config JSON validity.
say "[config] json"
for f in "$CLAUDE_DIR/settings.json" "$CLAUDE_DIR/settings.local.json"; do
  [ -e "$f" ] || continue
  run "jq empty $(basename "$f")" jq empty "$f"
done

# 4. Test suites (the cycle-1/2 tests, now aggregated).
say "[tests] suites"
[ -f "$PROJECT_ROOT/tests/swarm/swarm-orchestrator-smoke.sh" ] && \
  run "swarm-orchestrator smoke" bash "$PROJECT_ROOT/tests/swarm/swarm-orchestrator-smoke.sh"
[ -f "$CLAUDE_DIR/intelligence/test/characterization.test.mjs" ] && \
  run "intelligence characterization" node "$CLAUDE_DIR/intelligence/test/characterization.test.mjs"
[ -f "$CLAUDE_DIR/intelligence/test/reasoning-bank.test.mjs" ] && \
  run "reasoning-bank (verdict/distill/replay)" node "$CLAUDE_DIR/intelligence/test/reasoning-bank.test.mjs"

# Truth score + gate.
total=$((pass + fail))
score="$(awk -v p="$pass" -v t="$total" 'BEGIN { if (t == 0) print "0.000"; else printf "%.3f", p / t }')"
verdict="$(awk -v s="$score" -v th="$THRESHOLD" 'BEGIN { print (s + 1e-9 >= th) ? "PASS" : "FAIL" }')"

if [ "$fail" -gt 0 ] && [ "$QUIET" -eq 0 ]; then
  echo -e "  failed:$failed_checks"
fi
echo "== truth score: $score ($pass/$total) — $verdict (threshold $THRESHOLD) =="
[ "$verdict" = "PASS" ]
