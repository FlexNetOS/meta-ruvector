# Codex Automation Extraction Surface

This directory is generated from tracked `.claude` source material by the Rust
`codex-env` harness. It is the Rust automation extraction frontier for this
repository, not a vendor harness and not a user-global prompt dump.

`.claude` remains source material. `.codex/mirror/.claude` is byte-for-byte
evidence/debug material. The crate-owned runtime target is the compact,
deterministic automation layer generated under `.codex`, especially
`.codex/automation-graph.json`, `.codex/agent-teams.json`,
`.codex/hooks.json`, `.codex/hooks/`, `.codex/prompts/`, and
`.codex/agents/`.

## Refresh

```bash
cargo run -p codex-env -- install
cargo run -p codex-env -- run --dry-run "inspect the Codex surface"
cargo run -p codex-env -- team-run --dry-run --team rust "inspect Rust parity gaps"
cargo run -p codex-env -- auto-loop --dry-run --team core "inspect autonomous loop wiring"
cargo run -p codex-env -- tdd-workflow --dry-run "trace Codex Rust tool ownership"
cargo run -p codex-env -- mirror --check
cargo run -p codex-env -- install-prompts --check
cargo run -p codex-env -- doctor
```

## Mirrored Surfaces

- `.claude/**` -> `.codex/mirror/.claude/**` byte-for-byte
- `.claude/**` -> `.codex/mirror-symbols.json` deterministic file/symbol evidence inventory
- `.claude/**` -> `.codex/automation-graph.json` compact crate-owned capability graph
- `.claude/settings.json` -> `.codex/hooks.json` and shell environment defaults
- `.claude/hooks/` -> normalized `.codex/hooks/` runtime scripts
- `.claude/skills/` -> `.agents/skills/`
- `.claude/commands/**/*.md` -> `.agents/skills/source-command-*`
- `.claude/commands/**/*.md` -> repo-local `.codex/prompts/*.md` for `/prompts:*`,
  including Claude namespace aliases such as `/prompts:sparc:code`
- Codex-native workflow upgrades -> `.agents/skills/codex-*` and
  `.codex/prompts/codex-*`

Use `--lua-policy <path>` when a repo-local transformation is needed. The Lua
script receives a `mirror` table with `repo_root` and `claude_dir`, and may
return `{ config_footer = "...", skill_prelude = "..." }`.

## Prompt Commands

Prompt commands generated for this repository stay in this repository's
`.codex/prompts`; do not copy meta-ruvector prompts into user-global
`~/.codex/prompts` or the meta root prompt set. Refresh and verify with:

```bash
.codex/helpers/install-prompts.sh
```

That helper runs `cargo run -p codex-env -- install`, which mirrors `.claude`
into repo-local Codex surfaces and runs doctor validation in one pass. Restart
Codex from this repository after refreshing. The Claude command mirrors then
appear as Codex prompt commands such as `/prompts:sparc-code`,
`/prompts:sparc:code`, and `/prompts:claude-flow-swarm`.

## Automation Ownership

Use `.codex/automation-graph.json` as the low-token routing and capability
index before loading bulk mirrored Markdown. Agent teams are generated from
actual configured Codex agent roles and expose both `agents` and `members` for
runtime consumers. Generated runtime hooks must resolve the repository root
dynamically; stale absolute paths such as `/workspaces/ruvector` are rejected by
doctor checks. Do not move this automation into a vendor harness.

## Run Actual Work

Use the repo-owned runner when you want Codex to execute a bounded task from the
validated local environment and leave artifacts:

```bash
cargo run -p codex-env -- run "fix the next Codex parity gap"
cargo run -p codex-env -- team-run --team rust "trace and fix the next Rust harness gap"
cargo run -p codex-env -- auto-loop --team core --max-iterations 3 "finish the Codex parity goal"
cargo run -p codex-env -- tdd-workflow "build, verify, and trace the Codex Rust tools"
cargo run -p codex-env -- tdd-next --check
cargo run -p codex-env -- tdd-auto-loop --dry-run
cargo run -p codex-env -- tdd-cycle --dry-run "supervise the full Codex TDD cycle"
cargo run -p codex-env -- tdd-supervise
```

