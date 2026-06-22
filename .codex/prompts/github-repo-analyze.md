---
description: 'repo-analyze'
argument-hint: [ARGUMENTS]
---

You are executing the Codex-native prompt mirror for Claude Code command `/github/repo-analyze`.

Use Codex-native tools, skills, subagents, MCP servers, and project `AGENTS.md` instructions. Treat Claude-specific tool names, hooks, or MCP names as compatibility context unless this repository exposes the same local command.

Source: `.claude/commands/github/repo-analyze.md`

Arguments supplied to this prompt: $ARGUMENTS

# repo-analyze

Deep analysis of GitHub repository with AI insights.

## Usage
```bash
npx claude-flow github repo-analyze [options]
```

## Options
- `--repository <owner/repo>` - Repository to analyze
- `--deep` - Enable deep analysis
- `--include <areas>` - Include specific areas (issues, prs, code, commits)

## Examples
```bash
# Basic analysis
npx claude-flow github repo-analyze --repository myorg/myrepo

# Deep analysis
npx claude-flow github repo-analyze --repository myorg/myrepo --deep

# Specific areas
npx claude-flow github repo-analyze --repository myorg/myrepo --include issues,prs
```
