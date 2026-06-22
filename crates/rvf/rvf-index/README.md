# rvf-index

Progressive HNSW indexing with tiered Layer A/B/C search for RuVector Format.

## Overview

`rvf-index` implements the three-layer **progressive** HNSW indexing model, where each layer is independently useful and is loaded in order so a reader can serve queries before the full graph is available:

- **Layer A** (`LayerA`) -- entry points + coarse routing. Always present, loaded first (**< 5ms load, ~0.70 recall**). Holds HNSW entry points, top-layer adjacency, cluster centroids, and the centroid-to-partition map (`PartitionEntry`).
- **Layer B** (`LayerB`) -- partial adjacency for the hot region. Loaded second (**100ms-1s load, ~0.85 recall**); typically covers 10-20% of nodes (`partial_adjacency`).
- **Layer C** (`LayerC`) -- full HNSW adjacency for every node. Loaded last (**seconds load, >= 0.95 recall**).
- **Progressive build** -- `build_layer_a`, `build_layer_b`, `build_layer_c`, and `build_full_index` construct the layers; `ProgressiveIndex` drives availability and `IndexState` tracks which layers are loaded.
- **Vamana alpha-pruning** -- diversity-aware neighbor selection during build
  (recall@10 0.986 -> 0.996 at ef_search=30, measured at 100k x 64-dim,
  Windows x64 criterion release)
- **Hardened codec** -- INDEX_SEG decoding validates lengths and counts
  against the payload size before allocating (crafted-file DoS resistance)
- **Deterministic search** -- `(distance, id)` tie-breaking for stable result
  ordering across runs

## Usage

```toml
[dependencies]
rvf-index = "0.2"
```

## Features

- `std` (default) -- enable `std` support
- `simd` -- enable SIMD-accelerated distance computations

## License

MIT OR Apache-2.0
