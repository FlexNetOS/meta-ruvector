# ruvector-mincut-node

Node.js native bindings for [ruvector-mincut](https://crates.io/crates/ruvector-mincut) - a dynamic minimum-cut implementation based on the algorithms in arXiv:2512.13105.

## Features

- **Native Performance**: Built with NAPI-RS
- **Dynamic min-cut**: Incremental insert/delete with fast queries
- **Paper algorithms**: 3-level hierarchy decomposition, deterministic local k-cut, connectivity-curve analysis
- **Type Definitions**: TypeScript support

## Installation

```bash
npm install ruvector-mincut-node
```

## Usage

### `MinCut` — dynamic minimum cut

```javascript
const { MinCut } = require('ruvector-mincut-node');

// Empty structure (optional config), or build from edges
const mincut = MinCut.fromEdges([
  [0, 1, 1.0],
  [1, 2, 1.0],
  [0, 2, 1.0],
]);

// Incremental updates return the new min-cut value
mincut.insertEdge(2, 3, 1.0);
mincut.deleteEdge(0, 2);

// minCutValue is a getter (property, not a method call)
console.log(mincut.minCutValue);

// Detailed result, partition, and cut edges
const result = mincut.minCut();           // { value, isExact, approximationRatio }
const { s, t } = mincut.partition();
const edges = mincut.cutEdges();          // [{ id, source, target, weight }]

console.log(mincut.numVertices, mincut.numEdges, mincut.isConnected());
console.log(mincut.stats);                // getter: { insertions, deletions, queries, avgUpdateTimeUs }
```

### Other exports

- **`ThreeLevelHierarchy`** — 3-level decomposition (Expander → Precluster → Cluster).
  `insertEdge`, `deleteEdge`, `build()`, `stats` (getter), `globalMinCut` (getter), `vertices()`.
- **`LocalKCut`** — deterministic local k-cut discovery (4-color coding).
  `new LocalKCut(lambdaMax, volumeBound, beta)`, `insertEdge`, `deleteEdge`, `query(source)`.
- **`MinCutWrapperNode`** — full API with connectivity-curve analysis.
  `insertEdge`, `deleteEdge`, `query()`, `localCuts(source, lambdaMax)`,
  `connectivityCurve(rankedEdges, kMax)`, `findElbow(curve)`, `detectorQuality(rankedEdges, trueCutSize)`.

## Performance

This implements the data structure from arXiv:2512.13105, which targets:

- **O(1)** worst-case query time for the maintained minimum-cut value
- **O(n^o(1))** (subpolynomial) amortized update time per edge insertion/deletion

(Empirical performance depends on graph structure and configuration.)

## Supported Platforms

- Linux x64 (glibc/musl)
- macOS x64/ARM64
- Windows x64

## License

MIT

## See Also

- [ruvector-mincut](https://crates.io/crates/ruvector-mincut) - Core Rust implementation
- [ruvector-mincut-wasm](https://crates.io/crates/ruvector-mincut-wasm) - WebAssembly bindings
