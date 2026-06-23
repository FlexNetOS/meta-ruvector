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
automation into Rust-owned crates. Do not move this automation into a vendor
harness.
