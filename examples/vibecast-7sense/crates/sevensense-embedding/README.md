# sevensense-embedding

[![Crate](https://img.shields.io/badge/crates.io-sevensense--embedding-orange.svg)](https://crates.io/crates/sevensense-embedding)
[![Docs](https://img.shields.io/badge/docs-sevensense--embedding-blue.svg)](https://docs.rs/sevensense-embedding)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](../../LICENSE)

> Neural embedding generation using Perch 2.0 for bioacoustic analysis.

**sevensense-embedding** transforms mel spectrograms into 1536-dimensional embedding
vectors using a Perch 2.0 ONNX model via ONNX Runtime. These embeddings capture the
acoustic essence of bird vocalizations, enabling similarity search, clustering, and
species identification.

The crate follows Domain-Driven Design (DDD):

- **Domain Layer**: Core entities (`Embedding`, `EmbeddingModel`, `EmbeddingMetadata`, `StorageTier`) and the `EmbeddingRepository` trait
- **Application Layer**: `EmbeddingService` for single and batch inference
- **Infrastructure Layer**: `ModelManager` (session caching + hot-swap) and `OnnxInference`
- **Quantization / Normalization**: free functions for F16/INT8 quantization and L2 normalization

## Features

- **Perch 2.0 Integration**: 1536-dimensional bird audio embeddings
- **ONNX Runtime**: Cross-platform inference (CPU, CUDA, CoreML, DirectML execution providers)
- **Batch Processing**: Efficient multi-segment inference with a configurable batch size
- **Model Hot-Swap**: `ModelManager` caches sessions and supports version switching
- **F16 & INT8 Quantization**: 50% / 75% storage reduction via the `quantization` module
- **L2 Normalization**: Optimized for cosine similarity search

## Use Cases

| Use Case | Description | Key API |
|----------|-------------|---------|
| Single Inference | Embed one spectrogram | `EmbeddingService::embed_segment()` |
| Batch Processing | Embed multiple spectrograms | `EmbeddingService::embed_batch()` |
| F16 Quantization | Halve storage footprint | `quantize_to_f16()` / `dequantize_f16()` |
| INT8 Quantization | Quarter storage footprint | `quantize_to_i8_full()` / `QuantizedEmbedding::dequantize()` |

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
sevensense-embedding = "0.1"
```

### ONNX Model Setup

The Perch 2.0 ONNX model is loaded from a local directory configured via
`ModelConfig::model_dir` (default: `models/`). Place the model files in that
directory before creating a `ModelManager`. The crate does **not** download models
automatically.

## Quick Start

`EmbeddingService::embed_segment` takes a `Spectrogram` (a `[1, 500, 128]` mel
spectrogram wrapper defined in this crate) and returns an `EmbeddingOutput`.

```rust,ignore
use std::sync::Arc;
use sevensense_embedding::{EmbeddingService, ModelManager, ModelConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize the model manager from a model directory
    let config = ModelConfig::default(); // model_dir = "models"
    let model_manager = Arc::new(ModelManager::new(config)?);

    // Create the embedding service with a batch size of 8
    let service = EmbeddingService::new(model_manager, 8);

    // Embed a spectrogram
    let output = service.embed_segment(&spectrogram).await?;

    Ok(())
}
```

`ModelManager::new` takes a `ModelConfig` and returns `Result<ModelManager, ModelError>`.
`EmbeddingService::new` takes an `Arc<ModelManager>` and a batch size; a
builder (`EmbeddingServiceBuilder`) and `EmbeddingService::with_config` are also available.

---

<details>
<summary><b>Tutorial: Basic Embedding Generation</b></summary>

```rust,ignore
use std::sync::Arc;
use sevensense_embedding::{EmbeddingService, ModelManager, ModelConfig};

# async fn example(spectrogram: &sevensense_embedding::application::services::Spectrogram)
#   -> Result<(), Box<dyn std::error::Error>> {
let model_manager = Arc::new(ModelManager::new(ModelConfig::default())?);
let service = EmbeddingService::new(model_manager, 8);

// Single-segment inference
let output = service.embed_segment(spectrogram).await?;

// The model produces 1536-dimensional vectors (EMBEDDING_DIM)
assert_eq!(sevensense_embedding::EMBEDDING_DIM, 1536);

// Check that the model is loaded and report its version
if service.is_ready().await {
    println!("Model version: {}", service.model_version());
}
# Ok(())
# }
```

</details>

<details>
<summary><b>Tutorial: Batch Processing</b></summary>

```rust,ignore
use std::sync::Arc;
use sevensense_embedding::{EmbeddingService, ModelManager, ModelConfig};

# async fn example(spectrograms: &[sevensense_embedding::application::services::Spectrogram])
#   -> Result<(), Box<dyn std::error::Error>> {
// Configure batch size via the service constructor
let model_manager = Arc::new(ModelManager::new(ModelConfig::default())?);
let service = EmbeddingService::new(model_manager, 32);

// Embed a batch of spectrograms
let outputs = service.embed_batch(spectrograms).await?;
println!("Generated {} embeddings", outputs.len());
# Ok(())
# }
```

For finer control, build the service with `EmbeddingServiceBuilder`:

```rust,ignore
use std::sync::Arc;
use sevensense_embedding::{EmbeddingService, ModelManager, ModelConfig};
use sevensense_embedding::application::services::EmbeddingServiceBuilder;

# fn example(model_manager: Arc<ModelManager>) -> Result<(), Box<dyn std::error::Error>> {
let service = EmbeddingServiceBuilder::new()
    .model_manager(model_manager)
    .batch_size(32)
    .normalize(true)
    .validate_embeddings(true)
    .build()?;
# Ok(())
# }
```

</details>

<details>
<summary><b>Tutorial: Embedding Quantization</b></summary>

The `quantization` module provides free functions for F16 and INT8 quantization.
There is no product-quantization codebook; storage is reduced by lowering the
per-element precision.

### F16 Quantization (50% reduction)

```rust,ignore
use sevensense_embedding::quantization::{quantize_to_f16, dequantize_f16};

let embedding: Vec<f32> = generate_embedding();

// Quantize to half precision
let half = quantize_to_f16(&embedding);

// Round-trip back to f32
let restored = dequantize_f16(&half);
```

### INT8 Quantization (75% reduction)

```rust,ignore
use sevensense_embedding::quantization::{quantize_to_i8_full, QuantizedEmbedding};

let embedding: Vec<f32> = generate_embedding();

// Quantize with scale + zero-point captured in QuantizedEmbedding
let quantized: QuantizedEmbedding = quantize_to_i8_full(&embedding);
println!("Stored bytes: {}", quantized.size_bytes());

// Dequantize back to f32
let restored = quantized.dequantize();
```

Additional helpers include `quantize_to_i8` / `dequantize_i8`,
`quantize_to_u8` / `dequantize_u8`, `compute_quantization_error`,
`compute_cosine_preservation`, `QuantizationStats`, and `BatchQuantizer`.

</details>

<details>
<summary><b>Tutorial: Model Configuration</b></summary>

`ModelConfig` controls the model directory, threading, execution providers, and
session caching.

```rust,ignore
use std::path::PathBuf;
use sevensense_embedding::ModelConfig;
use sevensense_embedding::infrastructure::model_manager::ExecutionProvider;

let config = ModelConfig {
    model_dir: PathBuf::from("models"),
    intra_op_threads: 4,
    inter_op_threads: 1,
    verify_checksums: true,
    execution_providers: vec![
        ExecutionProvider::Cuda { device_id: 0 },
        ExecutionProvider::CoreML,
        ExecutionProvider::Cpu,
    ],
    max_cached_sessions: 4,
};
```

### Execution Providers

`ExecutionProvider` is an enum with the variants `Cpu`, `Cuda { device_id }`,
`CoreML`, and `DirectML { device_id }`. The default priority order is
CUDA → CoreML → CPU.

</details>

---

## Configuration

### `ModelConfig` Parameters

| Parameter | Default | Description |
|-----------|---------|-------------|
| `model_dir` | `models` | Directory containing model files |
| `intra_op_threads` | `min(num_cpus, 4)` | Intra-op parallelism threads |
| `inter_op_threads` | 1 | Inter-op parallelism threads |
| `verify_checksums` | true | Verify model checksums on load |
| `execution_providers` | CUDA, CoreML, CPU | Provider priority order |
| `max_cached_sessions` | 4 | Maximum cached model sessions |

### Model Specifications & Constants

| Constant | Value | Description |
|----------|-------|-------------|
| `EMBEDDING_DIM` | 1536 | Output embedding dimension |
| `TARGET_SAMPLE_RATE` | 32 000 | Target sample rate (Hz) |
| `TARGET_WINDOW_SECONDS` | 5.0 | Target window duration |
| `TARGET_WINDOW_SAMPLES` | 160 000 | 5 s at 32 kHz |
| `MEL_BINS` | 128 | Mel spectrogram bins |
| `MEL_FRAMES` | 500 | Mel spectrogram frames |

## Public API

Re-exported at the crate root (`sevensense_embedding::`):

| Category | Items |
|----------|-------|
| Service | `EmbeddingService` |
| Entities | `Embedding`, `EmbeddingId`, `EmbeddingMetadata`, `EmbeddingModel`, `InputSpecification`, `ModelVersion`, `StorageTier` |
| Repository | `EmbeddingRepository` |
| Infrastructure | `ModelConfig`, `ModelManager`, `OnnxInference` |
| Errors | `EmbeddingError`, `Result<T>` |

## Planned / Not Yet Implemented

The following are **not** part of the current public API:

- **Streaming embedding generation** (no `EmbeddingStream` type)
- **Product Quantization (PQ)** — quantization is F16/INT8 only

## Links

- **Homepage**: [ruv.io](https://ruv.io)
- **Repository**: [github.com/ruvnet/ruvector](https://github.com/ruvnet/ruvector)
- **Crates.io**: [crates.io/crates/sevensense-embedding](https://crates.io/crates/sevensense-embedding)
- **Documentation**: [docs.rs/sevensense-embedding](https://docs.rs/sevensense-embedding)

## License

MIT License - see [LICENSE](../../LICENSE) for details.

---

*Part of the [7sense Bioacoustic Intelligence Platform](https://ruv.io) by rUv*
