# Codex Mirror Surface

This directory is generated from the tracked `.claude` surface by the Rust
`codex-env` harness.

## Refresh

```bash
cargo run -p codex-env -- mirror
cargo run -p codex-env -- mirror --check
```

## Mirrored Surfaces

- `.claude/settings.json` -> `.codex/hooks.json` and shell environment defaults
- `.claude/hooks/` -> `.codex/hooks/`
- `.claude/skills/` -> `.agents/skills/`
- `.claude/commands/**/*.md` -> `.agents/skills/source-command-*`

Use `--lua-policy <path>` when a repo-local transformation is needed. The Lua
script receives a `mirror` table with `repo_root` and `claude_dir`, and may
return `{ config_footer = "...", skill_prelude = "..." }`.
