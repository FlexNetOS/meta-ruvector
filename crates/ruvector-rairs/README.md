# ruvector-rairs

RAIRS IVF: Redundant Assignment with Amplified Inverse Residual — RuVector's first IVF index family.

## Overview

`ruvector-rairs` is an Inverted File (IVF) index family that recovers the low-`nprobe` recall classic IVF loses near Voronoi-cell boundaries. It redundantly assigns each vector to a primary list and a residual-amplified secondary list, then stores shared copies in deduplicating 32-vector blocks (SEIL layout) so the second assignment costs no extra memory. Design rationale is in `docs/adr/ADR-193`, and the crate ships a `rairs-demo` binary for benchmarking. It is one of the ANN index families in the RuVector stack.

Provenance note: the "RAIRS / SEIL" naming and cited references in the design docs are not independently verified — treat this as an original implementation of the redundant-assignment idea (cf. spill lists / SOAR / multi-probe LSH) and judge it on the benchmarks.

## Key API

- `AnnIndex` — common index trait implemented by all three variants.
- `IvfFlat` — single-assignment flat IVF baseline (one list per vector).
- `RairsStrict` — dual redundant assignment, flat layout, no dedup.
- `RairsSeil` — dual redundant assignment with shared 32-vector SEIL blocks and query-time dedup.
- `SearchResult` — ranked result type.
- `RairsError` — crate error type.
- Supporting modules: `ivf`, `kmeans`, `rairs`, `seil`.

## License

MIT OR Apache-2.0
