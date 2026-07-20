---
name: codex-gap-hunt
description: 'Use when auditing Codex parity gaps across hooks, helpers, prompts, skills, custom agents, subagents, settings, MCP, and auto-loop workflows.'
---

# Codex Gap Hunt

Audit from current evidence, not memory. Start from `.codex/automation-graph.json`, then compare source and generated surfaces only as needed:

- .claude/commands -> .agents/skills/source-command-* and repo-local .codex/prompts
- .claude/agents -> .codex/agents custom-agent TOML schema and explicit subagent workflows
- .claude/settings.json -> .codex/config.toml and .codex/hooks.json using supported Codex hook events
- .claude/hooks and helpers -> .codex/hooks and .codex/helpers
- AGENTS.md, ICM, verification, commit/push/PR workflow

Rank gaps by user impact, then implement upgrades only. Verify with commands that prove the touched surface works.
