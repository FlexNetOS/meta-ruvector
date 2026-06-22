---
description: 'neural-train'
argument-hint: [ARGUMENTS]
---

You are executing the Codex-native prompt mirror for Claude Code command `/training:neural-train`.

Use Codex-native tools, skills, subagents, MCP servers, and project `AGENTS.md` instructions. Treat Claude-specific tool names, hooks, or MCP names as compatibility context unless this repository exposes the same local command.

Source: `.claude/commands/training/neural-train.md`

Arguments supplied to this prompt: $ARGUMENTS

# neural-train

Train neural patterns from operations.

## Usage
```bash
npx claude-flow training neural-train [options]
```

## Options
- `--data <source>` - Training data source
- `--model <name>` - Target model
- `--epochs <n>` - Training epochs

## Examples
```bash
# Train from recent ops
npx claude-flow training neural-train --data recent

# Specific model
npx claude-flow training neural-train --model task-predictor

# Custom epochs
npx claude-flow training neural-train --epochs 100
```
