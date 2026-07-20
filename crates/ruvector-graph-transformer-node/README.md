# @ruvector/graph-transformer

[![npm](https://img.shields.io/npm/v/@ruvector/graph-transformer.svg)](https://www.npmjs.com/package/@ruvector/graph-transformer)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Tests](https://img.shields.io/badge/tests-20_passing-brightgreen.svg)]()

**A self-contained graph-transformer implementation exposed to Node.js via NAPI-RS — proof-gated graph attention, verified training, and several specialized graph operations.**

Use graph-transformer operations from JavaScript and TypeScript with native Rust performance. Proof-gated operations and verified training steps produce proof receipts (attestations / certificates). The implementation is **embedded directly in this crate** (`src/transformer.rs`) rather than depending on a separate `ruvector-graph-transformer` crate, so it stands alone with no cross-crate coupling. The heavy computation runs in compiled Rust via NAPI-RS.

## Install

```bash
npm install @ruvector/graph-transformer
```

Prebuilt binaries are provided for:

| Platform | Architecture | Package |
|----------|-------------|---------|
| Linux | x64 (glibc) | `@ruvector/graph-transformer-linux-x64-gnu` |
| Linux | x64 (musl) | `@ruvector/graph-transformer-linux-x64-musl` |
| Linux | ARM64 (glibc) | `@ruvector/graph-transformer-linux-arm64-gnu` |
| macOS | x64 (Intel) | `@ruvector/graph-transformer-darwin-x64` |
| macOS | ARM64 (Apple Silicon) | `@ruvector/graph-transformer-darwin-arm64` |
| Windows | x64 | `@ruvector/graph-transformer-win32-x64-msvc` |

## Quick Start

```javascript
const { GraphTransformer } = require('@ruvector/graph-transformer');

const gt = new GraphTransformer();
console.log(gt.version()); // crate version

// Proof-gated mutation
const gate = gt.createProofGate(128);
console.log(gate.dimension); // 128

// Prove dimension equality
const proof = gt.proveDimension(128, 128);
console.log(proof.verified); // true

// Create attestation (82-byte proof receipt)
const attestation = gt.createAttestation(proof.proof_id);
console.log(attestation.length); // 82
```

## API Reference

### Proof-Gated Operations

```javascript
// Create a proof gate for a dimension
const gate = gt.createProofGate(dim);

// Prove two dimensions are equal
const proof = gt.proveDimension(expected, actual);

// Create 82-byte attestation for embedding in RVF witness chains
const bytes = gt.createAttestation(proofId);

// Verify attestation from bytes
const valid = gt.verifyAttestation(bytes);

// Compose a pipeline of type-checked stages
const composed = gt.composeProofs([
  { name: 'embed', input_type_id: 1, output_type_id: 2 },
  { name: 'align', input_type_id: 2, output_type_id: 3 },
]);
```

### Sublinear Attention

```javascript
// O(n log n) graph attention via PPR sparsification
const result = gt.sublinearAttention(
  [1.0, 0.5, -0.3],     // query vector
  [[1, 2], [0, 2], [0, 1]], // adjacency list
  3,                      // dimension
  2                       // top-k
);
console.log(result.top_k_indices, result.sparsity_ratio);

// Raw PPR scores
const scores = gt.pprScores(0, [[1], [0, 2], [1]], 0.15);
```

### Physics-Informed Layers

```javascript
// Symplectic leapfrog step (energy-conserving)
const state = gt.hamiltonianStep([1.0, 0.0], [0.0, 1.0], 0.01);
console.log(state.energy);

// With graph interactions
const state2 = gt.hamiltonianStepGraph(
  [1.0, 0.0], [0.0, 1.0],
  [{ src: 0, tgt: 1 }], 0.01
);
console.log(state2.energy_conserved); // true
```

### Biological Layers

```javascript
// Spiking neural attention (event-driven)
const output = gt.spikingAttention(
  [0.5, 1.5, 0.3],          // membrane potentials
  [[1], [0, 2], [1]],       // adjacency
  1.0                        // firing threshold
);

// Hebbian weight update (Hebb's rule)
const weights = gt.hebbianUpdate(
  [1.0, 0.0],  // pre-synaptic
  [0.0, 1.0],  // post-synaptic
  [0, 0, 0, 0], // current weights (flattened)
  0.1            // learning rate
);

// Full spiking step over feature matrix
const result = gt.spikingStep(
  [[0.8, 0.6], [0.1, 0.2]],  // n x dim features
  [0, 0.5, 0.3, 0]            // flat adjacency (n x n)
);
```

### Verified Training

```javascript
// Single verified SGD step with proof receipt
const result = gt.verifiedStep(
  [1.0, 2.0],  // weights
  [0.1, 0.2],  // gradients
  0.01          // learning rate
);
console.log(result.proof_id, result.loss_before, result.loss_after);

// Full training step with features and targets
const step = gt.verifiedTrainingStep(
  [1.0, 2.0],   // features
  [0.5, 1.0],   // targets
  [0.5, 0.5]    // weights
);
console.log(step.certificate_id, step.loss);
```

### Manifold Operations

```javascript
// Product manifold distance (mixed curvatures)
const d = gt.productManifoldDistance(
  [1, 0, 0, 1],    // point a
  [0, 1, 1, 0],    // point b
  [0.0, -1.0]      // curvatures (Euclidean, Hyperbolic)
);

// Product manifold attention
const result = gt.productManifoldAttention(
  [1.0, 0.5, -0.3, 0.8],
  [{ src: 0, tgt: 1 }]
);
```

### Temporal-Causal Attention

```javascript
// Causal attention (no future information leakage)
const scores = gt.causalAttention(
  [1.0, 0.0],                        // query
  [[1.0, 0.0], [0.0, 1.0], [0.5, 0.5]], // keys
  [1.0, 2.0, 3.0]                    // timestamps
);

// Causal attention over graph
const output = gt.causalAttentionGraph(
  [1.0, 0.5, 0.8],    // node features
  [1.0, 2.0, 3.0],    // timestamps
  [{ src: 0, tgt: 1 }, { src: 1, tgt: 2 }]
);

// Granger causality extraction
const dag = gt.grangerExtract(flatHistory, 3, 20);
console.log(dag.edges); // [{ source, target, f_statistic, is_causal }]
```

### Economic / Game-Theoretic

```javascript
// Nash equilibrium attention
const result = gt.gameTheoreticAttention(
  [1.0, 0.5, 0.8],  // utility values
  [{ src: 0, tgt: 1 }, { src: 1, tgt: 2 }]
);
console.log(result.allocations, result.nash_gap, result.converged);
```

### Stats & Control

```javascript
// Aggregate statistics
const stats = gt.stats();
// { proofs_constructed, proofs_verified, cache_hits, cache_misses,
//   attention_ops, physics_ops, bio_ops, training_steps }
console.log(stats.proofs_verified, stats.training_steps);

// Reset all internal state
gt.reset();
```

## Building from Source

```bash
# Install NAPI-RS CLI
npm install -g @napi-rs/cli

# Build native module
cd crates/ruvector-graph-transformer-node
napi build --platform --release

# Run tests
cargo test -p ruvector-graph-transformer-node
```

## Related Packages

> This crate is **self-contained**: its graph-transformer logic lives in
> `src/transformer.rs`. It does **not** depend on a separate
> `ruvector-graph-transformer` crate.

| Package | Description |
|---------|-------------|
| [`ruvector-graph-transformer-wasm`](../ruvector-graph-transformer-wasm) | Self-contained WASM graph-transformer implementation for browsers |

## License

MIT