Each run refreshes/validates the Codex surface, then invokes `codex exec --json`
with artifacts under `.codex/harness/runs/`: `prompt.md`, `events.jsonl`,
`stderr.log`, `last-message.md`, and `status.json`. Use `--dry-run` to materialize
the exact prompt and status without launching a nested Codex run. `team-run`
loads `.codex/agent-teams.json` plus the referenced `.codex/agents/*.toml`
profiles, starts every team member with its configured model and reasoning
effort in a read-only sandbox by default, then runs a parent consolidation
Codex pass that reads the member outputs, performs parent-owned edits, and
writes its own artifacts. Use `--member-sandbox workspace-write` only for a
deliberately isolated writable member scope.

`auto-loop` wraps `team-run` in bounded iterations, records
`auto-loop-status.json`, and stops early only when parent consolidation emits
`CODEX_AUTO_LOOP_STATUS: complete`. Otherwise it continues until
`--max-iterations` is reached.

`tdd-workflow` is the supervised red/green harness for the Rust-owned Codex
toolchain. It builds `codex-env`, then executes the built binary through
`mirror --check`, `install-prompts --check`, `doctor`, `inventory --check`,
and bounded dry-run `run`/`team-run`/`auto-loop` probes. Each status entry
records what the tool does, why it runs, where the behavior belongs, what Rust
extraction target owns it, and how Codex should supervise the background
terminal equivalent. Codex is the human-in-loop operator for this workflow:
start the background terminal equivalent, supervise status/artifacts, provide
follow-up guidance when the trace exposes a gap, end the worker session, and
extract durable behavior into the owning Rust crates rather than a vendor
harness. Non-dry-run workflow steps also write per-step stdout/stderr logs
under the workflow run directory so the supervising Codex session can inspect
what the background worker actually did before deciding the next extraction.
The workflow also writes `tdd-extraction-report.md` plus
`tdd-extraction-plan.json`; the JSON plan is the low-token machine-readable
crate-ownership handoff that turns the supervised trace into the next Rust
extraction action. `tdd-next` consumes the newest extraction plan, rejects any
vendor-harness ownership drift, and prints the selected Rust-owned actions for
the next autonomous loop handoff. `tdd-auto-loop` feeds that validated plan
directly into the bounded `auto-loop` harness so the next Codex run continues
from supervised evidence instead of reinterpreting token-heavy reports, and
writes `tdd-auto-loop-status.json` beside the auto-loop artifacts.
That status records the handoff state, supervision events, and start/end
timestamps so Codex can supervise the handoff like a background terminal rather
than waiting blind.
`tdd-cycle` stitches those phases into one Rust-owned cycle status:
workflow trace, extraction plan validation, and the auto-loop handoff status are
recorded under one `tdd-cycle-status.json` so the next Codex session can resume
from crate-owned evidence instead of a token-heavy vendor harness narrative.
The cycle status includes phase-level checkpoints, evidence paths, next actions,
and timestamps so a supervisor can see whether the current terminal is planned,
running, prepared, ended, or complete without waiting blind.
It also writes `tdd-cycle-guidance.md`, a human-readable supervision brief that
summarizes the current phase, evidence path, and next action without forcing the
next session to load token-heavy mirrored source. Use repeatable
`--supervisor-note` or `--supervisor-note-file` when Codex needs to inject
follow-up guidance into the handoff prompt after inspecting evidence.
`tdd-supervise` reads the newest cycle status, writes
`tdd-supervision-decision.json`, and tells Codex whether to proceed, guide, or
stop the background terminal based on Rust-owned evidence.
