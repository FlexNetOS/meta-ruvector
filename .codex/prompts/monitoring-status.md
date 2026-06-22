---
description: 'Check Coordination Status'
argument-hint: [ARGUMENTS]
---

You are executing the Codex-native prompt mirror for Claude Code command `/monitoring/status`.

Use Codex-native tools, skills, subagents, MCP servers, and project `AGENTS.md` instructions. Treat Claude-specific tool names, hooks, or MCP names as compatibility context unless this repository exposes the same local command.

Source: `.claude/commands/monitoring/status.md`

Arguments supplied to this prompt: $ARGUMENTS

# Check Coordination Status

## ð¯ Key Principle
**This tool coordinates Claude Code's actions. It does NOT write code or create content.**

## MCP Tool Usage in Claude Code

**Tool:** `mcp__claude-flow__swarm_status`

## Parameters
```json
{
  "swarmId": "current"
}
```

## Description
Monitor the effectiveness of current coordination patterns

## Details
Shows:
- Active coordination topologies
- Current cognitive patterns in use
- Task breakdown and progress
- Resource utilization for coordination
- Overall system health

## Example Usage

**In Claude Code:**
1. Check swarm status: Use tool `mcp__claude-flow__swarm_status`
2. Monitor in real-time: Use tool `mcp__claude-flow__swarm_monitor` with parameters `{"interval": 1000}`
3. Get agent metrics: Use tool `mcp__claude-flow__agent_metrics` with parameters `{"agentId": "agent-123"}`
4. Health check: Use tool `mcp__claude-flow__health_check` with parameters `{"components": ["swarm", "memory", "neural"]}`

## Important Reminders
- â This tool provides coordination and structure
- â Claude Code performs all actual implementation
- â The tool does NOT write code
- â The tool does NOT access files directly
- â The tool does NOT execute commands

## See Also
- Main documentation: /CLAUDE.md
- Other commands in this category
- Workflow examples in /workflows/
