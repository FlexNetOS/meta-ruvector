# ruvector-dag-wasm

Minimal WebAssembly DAG library optimized for browser and embedded systems.

## Overview

This crate is the WebAssembly (wasm-bindgen) binding layer for a compact directed-acyclic-graph capability within the meta-ruvector workspace. It is self-contained — depending only on `wasm-bindgen`, `serde`, `serde_json`, and `bincode` — and is tuned for small binary size (compact integer/float node fields, inlined hot paths, optional `wee_alloc`). It supports building DAGs, topological sorting, critical-path analysis, attention scoring, and (de)serialization.

## Exports

- `WasmDag` — the DAG type, with:
  - `new`, `add_node`, `add_edge` (rejects edges that would create a cycle), `node_count`, `edge_count`.
  - `topo_sort` — Kahn's-algorithm topological order.
  - `critical_path` — longest-path-by-cost, returned as JSON.
  - `attention(mechanism)` — node attention scores (0 = topological, 1 = critical-path, 2 = uniform).
  - `to_bytes` / `from_bytes` (bincode) and `to_json` / `from_json` serialization.

## Building

```
wasm-pack build
```

## Features

- `default` — no extra features.
- `wee_alloc` — use `wee_alloc` as the global allocator for a roughly 10KB smaller WASM binary.

## License

MIT OR Apache-2.0
