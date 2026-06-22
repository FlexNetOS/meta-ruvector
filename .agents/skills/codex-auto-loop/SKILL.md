---
name: codex-auto-loop
description: 'Use when the user wants autonomous end-to-end Codex execution with memory recall, gap analysis, implementation, verification, commit, push, and PR updates.'
---

# Codex Auto Loop

Run this loop until the requested end state is true or a real blocker is proven:

1. Recall ICM memory and inspect the current repo/branch/PR state.
2. Derive concrete requirements and completion evidence.
3. Spawn focused Codex subagents for broad or uncertain work.
4. Implement upgrades in the parent thread using repo patterns.
5. Regenerate deterministic Codex surfaces with codex-env when needed.
6. Run targeted gates, mirror checks, install checks, and risk-appropriate broader gates.
7. Commit, push, update or open the PR, and store ICM memory for significant work.
8. Continue to the next gap while the active objective remains incomplete.
