# ruvector-rabitq-wasm

WebAssembly bindings for `ruvector-rabitq` — a 1-bit quantized vector index for browsers and edge runtimes.

## Overview

This crate is the WebAssembly (wasm-bindgen) binding layer for the `ruvector-rabitq` capability within the meta-ruvector workspace. It exposes the RaBitQ 1-bit quantized approximate-nearest-neighbor index as a JavaScript-friendly class for browsers and edge runtimes (Cloudflare Workers, Deno, Bun). It is single-threaded — the parallel build path falls back to deterministic sequential iteration on wasm32, producing bit-identical codes.

## Exports

- `RabitqIndex` — the 1-bit quantized index:
  - `build(vectors, dim, seed, rerank_factor)` — build from a flat `Float32Array` of length `n * dim`; the same `(seed, dim, vectors)` triple is deterministic.
  - `search(query, k)` — k-nearest-neighbor search, returning `SearchResult` hits in ascending distance.
  - `len` / `isEmpty` getters.
- `SearchResult` — a single hit: `id` (vector id) and `distance` (approximate L2² after rerank).
- `version()` — crate version string.

## Building

```
wasm-pack build
```

## Features

- `default` — enables `console_error_panic_hook`.
- `console_error_panic_hook` — routes Rust panics to the browser console.

## License

MIT OR Apache-2.0
