# ruvector-consciousness-wasm

WebAssembly bindings for `ruvector-consciousness` — IIT Φ, causal emergence, and quantum-inspired collapse.

## Overview

This crate is the WebAssembly (wasm-bindgen) binding layer for the `ruvector-consciousness` capability within the meta-ruvector workspace. It provides JavaScript-friendly APIs for computing Φ (integrated information) over a transition probability matrix, analyzing causal emergence, and running quantum-inspired partition collapse. Multiple Φ algorithms are exposed so callers can trade accuracy against compute budget.

## Exports

- `WasmConsciousness` — the main engine:
  - Budget controls — `setMaxTime`, `setMaxPartitions`.
  - Φ computation — `computePhi` (auto-selects an algorithm), `computePhiExact`, `computePhiSpectral`, `computePhiStochastic`, `computePhiCollapse`, `computePhiGeoMip`.
  - Emergence — `computeEmergence`, `computeRsvdEmergence`, `effectiveInformation`.
- `version()` — crate version string.

Result objects (Φ, emergence, RSVD emergence) are returned as JSON values.

## Building

```
wasm-pack build
```

## License

MIT
