---
description: 'workflow-select'
argument-hint: [ARGUMENTS]
---

You are executing the Codex-native prompt mirror for Claude Code command `/automation/workflow-select`.

Use Codex-native tools, skills, subagents, MCP servers, and project `AGENTS.md` instructions. Treat Claude-specific tool names, hooks, or MCP names as compatibility context unless this repository exposes the same local command.

Source: `.claude/commands/automation/workflow-select.md`

Arguments supplied to this prompt: $ARGUMENTS

# workflow-select

Automatically select optimal workflow based on task type.

## Usage
```bash
npx claude-flow automation workflow-select [options]
```

## Options
- `--task <description>` - Task description
- `--constraints <list>` - Workflow constraints
- `--preview` - Preview without executing

## Examples
```bash
# Select workflow for task
npx claude-flow automation workflow-select --task "Deploy to production"

# With constraints
npx claude-flow automation workflow-select --constraints "no-downtime,rollback"

# Preview mode
npx claude-flow automation workflow-select --task "Database migration" --preview
```
