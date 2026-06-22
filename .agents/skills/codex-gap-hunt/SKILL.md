---
name: codex-gap-hunt
description: 'Use when auditing Codex parity gaps across hooks, helpers, prompts, skills, custom agents, subagents, settings, MCP, and auto-loop workflows.'
---

# Codex Gap Hunt

Audit from current evidence, not memory. Compare source and generated surfaces:

- .claude/commands -> .agents/skills/source-command-* and .codex/prompts -> CODEX_HOME/prompts
- .claude/agents -> .codex/agents custom-agent TOML schema and explicit subagent workflows
- .claude/settings.json -> .codex/config.toml and .codex/hooks.json using supported Codex hook events
- .claude/hooks and helpers -> .codex/hooks and .codex/helpers
- AGENTS.md, ICM, verification, commit/push/PR workflow

Rank gaps by user impact, then implement upgrades only. Verify with commands that prove the touched surface works.
