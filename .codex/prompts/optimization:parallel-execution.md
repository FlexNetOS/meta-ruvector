---
description: 'Parallel Task Execution'
argument-hint: [ARGUMENTS]
---

You are executing the Codex-native prompt mirror for Claude Code command `/optimization:parallel-execution`.

Use Codex-native tools, skills, subagents, MCP servers, and project `AGENTS.md` instructions. Treat Claude-specific tool names, hooks, or MCP names as compatibility context unless this repository exposes the same local command.

Source: `.claude/commands/optimization/parallel-execution.md`

Arguments supplied to this prompt: $ARGUMENTS

# Parallel Task Execution

## Purpose
Execute independent subtasks in parallel for maximum efficiency.

## Coordination Strategy

### 1. Task Decomposition
```
Tool: mcp__claude-flow__task_orchestrate
Parameters: {
  "task": "Build complete REST API with auth, CRUD operations, and tests",
  "strategy": "parallel",
  "maxAgents": 8
}
```

### 2. Parallel Workflows
The system automatically:
- Identifies independent components
- Assigns specialized agents
- Executes in parallel where possible
- Synchronizes at dependency points

### 3. Example Breakdown
For the REST API task:
- **Agent 1 (Architect)**: Design API structure
- **Agent 2-3 (Coders)**: Implement auth & CRUD in parallel
- **Agent 4 (Tester)**: Write tests as features complete
- **Agent 5 (Documenter)**: Update docs continuously

## CLI Usage
```bash
# Execute parallel tasks via CLI
npx claude-flow parallel "Build REST API" --max-agents 8
```

## Performance Gains
- ð 2.8-4.4x faster execution
- ðª Optimal CPU utilization
- ð Automatic load balancing
- ð Linear scalability with agents

## Monitoring
```
Tool: mcp__claude-flow__swarm_monitor
Parameters: {"interval": 1000, "swarmId": "current"}
```

Watch real-time parallel execution progress!
