---
description: 'hive-mind-init'
argument-hint: [ARGUMENTS]
---

You are executing the Codex-native prompt mirror for Claude Code command `/hive-mind:hive-mind-init`.

Use Codex-native tools, skills, subagents, MCP servers, and project `AGENTS.md` instructions. Treat Claude-specific tool names, hooks, or MCP names as compatibility context unless this repository exposes the same local command.

Source: `.claude/commands/hive-mind/hive-mind-init.md`

Arguments supplied to this prompt: $ARGUMENTS

# hive-mind-init

Initialize the Hive Mind collective intelligence system.

## Usage
```bash
npx claude-flow hive-mind init [options]
```

## Options
- `--force` - Force reinitialize
- `--config <file>` - Configuration file

## Examples
```bash
npx claude-flow hive-mind init
npx claude-flow hive-mind init --force
```
