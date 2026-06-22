---
name: source-command-optimization-topology-optimize
description: 'topology-optimize'
---

# /optimization/topology-optimize

Source: `.claude/commands/optimization/topology-optimize.md`

# topology-optimize

Optimize swarm topology for current workload.

## Usage
```bash
npx claude-flow optimization topology-optimize [options]
```

## Options
- `--analyze-first` - Analyze before optimizing
- `--target <metric>` - Optimization target
- `--apply` - Apply optimizations

## Examples
```bash
# Analyze and suggest
npx claude-flow optimization topology-optimize --analyze-first

# Optimize for speed
npx claude-flow optimization topology-optimize --target speed

# Apply changes
npx claude-flow optimization topology-optimize --target efficiency --apply
```
