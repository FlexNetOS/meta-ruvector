# ruvector-diskann

DiskANN/Vamana — SSD-friendly approximate nearest neighbor search with product quantization.

## Overview

`ruvector-diskann` implements the Vamana graph index for billion-scale approximate nearest neighbor (ANN) search, following Subramanya et al., "DiskANN" (NeurIPS 2019). It combines greedy graph search with α-robust pruning for bounded out-degree, product quantization for compressed candidate filtering, and memory-mapped graph storage so neighbors are loaded from SSD on demand. It is one of the ANN index families in the RuVector stack and is wrapped for Node.js by `ruvector-diskann-node`.

## Key API

- `DiskAnnIndex` — build, insert, batch-insert, search, delete, count, save, and load.
- `DiskAnnConfig` — `dim`, `max_degree`, `build_beam`, `search_beam`, `alpha`, PQ subspaces/iterations, and `storage_path`.
- `ProductQuantizer` — PQ codebook used for compressed distance estimation.
- `DiskAnnError` / `Result` — crate error type and result alias.
- `DriftingIndex`, `RebuildPolicy`, `RecallTrigger` — fixed-topology reuse + periodic rebuild under metric drift (feature-gated; BET 1, ADR-200).
- Module layout: `distance`, `graph`, `index`, `pq`, `reuse`.

## Features

- `gpu` — feature flag for GPU acceleration (CUDA/Metal stubs).
- `simd` — enables `simsimd`-backed distance kernels.
- `reuse-under-drift` — enables the `reuse` module (`DriftingIndex` and friends).

## License

MIT
