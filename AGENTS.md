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

<!-- gitnexus:start -->
# GitNexus — Code Intelligence

This project is indexed by GitNexus as **meta-ruvector** (282012 symbols, 569713 relationships, 300 execution flows). Use the GitNexus MCP tools to understand code, assess impact, and navigate safely.

> Index stale? Run `node .gitnexus/run.cjs analyze` from the project root — it auto-selects an available runner. No `.gitnexus/run.cjs` yet? `npx gitnexus analyze` (npm 11 crash → `npm i -g gitnexus`; #1939).

## Always Do

- **MUST run impact analysis before editing any symbol.** Before modifying a function, class, or method, run `impact({target: "symbolName", direction: "upstream"})` and report the blast radius (direct callers, affected processes, risk level) to the user.
- **MUST run `detect_changes()` before committing** to verify your changes only affect expected symbols and execution flows. For regression review, compare against the default branch: `detect_changes({scope: "compare", base_ref: "main"})`.
- **MUST warn the user** if impact analysis returns HIGH or CRITICAL risk before proceeding with edits.
- When exploring unfamiliar code, use `query({search_query: "concept"})` to find execution flows instead of grepping. It returns process-grouped results ranked by relevance.
- When you need full context on a specific symbol — callers, callees, which execution flows it participates in — use `context({name: "symbolName"})`.
- For security review, `explain({target: "fileOrSymbol"})` lists taint findings (source→sink flows; needs `analyze --pdg`).

## Never Do

- NEVER edit a function, class, or method without first running `impact` on it.
- NEVER ignore HIGH or CRITICAL risk warnings from impact analysis.
- NEVER rename symbols with find-and-replace — use `rename` which understands the call graph.
- NEVER commit changes without running `detect_changes()` to check affected scope.

## Resources

| Resource | Use for |
|----------|---------|
| `gitnexus://repo/meta-ruvector/context` | Codebase overview, check index freshness |
| `gitnexus://repo/meta-ruvector/clusters` | All functional areas |
| `gitnexus://repo/meta-ruvector/processes` | All execution flows |
| `gitnexus://repo/meta-ruvector/process/{name}` | Step-by-step execution trace |

## CLI

| Task | Read this skill file |
|------|---------------------|
| Understand architecture / "How does X work?" | `.claude/skills/gitnexus/gitnexus-exploring/SKILL.md` |
| Blast radius / "What breaks if I change X?" | `.claude/skills/gitnexus/gitnexus-impact-analysis/SKILL.md` |
| Trace bugs / "Why is X failing?" | `.claude/skills/gitnexus/gitnexus-debugging/SKILL.md` |
| Rename / extract / split / refactor | `.claude/skills/gitnexus/gitnexus-refactoring/SKILL.md` |
| Tools, resources, schema reference | `.claude/skills/gitnexus/gitnexus-guide/SKILL.md` |
| Index, status, clean, wiki CLI commands | `.claude/skills/gitnexus/gitnexus-cli/SKILL.md` |

<!-- gitnexus:end -->
