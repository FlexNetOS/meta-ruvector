---
description: 'Build and execute the Rust-owned Codex TDD workflow gates'
argument-hint: [GOAL]
---

Run the Codex Rust TDD workflow for this goal: $ARGUMENTS

Use the Rust harness when shell execution is appropriate:

```bash
cargo run -p codex-env -- tdd-workflow "$ARGUMENTS"
```

This builds `crates/codex-env`, then executes the built Codex Rust tools in
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
auto-loop handoff from the validated plan. Do not move this automation into a
vendor harness.
