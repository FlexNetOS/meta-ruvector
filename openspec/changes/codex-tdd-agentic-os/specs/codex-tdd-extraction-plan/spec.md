## Purpose

Define the machine-readable extraction plan emitted by the Codex TDD workflow so autonomous Codex loops can route supervised tool evidence into Rust-owned crate work without treating token-heavy Markdown mirrors or vendor harnesses as runtime ownership.

## ADDED Requirements

### Requirement: Codex TDD workflow SHALL emit a machine-readable extraction plan
The Codex TDD workflow SHALL write a deterministic JSON extraction plan next to the human-readable report so autonomous loops can route the next crate-owned action without treating mirrored Markdown as the runtime representation.

#### Scenario: Dry-run materializes extraction plan
- **GIVEN** a repository with Claude source material and Codex TDD workflow inputs
- **WHEN** `codex-env tdd-workflow --dry-run` is executed
- **THEN** the run directory contains `tdd-extraction-plan.json`
- **AND** the plan identifies `crates/codex-env` as the target crate
- **AND** the plan identifies `vendor harness` as a forbidden target
- **AND** each workflow step is represented as an extraction action with status, worker state, crate owner, extraction target, next action, and stdout/stderr evidence paths

#### Scenario: Passing trace routes follow-up to Rust-owned crates
- **GIVEN** all Codex TDD workflow steps have passed
- **WHEN** the extraction plan is written
- **THEN** the top-level next action tells the operator to promote uncovered automation behavior into `crates/codex-env`
- **AND** no plan action routes ownership to a vendor harness
