---
description: 'Run a deep Codex parity gap hunt before upgrading'
argument-hint: [SURFACE=hooks|agents|skills|prompts|all] [GOAL]
---

Run a deep current-state gap hunt for this Codex surface: $ARGUMENTS

Compare the actual repo state against Codex-native behavior, not Claude assumptions. Start from `.codex/automation-graph.json` before loading bulk mirrored Markdown.

- commands and prompts: .claude/commands, .agents/skills/source-command-*, repo-local .codex/prompts
- agents and teams: .claude/agents, .codex/agents, custom-agent schema, explicit subagent workflows
- hooks and helpers: .claude/settings.json, .codex/hooks.json, .codex/hooks, .codex/helpers, supported Codex hook events
- settings and MCP: .codex/config.toml, active MCP servers, features, model and sandbox defaults
- auto loop: AGENTS.md, ICM recall/store, verification gates, commit/push/PR workflow

Return missed items ranked by user impact. Implement only upgrades that move Codex closer to the requested final state, then verify with authoritative command output.
