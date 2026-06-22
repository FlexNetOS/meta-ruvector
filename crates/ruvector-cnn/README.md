# ruvector-cnn

[![Crates.io](https://img.shields.io/crates/v/ruvector-cnn.svg)](https://crates.io/crates/ruvector-cnn)
[![Documentation](https://docs.rs/ruvector-cnn/badge.svg)](https://docs.rs/ruvector-cnn)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.77%2B-orange.svg)](https://www.rust-lang.org)

**Turn images into searchable vectors -- pure Rust, portable, no heavy dependencies.**

## What is This?

`ruvector-cnn` converts images into numerical representations (embeddings) for similarity search and clustering. Think of an embedding as a fingerprint: two photos of red sneakers produce similar fingerprints, while a red sneaker and a blue handbag produce different ones.

Once you have embeddings, you can:

- **Find similar images**: Compare embedding distances
- **Cluster visual content**: Group images by visual similarity
- **Detect near-duplicates**: Find copied, resized, or edited images
- **Build multimodal search**: Combine image embeddings with text embeddings in one index

The key difference from PyTorch/TensorFlow: **this runs anywhere Rust compiles** -- your laptop, a Raspberry Pi, a web browser (WASM), or a serverless function -- without installing Python, GPU drivers, or heavy runtimes.

> **Honesty note:** the default `CnnEmbedder` is a lightweight, randomly-initialized
> convolutional feature extractor (one conv + batch-norm + ReLU + global pool +
> projection). It is **not** a pretrained ImageNet model. It produces deterministic,
> L2-normalized vectors suitable for wiring up and testing similarity pipelines.
> The full MobileNet-V3 backbone lives behind the `backbone` feature (see below) and
> is still a work in progress. See [Status](#status) for what is real today.

## Quick Start

### Basic: Extract an Embedding

```rust
use ruvector_cnn::{CnnEmbedder, EmbeddingConfig};

# fn main() -> ruvector_cnn::CnnResult<()> {
// Default embedder (512-dim, ImageNet-style normalization, L2 output)
let embedder = CnnEmbedder::new(EmbeddingConfig::default())?;

// Image data is RGBA bytes: width * height * 4 bytes
let width = 224;
let height = 224;
let image_data = vec![128u8; (width * height * 4) as usize];

// Extract the embedding (Vec<f32>, length == embedding_dim)
let embedding = embedder.extract(&image_data, width, height)?;
println!("Embedding dim: {}", embedding.len());
# Ok(())
# }
```

You can also construct preset configurations matching MobileNet-V3 output dimensions:

```rust
use ruvector_cnn::CnnEmbedder;

# fn main() -> ruvector_cnn::CnnResult<()> {
let small = CnnEmbedder::new_v3_small()?; // embedding_dim == 576
let large = CnnEmbedder::new_v3_large()?; // embedding_dim == 960
println!("{} / {}", small.embedding_dim(), large.embedding_dim());
# Ok(())
# }
```

> `new_v3_small()` / `new_v3_large()` set the **output dimension** to match the named
> MobileNet-V3 variant. They use the same lightweight feature extractor as
> `new(..)` -- they do not load MobileNet-V3 weights. The weight-bearing backbone is
> the `backbone`-feature `MobileNetEmbedder` (WIP).

### Similarity Search: Find Similar Images

```rust
use ruvector_cnn::{CnnEmbedder, EmbeddingConfig};

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    dot / (norm_a * norm_b)
}

# fn load_rgba(_p: &str) -> (Vec<u8>, u32, u32) { (vec![0u8; 224*224*4], 224, 224) }
# fn main() -> ruvector_cnn::CnnResult<()> {
let embedder = CnnEmbedder::new(EmbeddingConfig::default())?;

// Query image (your own RGBA decode → bytes)
let (query_px, qw, qh) = load_rgba("user_upload.jpg");
let query_emb = embedder.extract(&query_px, qw, qh)?;

// Compare against your catalog
let catalog = ["product_001.jpg", "product_002.jpg", "product_003.jpg"];
let mut results: Vec<(f32, &str)> = catalog
    .iter()
    .map(|path| {
        let (px, w, h) = load_rgba(path);
        let emb = embedder.extract(&px, w, h).unwrap();
        (cosine_similarity(&query_emb, &emb), *path)
    })
    .collect();

results.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
println!("Most similar: {} (score: {:.3})", results[0].1, results[0].0);
# Ok(())
# }
```

> **Input format:** `extract(image_data, width, height)` expects **RGBA** bytes
> (`width * height * 4`). Decode your JPEG/PNG to RGBA with a crate like `image`
> first. There is no built-in image loader in this crate.

### Batch Processing: Embed a Dataset

```rust
use ruvector_cnn::{CnnEmbedder, EmbeddingConfig};
use rayon::prelude::*;

# fn load_rgba(_p: &&str) -> (Vec<u8>, u32, u32) { (vec![0u8; 224*224*4], 224, 224) }
# fn main() -> ruvector_cnn::CnnResult<()> {
let embedder = CnnEmbedder::new(EmbeddingConfig::default())?;
let image_paths: Vec<&str> = vec![/* paths */];

// Process in parallel using all CPU cores (bring your own RGBA decode)
let embeddings: Vec<Vec<f32>> = image_paths
    .par_iter()
    .map(|path| {
        let (px, w, h) = load_rgba(path);
        embedder.extract(&px, w, h).unwrap()
    })
    .collect();

println!("Embedded {} images", embeddings.len());
# Ok(())
# }
```

### INT8 Quantization

The `int8` and `quantize` modules provide quantization primitives for embeddings.
See the module docs (`ruvector_cnn::int8`, `ruvector_cnn::quantize`) and the `simd`
module for SIMD-accelerated kernels.

## Why Pure Rust?

| Problem | How ruvector-cnn Solves It |
|---------|---------------------------|
| "PyTorch is 500MB and needs Python" | Pure Rust, compiles to a single executable |
| "I need this to run in a browser" | WASM-friendly (no native deps in the core extractor) |
| "Quantization is a separate toolchain" | INT8 quantization primitives included (`int8`, `quantize`) |
| "I can't install CUDA on my device" | CPU-only, no GPU required |
| "ONNX Runtime has native dependencies" | Zero native deps in the core path -- cross-compile from any OS |

### When to Use This vs. Alternatives

**Use ruvector-cnn when:**
- You need CPU embeddings without heavy dependencies
- You are deploying to WASM, edge devices, or constrained environments
- You need to integrate directly with vector search indices
- Binary size matters

**Consider PyTorch/ONNX when:**
- You need GPU acceleration for training
- You need pretrained, high-accuracy ImageNet weights today
- You are already in a Python ecosystem

## Installation

Add `ruvector-cnn` to your `Cargo.toml`:

```toml
[dependencies]
ruvector-cnn = "0.1"
```

### Feature Flags

```toml
[dependencies]
# Default
ruvector-cnn = "0.1"

# With the MobileNet-V3 backbone + MobileNetEmbedder (WIP)
ruvector-cnn = { version = "0.1", features = ["backbone"] }
```

The core `CnnEmbedder`, `EmbeddingConfig`, `Tensor`, and the `kernels`, `layers`,
`simd`, `int8`, `quantize`, and `contrastive` modules are always available. The
`backbone` and `embedding` modules (and their re-exported types) require the
`backbone` feature.

## API Overview

### Always Available (crate root)

```rust
use ruvector_cnn::{CnnEmbedder, EmbeddingConfig, EmbeddingExtractor, Tensor, CnnError, CnnResult};
```

```rust
/// Configuration for CNN embedding extraction
pub struct EmbeddingConfig {
    pub input_size: u32,      // assumes square input (default 224)
    pub embedding_dim: usize, // output dimension (default 512)
    pub normalize: bool,      // L2-normalize output (default true)
    pub quantized: bool,      // use INT8 quantization (default false)
}

impl CnnEmbedder {
    pub fn new(config: EmbeddingConfig) -> CnnResult<Self>;
    pub fn new_v3_small() -> CnnResult<Self>; // embedding_dim = 576
    pub fn new_v3_large() -> CnnResult<Self>; // embedding_dim = 960
    pub fn extract(&self, image_data: &[u8], width: u32, height: u32) -> CnnResult<Vec<f32>>;
    pub fn embedding_dim(&self) -> usize;
    pub fn input_size(&self) -> u32;
}

/// Trait implemented by CnnEmbedder
pub trait EmbeddingExtractor {
    fn extract(&self, image_data: &[u8], width: u32, height: u32) -> CnnResult<Vec<f32>>;
    fn embedding_dim(&self) -> usize;
}
```

### Behind the `backbone` feature

```rust
// requires features = ["backbone"]
use ruvector_cnn::{
    MobileNetEmbedder, MobileNetEmbeddingConfig, MobileNetV3, MobileNetV3Config,
    MobileNetV3Small, MobileNetV3Large, MobileNetConfig, BackboneType,
    create_backbone, mobilenet_v3_small, mobilenet_v3_large,
    cosine_similarity, euclidean_distance,
};
```

See the crate-level docs for the `backbone` example using `MobileNetEmbedder::v3_small()`.

## Architecture

```
ruvector-cnn/
├── src/
│   ├── lib.rs          # CnnEmbedder, EmbeddingConfig, EmbeddingExtractor
│   ├── error.rs        # CnnError, CnnResult
│   ├── tensor.rs       # Tensor
│   ├── kernels/        # low-level kernels (always available)
│   ├── layers/         # conv2d_3x3, batch_norm, pooling, activations
│   ├── simd/           # SIMD-accelerated kernels
│   ├── int8/           # INT8 types
│   ├── quantize/       # quantization primitives
│   ├── contrastive/    # contrastive learning building blocks
│   ├── backbone/       # MobileNet-V3 backbone        (feature = "backbone")
│   └── embedding/      # MobileNetEmbedder             (feature = "backbone")
```

## Status

What is real today:

- [x] `CnnEmbedder` lightweight feature extractor (conv → BN → ReLU → global pool → projection → optional L2)
- [x] RGBA input via `extract(image_data, width, height)`
- [x] `new_v3_small()` (576-dim) / `new_v3_large()` (960-dim) preset configs
- [x] `kernels`, `layers`, `simd`, `int8`, `quantize`, `contrastive` modules
- [x] `EmbeddingExtractor` trait

Planned / Work in Progress:

- [ ] Weight-bearing MobileNet-V3 backbone (`MobileNetEmbedder`, `backbone` feature)
- [ ] Pretrained weight loading / ONNX import
- [ ] Parallel batch embedding (`parallel` feature -- not yet implemented)

> Performance numbers, GPU comparisons, and pretrained-accuracy tables are **not**
> claimed for the default extractor. They will be added once the `backbone` path
> ships real weights and is benchmarked.

## Building and Testing

```bash
# Build with default features
cargo build --release -p ruvector-cnn

# Build with the MobileNet-V3 backbone (WIP)
cargo build --release -p ruvector-cnn --features backbone

# Run tests
cargo test -p ruvector-cnn
```

## Related Crates

- **[ruvector-core](../ruvector-core/)** - Vector database engine for storing embeddings
- **[ruvector-attention](../ruvector-attention/)** - Attention mechanisms

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT License ([LICENSE-MIT](LICENSE-MIT))

at your option.

---

<div align="center">

**Part of [RuVector](https://github.com/ruvnet/ruvector) - Built by [rUv](https://ruv.io)**

[Documentation](https://docs.rs/ruvector-cnn) | [Crates.io](https://crates.io/crates/ruvector-cnn) | [GitHub](https://github.com/ruvnet/ruvector)

</div>
