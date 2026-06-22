---
description: 'hive-mind'
argument-hint: [ARGUMENTS]
---

You are executing the Codex-native prompt mirror for Claude Code command `/hive-mind/hive-mind`.

Use Codex-native tools, skills, subagents, MCP servers, and project `AGENTS.md` instructions. Treat Claude-specific tool names, hooks, or MCP names as compatibility context unless this repository exposes the same local command.

Source: `.claude/commands/hive-mind/hive-mind.md`

Arguments supplied to this prompt: $ARGUMENTS

# hive-mind

Hive Mind collective intelligence system for advanced swarm coordination.

## Usage
```bash
npx claude-flow hive-mind [subcommand] [options]
```

## Subcommands
- `init` - Initialize hive mind system
- `spawn` - Spawn hive mind swarm
- `status` - Show hive mind status
- `resume` - Resume paused session
- `stop` - Stop running session

## Examples
```bash
# Initialize hive mind
npx claude-flow hive-mind init

# Spawn swarm
npx claude-flow hive-mind spawn "Build microservices"

# Check status
npx claude-flow hive-mind status
```
