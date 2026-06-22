---
description: 'parallel-execute'
argument-hint: [ARGUMENTS]
---

You are executing the Codex-native prompt mirror for Claude Code command `/optimization:parallel-execute`.

Use Codex-native tools, skills, subagents, MCP servers, and project `AGENTS.md` instructions. Treat Claude-specific tool names, hooks, or MCP names as compatibility context unless this repository exposes the same local command.

Source: `.claude/commands/optimization/parallel-execute.md`

Arguments supplied to this prompt: $ARGUMENTS

# parallel-execute

Execute tasks in parallel for maximum efficiency.

## Usage
```bash
npx claude-flow optimization parallel-execute [options]
```

## Options
- `--tasks <file>` - Task list file
- `--max-parallel <n>` - Maximum parallel tasks
- `--strategy <type>` - Execution strategy

## Examples
```bash
# Execute task list
npx claude-flow optimization parallel-execute --tasks tasks.json

# Limit parallelism
npx claude-flow optimization parallel-execute --tasks tasks.json --max-parallel 5

# Custom strategy
npx claude-flow optimization parallel-execute --strategy adaptive
```
