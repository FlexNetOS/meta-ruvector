# ruvector-graph-transformer-wasm

[![Crates.io](https://img.shields.io/crates/v/ruvector-graph-transformer-wasm.svg)](https://crates.io/crates/ruvector-graph-transformer-wasm)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

**A self-contained WebAssembly graph-transformer implementation — proof-gated graph attention, verified training, and several specialized graph operations running client-side in the browser.**

Run the graph transformer in any browser tab — no server, no API calls, no data leaving the device. Proof-gated operations and verified-training steps produce proof receipts client-side. The transformer logic is **embedded directly in this crate** (`src/transformer.rs`); it does **not** wrap or bind a separate `ruvector-graph-transformer` crate. The WASM binary is size-optimized and loads in milliseconds.

## Install

```bash
# With wasm-pack (recommended)
wasm-pack build crates/ruvector-graph-transformer-wasm --target web

# Or from npm (when published)
npm install ruvector-graph-transformer-wasm
```

## Quick Start

```javascript
import init, { JsGraphTransformer } from "ruvector-graph-transformer-wasm";

await init();
const gt = new JsGraphTransformer();   // constructor takes an optional config value
console.log(gt.version()); // crate version

// Proof-gated mutation
const gate = gt.create_proof_gate(128);
const proof = gt.prove_dimension(128, 128);
console.log(proof.verified); // true

// 82-byte attestation for RVF witness chains
const attestation = gt.create_attestation(proof.proof_id);
console.log(attestation.length); // 82

// Sublinear attention — adjacency list ([[u32, ...], ...]), top-k via PPR
const result = gt.sublinear_attention(
  new Float64Array([0.1, 0.2]),          // query (Float64)
  [[1, 2], [0, 2], [0, 1]],              // adjacency list
  2,                                     // dim
  2                                      // top-k
);
console.log(result.top_k_indices, result.sparsity_ratio);

// Verified training step: (features, targets, weights) -> certificate
const step = gt.verified_training_step(
  [1.0, 2.0], [0.5, 1.0], [0.5, 0.5]
);
console.log(step.weights, step.certificate_id);

// Physics: symplectic integration over graph edges ([{ src, tgt }, ...])
const state = gt.hamiltonian_step([1.0, 0.0], [0.0, 1.0], [{ src: 0, tgt: 1 }]);
console.log(state.energy);

// Biological: full spiking step (2D features + flat adjacency)
const spikes = gt.spiking_step([[0.8, 0.6], [0.1, 0.2]], [0, 0.5, 0.3, 0]);

// Manifold: mixed-curvature distance
const d = gt.product_manifold_distance(
  [1, 0, 0, 1], [0, 1, 1, 0], [0.0, -1.0]
);

// Temporal: causal attention over graph (features, timestamps, edges)
const attn = gt.causal_attention(
  [1.0, 0.5, 0.8],
  [1.0, 2.0, 3.0],
  [{ src: 0, tgt: 1 }, { src: 1, tgt: 2 }]
);

// Economic: Nash equilibrium
const nash = gt.game_theoretic_attention(
  [1.0, 0.5, 0.8],
  [{ src: 0, tgt: 1 }, { src: 1, tgt: 2 }]
);
console.log(nash.converged);

// Stats
console.log(gt.stats());
```

## API

### Proof-Gated Operations

| Method | Returns | Description |
|--------|---------|-------------|
| `new JsGraphTransformer(config?)` | `JsGraphTransformer` | Create transformer instance |
| `version()` | `string` | Crate version |
| `create_proof_gate(dim)` | `object` | Create proof gate for dimension |
| `prove_dimension(expected, actual)` | `object` | Prove dimension equality |
| `create_attestation(proof_id)` | `Uint8Array` | 82-byte proof attestation |
| `verify_attestation(bytes)` | `boolean` | Verify attestation from bytes |
| `compose_proofs(stages)` | `object` | Type-checked pipeline composition |

### Sublinear Attention

| Method | Returns | Description |
|--------|---------|-------------|
| `sublinear_attention(q, edges, dim, k)` | `object` | Graph-sparse top-k attention |
| `ppr_scores(source, adj, alpha)` | `Float64Array` | Personalized PageRank scores |

### Physics-Informed

| Method | Returns | Description |
|--------|---------|-------------|
| `hamiltonian_step(positions, momenta, edges)` | `object` | Symplectic leapfrog step over graph edges (`[{ src, tgt }, ...]`) |
| `verify_energy_conservation(before, after, tol)` | `object` | Energy conservation check |

### Biological

| Method | Returns | Description |
|--------|---------|-------------|
| `hebbian_update(pre, post, weights)` | `Float64Array` | Hebbian weight update |
| `spiking_step(features, adjacency)` | `object` | Full spiking step over feature matrix |

### Verified Training

| Method | Returns | Description |
|--------|---------|-------------|
| `verified_step(weights, gradients, lr)` | `object` | SGD step + proof receipt |
| `verified_training_step(features, targets, weights)` | `object` | Training step + certificate |

### Manifold

| Method | Returns | Description |
|--------|---------|-------------|
| `product_manifold_distance(a, b, curvatures)` | `number` | Mixed-curvature distance |
| `product_manifold_attention(features, edges)` | `object` | Product manifold attention |

### Temporal-Causal

| Method | Returns | Description |
|--------|---------|-------------|
| `causal_attention(features, timestamps, edges)` | `Float64Array` | Causal graph attention (no future leakage) |
| `granger_extract(history, num_nodes, num_steps)` | `object` | Granger causality DAG |

### Economic

| Method | Returns | Description |
|--------|---------|-------------|
| `game_theoretic_attention(features, edges)` | `object` | Nash equilibrium attention |

### Meta

| Method | Returns | Description |
|--------|---------|-------------|
| `stats()` | `object` | Aggregate proof/attestation statistics |
| `reset()` | `void` | Reset all internal state |

## Building

```bash
# Web target (recommended for browsers)
wasm-pack build crates/ruvector-graph-transformer-wasm --target web

# Node.js target
wasm-pack build crates/ruvector-graph-transformer-wasm --target nodejs

# Cargo check
cargo check -p ruvector-graph-transformer-wasm
```

## Bundle Size

The WASM binary is optimized for size with `opt-level = "s"`, LTO, and single codegen unit.

## Related Packages

> This crate is **self-contained**: its graph-transformer logic lives in
> `src/transformer.rs`. It does **not** wrap a separate
> `ruvector-graph-transformer` crate.

| Package | Description |
|---------|-------------|
| [`ruvector-graph-transformer-node`](../ruvector-graph-transformer-node) | Self-contained graph-transformer implementation exposed to Node.js via NAPI-RS |

## License

MIT
