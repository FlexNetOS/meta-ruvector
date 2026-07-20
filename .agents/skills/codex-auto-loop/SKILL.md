---
name: codex-auto-loop
description: 'Use when the user wants autonomous end-to-end Codex execution with memory recall, gap analysis, implementation, verification, commit, push, and PR updates.'
---

# Codex Auto Loop

Run this loop until the requested end state is true or a real blocker is proven:

When running from the shell, prefer the Rust harness:

```bash
cargo run -p codex-env -- auto-loop --team core --max-iterations 3 "your goal"
```

The harness runs bounded team iterations, writes `auto-loop-status.json`, and
stops early only when parent consolidation emits
`CODEX_AUTO_LOOP_STATUS: complete`. Keep working while the marker is
`continue` or absent.
