# exo-node

Node.js bindings for EXO-AI cognitive substrate via NAPI-RS.

[![Crates.io](https://img.shields.io/crates/v/exo-node.svg)](https://crates.io/crates/exo-node)
[![Documentation](https://docs.rs/exo-node/badge.svg)](https://docs.rs/exo-node)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE)

## Overview

`exo-node` provides native Node.js bindings for the EXO classical substrate
(`exo-backend-classical`), exposing vector similarity search backed by the
high-performance ruvector database:

- **NAPI-RS Bindings**: High-performance native module
- **TypeScript Types**: Generated TypeScript definitions
- **Native Performance**: Direct Rust execution

## Installation

```bash
npm install exo-node
```

## Usage

```javascript
const { ExoSubstrateNode } = require('exo-node');

// Create a substrate with a given embedding dimensionality
const substrate = new ExoSubstrateNode(384);
// Or via the factory:
// const substrate = ExoSubstrateNode.withDimensions(384);

// Store a pattern (embedding required; metadata/salience optional)
const id = substrate.store({
  embedding: new Float32Array(384),
  metadata: '{"text":"example"}',
  salience: 1.0,
});

// Search for the k most similar patterns
const results = substrate.search(new Float32Array(384), 10);
console.log(`Dimensions: ${substrate.dimensions()}, results: ${results.length}`);
```

### API

| Export | Description |
|--------|-------------|
| `new ExoSubstrateNode(dimensions)` | Construct a substrate with the given dimensionality |
| `ExoSubstrateNode.withDimensions(n)` | Factory alternative to the constructor |
| `substrate.store(pattern)` | Store a pattern (`{ embedding, metadata?, antecedents?, salience? }`), returns its ID |
| `substrate.search(embedding, k)` | Return the `k` most similar patterns |
| `substrate.dimensions()` | Return the configured embedding dimensionality |
| `substrate.hypergraphQuery(query)` | Topological query — see Planned below |
| `version()` / `hello()` | Library version string / smoke-test greeting |

## Planned / Not Yet Implemented

- **Hypergraph topology queries**: `hypergraphQuery(query)` is present but the
  classical backend does not yet implement it — it currently returns
  `{"NotSupported":null}`.

## Links

- [GitHub](https://github.com/ruvnet/ruvector)
- [Website](https://ruv.io)
- [EXO-AI Documentation](https://github.com/ruvnet/ruvector/tree/main/examples/exo-ai-2025)

## License

MIT OR Apache-2.0
