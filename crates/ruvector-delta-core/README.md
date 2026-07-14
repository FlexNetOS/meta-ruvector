# ruvector-delta-core

Core delta types and traits for behavioral vector change tracking.

## Overview

`ruvector-delta-core` provides the fundamental abstractions for computing, applying, composing, and inverting deltas on vector data. It is the foundation of the RuVector delta-CRDT stack in the meta-ruvector workspace: every higher-level crate (graph, index, consensus, WASM bindings) builds on the `Delta` trait and `VectorDelta` type defined here. The crate supports sparse, dense, and hybrid encodings, delta streaming for event sourcing, and time-bounded windowed aggregation. It is `no_std`-capable (the `std` feature is on by default).

## Key API

- `Delta` — core trait defining `compute`, `apply`, `compose`, `inverse`, `is_identity`, and `byte_size` for any delta type.
- `VectorDelta` — the primary vector delta; build with `compute`, `from_dense`, `from_sparse`, or `new`, and inspect via `l2_norm`, `l1_norm`, and `is_identity`.
- `DeltaValue` / `DeltaOp` / `SparseDelta` — the underlying value representations (identity, sparse, dense, replace).
- `DeltaEncoding` and the `DenseEncoding`, `SparseEncoding`, `RunLengthEncoding`, `HybridEncoding` implementations with `EncodingType` — encode/decode deltas to and from bytes.
- `DeltaCompressor` / `CompressionCodec` / `CompressionLevel` — delta-specific compression.
- `DeltaStream` with `DeltaStreamConfig` and `StreamCheckpoint` — ordered delta sequences with replay and checkpointing for event sourcing.
- `DeltaWindow` with `WindowAggregator`, `WindowConfig`, `WindowResult`, `WindowType` — tumbling, sliding, and count-based windowed aggregation.
- `DeltaError` / `Result` — crate error type and result alias.
- `prelude` — convenience module re-exporting the most common types.

## Features

- `std` (default) — enables the standard library; without it the crate builds `no_std` (using `alloc`).
- `simd` — enables SIMD-accelerated operations via `simsimd`.
- `serde` — derives `serde` serialization for delta types (also pulls in `serde_json`).
- `compression` — enables the `lz4_flex` and `zstd` compression codecs.

## License

Licensed under MIT OR Apache-2.0.
