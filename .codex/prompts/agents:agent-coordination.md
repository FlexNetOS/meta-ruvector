---
description: 'agent-coordination'
argument-hint: [ARGUMENTS]
---

You are executing the Codex-native prompt mirror for Claude Code command `/agents:agent-coordination`.

Use Codex-native tools, skills, subagents, MCP servers, and project `AGENTS.md` instructions. Treat Claude-specific tool names, hooks, or MCP names as compatibility context unless this repository exposes the same local command.

Source: `.claude/commands/agents/agent-coordination.md`

Arguments supplied to this prompt: $ARGUMENTS

# agent-coordination

Coordination patterns for multi-agent collaboration.

## Coordination Patterns

### Hierarchical
Queen-led with worker specialization
```bash
npx claude-flow swarm init --topology hierarchical
```

### Mesh
Peer-to-peer collaboration
```bash
npx claude-flow swarm init --topology mesh
```

### Adaptive
Dynamic topology based on workload
```bash
npx claude-flow swarm init --topology adaptive
```

## Best Practices
- Use hierarchical for complex projects
- Use mesh for research tasks
- Use adaptive for unknown workloads
