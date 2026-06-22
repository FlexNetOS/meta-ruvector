# Ruvector Graph Node

[![npm](https://img.shields.io/npm/v/@ruvector/graph.svg)](https://www.npmjs.com/package/@ruvector/graph)
[![Crates.io](https://img.shields.io/crates/v/ruvector-graph-node.svg)](https://crates.io/crates/ruvector-graph-node)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)

**Node.js bindings for RuVector Graph Database via NAPI-RS.**

`ruvector-graph-node` provides native Node.js bindings for the Ruvector graph database, exposing a single `GraphDatabase` class that combines a hypergraph index, a causal-memory index, and a property graph with Cypher-like query parsing — directly from JavaScript/TypeScript. Part of the [Ruvector](https://github.com/ruvnet/ruvector) ecosystem.

## Why Ruvector Graph Node?

- **Native Performance**: Rust speed in Node.js via NAPI-RS
- **Async/Await**: Most operations are async (run on a blocking thread pool)
- **Hypergraph + Property Graph**: Nodes, edges, and multi-node hyperedges in one store
- **Cypher-like Queries**: A built-in Cypher parser drives `MATCH ... RETURN` lookups
- **Optional Persistence**: Back the database with on-disk storage

## Features

### Core Capabilities

- **Graph CRUD**: Create nodes, edges, and hyperedges; delete nodes/edges/hyperedges
- **Cypher-like Queries**: `query()` / `querySync()` over a built-in parser (label-based `MATCH`)
- **Hyperedge Vector Search**: `searchHyperedges()` finds similar hyperedges by embedding
- **k-Hop Traversal**: `kHopNeighbors()` returns nodes reachable within k hops
- **Batch Operations**: `batchInsert()` for bulk nodes and edges
- **Transactions**: `begin()` / `commit()` / `rollback()` (transaction manager)
- **Statistics**: `stats()`

> **Note on traversal:** this binding exposes k-hop neighborhood expansion
> (`kHopNeighbors`) and hyperedge similarity search. General BFS/DFS and
> shortest-path algorithms are **not** implemented — see [Planned](#planned--not-yet-implemented).

## Installation

```bash
npm install @ruvector/graph
# or
yarn add @ruvector/graph
# or
pnpm add @ruvector/graph
```

## Quick Start

### Create a database

```javascript
const { GraphDatabase } = require('@ruvector/graph');

// In-memory
const db = new GraphDatabase({ distanceMetric: 'Cosine', dimensions: 384 });

// Or persisted to disk
const persisted = GraphDatabase.open('./my-graph.db');
console.log(persisted.isPersistent());      // true
console.log(persisted.getStoragePath());    // './my-graph.db'
```

### Create nodes, edges, and hyperedges

```javascript
// Create nodes (id is required; embedding + labels + properties optional)
const aliceId = await db.createNode({
  id: 'alice',
  embedding: new Float32Array([0.1, 0.2, 0.3]),
  labels: ['Person'],
  properties: { name: 'Alice', age: 30 },
});

const bobId = await db.createNode({
  id: 'bob',
  embedding: new Float32Array([0.2, 0.1, 0.4]),
  labels: ['Person'],
});

// Create an edge (a 2-node hyperedge under the hood)
const edgeId = await db.createEdge({
  from: 'alice',
  to: 'bob',
  description: 'knows',
  embedding: new Float32Array([0.5, 0.5, 0.5]),
  confidence: 0.95,
});

// Create a hyperedge connecting multiple nodes
const hyperedgeId = await db.createHyperedge({
  nodes: ['alice', 'bob'],
  description: 'collaborated_on_project',
  embedding: new Float32Array([0.3, 0.6, 0.9]),
  confidence: 0.85,
});
```

### Cypher-like queries

```javascript
// Label-based MATCH ... RETURN over the property graph
const result = await db.query('MATCH (p:Person) RETURN p LIMIT 10');
console.log(result.nodes);  // matched JsNodeResult[]
console.log(result.stats);  // { totalNodes, totalEdges, avgDegree }

// Synchronous stats-only variant
const sync = db.querySync('MATCH (n) RETURN n');
```

### Hyperedge similarity search

```javascript
const matches = await db.searchHyperedges({
  embedding: new Float32Array([0.5, 0.5, 0.5]),
  k: 10,
});
matches.forEach(m => console.log(m.id, m.score));
```

### k-hop traversal

```javascript
const neighbors = await db.kHopNeighbors('alice', 2);
console.log(neighbors); // string[] of node ids within 2 hops
```

### Transactions

```javascript
const txId = await db.begin();
// ... mutations ...
await db.commit(txId);   // or await db.rollback(txId);
```

### Deletes and batch insert

```javascript
await db.batchInsert({
  nodes: [{ id: 'n1', embedding: new Float32Array([1, 2, 3]) }],
  edges: [{ from: 'n1', to: 'alice', description: 'mentions',
            embedding: new Float32Array([0, 0, 0]) }],
});

const del = await db.deleteNode('n1', { cascade: true });
console.log(del.deletedNode, del.deletedEdges);

await db.deleteEdge(edgeId);
await db.deleteHyperedge(hyperedgeId);
```

## API Reference

### `GraphDatabase`

```typescript
class GraphDatabase {
  constructor(options?: JsGraphOptions);
  static open(path: string): GraphDatabase;

  isPersistent(): boolean;
  getStoragePath(): string | null;

  // Mutations (async)
  createNode(node: JsNode): Promise<string>;
  createEdge(edge: JsEdge): Promise<string>;
  createHyperedge(hyperedge: JsHyperedge): Promise<string>;
  batchInsert(batch: JsBatchInsert): Promise<JsBatchResult>;
  deleteNode(id: string, opts?: JsDeleteNodeOptions): Promise<JsDeleteNodeResult>;
  deleteEdge(id: string): Promise<JsDeleteResult>;
  deleteHyperedge(id: string): Promise<JsDeleteResult>;

  // Query & search
  query(cypher: string): Promise<JsQueryResult>;
  querySync(cypher: string): JsQueryResult;
  searchHyperedges(query: JsHyperedgeQuery): Promise<JsHyperedgeResult[]>;
  kHopNeighbors(startNode: string, k: number): Promise<string[]>;

  // Transactions
  begin(): Promise<string>;
  commit(txId: string): Promise<void>;
  rollback(txId: string): Promise<void>;

  // Misc
  subscribe(callback: (change: unknown) => void): void; // placeholder, no-op
  stats(): Promise<JsGraphStats>;
}

// Free functions
function version(): string;  // crate version, e.g. "2.2.3"
function hello(): string;
```

### Types

```typescript
interface JsGraphOptions {
  distanceMetric?: 'Cosine' | 'Euclidean' | 'DotProduct';
  dimensions?: number;
  storagePath?: string;
}

interface JsNode {
  id: string;
  embedding: Float32Array;
  labels?: string[];
  properties?: Record<string, unknown>;
}

interface JsEdge {
  from: string;
  to: string;
  description: string;
  embedding: Float32Array;
  confidence?: number;
}

interface JsHyperedge {
  nodes: string[];
  description: string;
  embedding: Float32Array;
  confidence?: number;
}

interface JsHyperedgeQuery { embedding: Float32Array; k: number; }
interface JsGraphStats { totalNodes: number; totalEdges: number; avgDegree: number; }
```

## Planned / Not Yet Implemented

These are **not** part of the current API:

- General **BFS / DFS** traversal and **shortest-path** algorithms (only `kHopNeighbors` exists)
- Full ACID transaction durability (a transaction manager exists; semantics are best-effort)
- Change subscriptions — `subscribe()` is currently a no-op placeholder
- General property/vector similarity search over *nodes* (search is over *hyperedges*)

## Building from Source

```bash
git clone https://github.com/ruvnet/ruvector.git
cd ruvector/crates/ruvector-graph-node
npm install
npm run build
npm test
```

## Platform Support

| Platform | Architecture | Status |
|----------|-------------|--------|
| Linux | x64 | ✅ |
| Linux | arm64 | ✅ |
| macOS | x64 | ✅ |
| macOS | arm64 (M1/M2) | ✅ |
| Windows | x64 | ✅ |

## Related Packages

- **[ruvector-graph](../ruvector-graph/)** - Core graph database engine
- **[@ruvector/core](https://www.npmjs.com/package/@ruvector/core)** - Core vector bindings

## Documentation

- **[API Documentation](https://docs.rs/ruvector-graph-node)** - Full API reference
- **[GitHub Repository](https://github.com/ruvnet/ruvector)** - Source code

## License

**MIT License** - see [LICENSE](../../LICENSE) for details.

---

<div align="center">

**Part of [Ruvector](https://github.com/ruvnet/ruvector) - Built by [rUv](https://ruv.io)**

[![Star on GitHub](https://img.shields.io/github/stars/ruvnet/ruvector?style=social)](https://github.com/ruvnet/ruvector)

[Documentation](https://docs.rs/ruvector-graph-node) | [npm](https://www.npmjs.com/package/@ruvector/graph) | [GitHub](https://github.com/ruvnet/ruvector)

</div>
