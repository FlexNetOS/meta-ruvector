# ruvector-cnn-wasm

WebAssembly bindings for `ruvector-cnn` — image embedding extraction with contrastive-learning losses and SIMD-backed tensor primitives.

## Overview

This crate is the WebAssembly (wasm-bindgen) binding layer for the `ruvector-cnn` capability within the meta-ruvector workspace. It exposes a lightweight image embedder, contrastive-learning loss functions, and a handful of SIMD-backed neural primitives to JavaScript/TypeScript. The embedder builds a feature vector from spatial statistics (per-channel mean and standard deviation plus per-block average luminance) and optionally L2-normalizes it; it does not run a learned CNN.

## Exports

- `EmbedderConfig` — embedder configuration (`input_size`, `embedding_dim`, `normalize`).
- `WasmCnnEmbedder` — `extract` image embeddings, query `embedding_dim`, and compute `cosine_similarity`.
- `WasmInfoNCELoss` — InfoNCE (SimCLR-style) contrastive loss with a temperature parameter.
- `WasmTripletLoss` — triplet metric-learning loss (`forward_single` and batched `forward`) with a margin.
- `SimdOps` — `dot_product`, `relu`, `relu6`, `l2_normalize`.
- `LayerOps` — `batch_norm`, `global_avg_pool`.

## Building

```
wasm-pack build
```

## Features

- `default` — enables `console_error_panic_hook`.
- `console_error_panic_hook` — routes Rust panics to the browser console.
- `simd` — enables SIMD code paths.

## License

MIT OR Apache-2.0
