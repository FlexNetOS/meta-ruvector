# Backlog: mature codex-env TDD OS automation beyond wrapper

## Context

After PR #77 (`feat(codex): add one-command TDD OS wrapper`), `codex-env tdd-os --check` provides a Rust-owned one-command control-plane wrapper that drives the bounded TDD loop, audits the resulting evidence, and persists `tdd-os-status.json`.

This backlog task captures the next layer of work from the session review: the wrapper is real and useful, but the broader automation layer should keep moving from evidence/status orchestration toward semantic extraction and self-healing crate-owned runtime behavior.

## What could work next

- [ ] Make `tdd-os` the canonical self-healing loop.
  - Consume failed audit requirements automatically.
  - Open or select the next extraction task from the failing requirement.
  - Continue only through Rust-owned actions and evidence, not owner chat state.

- [ ] Promote the capability graph into runtime behavior.
  - Treat `.codex/automation-graph.json` and generated agent/team definitions as the low-token Rust-native planning substrate.
  - Reduce skill overload, MCP rot, and token burn by routing through normalized capabilities before loading mirrored Markdown.

- [ ] Add issue/PR-aware continuation.
  - Select a GitHub issue or backlog item.
  - Create a feature branch.
  - Run `tdd-os`.
  - Implement the extracted Rust-owned action.
  - Verify, open PR, merge, sync, and leave the repo clean.

- [ ] Add stall detection for background agents.
  - Poll run artifacts/transcripts instead of waiting blind.
  - Treat no notification as UNKNOWN, not healthy.
  - Emit supervisor decisions for stalled, progressing, complete, and failed worker states.

- [ ] Make completion harder to fake.
  - Extend `tdd-os`/`tdd-audit` to verify actual semantic extraction changes, not only that trace fields route to `crates/codex-env`.
  - Require evidence that `.claude/.codex` semantics were compiled into crate-owned behavior when an audit claims extraction completion.

## Why

The current `tdd-os` command proves the Codex TDD control spine can drive, supervise, audit, and close over its own evidence. The next step is to make that spine continuously extract, route, and apply real automation semantics into the correct Rust crates without falling back into vendor harness narrative, token-heavy mirrored surfaces, or owner-message clocking.
