# exo-wasm

WASM bindings for the EXO-AI 2025 Cognitive Substrate, enabling browser- and
Node.js-based vector storage and similarity search. The current implementation is
backed by `ruvector-core` (an in-memory `VectorDB`), exposed through the
`ExoSubstrate` and `Pattern` classes.

[![Crates.io](https://img.shields.io/crates/v/exo-wasm.svg)](https://crates.io/crates/exo-wasm)
[![Documentation](https://docs.rs/exo-wasm/badge.svg)](https://docs.rs/exo-wasm)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE)

## Features

- **Pattern Storage**: Store, retrieve, and delete cognitive patterns with
  embeddings (`store`, `get`, `delete`, `len`, `isEmpty`)
- **Similarity Search**: Vector search with multiple distance metrics —
  euclidean, cosine, dotproduct, manhattan (`query`)
- **Optional HNSW index**: Enable via the `use_hnsw` config flag
- **Browser-First**: Compiles to WASM for browser and Node.js targets;
  `detectSIMD()` reports SIMD128 availability

## Planned / Not Yet Implemented

The `Pattern` type carries a `timestamp` and `antecedents` (causal antecedent
IDs), and the substrate config accepts `enable_temporal` / `enable_causal` flags,
but the corresponding query surfaces are **not yet implemented** in this crate:

- **Temporal Memory**: There is no temporal-window query API; timestamps are
  stored but not queryable as a temporal index.
- **Causal Queries**: There is no causal-cone query API; antecedents are stored
  on patterns but cannot yet be queried.

The `enable_temporal` / `enable_causal` flags are accepted and reflected in
`stats()`, but currently have no behavioural effect.

## Installation

```bash
# Build the WASM package
wasm-pack build --target web

# Or for Node.js
wasm-pack build --target nodejs
```

## Usage

### Browser (ES Modules)

```javascript
import init, { ExoSubstrate, Pattern } from './pkg/exo_wasm.js';

async function main() {
  // Initialize WASM module
  await init();

  // Create substrate
  const substrate = new ExoSubstrate({
    dimensions: 384,
    distance_metric: "cosine",
    use_hnsw: true,
    enable_temporal: true,
    enable_causal: true
  });

  // Create a pattern
  const embedding = new Float32Array(384);
  for (let i = 0; i < 384; i++) {
    embedding[i] = Math.random();
  }

  const pattern = new Pattern(
    embedding,
    { type: "concept", name: "example" },
    [] // antecedents
  );

  // Store pattern
  const id = substrate.store(pattern);
  console.log("Stored pattern:", id);

  // Query for similar patterns
  const results = await substrate.query(embedding, 5);
  console.log("Search results:", results);

  // Get stats
  const stats = substrate.stats();
  console.log("Substrate stats:", stats);
}

main();
```

### Node.js

```javascript
const { ExoSubstrate, Pattern } = require('./pkg/exo_wasm.js');

const substrate = new ExoSubstrate({
  dimensions: 128,
  distance_metric: "euclidean",
  use_hnsw: false
});

// Use as shown above
```

## API Reference

### ExoSubstrate

Main substrate interface.

#### Constructor

```javascript
new ExoSubstrate(config)
```

**Config options:**
- `dimensions` (number): Vector dimensions (required)
- `distance_metric` (string): "euclidean", "cosine", "dotproduct", or "manhattan" (default: "cosine")
- `use_hnsw` (boolean): Enable HNSW index (default: true)
- `enable_temporal` (boolean): Accepted and reflected in `stats()`, but no
  temporal query API yet (default: true) — see Planned above
- `enable_causal` (boolean): Accepted and reflected in `stats()`, but no causal
  query API yet (default: true) — see Planned above

#### Methods

- `store(pattern)`: Store a pattern, returns pattern ID
- `query(embedding, k)`: Search for k similar patterns (returns Promise)
- `get(id)`: Retrieve pattern by ID
- `delete(id)`: Delete pattern by ID
- `len()`: Get number of patterns
- `isEmpty()`: Check if substrate is empty
- `stats()`: Get substrate statistics

### Pattern

Represents a cognitive pattern.

#### Constructor

```javascript
new Pattern(embedding, metadata, antecedents)
```

**Parameters:**
- `embedding` (Float32Array): Vector embedding
- `metadata` (object, optional): Arbitrary metadata
- `antecedents` (string[], optional): IDs of causal antecedents

#### Properties

- `id`: Pattern ID (set after storage)
- `embedding`: Vector embedding (Float32Array)
- `metadata`: Pattern metadata
- `timestamp`: Creation timestamp (milliseconds since epoch)
- `antecedents`: Causal antecedent IDs

## Building

### Prerequisites

- Rust 1.75+
- wasm-pack
- Node.js (for testing)

### Build Commands

```bash
# Development build
wasm-pack build --dev

# Production build (optimized)
wasm-pack build --release

# Build for specific target
wasm-pack build --target web      # Browser ES modules
wasm-pack build --target nodejs   # Node.js
wasm-pack build --target bundler  # Webpack/Rollup
```

## Testing

```bash
# Run tests in browser
wasm-pack test --headless --firefox

# Run tests in Node.js
wasm-pack test --node
```

## Performance

The WASM bindings are optimized for browser deployment:

- **Size**: ~2MB gzipped (with SIMD)
- **Initialization**: <50ms on modern browsers
- **Search**: 10k+ queries/second (HNSW enabled)
- **Zero-copy**: Uses transferable objects where possible

## Architecture

This crate provides WASM bindings for the EXO-AI 2025 cognitive substrate. It currently uses `ruvector-core` as the underlying implementation, with plans to integrate with the full EXO substrate layer.

```
exo-wasm/
├── src/
│   ├── lib.rs      # Main WASM bindings
│   ├── types.rs    # Type conversions
│   └── utils.rs    # Utility functions
├── Cargo.toml
└── README.md
```

## Links

- [GitHub](https://github.com/ruvnet/ruvector)
- [Website](https://ruv.io)
- [EXO-AI Documentation](https://github.com/ruvnet/ruvector/tree/main/examples/exo-ai-2025)

## License

MIT OR Apache-2.0
