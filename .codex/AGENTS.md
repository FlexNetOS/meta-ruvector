# Codex Mirror Surface

This directory is generated from the tracked `.claude` surface by the Rust
`codex-env` harness.

## Refresh

```bash
cargo run -p codex-env -- install
cargo run -p codex-env -- run --dry-run "inspect the Codex surface"
cargo run -p codex-env -- team-run --dry-run --team rust "inspect Rust parity gaps"
cargo run -p codex-env -- mirror --check
cargo run -p codex-env -- install-prompts --check
cargo run -p codex-env -- doctor
```

## Mirrored Surfaces

- `.claude/**` -> `.codex/mirror/.claude/**` byte-for-byte
- `.claude/**` -> `.codex/mirror-symbols.json` deterministic file/symbol inventory
- `.claude/settings.json` -> `.codex/hooks.json` and shell environment defaults
- `.claude/hooks/` -> `.codex/hooks/`
- `.claude/skills/` -> `.agents/skills/`
- `.claude/commands/**/*.md` -> `.agents/skills/source-command-*`
- `.claude/commands/**/*.md` -> `.codex/prompts/*.md` for `/prompts:*`,
  including Claude namespace aliases such as `/prompts:sparc:code`
- Codex-native workflow upgrades -> `.agents/skills/codex-*` and
  `.codex/prompts/codex-*`

Use `--lua-policy <path>` when a repo-local transformation is needed. The Lua
script receives a `mirror` table with `repo_root` and `claude_dir`, and may
return `{ config_footer = "...", skill_prelude = "..." }`.

## Install Prompt Commands

Codex loads custom prompts from `$CODEX_HOME/prompts`, not directly from a
repository. Refresh the mirror and install the generated prompt commands with:

```bash
.codex/helpers/install-prompts.sh
```

That helper runs `cargo run -p codex-env -- install`, which mirrors `.claude`,
installs `$CODEX_HOME/prompts`, and runs doctor validation in one pass. Restart
Codex after installing. The Claude command mirrors then appear as Codex prompt
commands such as `/prompts:sparc-code`, `/prompts:sparc:code`, and
`/prompts:claude-flow-swarm`.

## Run Actual Work

Use the repo-owned runner when you want Codex to execute a bounded task from the
validated local environment and leave artifacts:

```bash
cargo run -p codex-env -- run "fix the next Codex parity gap"
cargo run -p codex-env -- team-run --team rust "trace and fix the next Rust harness gap"
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
