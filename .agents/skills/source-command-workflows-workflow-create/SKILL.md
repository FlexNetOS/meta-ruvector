---
name: source-command-workflows-workflow-create
description: 'workflow-create'
---

# /workflows:workflow-create

Source: `.claude/commands/workflows/workflow-create.md`

# workflow-create

Create reusable workflow templates.

## Usage
```bash
npx claude-flow workflow create [options]
```

## Options
- `--name <name>` - Workflow name
- `--from-history` - Create from history
- `--interactive` - Interactive creation

## Examples
```bash
# Create workflow
npx claude-flow workflow create --name "deploy-api"

# From history
npx claude-flow workflow create --name "test-suite" --from-history

# Interactive mode
npx claude-flow workflow create --interactive
```
