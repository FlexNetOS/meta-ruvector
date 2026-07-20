---
description: 'pr-enhance'
argument-hint: [ARGUMENTS]
---

You are executing the Codex-native prompt mirror for Claude Code command `/github:pr-enhance`.

Use Codex-native tools, skills, subagents, MCP servers, and project `AGENTS.md` instructions. Treat Claude-specific tool names, hooks, or MCP names as compatibility context unless this repository exposes the same local command.

Source: `.claude/commands/github/pr-enhance.md`

Arguments supplied to this prompt: $ARGUMENTS

# pr-enhance

AI-powered pull request enhancements.

## Usage
```bash
npx claude-flow github pr-enhance [options]
```

## Options
- `--pr-number <n>` - Pull request number
- `--add-tests` - Add missing tests
- `--improve-docs` - Improve documentation
- `--check-security` - Security review

## Examples
```bash
# Enhance PR
npx claude-flow github pr-enhance --pr-number 123

# Add tests
npx claude-flow github pr-enhance --pr-number 123 --add-tests

# Full enhancement
npx claude-flow github pr-enhance --pr-number 123 --add-tests --improve-docs
```
