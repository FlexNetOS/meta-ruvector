# ruvector-fpga-transformer-wasm

WebAssembly bindings for the FPGA Transformer backend.

## Overview

This crate is the WebAssembly (wasm-bindgen) binding layer for the `ruvector-fpga-transformer` capability within the meta-ruvector workspace. It re-exports the engine's WASM FFI so that browser and Node.js environments can run transformer inference with the same API as native Rust — loading model artifacts and running token inference that returns top-k predictions and a witness record.

## Exports

- `WasmEngine` — the inference engine (re-exported from `ruvector_fpga_transformer::ffi::wasm_bindgen`), with artifact loading and `infer`.
- `microShape` — micro-shape helper.
- `validateArtifact` — validate a model artifact.
- `version()` — the underlying crate version.
- `isReady()` — readiness check.
- `init()` — module start hook.

## Building

```
wasm-pack build
```

## License

MIT OR Apache-2.0
