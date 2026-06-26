# codex-env

Rust-native Codex environment mirror for the tracked `.claude` surface.

## Overview

`codex-env` reads the canonical `.claude` configuration — agent roles, command
prompts, hooks, skills, and workflow definitions — and materializes the equivalent
Codex surface, keeping the two environments in sync. It also ships a doctor that
audits drift between the surfaces and inventory helpers that compare the expected
versus the actually-generated artifacts. Within the meta-ruvector workspace it lets
the same authoring surface drive both the Claude Code and Codex toolchains. The
primary operations are configured through option structs (`MirrorOptions`,
`PromptInstallOptions`, `CodexInstallOptions`, and the team/loop/TDD workflow
options), each returning a matching report type. A `codex-env` binary
(`src/main.rs`) exposes these operations on the command line.

## Key API

Surface mirroring and inventory:

- `mirror_codex_surface(MirrorOptions) -> MirrorReport` — generate the Codex surface
  from `.claude` (supports a `check` mode for drift detection).
- `install_codex_prompts(PromptInstallOptions) -> PromptInstallReport` — install the
  generated prompts into a Codex home.
- `install_codex_env(CodexInstallOptions) -> CodexInstallReport` — install the full
  Codex environment.
- `inventory_codex_surface(CodexInventoryOptions) -> CodexInventoryReport` — compare
  expected versus generated artifacts.
- `doctor_codex_surface(DoctorOptions) -> DoctorReport` — audit drift between the
  Claude and Codex surfaces.
- `ensure_codex_home_settings(...)` — ensure baseline Codex home settings.

Workflow execution:

- `run_codex_task`, `run_codex_team`, `run_codex_auto_loop` — single-task, team, and
  auto-loop runners.
- `run_codex_tdd_workflow`, `run_codex_tdd_cycle`, `run_codex_tdd_auto_loop`,
  `run_codex_tdd_drive`, `run_codex_tdd_drive_loop`, `run_codex_tdd_os`,
  `codex_tdd_next_action`, `codex_tdd_supervise`, `audit_codex_tdd_os` — the TDD
  workflow surface.

## License

MIT
