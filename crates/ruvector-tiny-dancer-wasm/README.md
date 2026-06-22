# Ruvector Tiny Dancer WASM

[![npm](https://img.shields.io/npm/v/@ruvector/tiny-dancer.svg)](https://www.npmjs.com/package/@ruvector/tiny-dancer)
[![Crates.io](https://img.shields.io/crates/v/ruvector-tiny-dancer-wasm.svg)](https://crates.io/crates/ruvector-tiny-dancer-wasm)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)

**WebAssembly bindings for Tiny Dancer neural routing.**

`ruvector-tiny-dancer-wasm` brings embedding-based AI agent/model routing to the browser with WebAssembly. It runs FastGRNN neural inference to score candidate embeddings directly in client-side applications. Part of the [Ruvector](https://github.com/ruvnet/ruvector) ecosystem.

## Why Tiny Dancer WASM?

- **Browser Native**: Run neural routing in any browser
- **Offline Capable**: No server required for inference
- **Privacy First**: Routing decisions stay client-side

## What It Does

Tiny Dancer is an **embedding router**. You provide a query embedding and a list of
candidate embeddings, and `route()` returns routing decisions (which candidate, with
what confidence, and whether to use the lightweight model).

> **Honesty note:** this binding does **not** take raw text. There is no built-in
> tokenizer or feature-extraction-from-strings. You bring your own embeddings as
> `Float32Array`s. The router scores embeddings; it does not embed text for you.

## Installation

```bash
npm install @ruvector/tiny-dancer-wasm
# or
yarn add @ruvector/tiny-dancer-wasm
```

## Quick Start

### Basic Usage

```typescript
import init, { Router, RouterConfig, Candidate, RoutingRequest } from '@ruvector/tiny-dancer-wasm';

// Initialize the WASM module
await init();

// Build configuration (defaults are filled in by the constructor;
// override fields via property setters)
const config = new RouterConfig();
config.model_path = './models/fastgrnn.safetensors';
config.confidence_threshold = 0.85;
config.max_uncertainty = 0.15;

// Create the router
const router = new Router(config);

// Build candidates:
// new Candidate(id, embedding, metadataJson, createdAt, accessCount, successRate)
const candidates = [
  new Candidate('1', new Float32Array([/* ... */]), '{}', 0, 0, 0.0),
  new Candidate('2', new Float32Array([/* ... */]), '{}', 0, 0, 0.0),
];

// Build the request: new RoutingRequest(queryEmbedding, candidates)
const request = new RoutingRequest(new Float32Array([0.1, 0.2 /* ... */]), candidates);

// Route (synchronous)
const response = router.route(request);

// decisions are returned as a JSON string
const decisions = JSON.parse(response.decisions_json);
console.log(`Route to: ${decisions[0].candidate_id} (confidence: ${decisions[0].confidence})`);
console.log(`Inference time: ${response.inference_time_us} µs`);
console.log(`Candidates processed: ${response.candidates_processed}`);
```

### Circuit Breaker Status

```typescript
// true = circuit closed (healthy); false = open; null/undefined = disabled
const isHealthy = router.circuit_breaker_status();
```

### Request Metadata

```typescript
const request = new RoutingRequest(queryEmbedding, candidates);
request.metadata = '{"sessionId":"abc"}'; // optional JSON string
```

## API Reference

### `Router`

```typescript
class Router {
  constructor(config: RouterConfig);
  route(request: RoutingRequest): RoutingResponse;
  circuit_breaker_status(): boolean | undefined;
}
```

### `RouterConfig`

```typescript
class RouterConfig {
  constructor(); // sets defaults (modelPath = ./models/fastgrnn.safetensors, etc.)
  set model_path(path: string);
  set confidence_threshold(threshold: number);
  set max_uncertainty(uncertainty: number);
}
```

> Defaults: `confidence_threshold = 0.85`, `max_uncertainty = 0.15`,
> circuit breaker enabled (threshold 5), quantization enabled.

### `Candidate`

```typescript
class Candidate {
  // positional constructor
  constructor(
    id: string,
    embedding: Float32Array,
    metadata: string,     // JSON string
    createdAt: bigint,    // i64
    accessCount: bigint,  // u64
    successRate: number,
  );
}
```

### `RoutingRequest`

```typescript
class RoutingRequest {
  constructor(queryEmbedding: Float32Array, candidates: Candidate[]);
  set metadata(metadata: string); // optional JSON string
}
```

### `RoutingResponse`

```typescript
class RoutingResponse {
  get decisions_json(): string;     // JSON-encoded array of decisions
  get inference_time_us(): bigint;
  get candidates_processed(): number;
  get feature_time_us(): bigint;
}
```

### Module-level

- `init()` -- wasm-bindgen start hook (installs the panic hook)
- `version()` -- crate version string

## CDN Usage

```html
<script type="module">
  import init, { Router, RouterConfig } from 'https://unpkg.com/@ruvector/tiny-dancer-wasm';

  await init();
  const router = new Router(new RouterConfig());
</script>
```

## Building from Source

```bash
git clone https://github.com/ruvnet/ruvector.git
cd ruvector/crates/ruvector-tiny-dancer-wasm

# Build for web (ES modules)
wasm-pack build --target web --out-dir pkg
```

## Browser Support

| Browser | Version |
|---------|---------|
| Chrome | 89+ |
| Firefox | 89+ |
| Safari | 15+ |
| Edge | 89+ |

## Related Packages

- **[ruvector-tiny-dancer-core](../ruvector-tiny-dancer-core/)** - Core Rust implementation
- **[ruvector-tiny-dancer-node](../ruvector-tiny-dancer-node/)** - Node.js bindings
- **[ruvector-core](../ruvector-core/)** - Core vector database

## License

**MIT License** - see [LICENSE](../../LICENSE) for details.

---

<div align="center">

**Part of [Ruvector](https://github.com/ruvnet/ruvector) - Built by [rUv](https://ruv.io)**

[Documentation](https://docs.rs/ruvector-tiny-dancer-wasm) | [npm](https://www.npmjs.com/package/@ruvector/tiny-dancer) | [GitHub](https://github.com/ruvnet/ruvector)

</div>
