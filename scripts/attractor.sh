#!/usr/bin/env bash
#
# Attractor pipeline runner for ruvector (the brain side).
#
# Reads .attractor/integration.dot and walks the canonical
# Identify -> Implement -> Validate -> Optimize -> Distill loop with no
# human in the loop. Each node delegates to .attractor/nodes/<name>.sh,
# which is allowed to be a stub during phase-by-phase rollout.
#
# Usage:
#   scripts/attractor.sh validate          # parse the DOT, report node order
#   scripts/attractor.sh dry-run           # print what each node would do
#   scripts/attractor.sh run [--once]      # execute one (or N) iterations
#   scripts/attractor.sh node <identify|implement|validate|optimize|distill>
#                                          # invoke a single node directly
#                                          # (this is what the DOT's `command`
#                                          # attribute resolves to)
#   scripts/attractor.sh --help
#
# Exit codes:
#   0   success
#   1   user-error (bad args, missing DOT)
#   2   validate node failed (this is the contract; do NOT distill)
#   3   any other node failed
#
# This script is intentionally dependency-light: it does not require
# graphviz unless `validate --strict` is requested. Node scripts may
# pull in cargo, jq, curl, etc. as needed.

set -euo pipefail

readonly ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
readonly DOT_FILE="$ROOT/.attractor/integration.dot"
readonly NODES_DIR="$ROOT/.attractor/nodes"
readonly RUNS_DIR="$ROOT/.attractor/runs"
readonly NODE_ORDER=(identify implement validate optimize distill)

# ---- Logging helpers ------------------------------------------------------

if [ -t 1 ] && [ -z "${NO_COLOR:-}" ]; then
    BOLD=$'\e[1m'; DIM=$'\e[2m'; RED=$'\e[31m'; GREEN=$'\e[32m'
    YELLOW=$'\e[33m'; CYAN=$'\e[36m'; NC=$'\e[0m'
else
    BOLD=""; DIM=""; RED=""; GREEN=""; YELLOW=""; CYAN=""; NC=""
fi

log()    { printf "%s\n" "$*" >&2; }
info()   { printf "${CYAN}[attractor]${NC} %s\n" "$*" >&2; }
ok()     { printf "${GREEN}[attractor]${NC} %s\n" "$*" >&2; }
warn()   { printf "${YELLOW}[attractor]${NC} %s\n" "$*" >&2; }
err()    { printf "${RED}[attractor]${NC} %s\n" "$*" >&2; }

usage() {
    sed -n '2,/^$/p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'
}

# ---- Sanity ---------------------------------------------------------------

require_dot_file() {
    if [ ! -f "$DOT_FILE" ]; then
        err "missing pipeline graph: $DOT_FILE"
        exit 1
    fi
}

# Parse node order from the DOT. We trust the canonical NODE_ORDER for
# execution order (the DOT is the human-readable spec; a topological
# walk would also be valid but adds a graphviz dep).
list_nodes() {
    for node in "${NODE_ORDER[@]}"; do
        printf "%s\n" "$node"
    done
}

# ---- Subcommands ----------------------------------------------------------

cmd_validate() {
    require_dot_file
    info "validating $DOT_FILE"

    # Cheap syntactic check: ensure each node label is mentioned in the DOT.
    local missing=()
    for node in "${NODE_ORDER[@]}"; do
        if ! grep -qE "^\s*${node}\s*\[" "$DOT_FILE"; then
            missing+=("$node")
        fi
    done
    if [ "${#missing[@]}" -gt 0 ]; then
        err "DOT is missing required nodes: ${missing[*]}"
        exit 2
    fi

    # Strict mode: also ask graphviz to parse it. Skipped if dot(1)
    # is not on PATH so this works on bare CI runners.
    if [ "${1:-}" = "--strict" ]; then
        if command -v dot >/dev/null 2>&1; then
            if ! dot -Tcanon "$DOT_FILE" >/dev/null 2>&1; then
                err "graphviz failed to parse $DOT_FILE"
                exit 2
            fi
            ok "graphviz parse OK"
        else
            warn "graphviz (dot) not installed; skipping --strict parse"
        fi
    fi

    ok "DOT contains all 5 canonical nodes: ${NODE_ORDER[*]}"
}

cmd_dry_run() {
    require_dot_file
    info "dry-run topology for $(basename "$ROOT")"
    local i=0
    for node in "${NODE_ORDER[@]}"; do
        i=$((i + 1))
        local script="$NODES_DIR/${node}.sh"
        if [ -x "$script" ]; then
            printf "  ${BOLD}%d. %s${NC}  -> %s\n" "$i" "$node" "$script"
        else
            printf "  ${BOLD}%d. %s${NC}  -> ${DIM}(stub: %s missing or non-exec)${NC}\n" "$i" "$node" "$script"
        fi
    done
    ok "5 nodes scheduled"
}

cmd_node() {
    require_dot_file
    local node="${1:-}"
    if [ -z "$node" ]; then
        err "usage: scripts/attractor.sh node <identify|implement|validate|optimize|distill>"
        exit 1
    fi
    local found=0
    for known in "${NODE_ORDER[@]}"; do
        if [ "$known" = "$node" ]; then found=1; break; fi
    done
    if [ "$found" -eq 0 ]; then
        err "unknown node: $node (expected one of: ${NODE_ORDER[*]})"
        exit 1
    fi
    local script="$NODES_DIR/${node}.sh"
    if [ ! -x "$script" ]; then
        err "missing or non-executable node script: $script"
        exit 1
    fi
    exec "$script"
}

cmd_run() {
    require_dot_file
    mkdir -p "$RUNS_DIR"
    local stamp; stamp="$(date -u +%Y%m%dT%H%M%SZ)"
    local log_file="$RUNS_DIR/${stamp}.jsonl"

    info "executing pipeline; log -> $log_file"

    local i=0
    local overall_status=0
    for node in "${NODE_ORDER[@]}"; do
        i=$((i + 1))
        local script="$NODES_DIR/${node}.sh"
        local started; started="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
        local status="ok"
        local rc=0

        printf "${BOLD}[%d/5] %s${NC}\n" "$i" "$node" >&2
        if [ -x "$script" ]; then
            if "$script"; then
                ok "$node passed"
            else
                rc=$?
                status="fail"
                err "$node failed (rc=$rc)"
            fi
        else
            warn "no node script at $script -- recording as 'stub'"
            status="stub"
        fi

        local finished; finished="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
        printf '{"node":"%s","status":"%s","rc":%d,"started":"%s","finished":"%s"}\n' \
            "$node" "$status" "$rc" "$started" "$finished" >> "$log_file"

        # Validate is the contract; if it fails, do not distill.
        if [ "$node" = "validate" ] && [ "$status" = "fail" ]; then
            err "validate failed; skipping optimize + distill"
            overall_status=2
            break
        fi
        if [ "$status" = "fail" ]; then
            overall_status=3
        fi
    done

    if [ "$overall_status" -eq 0 ]; then
        ok "pipeline run complete"
    fi
    exit "$overall_status"
}

# ---- Dispatch -------------------------------------------------------------

main() {
    local subcmd="${1:---help}"
    case "$subcmd" in
        validate)  shift; cmd_validate "$@" ;;
        dry-run)   shift; cmd_dry_run ;;
        run)       shift; cmd_run ;;
        node)      shift; cmd_node "$@" ;;
        -h|--help) usage ;;
        *)
            err "unknown subcommand: $subcmd"
            usage
            exit 1
            ;;
    esac
}

main "$@"
