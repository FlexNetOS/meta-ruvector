<!-- icm:start -->
## Persistent Memory (ICM)

This repository uses ICM for durable task memory. Before non-trivial work, run:

```bash
rtk icm recall-context "meta-ruvector <task keywords>" --limit 5
```

Store only durable outcomes:

```bash
rtk icm store -t context-meta-ruvector -c "summary" -i high -k "codex,ruvector"
rtk icm store -t errors-resolved -c "resolution" -i high -k "keyword"
rtk icm store -t decisions-meta-ruvector -c "decision" -i high
rtk icm store -t preferences -c "preference" -i critical
```
<!-- icm:end -->

## Project Rules

- Use `rtk` for shell commands in this workspace.
- Read the relevant files before editing them.
- Keep changes surgical and preserve tracked `.claude/` as source material.
- Do not commit secrets, credentials, `.env`, or user-local state.
- Run focused tests for changed Rust crates before committing.
- When reporting Rust toolchain details, include the actual compiler path and wrapper flags used by Cargo.

## Codex Surface

The Rust-native Codex mirror is owned by `crates/codex-env`.

```bash
cargo run -p codex-env -- mirror
cargo run -p codex-env -- mirror --check
```

The mirror locates `.claude/`, then generates:

- `.codex/config.toml`
- `.codex/AGENTS.md`
- `.codex/hooks.json`
- `.codex/hooks/`
- `.agents/skills/` from `.claude/skills/`
- `.agents/skills/source-command-*` from `.claude/commands/**/*.md`

Use `--lua-policy <path>` only when a repo-local transform is needed; the harness evaluates it with `mlua`.

## Verification

For Codex env changes, run:

```bash
cargo fmt -p codex-env
cargo test -p codex-env
cargo run -p codex-env -- mirror --check
```
