# ruvector-decompiler-wasm

WebAssembly bindings for the RuVector JavaScript bundle decompiler (MinCut + Louvain pipeline).

## Overview

This crate is the WebAssembly (wasm-bindgen) binding layer for the `ruvector-decompiler` capability within the meta-ruvector workspace. It exposes the full Louvain graph-partitioning decompiler pipeline (parse → graph → partition → infer → witness) to Node.js and browser environments, taking a minified JavaScript bundle and returning recovered modules, inferred names, and a witness record as JSON.

## Exports

- `decompile(source, config_json)` — decompile a minified JS bundle; `config_json` is a JSON string of `DecompileConfig` fields (pass `"{}"` for defaults). Returns the `DecompileResult` as a JSON string, or a JSON object with an `"error"` field on failure.
- `version()` — the decompiler WASM module version.
- `init()` — module start hook that installs the panic hook.

## Building

```
wasm-pack build
```

## License

MIT
