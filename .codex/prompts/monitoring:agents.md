---
description: 'List Active Patterns'
argument-hint: [ARGUMENTS]
---

You are executing the Codex-native prompt mirror for Claude Code command `/monitoring:agents`.

Use Codex-native tools, skills, subagents, MCP servers, and project `AGENTS.md` instructions. Treat Claude-specific tool names, hooks, or MCP names as compatibility context unless this repository exposes the same local command.

Source: `.claude/commands/monitoring/agents.md`

Arguments supplied to this prompt: $ARGUMENTS

# List Active Patterns

## ð¯ Key Principle
**This tool coordinates Claude Code's actions. It does NOT write code or create content.**

## MCP Tool Usage in Claude Code

**Tool:** `mcp__claude-flow__agent_list`

## Parameters
```json
{
  "swarmId": "current"
}
```

## Description
View all active cognitive patterns and their current focus areas

## Details
Filters:
- **all**: Show all defined patterns
- **active**: Currently engaged patterns
- **idle**: Available but unused patterns
- **busy**: Patterns actively coordinating tasks

## Example Usage

**In Claude Code:**
1. List all agents: Use tool `mcp__claude-flow__agent_list`
2. Get specific agent metrics: Use tool `mcp__claude-flow__agent_metrics` with parameters `{"agentId": "coder-123"}`
3. Monitor agent performance: Use tool `mcp__claude-flow__swarm_monitor` with parameters `{"interval": 2000}`

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
