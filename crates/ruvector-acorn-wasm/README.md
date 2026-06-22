# @ruvector/acorn-wasm

WebAssembly bindings for [`ruvector-acorn`](../ruvector-acorn) — predicate-agnostic
filtered HNSW (ACORN, Patel et al., SIGMOD 2024). Exposes the `AcornIndex` class
for use in browsers, Cloudflare Workers, Deno, and Bun.

## Build

```bash
wasm-pack build --target web
```

## Usage (JavaScript)

```js
import init, { AcornIndex } from "@ruvector/acorn-wasm";
await init();

const dim = 128;
const n = 5_000;
const vectors = new Float32Array(n * dim); // populate
// gamma=2 → ACORN-γ (best recall at low selectivity); gamma=1 → ACORN-1
const idx = AcornIndex.build(vectors, dim, 2);

const query = new Float32Array(dim); // populate
const evenIds = (id) => id % 2 === 0;
const results = idx.search(query, 10, evenIds);
//  → [{ id, distance }, ...]
```

## API surface

- `AcornIndex.build(vectors: Float32Array, dim, gamma) -> AcornIndex` — `gamma=1`
  is ACORN-1 (M=16); `gamma=2` is ACORN-γ (M·γ=32 edges/node, ~2× memory, holds
  ~96% recall@10 at 1% selectivity).
- `AcornIndex.search(query: Float32Array, k, predicate) -> SearchResult[]` —
  `predicate(id: number)` is called once per node visited and must return truthy
  to admit the candidate. Results are `{ id, distance }` in ascending distance.
- `AcornIndex.dim` (getter) — vector dimensionality.
- `AcornIndex.memoryBytes` (getter) — approximate heap size in bytes.
- `AcornIndex.name` (getter) — variant label, e.g. `"ACORN-γ (γ=2, M=32)"`.
- `version()` — crate version string.

`SearchResult` has readonly `id: u32` and `distance: f32` (approximate L2²),
mirroring `@ruvector/rabitq-wasm`.

## License

MIT OR Apache-2.0
