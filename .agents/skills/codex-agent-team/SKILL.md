---
name: codex-agent-team
description: 'Use when a task should spawn a Codex-native team of project custom agents for parallel research, implementation planning, review, security, GitHub, or swarm coordination.'
---

# Codex Agent Team

Use Codex subagents explicitly. Pick the smallest effective team, spawn agents in parallel, wait for all results, then run parent consolidation.
Use the configured custom agent TOMLs as the model-routing source: heavy agents run on `gpt-5.5`, lighter explorer/template agents run on `gpt-5.4-mini`, and each agent carries its own reasoning effort.

Recommended teams:
- core: claude-core-planner, claude-core-researcher, claude-core-coder, claude-core-tester, claude-core-reviewer
- review: reviewer, claude-core-reviewer, claude-testing-production-validator, claude-v3-security-auditor
- rust: explorer, claude-core-coder, claude-core-tester, claude-v3-performance-engineer
- security: claude-v3-security-architect, claude-v3-security-auditor, claude-v3-pii-detector, claude-v3-aidefence-guardian
- github: claude-github-pr-manager, claude-github-code-review-swarm, claude-github-workflow-automation
- swarm: claude-swarm-hierarchical-coordinator, claude-hive-mind-queen-coordinator, claude-v3-v3-queen-coordinator

When running from the shell, prefer the Rust harness:

```bash
cargo run -p codex-env -- team-run --team core "your goal"
```

The harness runs every team member with its configured model/reasoning effort and then launches a parent consolidation Codex pass. Give each subagent a bounded brief and a required evidence format. Keep write ownership in the parent pass unless a subagent has an isolated file scope.
