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

### Requirement: Codex TDD next-action consumer SHALL fail closed on ownership drift
The Codex TDD workflow SHALL provide a Rust-native consumer for extraction plans so the autonomous loop can read the newest plan, select the next crate-owned action, and reject vendor-harness routing before continuing.

#### Scenario: Latest plan is ready for autonomous handoff
- **GIVEN** a completed TDD extraction plan whose actions all passed and belong to `crates/codex-env`
- **WHEN** `codex-env tdd-next --check` reads the plan
- **THEN** it reports the plan as ready for autonomous loop handoff
- **AND** it prints the next crate-owned action and selected extraction actions

#### Scenario: Plan routes ownership to a forbidden target
- **GIVEN** a TDD extraction plan with an action belonging to `vendor harness`
- **WHEN** `codex-env tdd-next` reads the plan
- **THEN** it fails before handing the action to the autonomous loop
