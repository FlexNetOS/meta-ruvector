---
description: 'performance-report'
argument-hint: [ARGUMENTS]
---

You are executing the Codex-native prompt mirror for Claude Code command `/analysis/performance-report`.

Use Codex-native tools, skills, subagents, MCP servers, and project `AGENTS.md` instructions. Treat Claude-specific tool names, hooks, or MCP names as compatibility context unless this repository exposes the same local command.

Source: `.claude/commands/analysis/performance-report.md`

Arguments supplied to this prompt: $ARGUMENTS

# performance-report

Generate comprehensive performance reports for swarm operations.

## Usage
```bash
npx claude-flow analysis performance-report [options]
```

## Options
- `--format <type>` - Report format (json, html, markdown)
- `--include-metrics` - Include detailed metrics
- `--compare <id>` - Compare with previous swarm

## Examples
```bash
# Generate HTML report
npx claude-flow analysis performance-report --format html

# Compare swarms
npx claude-flow analysis performance-report --compare swarm-123

# Full metrics report
npx claude-flow analysis performance-report --include-metrics --format markdown
```
