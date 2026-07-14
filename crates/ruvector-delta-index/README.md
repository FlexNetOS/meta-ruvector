# ruvector-delta-index

Delta-aware HNSW index with incremental updates and repair strategies.

## Overview

`ruvector-delta-index` is an HNSW (Hierarchical Navigable Small World) approximate-nearest-neighbor index optimized for frequent, small changes to vector embeddings. Instead of rebuilding the graph on every change, it applies `VectorDelta` updates in place, tracks cumulative drift per node, and repairs graph connectivity when quality degrades. It is the indexing layer of the RuVector delta-CRDT stack in the meta-ruvector workspace, built on `ruvector-delta-core`.

## Key API

- `DeltaHnsw` — the index; `new(dimensions, config)` constructs it, `insert` adds vectors, `apply_delta` / `apply_deltas_batch` apply incremental updates (triggering repair past a threshold), `search` returns k nearest neighbors, and `delete` removes a vector. Maintenance: `force_repair`, `compact_deltas`, `quality_metrics`, plus `len` / `is_empty` / `dimensions` / `config`.
- `DeltaHnswConfig` — tuning parameters (`m`, `m0`, `ef_construction`, `ef_search`, `max_elements`, `level_mult`, `repair_threshold`, `max_deltas`, `auto_monitor`).
- `SearchResult` — a result with `id`, `distance`, and optional `vector`.
- `IncrementalUpdater` — incremental update logic.
- `GraphRepairer` / `RepairConfig` / `RepairStrategy` — graph repair strategies for maintaining recall after deltas.
- `QualityMonitor` / `QualityMetrics` / `RecallEstimate` — recall-quality monitoring.
- `IndexError` / `Result` — crate error type and result alias.

## Features

- `parallel` (default) — enables parallel processing via `rayon`.
- `simd` — enables SIMD-accelerated distance via `simsimd`.
- `persistence` — enables `bincode`-based serialization for persisting the index.

## License

Licensed under MIT OR Apache-2.0.
