# Ruvector Snapshot

[![Crates.io](https://img.shields.io/crates/v/ruvector-snapshot.svg)](https://crates.io/crates/ruvector-snapshot)
[![Documentation](https://docs.rs/ruvector-snapshot/badge.svg)](https://docs.rs/ruvector-snapshot)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-1.77%2B-orange.svg)](https://www.rust-lang.org)

**Point-in-time snapshots and backup for Ruvector vector databases.**

`ruvector-snapshot` provides full snapshot creation, storage, and restoration for Ruvector collections, with GZIP compression and SHA-256 integrity verification. Part of the [Ruvector](https://github.com/ruvnet/ruvector) ecosystem.

## Why Ruvector Snapshot?

- **Point-in-Time Recovery**: Restore any saved snapshot
- **Compression**: GZIP compression for storage efficiency
- **Integrity Verification**: SHA-256 checksums verified on load
- **Async I/O**: Non-blocking snapshot operations (Tokio)
- **Pluggable Storage**: `SnapshotStorage` trait with a `LocalStorage` backend

## Features

### Core Capabilities

- **Full Snapshots**: Complete collection backup (`SnapshotData`)
- **Compression**: GZIP compression of serialized snapshot data
- **Checksums**: SHA-256 integrity verification on restore
- **Async Operations**: Tokio-based async I/O
- **Basic Retention**: `cleanup_old_snapshots(collection, keep_count)` keeps the N most recent

### Planned / Not Yet Implemented

These are roadmap items and are **not** present in the current code:

- **Incremental Snapshots**: Delta-based backups (only full snapshots today)
- **Snapshot Scheduling**: Automated snapshot creation
- **Retention Policies**: Time-based / rule-based policies (only manual `cleanup_old_snapshots`)
- **Remote Storage**: S3/GCS-compatible storage (`LocalStorage` only today)
- **Streaming / Parallel Restore**

## Installation

Add `ruvector-snapshot` to your `Cargo.toml`:

```toml
[dependencies]
ruvector-snapshot = "0.1.1"
```

## Quick Start

### Create Snapshot

```rust
use ruvector_snapshot::{
    SnapshotManager, SnapshotData, LocalStorage,
};
use ruvector_snapshot::{CollectionConfig, DistanceMetric, VectorRecord};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Choose a storage backend (LocalStorage implements SnapshotStorage)
    let storage = Box::new(LocalStorage::new(PathBuf::from("./snapshots")));
    let manager = SnapshotManager::new(storage);

    // Build the snapshot payload
    let config = CollectionConfig {
        dimension: 3,
        metric: DistanceMetric::Cosine,
        hnsw_config: None,
    };
    let vectors = vec![
        VectorRecord::new("v1".to_string(), vec![1.0, 0.0, 0.0], None),
        VectorRecord::new("v2".to_string(), vec![0.0, 1.0, 0.0], None),
    ];
    let data = SnapshotData::new("documents".to_string(), config, vectors);

    // Create a full snapshot
    let snapshot = manager.create_snapshot(data).await?;
    println!("Created snapshot: {} ({} bytes)", snapshot.id, snapshot.size_bytes);

    Ok(())
}
```

### Restore from Snapshot

```rust
// List available snapshots (newest first)
let snapshots = manager.list_snapshots().await?;
for snapshot in &snapshots {
    println!("{}: {} ({} bytes)", snapshot.id, snapshot.created_at, snapshot.size_bytes);
}

// Restore: returns the SnapshotData (vectors + config), checksum-verified
let restored = manager.restore_snapshot(&snapshots[0].id).await?;
println!("Restored {} vectors", restored.vectors_count());
```

### Retention

```rust
// Keep only the 2 most recent snapshots for a collection; returns # deleted
let deleted = manager.cleanup_old_snapshots("documents", 2).await?;
println!("Deleted {} old snapshots", deleted);
```

## API Overview

### Core Types

```rust
// Complete snapshot payload (what you create / restore)
pub struct SnapshotData {
    pub metadata: SnapshotMetadata,
    pub config: CollectionConfig,
    pub vectors: Vec<VectorRecord>,
}

// Snapshot metadata returned after saving
pub struct Snapshot {
    pub id: String,
    pub collection_name: String,
    pub created_at: DateTime<Utc>,
    pub vectors_count: usize,
    pub checksum: String,    // SHA-256
    pub size_bytes: u64,     // compressed size
}

// Collection config stored inside a snapshot
pub struct CollectionConfig {
    pub dimension: usize,
    pub metric: DistanceMetric,           // Cosine | Euclidean | DotProduct
    pub hnsw_config: Option<HnswConfig>,
}

// Storage backend trait + local implementation
pub trait SnapshotStorage { /* save, load, list, delete */ }
pub struct LocalStorage { /* filesystem backend */ }
```

### Manager Operations

```rust
impl SnapshotManager {
    pub fn new(storage: Box<dyn SnapshotStorage>) -> Self;

    // Creation & restoration
    pub async fn create_snapshot(&self, snapshot_data: SnapshotData) -> Result<Snapshot>;
    pub async fn restore_snapshot(&self, id: &str) -> Result<SnapshotData>;

    // Listing & info
    pub async fn list_snapshots(&self) -> Result<Vec<Snapshot>>;
    pub async fn list_snapshots_for_collection(&self, collection_name: &str) -> Result<Vec<Snapshot>>;
    pub async fn get_snapshot_info(&self, id: &str) -> Result<Snapshot>;

    // Management
    pub async fn delete_snapshot(&self, id: &str) -> Result<()>;
    pub async fn cleanup_old_snapshots(&self, collection_name: &str, keep_count: usize) -> Result<usize>;
    pub async fn total_size(&self) -> Result<u64>;
    pub async fn collection_size(&self, collection_name: &str) -> Result<u64>;
}
```

## Snapshot Format

With `LocalStorage`, each snapshot is written as two files in the base directory:

```text
{id}.snapshot.gz      # bincode-serialized SnapshotData, GZIP-compressed
{id}.metadata.json    # Snapshot metadata (id, collection, checksum, size, ...)
```

## Related Crates

- **[ruvector-core](../ruvector-core/)** - Core vector database engine
- **[ruvector-replication](../ruvector-replication/)** - Data replication

## Documentation

- **[Main README](../../README.md)** - Complete project overview
- **[API Documentation](https://docs.rs/ruvector-snapshot)** - Full API reference
- **[GitHub Repository](https://github.com/ruvnet/ruvector)** - Source code

## License

**MIT License** - see [LICENSE](../../LICENSE) for details.

---

<div align="center">

**Part of [Ruvector](https://github.com/ruvnet/ruvector) - Built by [rUv](https://ruv.io)**

[![Star on GitHub](https://img.shields.io/github/stars/ruvnet/ruvector?style=social)](https://github.com/ruvnet/ruvector)

[Documentation](https://docs.rs/ruvector-snapshot) | [Crates.io](https://crates.io/crates/ruvector-snapshot) | [GitHub](https://github.com/ruvnet/ruvector)

</div>
