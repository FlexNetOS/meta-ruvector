---
description: 'Build and execute the Rust-owned Codex TDD workflow gates'
argument-hint: [GOAL]
---

Run the Codex Rust TDD workflow for this goal: $ARGUMENTS

Use the Rust harness when shell execution is appropriate:

```bash
cargo run -p codex-env -- tdd-cycle "$ARGUMENTS"
```

This runs the full Rust-owned TDD cycle: it builds `crates/codex-env`, executes
the built Codex Rust tools, validates the extraction plan, and prepares the
bounded auto-loop handoff. The workflow phase executes the tools in
order: mirror check, repo-local prompt check, doctor, inventory check, and
bounded dry-run run/team-run/auto-loop probes. The workflow status records what
each tool does, why it runs, where the behavior belongs, and the Rust extraction
target. Treat Codex as the human-in-loop operator supervising a background
terminal: launch the workflow, watch status artifacts, give follow-up guidance
if the trace exposes a gap, end the worker session, then extract durable
automation into Rust-owned crates. Inspect each step's stdout/stderr log paths
and supervision events before deciding whether to proceed, guide, or stop the
worker. Then read `tdd-extraction-plan.json` first as the low-token
machine-readable next-action handoff, using `tdd-extraction-report.md` as the
human-readable evidence summary. Run `cargo run -p codex-env -- tdd-next
--check` to fail closed before handing the plan to the next autonomous loop, or
`cargo run -p codex-env -- tdd-auto-loop --dry-run` to materialize the bounded
auto-loop handoff from the validated plan and write `tdd-auto-loop-status.json`.
Prefer `cargo run -p codex-env -- tdd-cycle --dry-run "$ARGUMENTS"` when you
need a single cycle status before executing nested workers. The handoff and
cycle statuses record supervision events and timestamps for the terminal
handoff. The cycle status also records explicit phase checkpoints with evidence
paths and next actions so a resumed Codex session can continue from source truth
instead of reloading token-heavy mirrored material. Read
`tdd-cycle-guidance.md` for the concise human-in-loop guidance artifact before
opening per-step logs.
Use repeatable `--supervisor-note` or `--supervisor-note-file` to inject
follow-up guidance into the bounded handoff prompt when the supervisor has
inspected evidence and needs to steer the worker.
Run `cargo run -p codex-env -- tdd-supervise` after a cycle to persist the
supervisor proceed/guide/stop decision before launching or closing a worker.
Do not move this automation into a vendor harness.
