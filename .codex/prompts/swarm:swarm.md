---
description: 'swarm'
argument-hint: [ARGUMENTS]
---

You are executing the Codex-native prompt mirror for Claude Code command `/swarm:swarm`.

Use Codex-native tools, skills, subagents, MCP servers, and project `AGENTS.md` instructions. Treat Claude-specific tool names, hooks, or MCP names as compatibility context unless this repository exposes the same local command.

Source: `.claude/commands/swarm/swarm.md`

Arguments supplied to this prompt: $ARGUMENTS

# swarm

Main swarm orchestration command for Claude Flow.

## Usage
```bash
npx claude-flow swarm <objective> [options]
```

## Options
- `--strategy <type>` - Execution strategy (research, development, analysis, testing)
- `--mode <type>` - Coordination mode (centralized, distributed, hierarchical, mesh)
- `--max-agents <n>` - Maximum number of agents (default: 5)
- `--claude` - Open Claude Code CLI with swarm prompt
- `--parallel` - Enable parallel execution

## Examples
```bash
# Basic swarm
npx claude-flow swarm "Build REST API"

# With strategy
npx claude-flow swarm "Research AI patterns" --strategy research

# Open in Claude Code
npx claude-flow swarm "Build API" --claude
```
