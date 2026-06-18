#!/bin/bash
# Claude Flow V3 - Swarm Orchestrator
#
# The top-level entry point that unifies the existing swarm coordination
# primitives (swarm-hooks.sh: messaging / consensus / handoff) into the
# declared hierarchical-mesh topology. This is the missing layer above the
# per-agent hooks: it owns swarm-level lifecycle and task distribution.
#
#   init       Initialize a swarm (topology + maxAgents from settings.json)
#   spawn      Register an agent of a given type (first becomes coordinator
#              in a hierarchical topology); enforces the maxAgents cap
#   orchestrate  Distribute a task across the mesh (coordinator -> workers
#              via swarm-hooks handoffs) and broadcast task context
#   status     Show swarm topology, roster, and coordination stats
#   shutdown   Mark the swarm inactive and clear the roster
#
# State lives under .claude-flow/swarm/ (gitignored), shared with swarm-hooks.sh.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
SWARM_DIR="$PROJECT_ROOT/.claude-flow/swarm"
SWARM_FILE="$SWARM_DIR/swarm.json"
ROSTER_FILE="$SWARM_DIR/roster.json"
HOOKS="$SCRIPT_DIR/swarm-hooks.sh"
SETTINGS="$PROJECT_ROOT/.claude/settings.json"

mkdir -p "$SWARM_DIR"

log() { echo "[swarm] $*"; }
die() { echo "[swarm] error: $*" >&2; exit 1; }

new_id() { echo "${1}_$(date +%s)_$(head -c 4 /dev/urandom | xxd -p)"; }

# --- config (defaults from settings.json .claudeFlow.swarm) -------------------
cfg_topology() { jq -r '.claudeFlow.swarm.topology // "hierarchical-mesh"' "$SETTINGS" 2>/dev/null || echo "hierarchical-mesh"; }
cfg_max_agents() { jq -r '.claudeFlow.swarm.maxAgents // 8' "$SETTINGS" 2>/dev/null || echo 8; }

require_active() {
  [ -f "$SWARM_FILE" ] && [ "$(jq -r '.status' "$SWARM_FILE" 2>/dev/null)" = "active" ] \
    || die "no active swarm — run: $(basename "$0") init"
}

# --- lifecycle ---------------------------------------------------------------
swarm_init() {
  local topology="${1:-$(cfg_topology)}"
  local max="${2:-$(cfg_max_agents)}"
  if [ -f "$SWARM_FILE" ] && [ "$(jq -r '.status' "$SWARM_FILE" 2>/dev/null)" = "active" ]; then
    log "swarm already active ($(jq -r '.id' "$SWARM_FILE")) — reusing"
    return 0
  fi
  local id; id="$(new_id swarm)"
  jq -n --arg id "$id" --arg t "$topology" --argjson m "$max" --arg ts "$(date -Iseconds)" \
    '{id:$id, topology:$t, maxAgents:$m, status:"active", createdAt:$ts, coordinator:null}' > "$SWARM_FILE"
  echo '{"agents":[]}' > "$ROSTER_FILE"
  "$HOOKS" stats >/dev/null 2>&1 || true   # ensure the hooks stats file exists
  log "initialized $id (topology=$topology, maxAgents=$max)"
}

swarm_spawn() {
  require_active
  local type="${1:?type required}"
  local name="${2:-$type}"
  local max count topo coord id role="worker"
  max="$(jq -r '.maxAgents' "$SWARM_FILE")"
  count="$(jq '.agents | length' "$ROSTER_FILE")"
  [ "$count" -ge "$max" ] && die "maxAgents ($max) reached — cannot spawn more"
  topo="$(jq -r '.topology' "$SWARM_FILE")"
  coord="$(jq -r '.coordinator' "$SWARM_FILE")"
  id="$(new_id agent)"
  # In a hierarchical topology the first agent becomes the coordinator (queen).
  if [ "$coord" = "null" ] && [[ "$topo" == *hierarchical* ]]; then
    role="coordinator"
    jq --arg id "$id" '.coordinator = $id' "$SWARM_FILE" > "$SWARM_FILE.tmp" && mv "$SWARM_FILE.tmp" "$SWARM_FILE"
  fi
  # Register into the coordination layer (a broadcast triggers swarm-hooks register_agent).
  AGENTIC_FLOW_AGENT_ID="$id" AGENTIC_FLOW_AGENT_NAME="$name" \
    "$HOOKS" broadcast "spawn type=$type role=$role" >/dev/null 2>&1 || true
  jq --arg id "$id" --arg t "$type" --arg n "$name" --arg r "$role" --arg ts "$(date -Iseconds)" \
    '.agents += [{id:$id, type:$t, name:$n, role:$r, status:"idle", spawnedAt:$ts}]' \
    "$ROSTER_FILE" > "$ROSTER_FILE.tmp" && mv "$ROSTER_FILE.tmp" "$ROSTER_FILE"
  log "spawned $type '$name' ($id) role=$role [$((count + 1))/$max]"
  echo "$id"
}

