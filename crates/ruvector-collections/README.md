# Ruvector Collections

[![Crates.io](https://img.shields.io/crates/v/ruvector-collections.svg)](https://crates.io/crates/ruvector-collections)
[![Documentation](https://docs.rs/ruvector-collections/badge.svg)](https://docs.rs/ruvector-collections)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-1.77%2B-orange.svg)](https://www.rust-lang.org)

**High-performance collection management for Ruvector vector databases.**

`ruvector-collections` provides multi-collection management with on-disk persistence, per-collection configuration, and aliases. Part of the [Ruvector](https://github.com/ruvnet/ruvector) ecosystem.

## Why Ruvector Collections?

- **Multiple Collections**: Organize vectors into separate, isolated collections
- **Per-Collection Config**: Dimensions, distance metric, HNSW, and quantization per collection
- **Thread-Safe**: Concurrent access with `DashMap`
- **Persistence**: Collections and aliases are stored on disk
- **Aliases**: Human-readable, switchable names for collections

## Features

### Core Capabilities

- **Collection CRUD**: Create, get, list, and delete collections
- **Per-Collection Config**: `CollectionConfig` (dimensions, distance metric, HNSW, quantization, on-disk payload)
- **Config Validation**: Dimensions and HNSW parameters are validated on creation
- **Statistics**: `CollectionStats` (vector count, segment count, disk/RAM size)
- **Alias Support**: Create, delete, and switch aliases; resolve names through aliases

### Planned / Not Yet Implemented

These are roadmap items and are **not** present in the current code:

- **Collection Groups**: Organize collections hierarchically
- **Access Control**: Collection-level permissions
- **Versioning**: Collection schema versioning
- **Migration**: Tools for collection migration

## Installation

Add `ruvector-collections` to your `Cargo.toml`:

```toml
[dependencies]
ruvector-collections = "0.1.1"
```

## Quick Start

### Create a Collection

```rust
use ruvector_collections::{CollectionManager, CollectionConfig};
use ruvector_core::types::{DistanceMetric, HnswConfig};
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a collection manager rooted at a directory on disk
    let manager = CollectionManager::new(PathBuf::from("./collections"))?;

    // Define the collection configuration
    let config = CollectionConfig {
        dimensions: 384,
        distance_metric: DistanceMetric::Cosine,
        hnsw_config: Some(HnswConfig::default()),
        quantization: None,
        on_disk_payload: true,
    };
    // Shortcut: CollectionConfig::with_dimensions(384) builds sensible defaults.

    // Create the collection (named, not stored on the config)
    manager.create_collection("documents", config)?;
    println!("Created collection: documents");

    Ok(())
}
```

### Manage Collections

```rust
use ruvector_collections::CollectionManager;
use std::path::PathBuf;

let manager = CollectionManager::new(PathBuf::from("./collections"))?;

// List all collection names
for name in manager.list_collections() {
    let stats = manager.collection_stats(&name)?;
    println!("{}: {} vectors", name, stats.vectors_count);
}

// Check existence
if manager.collection_exists("documents") {
    // Get a collection handle (Arc<RwLock<Collection>>) by name or alias
    let docs = manager.get_collection("documents").unwrap();
    let guard = docs.read();
    println!("dims: {}", guard.config.dimensions);
}

// Delete a collection (must have no active aliases)
manager.delete_collection("old_collection")?;
```

### Collection Aliases

```rust
// Create an alias pointing at a collection
manager.create_alias("current_docs", "documents")?;

// Switch the alias to a different collection (zero-downtime swap)
manager.switch_alias("current_docs", "documents_v3")?;

// get_collection resolves aliases transparently
let collection = manager.get_collection("current_docs").unwrap();

// Inspect / remove aliases
for (alias, target) in manager.list_aliases() {
    println!("{} -> {}", alias, target);
}
manager.delete_alias("current_docs")?;
```

## API Overview

### Core Types

```rust
// Collection configuration (the name is passed separately to create_collection)
pub struct CollectionConfig {
    pub dimensions: usize,
    pub distance_metric: DistanceMetric,        // from ruvector_core::types
    pub hnsw_config: Option<HnswConfig>,        // from ruvector_core::types
    pub quantization: Option<QuantizationConfig>,
    pub on_disk_payload: bool,
}

// Collection statistics
pub struct CollectionStats {
    pub vectors_count: usize,
    pub segments_count: usize,
    pub disk_size_bytes: u64,
    pub ram_size_bytes: u64,
}
```

### Manager Operations

```rust
impl CollectionManager {
    pub fn new(base_path: PathBuf) -> Result<Self>;

    // Collections
    pub fn create_collection(&self, name: &str, config: CollectionConfig) -> Result<()>;
    pub fn delete_collection(&self, name: &str) -> Result<()>;
    pub fn get_collection(&self, name: &str) -> Option<Arc<RwLock<Collection>>>;
    pub fn list_collections(&self) -> Vec<String>;
    pub fn collection_exists(&self, name: &str) -> bool;
    pub fn collection_stats(&self, name: &str) -> Result<CollectionStats>;

    // Aliases
    pub fn create_alias(&self, alias: &str, collection: &str) -> Result<()>;
    pub fn delete_alias(&self, alias: &str) -> Result<()>;
    pub fn switch_alias(&self, alias: &str, new_collection: &str) -> Result<()>;
    pub fn resolve_alias(&self, name_or_alias: &str) -> Option<String>;
    pub fn list_aliases(&self) -> Vec<(String, String)>;
    pub fn is_alias(&self, name: &str) -> bool;
}
```

## Related Crates

- **[ruvector-core](../ruvector-core/)** - Core vector database engine
- **[ruvector-server](../ruvector-server/)** - REST API server
- **[ruvector-filter](../ruvector-filter/)** - Metadata filtering

## Documentation

- **[Main README](../../README.md)** - Complete project overview
- **[API Documentation](https://docs.rs/ruvector-collections)** - Full API reference
- **[GitHub Repository](https://github.com/ruvnet/ruvector)** - Source code

## License

**MIT License** - see [LICENSE](../../LICENSE) for details.

---

<div align="center">

**Part of [Ruvector](https://github.com/ruvnet/ruvector) - Built by [rUv](https://ruv.io)**

[![Star on GitHub](https://img.shields.io/github/stars/ruvnet/ruvector?style=social)](https://github.com/ruvnet/ruvector)

[Documentation](https://docs.rs/ruvector-collections) | [Crates.io](https://crates.io/crates/ruvector-collections) | [GitHub](https://github.com/ruvnet/ruvector)

</div>
