# Codex Mirror Surface

This directory is generated from the tracked `.claude` surface by the Rust
`codex-env` harness.

## Refresh

```bash
cargo run -p codex-env -- mirror
cargo run -p codex-env -- mirror --check
```

## Mirrored Surfaces

- `.claude/**` -> `.codex/mirror/.claude/**` byte-for-byte
- `.claude/**` -> `.codex/mirror-symbols.json` deterministic file/symbol inventory
- `.claude/settings.json` -> `.codex/hooks.json` and shell environment defaults
- `.claude/hooks/` -> `.codex/hooks/`
- `.claude/skills/` -> `.agents/skills/`
- `.claude/commands/**/*.md` -> `.agents/skills/source-command-*`
- `.claude/commands/**/*.md` -> `.codex/prompts/*.md` for `/prompts:*`
- Codex-native workflow upgrades -> `.agents/skills/codex-*` and
  `.codex/prompts/codex-*`

Use `--lua-policy <path>` when a repo-local transformation is needed. The Lua
script receives a `mirror` table with `repo_root` and `claude_dir`, and may
return `{ config_footer = "...", skill_prelude = "..." }`.

## Install Prompt Commands

Codex loads custom prompts from `$CODEX_HOME/prompts`, not directly from a
repository. After refreshing this mirror, install the generated prompt commands
with:

```bash
.codex/helpers/install-prompts.sh
```

Restart Codex after installing. The Claude command mirrors then appear as Codex
prompt commands such as `/prompts:sparc-code` and
`/prompts:claude-flow-swarm`.
