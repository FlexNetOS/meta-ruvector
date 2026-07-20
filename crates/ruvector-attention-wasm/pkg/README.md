# ruvector-attention-wasm

WebAssembly bindings for the [`ruvector-attention`](../ruvector-attention/) crate, providing attention mechanisms for browser and Node.js environments.

## Features

- **Attention Mechanisms** (exported Wasm classes):
  - Multi-Head Attention (`WasmMultiHeadAttention`)
  - Hyperbolic Attention (`WasmHyperbolicAttention`) -- for hierarchical data
  - Linear Attention (`WasmLinearAttention`) -- Performer-style
  - Flash Attention (`WasmFlashAttention`) -- memory-efficient, tiled
  - Local-Global Attention (`WasmLocalGlobalAttention`)
  - Mixture of Experts (MoE) Attention (`WasmMoEAttention`)
  - Scaled Dot-Product Attention (`scaledDotAttention()`, a free function)

- **Training Utilities**:
  - InfoNCE contrastive loss (`WasmInfoNCELoss`)
  - Adam optimizer (`WasmAdam`)
  - AdamW optimizer with decoupled weight decay (`WasmAdamW`)
  - SGD optimizer (`WasmSGD`)
  - Learning rate scheduler with warmup + cosine decay (`WasmLRScheduler`)

- **TypeScript Support**: generated type definitions via `wasm-bindgen`

> **Note:** the wasm-bindgen class names are exported verbatim (e.g.
> `WasmMultiHeadAttention`), while free functions are camelCased
> (e.g. `scaledDotAttention`, `cosineSimilarity`). Constructors take **positional**
> arguments (numbers), not an options object.

## Installation

```bash
npm install ruvector-attention-wasm
```

## Usage

### TypeScript/JavaScript

```typescript
import init, { WasmMultiHeadAttention, cosineSimilarity } from 'ruvector-attention-wasm';

// Initialize the WASM module
await init();

// Create multi-head attention: new WasmMultiHeadAttention(dim, numHeads)
// (dim must be divisible by numHeads)
const attention = new WasmMultiHeadAttention(64, 8);

// Prepare inputs
const query = new Float32Array(64);
const keys = [new Float32Array(64), new Float32Array(64)];
const values = [new Float32Array(64), new Float32Array(64)];

// Compute attention (keys/values are arrays of Float32Array)
const output = attention.compute(query, keys, values);

// Introspection getters
console.log(attention.num_heads, attention.dim);

// Utility
const similarity = cosineSimilarity(query, keys[0]);
```

### Functional Scaled Dot-Product Attention

```typescript
import init, { scaledDotAttention } from 'ruvector-attention-wasm';

await init();

const query = new Float32Array(64);
const keys = [new Float32Array(64), new Float32Array(64)];
const values = [new Float32Array(64), new Float32Array(64)];

// Optional scale argument; defaults to 1/sqrt(dim)
const output = scaledDotAttention(query, keys, values, undefined);
```

### Advanced Examples

#### Hyperbolic Attention

```typescript
import { WasmHyperbolicAttention } from 'ruvector-attention-wasm';

// new WasmHyperbolicAttention(dim, curvature)
const hyperbolic = new WasmHyperbolicAttention(128, 1.0);

const output = hyperbolic.compute(query, keys, values);
console.log('curvature:', hyperbolic.curvature);
```

#### MoE Attention

```typescript
import { WasmMoEAttention } from 'ruvector-attention-wasm';

// new WasmMoEAttention(dim, numExperts, topK)
const moe = new WasmMoEAttention(64, 4, 2);

const output = moe.compute(query, keys, values);
```

#### Flash / Linear / Local-Global Attention

```typescript
import {
  WasmFlashAttention,
  WasmLinearAttention,
  WasmLocalGlobalAttention,
} from 'ruvector-attention-wasm';

const flash = new WasmFlashAttention(64, /* blockSize */ 16);
const linear = new WasmLinearAttention(64, /* numFeatures */ 32);
const localGlobal = new WasmLocalGlobalAttention(64, /* localWindow */ 8, /* globalTokens */ 4);

const a = flash.compute(query, keys, values);
const b = linear.compute(query, keys, values);
const c = localGlobal.compute(query, keys, values);
```

#### Training with InfoNCE Loss

