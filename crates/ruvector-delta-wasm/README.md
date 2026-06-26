# ruvector-delta-wasm

WASM bindings for delta operations on vectors.

## Overview

`ruvector-delta-wasm` exposes the `ruvector-delta-core` delta engine to JavaScript and WebAssembly via `wasm-bindgen`. It provides delta capture from vector pairs, in-place and cloning delta application, sparse/dense/byte serialization, streaming with checkpoints, and windowed aggregation — all callable from JS. It is the browser/Node-facing layer of the RuVector delta-CRDT stack in the meta-ruvector workspace, and builds as both a `cdylib` (for WASM) and an `rlib`.

## Key API

- `DeltaEngine` — main entry point constructed with `new(dimensions)`; `capture` computes a delta between two `Float32Array`s, `apply` / `applyClone` apply a delta, `fromSparse` / `fromDense` / `identity` build deltas, `captureBatch` processes many pairs, `composeTwo` composes deltas, and `setSparsityThreshold` tunes encoding.
- `JsDelta` — JS-friendly delta wrapper exposing `dimensions`, `isIdentity`, `sparsity`, `nnz`, `byteSize`, `l2Norm`, `l1Norm`, and `scale`, `clip`, `compose`, `inverse`, `toDense`, `toSparse`, `toBytes`, `fromBytes`.
- `JsDeltaStream` — delta stream for event sourcing: `push`, `replay`, `createCheckpoint`, `replayFromCheckpoint`, `compact`, `clear`, with `sequence` / `length` / `checkpointCount`.
- `JsDeltaWindow` — time-bounded aggregation via `tumbling`, `sliding`, and `countBased` constructors, plus `add`, `isComplete`, `emit`, and `clear`.
- Free functions: `init` (panic-hook/tracing setup, runs on start), `version`, and `hasSIMD`.

## Features

- `console_error_panic_hook` (default) — installs a panic hook that surfaces Rust panics as readable JS console errors.
- `simd` — enables SIMD acceleration through `ruvector-delta-core/simd`.
- `parallel` — enables parallel processing via `rayon`.

## License

Licensed under MIT OR Apache-2.0.
