#!/bin/bash
# Claude Flow V3 - Unified Worker Manager
# Orchestrates all background workers with proper scheduling

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
METRICS_DIR="$PROJECT_ROOT/.claude-flow/metrics"
PID_FILE="$METRICS_DIR/worker-manager.pid"
LOG_FILE="$METRICS_DIR/worker-manager.log"
# Absolute path to THIS script, so the detached daemon re-exec resolves regardless of CWD.
SELF="$SCRIPT_DIR/$(basename "${BASH_SOURCE[0]}")"

mkdir -p "$METRICS_DIR"

# Worker definitions: name:script:interval_seconds
WORKERS=(
  "perf:perf-worker.sh:300"           # 5 min
  "health:health-monitor.sh:300"       # 5 min
  "patterns:pattern-consolidator.sh:900"  # 15 min
  "ddd:ddd-tracker.sh:600"             # 10 min
  "adr:adr-compliance.sh:900"          # 15 min
  "security:security-scanner.sh:1800"  # 30 min
  "learning:learning-optimizer.sh:1800" # 30 min
)

log() {
  echo "[$(date '+%Y-%m-%d %H:%M:%S')] $*" | tee -a "$LOG_FILE"
}

run_worker() {
  local name="$1"
  local script="$2"
  local script_path="$SCRIPT_DIR/$script"

  if [ -x "$script_path" ]; then
    "$script_path" check 2>/dev/null &
  fi
}

run_all_workers() {
  log "Running all workers (non-blocking)..."

  for worker_def in "${WORKERS[@]}"; do
    IFS=':' read -r name script interval <<< "$worker_def"
    run_worker "$name" "$script"
  done

  # Don't wait - truly non-blocking
  log "All workers spawned"
}

# Internal: the detached daemon loop. Reached ONLY via the `__daemon` dispatch (see bottom); never
# call directly. Runs as the main process of its own session (started under `setsid`), so here `$$`
# IS the authoritative daemon PID — recorded for `status`/`stop` to read. This fixes the old bug
# where `echo $$` ran inside a backgrounded subshell and recorded the launcher's PID instead.
__daemon_loop() {
  local interval="${1:-60}"

  echo "$$" > "$PID_FILE"
  # Clean up the pidfile on orderly shutdown so `status` never sees a stale entry.
  trap 'log "Worker manager daemon shutting down (signal)"; rm -f "$PID_FILE"; exit 0' SIGTERM SIGINT

  log "Worker manager daemon online (PID $$, interval ${interval}s)"
  while true; do
    run_all_workers
    sleep "$interval"
  done
}

status_all() {
  echo "╔══════════════════════════════════════════════════════════════╗"
  echo "║           Claude Flow V3 - Worker Status                      ║"
  echo "╠══════════════════════════════════════════════════════════════╣"

  for worker_def in "${WORKERS[@]}"; do
    IFS=':' read -r name script interval <<< "$worker_def"
    local script_path="$SCRIPT_DIR/$script"

    if [ -x "$script_path" ]; then
      local status=$("$script_path" status 2>/dev/null || echo "No data")
      printf "║ %-10s │ %-48s ║\n" "$name" "$status"
    fi
  done

  echo "╠══════════════════════════════════════════════════════════════╣"

  # Check if daemon is running
  if [ -f "$PID_FILE" ] && kill -0 "$(cat "$PID_FILE")" 2>/dev/null; then
    echo "║ Daemon: RUNNING (PID: $(cat "$PID_FILE"))                           ║"
  else
    echo "║ Daemon: NOT RUNNING                                          ║"
  fi

  echo "╚══════════════════════════════════════════════════════════════╝"
}

force_all() {
  log "Force running all workers..."

  for worker_def in "${WORKERS[@]}"; do
    IFS=':' read -r name script interval <<< "$worker_def"
    local script_path="$SCRIPT_DIR/$script"

    if [ -x "$script_path" ]; then
      log "Running $name..."
      "$script_path" force 2>&1 | while read -r line; do
        log "  [$name] $line"
      done
    fi
  done

  log "All workers completed"
}

case "${1:-help}" in
  "start"|"daemon")
    if [ -f "$PID_FILE" ] && kill -0 "$(cat "$PID_FILE" 2>/dev/null)" 2>/dev/null; then
      # Idempotent: already-running is success, so session hooks can call `start` freely.
      echo "Worker manager already running (PID: $(cat "$PID_FILE"))"
      exit 0
    fi
    rm -f "$PID_FILE"   # clear any stale pidfile from a crashed/SIGHUP'd run
    interval="${2:-60}"
    # Fully detach the daemon: setsid (own session, no controlling tty) + nohup (ignore SIGHUP) +
    # closed stdin + output to the log. SIGHUP-immune, so it survives the launching shell/pipe
    # closing (the old `run_daemon &` did not, and died on SIGHUP). The daemon records its OWN PID.
    setsid nohup bash "$SELF" __daemon "$interval" >> "$LOG_FILE" 2>&1 < /dev/null &
    # Wait up to ~2s for the daemon to write its authoritative PID, then verify it's alive.
    for _ in $(seq 1 20); do [ -s "$PID_FILE" ] && break; sleep 0.1; done
    if [ -s "$PID_FILE" ] && kill -0 "$(cat "$PID_FILE")" 2>/dev/null; then
      echo "Worker manager started (PID: $(cat "$PID_FILE"))"
    else
      echo "Worker manager failed to start (see $LOG_FILE)" >&2
      exit 1
    fi
    ;;
  "__daemon")
    # Internal entrypoint for the detached loop (invoked by `start` under setsid). Not for direct use.
    __daemon_loop "${2:-60}"
    ;;
  "stop")
    if [ -f "$PID_FILE" ] && kill -0 "$(cat "$PID_FILE" 2>/dev/null)" 2>/dev/null; then
      pid="$(cat "$PID_FILE")"
      kill -TERM "$pid" 2>/dev/null || true
      for _ in $(seq 1 20); do kill -0 "$pid" 2>/dev/null || break; sleep 0.1; done
      if kill -0 "$pid" 2>/dev/null; then kill -KILL "$pid" 2>/dev/null || true; fi
      rm -f "$PID_FILE"
      echo "Worker manager stopped (PID: $pid)"
    else
      rm -f "$PID_FILE"   # drop any stale pidfile
      echo "Worker manager not running"
    fi
    ;;
  "restart")
    "$SELF" stop || true
    exec "$SELF" start "${2:-60}"
    ;;
  "run"|"once")
    run_all_workers
    ;;
  "force")
    force_all
    ;;
  "status")
    status_all
    ;;
  "logs")
    tail -50 "$LOG_FILE" 2>/dev/null || echo "No logs available"
    ;;
  "help"|*)
    cat << EOF
Claude Flow V3 - Worker Manager

Usage: $0 <command> [options]

Commands:
  start [interval]  Start daemon detached (default: 60s cycle; idempotent)
  stop              Stop daemon (graceful TERM, then KILL)
  restart [intv]    Stop then start
  run               Run all workers once
  force             Force run all workers (ignore throttle)
  status            Show all worker status
  logs              Show recent logs

Workers:
  perf              Performance benchmarks (5 min)
  health            System health monitoring (5 min)
  patterns          Pattern consolidation (15 min)
  ddd               DDD progress tracking (10 min)
  adr               ADR compliance checking (15 min)
  security          Security scanning (30 min)
  learning          Learning optimization (30 min)

Examples:
  $0 start 120      # Start with 2-minute cycle
  $0 force          # Run all now
  $0 status         # Check all status
EOF
    ;;
esac
