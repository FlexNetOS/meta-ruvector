---
name: codex-tdd-workflow
description: 'Use when the task needs a supervised TDD workflow that builds codex-env, executes the Codex Rust tools, traces their purpose, and extracts durable behavior into Rust-owned crates.'
---

# Codex TDD Workflow

Use this when Codex needs to act as the human-in-loop operator for the
repo-owned automation layer.

When running from the shell, prefer the Rust harness:

```bash
cargo run -p codex-env -- tdd-cycle "your goal"
```

The cycle builds `crates/codex-env`, executes the built binary through
mirror, prompt, doctor, inventory, run, team-run, and auto-loop probes. Its
status file records what each tool does, why it runs, where the behavior
belongs, and the Rust extraction target. Supervise it like a background
terminal: inspect status artifacts, provide follow-up guidance if a probe
exposes a gap, terminate the worker session when the trace is complete, and
move durable automation into the correct Rust crate instead of a vendor harness.
Non-dry-run steps capture stdout/stderr logs and supervision events for
post-run extraction, then emit `tdd-extraction-plan.json` for machine-readable
next-action routing and `tdd-extraction-report.md` as the human-readable
summary. Run `codex-env tdd-next --check` after the workflow to consume the
latest plan, reject vendor-harness routing, and select the next Rust-owned
action for autonomous continuation. Run `codex-env tdd-auto-loop --dry-run` to
turn that validated plan into bounded auto-loop artifacts before allowing a real
autonomous continuation; inspect `tdd-auto-loop-status.json` as the durable
handoff status with supervision events and start/end timestamps. Prefer
`codex-env tdd-cycle --dry-run` for a single Rust-owned status that proves the
workflow-to-handoff chain is wired before launching nested workers. The cycle
status includes phase checkpoints, evidence paths, next actions, supervision
events, and timestamps so Codex does not wait blind on a background terminal.
Read `tdd-cycle-guidance.md` first when resuming or guiding the worker.
Use `--supervisor-note` or `--supervisor-note-file` to pass follow-up guidance
into the next bounded handoff.
Run `codex-env tdd-supervise` to turn the latest cycle status into an explicit
proceed, guide, or stop decision before launching or closing a worker.
