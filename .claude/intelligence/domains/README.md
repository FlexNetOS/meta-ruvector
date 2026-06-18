# Intelligence — Bounded Contexts (DDD)

`intelligence/index.js` was a 1,161-line god object. It is now a thin **facade**
that composes seven bounded-context modules into the `RuVectorIntelligence`
aggregate and re-exports the original public interface (behavior-preserving,
pinned by `test/characterization.test.mjs`).

## Shared kernel
- **`../shared-kernel.js`** — node builtins, data-file path constants, optional
  native deps (`@ruvector/core` HNSW, hyperbolic-attention WASM), and the
  embedding/distance math (`poincareDistance`, `textToEmbedding`,
  `cosineSimilarity`). Kept at the intelligence root so `import.meta.url`-relative
  paths are unchanged.

## Bounded contexts (`domains/`)
| Module | Responsibility | Data file |
|--------|----------------|-----------|
| `vector-memory.js` (`VectorMemory`) | semantic memory store + search (HNSW / cosine) | `memory.json` |
| `reasoning-bank.js` (`ReasoningBank`) | trajectories, Q-table, A/B, decay, + verdict judgment / memory distillation / experience replay | `trajectories.json`, `patterns.json` |
| `calibration-tracker.js` (`CalibrationTracker`) | predicted-vs-actual confidence calibration | `calibration.json` |
| `feedback-loop.js` (`FeedbackLoop`) | learn from followed/ignored suggestions | `feedback.json` |
| `error-pattern-tracker.js` (`ErrorPatternTracker`) | error categorization + fix suggestion | `error-patterns.json` |
| `sequence-tracker.js` (`SequenceTracker`) | edit sequences + test-pairing suggestions | `sequences.json` |
| `neural-router.js` (`NeuralRouter`) | agent routing (deps injected, no direct domain imports) | — |

The aggregate (`RuVectorIntelligence` in `index.js`) is the application layer:
it wires the contexts together and exposes `remember/recall/learn/suggest/route/
record*/suggest*/stats`. Each context owns its own persistence and depends only on
the shared kernel — no context imports another.

## Test
```bash
node .claude/intelligence/test/characterization.test.mjs   # 30 assertions, behavior parity
```
