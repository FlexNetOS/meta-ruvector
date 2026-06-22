---
description: 'model-update'
argument-hint: [ARGUMENTS]
---

You are executing the Codex-native prompt mirror for Claude Code command `/training:model-update`.

Use Codex-native tools, skills, subagents, MCP servers, and project `AGENTS.md` instructions. Treat Claude-specific tool names, hooks, or MCP names as compatibility context unless this repository exposes the same local command.

Source: `.claude/commands/training/model-update.md`

Arguments supplied to this prompt: $ARGUMENTS

# model-update

Update neural models with new data.

## Usage
```bash
npx claude-flow training model-update [options]
```

## Options
- `--model <name>` - Model to update
- `--incremental` - Incremental update
- `--validate` - Validate after update

## Examples
```bash
# Update all models
npx claude-flow training model-update

# Specific model
npx claude-flow training model-update --model agent-selector

# Incremental with validation
npx claude-flow training model-update --incremental --validate
```
