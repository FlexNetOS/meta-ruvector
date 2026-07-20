# exo-manifold

Simplified manifold storage using vector-similarity search for the EXO-AI
cognitive substrate. Patterns are stored with their embeddings and retrieved
by similarity.

> ⚠️ **Stub notice:** the `burn` neural-network dependency was **removed** (to
> avoid a `bincode` version conflict), so this crate currently implements a
> *simplified, Vec-based* manifold. **SIREN is a stub placeholder** — the
> `SirenLayer` type is an empty unit struct and `LearnedManifold` only stores
> dimensions, not a trained network. The "Planned / WIP" features below are
> **not yet implemented**.

## Features

- **Vector-similarity retrieval** -- stores patterns with their embeddings and
  retrieves the nearest ones via [`ManifoldEngine::retrieve`].
- **Pattern storage (`deform`)** -- [`ManifoldEngine::deform`] records a pattern
  with a salience weight for later retrieval.
- **Strategic forgetting (`forget`)** -- [`ManifoldEngine::forget`] prunes
  low-salience patterns below a threshold.
- **SIMD distance helpers** -- `cosine_similarity_simd`,
  `euclidean_distance_simd`, and `batch_distances` in the `simd_ops` module.

### Planned / WIP (not yet implemented)

- **SIREN coordinate network** -- sinusoidal representation networks for
  implicit neural coordinate spaces. *Stub only* (`SirenLayer` is empty); the
  `burn`-backed implementation was removed.
- **Smooth manifold deformation** -- warping a learned manifold while
  preserving neighbourhood structure. Current `deform` only stores patterns.
- **Transfer prior store with domain-pair indexing** -- the `transfer_store`
  module exists; treat domain-pair-indexed deformation priors as WIP.

## Quick Start

Add the dependency to your `Cargo.toml`:

```toml
[dependencies]
exo-manifold = "0.1"
```

Basic usage:

```rust
use exo_manifold::ManifoldEngine;
use exo_core::{ManifoldConfig, Pattern};

// Create the engine from a config (no backend generic, no device argument)
let config = ManifoldConfig::default();
let mut engine = ManifoldEngine::new(config);

// Store a high-salience pattern (simplified deformation)
let pattern = Pattern { /* ... */ };
engine.deform(pattern, 0.9)?;

// Retrieve the k most similar stored patterns
let query = vec![/* embedding */];
let results = engine.retrieve(&query, 10)?;

// Strategic forgetting: prune patterns below the salience threshold
engine.forget(0.5, 0.1)?;
```

## Crate Layout

| Module          | Purpose                                                  |
|-----------------|----------------------------------------------------------|
| `network`       | `LearnedManifold` storage struct (SIREN is a stub)       |
| `retrieval`     | `GradientDescentRetriever` similarity retrieval          |
| `deformation`   | `ManifoldDeformer` (simplified pattern storage)          |
| `forgetting`    | `StrategicForgetting` salience-based pruning             |
| `simd_ops`      | SIMD distance helpers (cosine, euclidean, batch)         |
| `transfer_store`| Transfer prior store with domain-pair indexing (WIP)     |

## Requirements

- Rust 2021 edition
- Depends on `exo-core`, `ruvector-domain-expansion`, `ndarray`,
  `parking_lot`, `serde` (the `burn` / `burn-ndarray` neural-network
  dependencies were **removed**)

## Links

- [GitHub](https://github.com/FlexNetOS/ruvector)
- [EXO-AI Documentation](https://github.com/FlexNetOS/ruvector/tree/main/examples/exo-ai-2025)

## License

MIT OR Apache-2.0
