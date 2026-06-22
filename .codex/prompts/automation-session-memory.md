---
description: 'Cross-Session Memory'
argument-hint: [ARGUMENTS]
---

You are executing the Codex-native prompt mirror for Claude Code command `/automation/session-memory`.

Use Codex-native tools, skills, subagents, MCP servers, and project `AGENTS.md` instructions. Treat Claude-specific tool names, hooks, or MCP names as compatibility context unless this repository exposes the same local command.

Source: `.claude/commands/automation/session-memory.md`

Arguments supplied to this prompt: $ARGUMENTS

# Cross-Session Memory

## Purpose
Maintain context and learnings across Claude Code sessions for continuous improvement.

## Memory Features

### 1. Automatic State Persistence
At session end, automatically saves:
- Active agents and specializations
- Task history and patterns
- Performance metrics
- Neural network weights
- Knowledge base updates

### 2. Session Restoration
```javascript
// Using MCP tools for memory operations
mcp__claude-flow__memory_usage({
  "action": "retrieve",
  "key": "session-state",
  "namespace": "sessions"
})

// Restore swarm state
mcp__claude-flow__context_restore({
  "snapshotId": "sess-123"
})
```

**Fallback with npx:**
```bash
npx claude-flow hook session-restore --session-id "sess-123"
```

### 3. Memory Types

**Project Memory:**
- File relationships
- Common edit patterns
- Testing approaches
- Build configurations

**Agent Memory:**
- Specialization levels
- Task success rates
- Optimization strategies
- Error patterns

**Performance Memory:**
- Bottleneck history
- Optimization results
- Token usage patterns
- Efficiency trends

### 4. Privacy & Control
```javascript
// List memory contents
mcp__claude-flow__memory_usage({
  "action": "list",
  "namespace": "sessions"
})

// Delete specific memory
mcp__claude-flow__memory_usage({
  "action": "delete",
  "key": "session-123",
  "namespace": "sessions"
})

// Backup memory
mcp__claude-flow__memory_backup({
  "path": "./backups/memory-backup.json"
})
```

**Manual control:**
```bash
# View stored memory
ls .claude-flow/memory/

# Disable memory
export CLAUDE_FLOW_MEMORY_PERSIST=false
```

## Benefits
- ð§  Contextual awareness
- ð Cumulative learning
- â¡ Faster task completion
- ð¯ Personalized optimization
