# Tasks

- [x] Add a machine-readable TDD extraction plan emitted by `codex-env tdd-workflow`.
- [x] Include per-step crate ownership, evidence logs, status, and next action semantics.
- [x] Keep the extraction target in `crates/codex-env` and explicitly exclude vendor harness ownership.
- [x] Add tests proving dry-run materializes the plan and rejects token-heavy Markdown as the only runtime representation.
- [x] Regenerate Codex docs/prompts/skills and run verification.

- [x] Add a Rust-native `tdd-next` consumer for `tdd-extraction-plan.json`.
- [x] Fail closed when an extraction plan routes ownership to a vendor harness or outside `crates/codex-env`.
- [x] Verify `tdd-next --check` against a real supervised TDD workflow run.

- [x] Add a Rust-native `tdd-auto-loop` handoff from validated TDD extraction plans into `auto-loop`.
- [x] Ensure the handoff prompt preserves Codex-as-human supervision evidence and forbids vendor harness routing.
- [x] Verify `tdd-auto-loop --dry-run` against a real supervised TDD plan.

- [x] Persist `tdd-auto-loop-status.json` so the autonomous handoff has durable status evidence.
- [x] Record `tdd-auto-loop` handoff state, supervision events, and timestamps in the durable status.
