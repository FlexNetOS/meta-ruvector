---
description: 'Run the full Codex autonomous implementation loop'
argument-hint: [GOAL]
---

Run the Codex autonomous loop for this goal: $ARGUMENTS

Use the Rust harness when shell execution is appropriate:

```bash
cargo run -p codex-env -- auto-loop --team core --max-iterations 3 "$ARGUMENTS"
```

The harness runs bounded team iterations, stores artifacts under
`.codex/harness/runs/`, and requires the parent consolidation pass to emit
`CODEX_AUTO_LOOP_STATUS: complete` before the loop stops early.
