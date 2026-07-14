# neural-trader-wasm

WASM bindings for the Neural Trader crates — market events, coherence gates, and replay memory (ADR-085 / ADR-086).

## Overview

This crate exposes the neural-trader stack to JavaScript/WASM via `wasm-bindgen`. It wraps the
core market-event types, the coherence gate from `neural-trader-coherence`, and the replay memory
from `neural-trader-replay` in JS-friendly bindings, including BigInt-safe JSON serialization for
`u64` fields and hex helpers for 16-byte hash fields. The crate builds as both `cdylib` (for
`wasm-pack`) and `rlib` (so the bindings can be exercised in native tests).

## Key API

- `init()` — `#[wasm_bindgen(start)]` entry point; installs the panic hook when enabled.
- `version()`, `healthCheck()` — crate version and load smoke-test.
- `MarketEventWasm` — constructor, typed getters/setters, and `toJson` / `fromJson` round-trip.
- `GraphDeltaWasm` — accessors for added nodes/edges and updated properties.
- `GateConfigWasm`, `GateContextWasm`, `ThresholdGateWasm` — configure and run the coherence gate; `evaluate()` returns a `CoherenceDecisionWasm`.
- `CoherenceDecisionWasm` — gate flags (`allowRetrieve` / `allowWrite` / `allowLearn` / `allowAct`), `reasons`, `allAllowed()`, `fullyBlocked()`, `toJson`.
- `ReplaySegmentWasm`, `ReservoirStoreWasm` — replay segment access plus gated `maybeWrite` and `retrieveBySymbol`.
- WASM enums: `EventTypeWasm`, `SideWasm`, `RegimeLabelWasm`, `SegmentKindWasm`, `NodeKindWasm`, `EdgeKindWasm`, `PropertyKeyWasm`.

## Features

| Feature | Default | Description |
|---------|---------|-------------|
| `console_error_panic_hook` | yes | Routes Rust panics to the browser console for debugging. |

## License

MIT

## Disclaimer

Research and experimental software. Not financial advice and not a recommendation to trade.
