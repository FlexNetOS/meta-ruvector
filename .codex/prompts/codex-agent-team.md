---
description: 'Spawn a Codex-native subagent team from repo custom agents'
argument-hint: [TEAM=core|review|rust|security|github|swarm] [GOAL]
---

Use Codex-native subagents for this goal: $ARGUMENTS

Select the smallest effective team. Spawn the agents in parallel, wait for all results, then consolidate:
Use the configured custom agent TOMLs as the routing source: heavy agents run on `gpt-5.5`, lighter explorer/template agents run on `gpt-5.4-mini`, and each agent carries its own reasoning effort.

- core: claude-core-planner, claude-core-researcher, claude-core-coder, claude-core-tester, claude-core-reviewer
- review: reviewer, claude-core-reviewer, claude-testing-production-validator, claude-v3-security-auditor
- rust: explorer, claude-core-coder, claude-core-tester, claude-v3-performance-engineer
- security: claude-v3-security-architect, claude-v3-security-auditor, claude-v3-pii-detector, claude-v3-aidefence-guardian
- github: claude-github-pr-manager, claude-github-code-review-swarm, claude-github-workflow-automation
- swarm: claude-swarm-hierarchical-coordinator, claude-hive-mind-queen-coordinator, claude-v3-v3-queen-coordinator

Give each subagent a bounded brief with concrete evidence to return. Do not let subagents modify the same file concurrently. After all results return, decide the implementation path, make the edits in the parent thread, verify, commit, push, and update the PR when publishing applies.
