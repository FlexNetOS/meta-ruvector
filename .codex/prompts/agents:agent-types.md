---
description: 'agent-types'
argument-hint: [ARGUMENTS]
---

You are executing the Codex-native prompt mirror for Claude Code command `/agents:agent-types`.

Use Codex-native tools, skills, subagents, MCP servers, and project `AGENTS.md` instructions. Treat Claude-specific tool names, hooks, or MCP names as compatibility context unless this repository exposes the same local command.

Source: `.claude/commands/agents/agent-types.md`

Arguments supplied to this prompt: $ARGUMENTS

# agent-types

Complete guide to all 54 available agent types in Claude Flow.

## Core Development Agents
- `coder` - Implementation specialist
- `reviewer` - Code quality assurance
- `tester` - Test creation and validation
- `planner` - Strategic planning
- `researcher` - Information gathering

## Swarm Coordination Agents
- `hierarchical-coordinator` - Queen-led coordination
- `mesh-coordinator` - Peer-to-peer networks
- `adaptive-coordinator` - Dynamic topology

## Specialized Agents
- `backend-dev` - API development
- `mobile-dev` - React Native development
- `ml-developer` - Machine learning
- `system-architect` - High-level design

For full list and details:
```bash
npx claude-flow agents list
```
