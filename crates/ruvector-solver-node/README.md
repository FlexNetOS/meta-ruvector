# @ruvector/solver (ruvector-solver-node)

Node.js NAPI bindings for the RuVector sublinear-time solver. Provides
high-performance sparse linear system solving, PageRank, and complexity
estimation. All heavy computation runs on worker threads via
`tokio::task::spawn_blocking` so the Node.js event loop is never blocked.

## Usage

```javascript
const { NapiSolver } = require('@ruvector/solver');

const solver = new NapiSolver();
const result = await solver.solve({
  values: [4, -1, -1, 4, -1, -1, 4], // CSR non-zeros
  colIndices: [0, 1, 0, 1, 2, 1, 2],
  rowPtrs: [0, 2, 5, 7],             // length = rows + 1
  rows: 3, cols: 3,
  rhs: [1, 0, 1],
  // tolerance: 1e-6 (default), maxIterations: 1000 (default)
  // algorithm: 'jacobi' (default) | 'neumann' | 'gauss-seidel' | 'conjugate-gradient'
});
console.log('Solution:', result.solution, 'Converged:', result.converged);
```

## API

`NapiSolver` methods:

- `solve(config: SolveConfig) -> Promise<SolveResult>` — solve `Ax = b` (CSR
  matrix). `SolveResult`: `{ solution, iterations, residual, converged, algorithm, timeUs }`.
- `solveWithHistory(config) -> Promise<SolveWithHistoryResult>` — same as `solve`
  plus per-iteration `convergenceHistory: { iteration, residual }[]`.
- `solveJson(json: string) -> Promise<string>` — JSON-in/JSON-out form of `solve`.
- `pagerank(config: PageRankConfig) -> Promise<PageRankResult>` — power-iteration
  PageRank (`damping` default 0.85). `PageRankResult`: `{ scores, iterations, residual, converged, timeUs }`.
- `estimateComplexity(config: ComplexityConfig) -> ComplexityResult` — synchronous
  O(1) estimate: `{ complexityClass, estimatedFlops, recommendedAlgorithm, estimatedTimeUs, sparsity }`.

Free functions: `version()`, `info() -> LibraryInfo`, `availableAlgorithms() -> string[]`.

Algorithms: `neumann`, `jacobi`, `gauss-seidel`, `conjugate-gradient` (plus
`forward-push` / `backward-push`, which fall back to Jacobi for general solves).

## License

MIT OR Apache-2.0
