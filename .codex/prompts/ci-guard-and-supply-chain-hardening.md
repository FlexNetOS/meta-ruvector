---
description: 'Workflow command scaffold for ci-guard-and-supply-chain-hardening in ruvector.'
argument-hint: [ARGUMENTS]
---

You are executing the Codex-native prompt mirror for Claude Code command `/ci-guard-and-supply-chain-hardening`.

Use Codex-native tools, skills, subagents, MCP servers, and project `AGENTS.md` instructions. Treat Claude-specific tool names, hooks, or MCP names as compatibility context unless this repository exposes the same local command.

Source: `.claude/commands/ci-guard-and-supply-chain-hardening.md`

Arguments supplied to this prompt: $ARGUMENTS

# /ci-guard-and-supply-chain-hardening

Use this workflow when working on **ci-guard-and-supply-chain-hardening** in `ruvector`.

## Goal

Adds or updates CI workflows to enforce security, dependency, and regression checks, and updates dependency policies and lockfiles.

## Common Files

- `.github/workflows/*.yml`
- `.github/dependabot.yml`
- `deny.toml`
- `npm/package-lock.json`
- `npm/package.json`

## Suggested Sequence

1. Understand the current state and failure mode before editing.
2. Make the smallest coherent change that satisfies the workflow goal.
3. Run the most relevant verification for touched files.
4. Summarize what changed and what still needs review.

## Typical Commit Signals

- Add or update GitHub Actions workflow files for CI checks.
- Update dependency policy files (e.g., deny.toml).
- Update lockfiles and package manifests as needed.
- Commit all CI and policy changes.

## Notes

- Treat this as a scaffold, not a hard-coded script.
- Update the command if the workflow evolves materially.
