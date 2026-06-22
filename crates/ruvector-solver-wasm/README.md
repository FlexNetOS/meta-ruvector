# ruvector-solver-wasm

WebAssembly bindings for the RuVector sublinear-time solver. Exposes a `JsSolver`
class that solves sparse linear systems, computes Personalized PageRank, and
estimates solve complexity — in the browser or any WASM runtime. Matrices are
passed as CSR arrays directly from JS typed arrays.

## Build

```bash
wasm-pack build --target web
```

## Quick start (JavaScript)

```js
import { JsSolver } from "ruvector-solver-wasm";

const solver = new JsSolver();

// CSR representation of a 3x3 diagonally-dominant matrix.
const values  = new Float32Array([4, -1, -1, 4, -1, -1, 4]);
const colIdx  = new Uint32Array([0, 1, 0, 1, 2, 1, 2]);
const rowPtrs = new Uint32Array([0, 2, 5, 7]);
const rhs     = new Float32Array([1, 0, 1]);

const result = solver.solve(values, colIdx, rowPtrs, 3, 3, rhs);
console.log(result);
```

## API

`JsSolver` (constructor defaults: `maxIterations=1000`, `tolerance=1e-6`,
`alpha=0.15`):

- `solve(values, colIndices, rowPtrs, rows, cols, rhs)` — solve `Ax = b`
  (square matrix). The algorithm is auto-selected from a sparsity analysis
  (Neumann series for diagonally-dominant, CG otherwise).
- `pagerank(values, colIndices, rowPtrs, rows, source, tolerance)` —
  Personalized PageRank from a single source via power iteration.
- `estimateComplexity(values, colIndices, rowPtrs, rows, cols)` — estimate
  algorithm, FLOPS, iterations, memory, and complexity class without solving.
- `setMaxIterations(n)`, `setTolerance(t)`, `setAlpha(a)` — adjust defaults.
- `version()` (free function) — crate version string.

## Returned types (serde-serialized JS objects)

| Method | Returns |
|--------|---------|
| `solve` | `{ solution, iterations, residual, converged, algorithm, time_us }` |
| `pagerank` | `{ scores, iterations, residual, converged, time_us }` |
| `estimateComplexity` | `{ algorithm, estimated_flops, estimated_iterations, estimated_memory_bytes, complexity_class, density, is_diag_dominant, estimated_spectral_radius }` |

## License

MIT OR Apache-2.0
