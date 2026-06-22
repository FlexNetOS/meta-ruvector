---
name: source-command-optimization-parallel-execute
description: 'parallel-execute'
---

# /optimization/parallel-execute

Source: `.claude/commands/optimization/parallel-execute.md`

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
