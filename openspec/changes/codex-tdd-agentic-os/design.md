# Design: Codex TDD extraction plan

## Ownership

`crates/codex-env` owns the supervised TDD workflow and all extraction semantics. `.claude` remains source material, `.codex` remains the generated extraction frontier, and the vendor harness is an explicit non-target.

## Artifact model

Each `codex-env tdd-workflow` run writes three coordinated artifacts in its run directory:

- `tdd-workflow-status.json`: full operational status for the supervised background-terminal equivalent.
- `tdd-extraction-report.md`: human-readable evidence summary.
- `tdd-extraction-plan.json`: low-token machine-readable next-action handoff.

The JSON plan contains a top-level target crate, forbidden target, source material summary, runtime representation label, next action, and one action per supervised step. Per-step actions preserve the step status, worker state, crate owner, extraction target, and stdout/stderr paths so the next autonomous loop can inspect evidence only when needed.

## Runtime behavior

Dry-run plans route the next action to a real supervised workflow execution. Failed plans route the next action to the failed Rust-owned extraction target and captured logs. Passing plans route the next action to promote the next uncovered automation behavior into `crates/codex-env`, not a vendor harness.

## Non-goals

- No destructive edits to `.claude` source material.
- No user-global prompt installation.
- No migration of automation ownership into a vendor harness.
