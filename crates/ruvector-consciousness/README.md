# ruvector-consciousness

Consciousness metrics for Rust: IIT Φ computation, causal emergence, and effective information with SIMD acceleration and sublinear approximations.

## Overview

`ruvector-consciousness` computes integrated-information-theory (IIT) quantities over transition probability matrices. It provides exact, spectral, and stochastic estimators for Φ, plus causal-emergence / effective-information analysis and a quantum-inspired minimum-information-partition (MIP) search. Within the RuVector stack it is a research-tier crate that can borrow acceleration substrates from sibling crates (`ruvector-solver`, `ruvector-sparsifier`, `ruvector-mincut`, `ruvector-math`, `ruvector-coherence`) via optional features.

## Key API

- `types::TransitionMatrix`, `types::ComputeBudget` — system description and compute limits.
- `phi::auto_compute_phi(&tpm, mechanism, &budget)` — selects an algorithm by system size and returns Φ plus the algorithm used.
- `phi` module — exact (`O(2^n·n²)`), spectral (`O(n²·log n)`), and stochastic (`O(k·n²)`) Φ estimators.
- `emergence`, `rsvd_emergence` — causal emergence / effective information.
- `collapse` — quantum-inspired MIP partition search.
- `iit4`, `ces`, `phi_id`, `pid`, `streaming`, `bounds` — IIT 4.0 / SOTA extensions (cause-effect structure, ΦID, partial information decomposition, streaming, error bounds).
- `simd`, `arena` — AVX2 kernels (KL-divergence, entropy, matvec) and a zero-alloc bump arena.

## Features

- `phi`, `emergence`, `collapse` (default) — core algorithm modules.
- `simd`, `parallel` (rayon + crossbeam), `wasm` — execution backends.
- `solver-accel`, `sparsifier-accel`, `mincut-accel`, `math-accel`, `coherence-accel`, `witness` — optional sibling-crate accelerators.
- `full` — enables all of the above.

## License

MIT
