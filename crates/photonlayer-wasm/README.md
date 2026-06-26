# photonlayer-wasm

PhotonLayer WASM bindings for browser optics playback and receipt verification (ADR-260 Phase 3).

## Overview

`photonlayer-wasm` exposes the deterministic PhotonLayer optical pipeline
(`photonlayer-core`) to the browser via `wasm-bindgen`, so the five-view studio UI can render
without any server-side inference and so experiment receipts can be verified client-side
(the anti-swap guarantee). Untrusted image dimensions and `OpticalConfig` JSON are validated
before driving any allocation. The crate builds as both `cdylib` (for WASM) and `rlib`, and its
core pipeline is pure Rust so it is unit-tested natively. It is part of the PhotonLayer stack
within the meta-ruvector workspace.

## Key API

WASM-bindgen exports (callable from JavaScript):

- `simulate(image_bytes, w, h, mask_kind, mask_seed, mask_strength, config_json)` — run the five-view pipeline; returns a `WasmTraceResult`.
- `WasmTraceResult` — getters: `width`, `height`, `incoming_buf`, `mask_buf`, `masked_intensity_buf`, `sensor_buf`, `frame_hash` (canvas-ready grayscale buffers plus a BLAKE3 frame digest).
- `verify_receipt_json(json)` — re-derive and check an `ExperimentReceipt` hash; `true` iff untampered.
- `default_config_json(width, height)` — JSON-serialized `OpticalConfig::demo` starting config.
- `photonlayer_version()` — crate version string.

Pure-Rust helpers (also usable natively):

- `run_trace(...)`, `TraceResult` — the core pipeline behind `simulate`.
- `build_mask(width, height, kind, seed, strength)` — construct an `identity`, `random`, or `lens` mask.
- `normalize_to_u8`, `field_amplitude_u8`, `field_intensity_u8`, `phase_to_u8` — view-buffer normalization helpers.

## License

MIT
