# ruvector-sparse-inference-wasm

WebAssembly bindings for PowerInfer-style sparse inference engine.

## Overview

This crate provides WASM bindings for the RuVector sparse inference engine, enabling efficient neural network embedding inference in web browsers and Node.js environments with:

- **Sparse Activation**: PowerInfer-style neuron prediction
- **GGUF Support**: Load quantized models in GGUF format
- **Streaming Loading**: Fetch large models incrementally

The crate exposes two structs — `SparseInferenceEngine` and `EmbeddingModel` —
plus the free functions `measure_inference_time` and `version`.

## Building

### For Web Browsers

```bash
wasm-pack build --target web --release
```

### For Node.js

```bash
wasm-pack build --target nodejs --release
```

### For Bundlers (webpack, rollup, etc.)

```bash
wasm-pack build --target bundler --release
```

## Installation

```bash
npm install ruvector-sparse-inference-wasm
```

Or build locally:

```bash
wasm-pack build --target web
cd pkg && npm link
```

## Usage

### `SparseInferenceEngine`

```typescript
import init, { SparseInferenceEngine } from 'ruvector-sparse-inference-wasm';

// Initialize WASM module
await init();

// Load model
const modelBytes = await fetch('/models/model.gguf').then(r => r.arrayBuffer());
const config = {
  sparsity: {
    enabled: true,
    threshold: 0.1  // 10% neuron activation
  },
  temperature: 1.0,
  top_k: 50
};

const engine = new SparseInferenceEngine(
  new Uint8Array(modelBytes),
  JSON.stringify(config)
);

// Run embedding inference (forward_embedding under the hood)
const input = new Float32Array(4096);  // Your input embedding
const output = engine.infer(input);

console.log('Sparsity stats:', engine.sparsity_stats()); // JSON string
console.log('Model metadata:', engine.metadata());       // JSON string
```

`SparseInferenceEngine` methods:

| Method | Signature | Description |
|--------|-----------|-------------|
| constructor | `new SparseInferenceEngine(modelBytes: Uint8Array, configJson: string)` | Parse a GGUF model + config |
| `SparseInferenceEngine.load_streaming` | `(url: string, configJson: string) => Promise<SparseInferenceEngine>` | Fetch model bytes from a URL, then construct |
| `infer` | `(input: Float32Array) => Float32Array` | Run forward embedding |
| `metadata` | `() => string` | Model metadata as JSON |
| `sparsity_stats` | `() => string` | Sparsity statistics as JSON |
| `calibrate` | `(samples: Float32Array, sampleDim: number) => void` | Calibrate predictors from sample data |

### Streaming Model Loading

For large models, use streaming:

```typescript
const engine = await SparseInferenceEngine.load_streaming(
  'https://example.com/large-model.gguf',
  JSON.stringify(config)
);
```

### `EmbeddingModel`

For sentence transformers and embedding generation:

```typescript
import { EmbeddingModel } from 'ruvector-sparse-inference-wasm';

const modelBytes = await fetch('/models/all-MiniLM-L6-v2.gguf').then(r => r.arrayBuffer());
const embedder = new EmbeddingModel(new Uint8Array(modelBytes));

// Encode single sequence (requires tokenization first)
const inputIds = new Uint32Array([101, 2023, 2003 /* ... */]);  // Tokenized input
const embedding = embedder.encode(inputIds);

console.log('Embedding dimension:', embedder.dimension());

// Batch encoding
const batchIds = new Uint32Array([/* all tokenized sequences */]);
const lengths = new Uint32Array([10, 15, 12]);  // Length of each sequence
const embeddings = embedder.encode_batch(batchIds, lengths);
```

`EmbeddingModel` methods:

| Method | Signature | Description |
|--------|-----------|-------------|
| constructor | `new EmbeddingModel(modelBytes: Uint8Array)` | Wraps a `SparseInferenceEngine` with a default embedding config |
| `encode` | `(inputIds: Uint32Array) => Float32Array` | Encode one token sequence |
| `encode_batch` | `(inputIds: Uint32Array, lengths: Uint32Array) => Float32Array` | Encode multiple sequences (concatenated) |
| `dimension` | `() => number` | Embedding dimension (model `hidden_size`) |

### Calibration

Improve predictor accuracy with sample data:

```typescript
// Collect representative samples (each `sampleDim` long, flattened)
const samples = new Float32Array([
  /* embedding1 (512 dims) */
  /* embedding2 (512 dims) */
  /* embedding3 (512 dims) */
]);

engine.calibrate(samples, 512);  // 512 = dimension of each sample
```

### Performance Measurement

```typescript
import { measure_inference_time } from 'ruvector-sparse-inference-wasm';

const input = new Float32Array(4096);
const avgTime = measure_inference_time(engine, input, 100);  // 100 iterations

console.log(`Average inference time: ${avgTime.toFixed(2)}ms`);
```

### Version

```typescript
import { version } from 'ruvector-sparse-inference-wasm';
console.log(version());
```

## Configuration

The config JSON is deserialized into the core `InferenceConfig`. The shape used
in the examples above:

```typescript
interface InferenceConfig {
  sparsity: {
    enabled: boolean;   // Enable sparse inference
    threshold: number;  // Activation threshold
  };
  temperature: number;  // Sampling temperature
  top_k: number;        // Top-k
}
```

## Browser Compatibility

- Chrome/Edge 91+ (WebAssembly SIMD)
- Firefox 89+
- Safari 15+
- Node.js 16+

For older browsers, build without SIMD:

```bash
wasm-pack build --target web -- --no-default-features
```

## Example: Web Worker Integration

```typescript
// worker.js
import init, { SparseInferenceEngine } from 'ruvector-sparse-inference-wasm';

let engine;

self.onmessage = async (e) => {
  if (e.data.type === 'init') {
    await init();
    engine = new SparseInferenceEngine(e.data.modelBytes, e.data.config);
    self.postMessage({ type: 'ready' });
  } else if (e.data.type === 'infer') {
    const output = engine.infer(e.data.input);
    self.postMessage({ type: 'result', output });
  }
};

// main.js
const worker = new Worker('worker.js', { type: 'module' });

worker.postMessage({
  type: 'init',
  modelBytes: new Uint8Array(modelBytes),
  config: JSON.stringify(config)
});

worker.onmessage = (e) => {
  if (e.data.type === 'ready') {
    worker.postMessage({
      type: 'infer',
      input: new Float32Array(4096)
    });
  } else if (e.data.type === 'result') {
    console.log('Inference result:', e.data.output);
  }
};
```

## Error Handling

```typescript
try {
  const engine = new SparseInferenceEngine(modelBytes, config);
  const output = engine.infer(input);
} catch (error) {
  if (error.message.includes('parse')) {
    console.error('Invalid GGUF model format');
  } else if (error.message.includes('config')) {
    console.error('Invalid configuration');
  } else {
    console.error('Inference failed:', error);
  }
}
```

## Development

### Run Tests

```bash
wasm-pack test --headless --chrome
wasm-pack test --headless --firefox
```

### Build Documentation

```bash
cargo doc --open --target wasm32-unknown-unknown
```

## License

Same as parent RuVector project.

## Related Crates

- `ruvector-sparse-inference` - Core Rust implementation
- `ruvector-core` - Main RuVector library
- `rvlite` - Lightweight WASM vector database

## Contributing

See main RuVector repository for contribution guidelines.
