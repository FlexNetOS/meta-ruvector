# Tasks

- [x] Add a machine-readable TDD extraction plan emitted by `codex-env tdd-workflow`.
- [x] Include per-step crate ownership, evidence logs, status, and next action semantics.
- [x] Keep the extraction target in `crates/codex-env` and explicitly exclude vendor harness ownership.
- [x] Add tests proving dry-run materializes the plan and rejects token-heavy Markdown as the only runtime representation.
- [x] Regenerate Codex docs/prompts/skills and run verification.

- [x] Add a Rust-native `tdd-next` consumer for `tdd-extraction-plan.json`.
- [x] Fail closed when an extraction plan routes ownership to a vendor harness or outside `crates/codex-env`.
- [x] Verify `tdd-next --check` against a real supervised TDD workflow run.
