# ruvector-temporal-tensor-wasm

WebAssembly bindings for temporal tensor compression.

## Overview

This crate is the WebAssembly binding layer for the `ruvector-temporal-tensor` capability within the meta-ruvector workspace. It re-exports the parent crate's handle-based FFI interface so temporal tensor compression can run in WASM (and other FFI host) environments. Rather than wasm-bindgen classes, it exposes a flat C-ABI (`extern "C"`) surface backed by an internal handle store, so callers manage compressor lifecycles by integer handle. The library is built as a `cdylib`.

## Exports

The re-exported `extern "C"` FFI covers:

- Compressor lifecycle — `ttc_create`, `ttc_free`, `ttc_touch`, `ttc_set_access`.
- Frame compression — `ttc_push_frame`, `ttc_flush`.
- Segment decoding — `ttc_decode_segment`.
- Memory management — `ttc_alloc`, `ttc_dealloc`.

## Building

```
wasm-pack build
```

## License

MIT
