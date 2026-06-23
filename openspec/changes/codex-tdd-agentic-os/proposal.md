# codex-tdd-agentic-os

## Why

The Codex Rust tools now build and execute under a supervised TDD workflow, but the autonomous operating-system path still needs machine-readable extraction artifacts. A human-readable trace is useful evidence; the next layer must let the Rust harness decide the next crate-owned action without reloading token-heavy logs or moving behavior into a vendor harness.

## What Changes

- Keep `crates/codex-env` as the owner of the Codex TDD workflow and extraction semantics.
- Emit a deterministic machine-readable TDD extraction plan next to the Markdown report.
- Represent each supervised tool step as a crate-owned extraction action with status, evidence logs, next action, and vendor-harness exclusion.
- Use the plan as the next autonomous loop handoff artifact while preserving `.claude` as source material.

## Capabilities

### New Capabilities
- `codex-tdd-extraction-plan`: Machine-readable extraction plan for turning supervised Codex Rust tool traces into crate-owned follow-up actions.

### Modified Capabilities
- `codex-automation-layer`: Extends the existing automation layer with TDD workflow extraction planning.

## Impact

- `crates/codex-env/src/**`
- `crates/codex-env/tests/**`
- generated `.codex/**` and `.agents/skills/codex-*`
- TDD workflow artifacts under `.codex/harness/runs/**`
