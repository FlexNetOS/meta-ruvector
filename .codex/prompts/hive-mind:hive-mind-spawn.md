---
description: 'hive-mind-spawn'
argument-hint: [ARGUMENTS]
---

You are executing the Codex-native prompt mirror for Claude Code command `/hive-mind:hive-mind-spawn`.

Use Codex-native tools, skills, subagents, MCP servers, and project `AGENTS.md` instructions. Treat Claude-specific tool names, hooks, or MCP names as compatibility context unless this repository exposes the same local command.

Source: `.claude/commands/hive-mind/hive-mind-spawn.md`

Arguments supplied to this prompt: $ARGUMENTS

# hive-mind-spawn

Spawn a Hive Mind swarm with queen-led coordination.

## Usage
```bash
npx claude-flow hive-mind spawn <objective> [options]
```

## Options
- `--queen-type <type>` - Queen type (strategic, tactical, adaptive)
- `--max-workers <n>` - Maximum worker agents
- `--consensus <type>` - Consensus algorithm
- `--claude` - Generate Claude Code spawn commands

## Examples
```bash
npx claude-flow hive-mind spawn "Build API"
npx claude-flow hive-mind spawn "Research patterns" --queen-type adaptive
npx claude-flow hive-mind spawn "Build service" --claude
```
