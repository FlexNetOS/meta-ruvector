# exo-backend-classical

Classical substrate backend for the EXO-AI cognitive substrate. Implements the
`SubstrateBackend` trait from `exo-core` on top of the high-performance
`ruvector-core` vector database and `ruvector-graph` graph database, providing a
classical (discrete) implementation of the substrate abstractions.

## Features

- **Vector similarity search** -- `similarity_search` delegates to a
  `ruvector-core`-backed vector index for k-nearest-neighbour queries.
- **Discrete pattern insertion** -- `manifold_deform` performs a discrete
  insert into the vector index (no continuous deformation in the classical
  backend), returning a `ManifoldDelta::DiscreteInsert`.
- **Hypergraph storage** -- bundles a `GraphWrapper` (exposed via `graph_db()`)
  for hyperedge operations.
- **Dither quantization** (`dither_quantizer`) -- stochastic dithered
  quantization of activations (`DitheredQuantizer`).
- **Thermodynamic layer** (`thermo_layer`) -- Landauer-style energy accounting
  (`ThermoLayer`, `ThermoSignal`).
- **Domain bridge with Thompson sampling** (`domain_bridge`) -- cross-domain
  transfer adapters (`ExoRetrievalDomain`, `ExoGraphDomain`, `ExoTransferAdapter`).
- **Transfer orchestrator** (`transfer_orchestrator`) -- coordinates transfer
  cycles (`ExoTransferOrchestrator`).

## Quick Start

Add the dependency to your `Cargo.toml`:

```toml
[dependencies]
exo-backend-classical = "0.1"
```

Basic usage:

```rust
use exo_backend_classical::{ClassicalBackend, ClassicalConfig};
use exo_core::SubstrateBackend;

// With default configuration (768 dimensions, cosine distance)
let backend = ClassicalBackend::new(ClassicalConfig::default())?;

// Or construct with a specific dimensionality
let backend = ClassicalBackend::with_dimensions(384)?;

// Inspect the configured dimensionality
println!("Dimensions: {}", backend.dimension());

// Similarity search (k nearest neighbours, optional filter)
let query = vec![0.0_f32; 384];
let results = backend.similarity_search(&query, 10, None)?;
println!("Found {} results", results.len());
# Ok::<(), exo_core::Error>(())
```

## Crate Layout

| Module                  | Purpose                                              |
|-------------------------|------------------------------------------------------|
| `vector`                | `ruvector-core`-backed vector index wrapper          |
| `graph`                 | `ruvector-graph`-backed hypergraph wrapper           |
| `dither_quantizer`      | Stochastic dither quantization of activations        |
| `thermo_layer`          | Landauer-style thermodynamic energy accounting       |
| `domain_bridge`         | Domain bridge with Thompson sampling                 |
| `transfer_orchestrator` | Cross-domain transfer cycle orchestrator             |

## Requirements

- Rust 1.78+
- Depends on `exo-core`
- Optional: AVX2-capable CPU for best SIMD performance

## Links

- [GitHub](https://github.com/ruvnet/ruvector)
- [EXO-AI Documentation](https://github.com/ruvnet/ruvector/tree/main/examples/exo-ai-2025)

## License

MIT OR Apache-2.0
