---
description: 'swarm-monitor'
argument-hint: [ARGUMENTS]
---

You are executing the Codex-native prompt mirror for Claude Code command `/monitoring:swarm-monitor`.

Use Codex-native tools, skills, subagents, MCP servers, and project `AGENTS.md` instructions. Treat Claude-specific tool names, hooks, or MCP names as compatibility context unless this repository exposes the same local command.

Source: `.claude/commands/monitoring/swarm-monitor.md`

Arguments supplied to this prompt: $ARGUMENTS

# swarm-monitor

Real-time swarm monitoring.

## Usage
```bash
npx claude-flow swarm monitor [options]
```

## Options
- `--interval <ms>` - Update interval
- `--metrics` - Show detailed metrics
- `--export` - Export monitoring data

## Examples
```bash
# Start monitoring
npx claude-flow swarm monitor

# Custom interval
npx claude-flow swarm monitor --interval 5000

# With metrics
npx claude-flow swarm monitor --metrics
```
