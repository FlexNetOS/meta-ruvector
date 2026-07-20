# Ruvector GNN

[![Crates.io](https://img.shields.io/crates/v/ruvector-gnn.svg)](https://crates.io/crates/ruvector-gnn)
[![Documentation](https://docs.rs/ruvector-gnn/badge.svg)](https://docs.rs/ruvector-gnn)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-1.77%2B-orange.svg)](https://www.rust-lang.org)

**A graph-neural-network layer for refining embeddings over HNSW-style neighbor structure, with built-in continual-learning tooling.**

`ruvector-gnn` provides a single message-passing layer (`RuvectorLayer`) that combines multi-head attention, edge-weighted aggregation, a GRU update, and layer normalization, plus a self-supervised graph autoencoder (`GraphMAE`) and a continual-learning toolkit (Adam/SGD optimizers, an experience replay buffer, Elastic Weight Consolidation, and learning-rate schedulers). Part of the [RuVector](https://github.com/ruvnet/ruvector) ecosystem.

## Installation

Add `ruvector-gnn` to your `Cargo.toml`:

```toml
[dependencies]
ruvector-gnn = "2.2"
```

### Feature Flags

```toml
[dependencies]
# Default build with memory mapping
ruvector-gnn = { version = "2.2", features = ["mmap"] }
```

Available features:
- `mmap` (native, off-wasm): Memory-mapped gradient accumulation (`MmapManager`, `MmapGradientAccumulator`, `AtomicBitmap`)
- `cold-tier` (native, off-wasm): Cold-tier storage module

## Key Features

| Feature | What It Does | Why It Matters |
|---------|-------------|----------------|
| **`RuvectorLayer`** | One message-passing layer: per-node multi-head attention over neighbors, edge-weighted aggregation, a GRU update, and layer norm | Refines an embedding using its neighbors without manual feature engineering |
| **`GraphMAE`** | Masked graph autoencoder (`GATEncoder` + `GraphMAEDecoder`) with feature masking and SCE/MSE reconstruction loss | Self-supervised pretraining of node representations |
| **Continual learning** | `Optimizer` (Adam/SGD), `ReplayBuffer` (reservoir sampling), `ElasticWeightConsolidation`, `LearningRateScheduler` | Train incrementally while mitigating catastrophic forgetting |
| **Differentiable search** | `differentiable_search`, `hierarchical_forward`, `cosine_similarity` | Query/search helpers over learned embeddings |
| **Compression** | `TensorCompress` with `CompressionLevel` producing `CompressedTensor` | Compress stored tensors |
| **Memory mapping** (`mmap`) | `MmapGradientAccumulator` over `MmapManager` | Accumulate gradients backed by mmap'd storage |

## Quick Start

### A single `RuvectorLayer`

```rust
use ruvector_gnn::RuvectorLayer;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // input_dim, hidden_dim, attention heads, dropout
    let layer = RuvectorLayer::new(128, 64, 4, 0.1)?;

    // Current node embedding
    let node: Vec<f32> = vec![0.0; 128];

    // Neighbor embeddings (e.g. HNSW neighbors)
    let neighbors: Vec<Vec<f32>> = vec![vec![0.0; 128]; 8];

    // Edge weights (e.g. distances), one per neighbor
    let edge_weights: Vec<f32> = vec![1.0; 8];

    // Forward pass returns the updated node embedding
    let updated = layer.forward(&node, &neighbors, &edge_weights);
    println!("updated dim: {}", updated.len()); // 64
    Ok(())
}
```

### Self-supervised pretraining with `GraphMAE`

```rust
use ruvector_gnn::{GraphMAE, GraphMAEConfig, GraphData, LossFn};

let config = GraphMAEConfig::default();
let mae = GraphMAE::new(config);

// GraphData holds node features and adjacency for masked reconstruction.
// See `GraphData`, `FeatureMasking`, and the `sce_loss` / `mse_loss` helpers.
```

### Continual learning (forgetting mitigation)

```rust
use ruvector_gnn::{
    Optimizer, OptimizerType,
    ReplayBuffer,
    ElasticWeightConsolidation,
    LearningRateScheduler, SchedulerType,
};

// Adam optimizer
let mut optimizer = Optimizer::new(OptimizerType::Adam {
    learning_rate: 0.001,
    beta1: 0.9,
    beta2: 0.999,
    epsilon: 1e-8,
});

// Experience replay (reservoir sampling)
let mut replay = ReplayBuffer::new(10_000);

// EWC to prevent catastrophic forgetting
let mut ewc = ElasticWeightConsolidation::new(0.4);

// Cosine-annealing schedule
let mut scheduler = LearningRateScheduler::new(
    SchedulerType::CosineAnnealing { t_max: 100, eta_min: 1e-6 },
    0.001,
);
```

### Differentiable search helpers

```rust
use ruvector_gnn::{cosine_similarity, differentiable_search, hierarchical_forward};

let a = [1.0_f32, 0.0, 0.0];
let b = [1.0_f32, 0.0, 0.0];
let sim = cosine_similarity(&a, &b);
```

## API Overview

### Public re-exports (from `lib.rs`)

```rust
// Layer
pub use layer::RuvectorLayer;

// Self-supervised autoencoder
pub use graphmae::{
    mse_loss, sce_loss, FeatureMasking, GATEncoder, GraphData, GraphMAE,
    GraphMAEConfig, GraphMAEDecoder, LossFn, MaskResult,
};

// Continual learning
pub use training::{
    info_nce_loss, local_contrastive_loss, sgd_step, Loss, LossType,
    OnlineConfig, Optimizer, OptimizerType, TrainConfig,
};
pub use replay::{DistributionStats, ReplayBuffer, ReplayEntry};
pub use ewc::ElasticWeightConsolidation;
pub use scheduler::{LearningRateScheduler, SchedulerType};

// Search & query
pub use search::{cosine_similarity, differentiable_search, hierarchical_forward};
pub use query::{QueryMode, QueryResult, RuvectorQuery, SubGraph};

// Compression
pub use compress::{CompressedTensor, CompressionLevel, TensorCompress};

// Errors
pub use error::{GnnError, Result};

// mmap feature only:
pub use mmap::{AtomicBitmap, MmapGradientAccumulator, MmapManager};
```

### `RuvectorLayer`

```rust
impl RuvectorLayer {
    /// input_dim, hidden_dim, heads, dropout (0.0..=1.0).
    /// Errors if dropout is out of range or hidden_dim is not divisible by heads.
    pub fn new(input_dim: usize, hidden_dim: usize, heads: usize, dropout: f32)
        -> Result<Self, GnnError>;

    /// Update one node's embedding from its neighbors and edge weights.
    pub fn forward(
        &self,
        node_embedding: &[f32],
        neighbor_embeddings: &[Vec<f32>],
        edge_weights: &[f32],
    ) -> Vec<f32>;
}
```

Internally `RuvectorLayer` is composed of `Linear`, `LayerNorm`, `MultiHeadAttention`, and `GRUCell` building blocks (also defined in the `layer` module).

## Modules

| Module | Contents |
|--------|----------|
| `layer` | `RuvectorLayer` and its building blocks |
| `graphmae` | Masked graph autoencoder (`GraphMAE`, `GATEncoder`, `GraphMAEDecoder`, masking, losses) |
| `training` | Optimizers, online training config, contrastive/SGD helpers |
| `replay` | Experience replay buffer |
| `ewc` | Elastic Weight Consolidation |
| `scheduler` | Learning-rate schedulers |
| `search` | Differentiable/hierarchical search helpers |
| `query` | Query types over learned embeddings |
| `compress` | Tensor compression |
| `tensor` | Tensor primitives |
| `error` | `GnnError` / `Result` |
| `mmap` (feature) | Memory-mapped gradient accumulation |
| `cold_tier` (feature) | Cold-tier storage |

## Related Crates

- **[ruvector-core](../ruvector-core/)** - Core vector database engine
- **[ruvector-graph](../ruvector-graph/)** - Graph database engine

## Documentation

- **[API Documentation](https://docs.rs/ruvector-gnn)** - Full API reference
- **[GitHub Repository](https://github.com/ruvnet/ruvector)** - Source code

## License

**MIT License** - see [LICENSE](../../LICENSE) for details.

---

<div align="center">

**Part of [RuVector](https://github.com/ruvnet/ruvector) - Built by [rUv](https://ruv.io)**

[![Star on GitHub](https://img.shields.io/github/stars/ruvnet/ruvector?style=social)](https://github.com/ruvnet/ruvector)

[Documentation](https://docs.rs/ruvector-gnn) | [Crates.io](https://crates.io/crates/ruvector-gnn) | [GitHub](https://github.com/ruvnet/ruvector)

</div>