```typescript
import { WasmInfoNCELoss, WasmAdam } from 'ruvector-attention-wasm';

const loss = new WasmInfoNCELoss(0.07);          // temperature
const optimizer = new WasmAdam(paramCount, 0.001); // (paramCount, learningRate)

// Training loop
const lossValue = loss.compute(anchor, positive, negatives);
optimizer.step(params, gradients);
```

#### Learning Rate Scheduling

```typescript
import { WasmLRScheduler, WasmAdamW } from 'ruvector-attention-wasm';

// new WasmLRScheduler(initialLR, warmupSteps, totalSteps)
const scheduler = new WasmLRScheduler(0.001, 1000, 10000);

// new WasmAdamW(paramCount, learningRate, weightDecay)
const optimizer = new WasmAdamW(paramCount, scheduler.get_lr(), 0.01);

for (let step = 0; step < 10000; step++) {
  optimizer.set_learning_rate(scheduler.get_lr());
  optimizer.step(params, gradients);
  scheduler.step();
}
```

## Building from Source

### Prerequisites

- Rust 1.70+
- wasm-pack

### Build Commands

```bash
# Build for web (ES modules)
wasm-pack build --target web --out-dir pkg

# Build for Node.js
wasm-pack build --target nodejs --out-dir pkg-node

# Build for bundlers (webpack, vite, etc.)
wasm-pack build --target bundler --out-dir pkg-bundler

# Run tests
wasm-pack test --headless --firefox
```

## API Reference

### Module-level functions

- `init()` -- wasm-bindgen start hook (installs the panic hook)
- `version()` -- crate version string
- `availableMechanisms()` -- list of available mechanism names
- `scaledDotAttention(query, keys, values, scale?)` -- functional scaled dot-product attention

### Attention Classes

- `WasmMultiHeadAttention(dim, numHeads)` -- getters: `num_heads`, `dim`
- `WasmHyperbolicAttention(dim, curvature)` -- getter: `curvature`
- `WasmLinearAttention(dim, numFeatures)`
- `WasmFlashAttention(dim, blockSize)`
- `WasmLocalGlobalAttention(dim, localWindow, globalTokens)`
- `WasmMoEAttention(dim, numExperts, topK)`

Each class exposes `compute(query, keys, values)` where `query` is a `Float32Array`
and `keys`/`values` are arrays of `Float32Array`.

### Training

- `WasmInfoNCELoss(temperature)` -- `compute(anchor, positive, negatives)`
- `WasmAdam(paramCount, learningRate)` -- `step`, `reset`, `learning_rate`, `set_learning_rate`
- `WasmAdamW(paramCount, learningRate, weightDecay)` -- `step`, `reset`, `learning_rate`, `set_learning_rate`, `weight_decay`
- `WasmSGD(paramCount, learningRate, momentum?)` -- `step`, `reset`, `learning_rate`, `set_learning_rate`
- `WasmLRScheduler(initialLR, warmupSteps, totalSteps)` -- `get_lr`, `step`, `reset`

### Utilities (free functions)

- `cosineSimilarity(a, b)` -- cosine similarity between vectors
- `l2Norm(vec)` -- L2 norm of a vector
- `normalize(vec)` -- normalize a vector to unit length (in place)
- `softmax(vec)` -- apply softmax (in place)
- `attentionWeights(scores, temperature?)` -- compute attention weights from scores (in place)
- `batchNormalize(vectors, epsilon?)` -- batch normalization
- `randomOrthogonalMatrix(dim)` -- generate a random orthogonal matrix
- `pairwiseDistances(vectors)` -- compute pairwise distances
- `log(message)` / `logError(message)` -- console logging helpers

## Planned / Not in This Crate

- **CGT Sheaf Attention** -- the sheaf-Laplacian / coherence-gated attention lives in
  the parent [`ruvector-attention`](../ruvector-attention/) crate behind its `sheaf`
  feature. It is **not** exported by these WASM bindings.
- **GPU acceleration (wgpu)** -- these bindings run on CPU/WASM only. There are no
  GPU/wgpu code paths in this crate.

## Performance

These bindings are built with `opt-level = "s"`, LTO, and a single codegen unit for
a small binary. Performance characteristics depend on the host engine; no specific
latency numbers are claimed.

## License

MIT OR Apache-2.0