swarm_orchestrate() {
  require_active
  local task="${1:?task description required}"
  local coord workers assigned=0
  coord="$(jq -r '.coordinator // empty' "$SWARM_FILE")"
  [ -z "$coord" ] && coord="$(jq -r '.agents[0].id // empty' "$ROSTER_FILE")"
  [ -z "$coord" ] && die "no agents spawned — run: $(basename "$0") spawn <type>"
  # Coordinator broadcasts the task context to the whole mesh.
  AGENTIC_FLOW_AGENT_ID="$coord" AGENTIC_FLOW_AGENT_NAME="coordinator" \
    "$HOOKS" broadcast "task: $task" >/dev/null 2>&1 || true
  # Distribute: coordinator hands the task off to each worker.
  workers="$(jq -r --arg c "$coord" '.agents[] | select(.id != $c) | .id' "$ROSTER_FILE")"
  if [ -n "$workers" ]; then
    while read -r aid; do
      [ -z "$aid" ] && continue
      AGENTIC_FLOW_AGENT_ID="$coord" \
        "$HOOKS" handoff "$aid" "$task" "{\"task\":\"$task\"}" >/dev/null 2>&1 || true
      jq --arg id "$aid" '(.agents[] | select(.id == $id) | .status) = "working"' \
        "$ROSTER_FILE" > "$ROSTER_FILE.tmp" && mv "$ROSTER_FILE.tmp" "$ROSTER_FILE"
      assigned=$((assigned + 1))
    done <<< "$workers"
  fi
  log "orchestrated task across $assigned worker(s) via coordinator $coord"
  [ "$assigned" -eq 0 ] && log "note: only the coordinator is present — it owns the task directly"
  return 0
}

swarm_status() {
  if [ ! -f "$SWARM_FILE" ]; then
    echo "No swarm initialized."
    return 0
  fi
  local id topo max status coord count
  id="$(jq -r '.id' "$SWARM_FILE")"; topo="$(jq -r '.topology' "$SWARM_FILE")"
  max="$(jq -r '.maxAgents' "$SWARM_FILE")"; status="$(jq -r '.status' "$SWARM_FILE")"
  coord="$(jq -r '.coordinator // "—"' "$SWARM_FILE")"
  count="$(jq '.agents | length' "$ROSTER_FILE" 2>/dev/null || echo 0)"
  echo "╔══════════════════════════════════════════════════════════╗"
  echo "║  Swarm: $id"
  echo "║  topology=$topo  status=$status  agents=$count/$max"
  echo "║  coordinator=$coord"
  echo "╠══════════════════════════════════════════════════════════╣"
  if [ "$count" -gt 0 ]; then
    jq -r '.agents[] | "║  • \(.role | .[0:1] | ascii_upcase)\(.role[1:]) \(.type) (\(.name)) — \(.status)"' "$ROSTER_FILE"
  else
    echo "║  (no agents)"
  fi
  echo "╠══════════════════════════════════════════════════════════╣"
  echo "║  coordination stats:"
  "$HOOKS" stats 2>/dev/null | jq -r 'to_entries[] | "║    \(.key): \(.value)"' 2>/dev/null || echo "║    (none)"
  echo "╚══════════════════════════════════════════════════════════╝"
}

swarm_shutdown() {
  if [ ! -f "$SWARM_FILE" ]; then
    echo "No swarm to shut down."
    return 0
  fi
  local id; id="$(jq -r '.id' "$SWARM_FILE")"
  jq '.status = "inactive"' "$SWARM_FILE" > "$SWARM_FILE.tmp" && mv "$SWARM_FILE.tmp" "$SWARM_FILE"
  echo '{"agents":[]}' > "$ROSTER_FILE"
  log "shutdown $id (roster cleared)"
}

case "${1:-help}" in
  "init")        swarm_init "${2:-}" "${3:-}" ;;
  "spawn")       swarm_spawn "${2:?usage: spawn <type> [name]}" "${3:-}" ;;
  "orchestrate") swarm_orchestrate "${2:?usage: orchestrate <task>}" ;;
  "status")      swarm_status ;;
  "shutdown")    swarm_shutdown ;;
  "help"|"-h"|"--help"|*)
    cat << 'EOF'
Claude Flow V3 - Swarm Orchestrator

Usage: swarm-orchestrator.sh <command> [args]

Lifecycle:
  init [topology] [max-agents]   Initialize swarm (defaults from settings.json)
  spawn <type> [name]            Register an agent (1st = coordinator if hierarchical)
  orchestrate <task>             Distribute a task across the mesh
  status                         Show topology, roster, and coordination stats
  shutdown                       Mark swarm inactive and clear the roster

Topology/maxAgents default to .claudeFlow.swarm in .claude/settings.json
(hierarchical-mesh / 15). Built on swarm-hooks.sh (messaging/consensus/handoff).

Examples:
  swarm-orchestrator.sh init
  swarm-orchestrator.sh spawn coordinator queen
  swarm-orchestrator.sh spawn coder
  swarm-orchestrator.sh orchestrate "implement feature X"
  swarm-orchestrator.sh status
EOF
    ;;
esac
