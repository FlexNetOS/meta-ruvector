# ruvector-domain-expansion-wasm

WebAssembly bindings for the domain expansion cross-domain transfer learning engine.

## Overview

This crate is the WebAssembly (wasm-bindgen) binding layer for the `ruvector-domain-expansion` capability within the meta-ruvector workspace. It exposes the Domain Expansion Engine — cross-domain transfer learning, Meta Thompson Sampling, PolicyKernel population search, and an acceleration scoreboard — to JavaScript/TypeScript. With the default `rvf` feature it also provides RVF segment serialization for packaging transfer priors, policy kernels, and cost curves into the RuVector Format wire protocol.

## Exports

- `WasmDomainExpansionEngine` — task generation, `evaluateAndRecord`, `selectArm`, `shouldSpeculate`, transfer (`initiateTransfer` / `verifyTransfer`), population evolution, and scoreboard/kernel/counterexample inspection.
- `WasmThompsonEngine` — standalone Meta Thompson Sampling: `initDomain`, `recordOutcome`, `selectArm`, `extractPrior`.
- `WasmPopulationSearch` — population-based policy search: `evolve`, `generation`, `populationSize`, `stats`.
- `WasmScoreboard` — acceleration scoreboard: `addCurve`, `computeAcceleration`, `progressiveAcceleration`, `summary`.
- `WasmRvfBridge` (feature `rvf`) — RVF segment serialization, witness hashing, segment assembly, and solver-prior extraction.

## Building

```
wasm-pack build
```

## Features

- `default` — enables `rvf`.
- `rvf` — RVF segment serialization and witness chains (forwards to `ruvector-domain-expansion/rvf`).

## License

MIT OR Apache-2.0
