#!/bin/bash
# Smoke test for .claude/helpers/swarm-orchestrator.sh
# Drives the full lifecycle (init -> spawn -> orchestrate -> status -> shutdown)
# against the real swarm-hooks.sh coordination layer and asserts observable state.
set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
ORCH="$REPO_ROOT/.claude/helpers/swarm-orchestrator.sh"
SWARM_DIR="$REPO_ROOT/.claude-flow/swarm"

pass=0; fail=0
ok()   { echo "  ✓ $1"; pass=$((pass + 1)); }
bad()  { echo "  ✗ $1"; fail=$((fail + 1)); }
check() { if eval "$2"; then ok "$1"; else bad "$1 ($2)"; fi; }

echo "== swarm-orchestrator smoke =="

# Clean slate so assertions are deterministic.
rm -rf "$SWARM_DIR"

# 1. init
"$ORCH" init >/dev/null
check "init writes active swarm.json"        '[ "$(jq -r .status "$SWARM_DIR/swarm.json")" = active ]'
check "topology from settings (hierarchical-mesh)" '[ "$(jq -r .topology "$SWARM_DIR/swarm.json")" = hierarchical-mesh ]'
MAX="$(jq -r .maxAgents "$SWARM_DIR/swarm.json")"
check "maxAgents is a positive integer"      '[ "$MAX" -ge 1 ] 2>/dev/null'

# 2. spawn coordinator (first agent in a hierarchical topology)
CID="$("$ORCH" spawn coordinator queen | tail -1)"
check "coordinator recorded on swarm"        '[ "$(jq -r .coordinator "$SWARM_DIR/swarm.json")" = "$CID" ]'
check "coordinator role is coordinator"      '[ "$(jq -r --arg id "$CID" '"'"'.agents[]|select(.id==$id)|.role'"'"' "$SWARM_DIR/roster.json")" = coordinator ]'

# 3. spawn workers
"$ORCH" spawn coder   >/dev/null
"$ORCH" spawn tester  >/dev/null
check "roster has 3 agents"                  '[ "$(jq ".agents|length" "$SWARM_DIR/roster.json")" -eq 3 ]'
check "two workers (non-coordinator)"        '[ "$(jq "[.agents[]|select(.role==\"worker\")]|length" "$SWARM_DIR/roster.json")" -eq 2 ]'

# 4. orchestrate -> workers move to "working", handoffs created
"$ORCH" orchestrate "implement feature X" >/dev/null
check "workers marked working after orchestrate" '[ "$(jq "[.agents[]|select(.status==\"working\")]|length" "$SWARM_DIR/roster.json")" -eq 2 ]'
check "handoffs were created"                '[ -d "$SWARM_DIR/handoffs" ] && [ "$(ls -1 "$SWARM_DIR/handoffs" 2>/dev/null | wc -l)" -ge 1 ]'

# 5. status runs clean and reports the swarm id.
# Capture to a var first: piping into `grep -q` would SIGPIPE the live status process
# mid-write (it closes the pipe on first match), which is a harness artifact, not a fault.
STATUS_OUT="$("$ORCH" status 2>/dev/null)"
if printf '%s' "$STATUS_OUT" | grep -q "$(jq -r .id "$SWARM_DIR/swarm.json")"; then
  ok "status reports swarm id"
else
  bad "status reports swarm id"
fi

# 6. maxAgents cap is enforced (fill to max, then expect failure)
CUR="$(jq ".agents|length" "$SWARM_DIR/roster.json")"
while [ "$CUR" -lt "$MAX" ]; do "$ORCH" spawn filler >/dev/null 2>&1; CUR=$((CUR + 1)); done
"$ORCH" spawn overflow >/dev/null 2>&1 && bad "maxAgents cap rejects overflow" || ok "maxAgents cap rejects overflow"

# 7. shutdown
"$ORCH" shutdown >/dev/null
check "shutdown marks swarm inactive"        '[ "$(jq -r .status "$SWARM_DIR/swarm.json")" = inactive ]'
check "shutdown clears the roster"           '[ "$(jq ".agents|length" "$SWARM_DIR/roster.json")" -eq 0 ]'

echo "== result: $pass passed, $fail failed =="
[ "$fail" -eq 0 ]
