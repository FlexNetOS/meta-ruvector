---
description: 'Claude Code Hooks for claude-flow'
argument-hint: [ARGUMENTS]
---

You are executing the Codex-native prompt mirror for Claude Code command `/hooks/overview`.

Use Codex-native tools, skills, subagents, MCP servers, and project `AGENTS.md` instructions. Treat Claude-specific tool names, hooks, or MCP names as compatibility context unless this repository exposes the same local command.

Source: `.claude/commands/hooks/overview.md`

Arguments supplied to this prompt: $ARGUMENTS

# Claude Code Hooks for claude-flow

## Purpose
Automatically coordinate, format, and learn from Claude Code operations using hooks.

## Available Hooks

### Pre-Operation Hooks
- **pre-edit**: Validate and assign agents before file modifications
- **pre-bash**: Check command safety and resource requirements
- **pre-task**: Auto-spawn agents for complex tasks

### Post-Operation Hooks
- **post-edit**: Auto-format code and train neural patterns
- **post-bash**: Log execution and update metrics
- **post-search**: Cache results and improve search patterns

### MCP Integration Hooks
- **mcp-initialized**: Persist swarm configuration
- **agent-spawned**: Update agent roster
- **task-orchestrated**: Monitor task progress
- **neural-trained**: Save pattern improvements

### Session Hooks
- **notify**: Custom notifications with swarm status
- **session-end**: Generate summary and save state
- **session-restore**: Load previous session state

## Configuration
Hooks are configured in `.claude/settings.json`:

```json
{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "^(Write|Edit|MultiEdit)$$",
        "hooks": [{
          "type": "command",
          "command": "npx claude-flow hook pre-edit --file '$${tool.params.file_path}'"
        }]
      }
    ]
  }
}
```

## Benefits
- ð¤ Automatic agent assignment based on file type
- ð¨ Consistent code formatting
- ð§  Continuous neural pattern improvement
- ð¾ Cross-session memory persistence
- ð Performance metrics tracking

## See Also
- [Pre-Edit Hook](./pre-edit.md)
- [Post-Edit Hook](./post-edit.md)
- [Session End Hook](./session-end.md)
