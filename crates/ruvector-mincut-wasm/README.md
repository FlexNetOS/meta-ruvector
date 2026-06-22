# ruvector-mincut-wasm

WebAssembly bindings for [ruvector-mincut](https://crates.io/crates/ruvector-mincut) - a dynamic minimum-cut implementation based on the algorithms in arXiv:2512.13105.

## Features

- **Browser & Node.js**: Works in any JavaScript environment with WASM support
- **Dynamic min-cut**: Incremental insert/delete with fast queries
- **Paper algorithms**: 3-level hierarchy decomposition, deterministic local k-cut, connectivity-curve analysis

## Installation

```bash
npm install ruvector-mincut-wasm
```

## Usage

### `WasmMinCut` — dynamic minimum cut

> **BigInt vertex ids:** vertex ids are 64-bit (`u64`), so the edge-mutation
> methods (`insertEdge`, `deleteEdge`, `updateEdge`) take **BigInt** arguments
> (e.g. `0n`, `1n`). The `fromEdges` / `batchInsert` array forms accept plain
> numbers and convert internally.

```javascript
import init, { WasmMinCut } from 'ruvector-mincut-wasm';

await init();

// Build from edges ([[u, v, weight], ...]) — numbers, converted internally
const mincut = WasmMinCut.fromEdges([[0, 1, 1.0], [1, 2, 1.0], [0, 2, 1.0]]);

// Or start empty and insert (BigInt vertex ids); returns the new min-cut value
const m2 = new WasmMinCut();
m2.insertEdge(0n, 1n, 1.0);
m2.insertEdge(1n, 2n, 2.0);
m2.deleteEdge(0n, 1n);

// minCutValue() is a method
console.log(mincut.minCutValue());

// Partition, cut edges, stats
const { s, t } = mincut.partition();
const edges = mincut.cutEdges();        // [{ u, v, weight }]
console.log(mincut.numVertices(), mincut.numEdges(), mincut.isConnected());
console.log(mincut.stats());            // { num_vertices, num_edges, min_cut_value, is_connected, num_operations }

// Batch helpers (array form, plain numbers)
mincut.batchInsert([[2, 3, 1.0], [3, 0, 1.0]]);
mincut.batchDelete([[2, 3]]);
mincut.updateEdge(0n, 1n, 5.0);
mincut.clear();
```

### Other exports

- **`WasmThreeLevelHierarchy`** — 3-level decomposition (Expander → Precluster → Cluster).
  `insertEdge`, `deleteEdge`, `build()`, `stats()`, `globalMinCut()`, `vertices()`, plus `WasmThreeLevelHierarchy.withPhi(phi)`.
- **`WasmLocalKCut`** — deterministic local k-cut discovery (4-color coding).
  `new WasmLocalKCut(lambdaMax, volumeBound, beta)`, `insertEdge`, `deleteEdge`, `query(source)`.
- **`WasmMinCutWrapper`** — full API with connectivity-curve analysis.
  `insertEdge`, `deleteEdge`, `query()`, `queryWithCertification(source)`,
  `localCuts(source, lambdaMax)`, `connectivityCurve(rankedEdges, kMax)`,
  `findElbow(curve)`, `detectorQuality(rankedEdges, trueCutSize)`.
- **`getVersion()`** — crate version string.

```javascript
import init, { WasmThreeLevelHierarchy, WasmLocalKCut } from 'ruvector-mincut-wasm';

await init();

const hierarchy = new WasmThreeLevelHierarchy();
hierarchy.insertEdge(0n, 1n, 1.0);
hierarchy.insertEdge(1n, 2n, 1.0);
hierarchy.build();
console.log(hierarchy.stats());

const lkcut = new WasmLocalKCut(5n, 100, 2);
lkcut.insertEdge(0n, 1n, 1.0);
const cuts = lkcut.query(0n);
```

## Performance

This implements the data structure from arXiv:2512.13105, which targets:

- **O(1)** worst-case query time for the maintained minimum-cut value
- **O(n^o(1))** (subpolynomial) amortized update time per edge insertion/deletion

(Empirical performance depends on graph structure and configuration.)

## License

MIT

## See Also

- [ruvector-mincut](https://crates.io/crates/ruvector-mincut) - Core Rust implementation
- [ruvector-mincut-node](https://crates.io/crates/ruvector-mincut-node) - Node.js native bindings
