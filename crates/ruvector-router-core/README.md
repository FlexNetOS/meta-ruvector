# Router Core

[![Rust](https://img.shields.io/badge/rust-1.77%2B-orange.svg)](https://www.rust-lang.org)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)
[![Performance](https://img.shields.io/badge/latency-<0.5ms-green.svg)](../../docs/TECHNICAL_PLAN.md)

**High-performance vector database engine built in Rust.**

`ruvector-router-core` is the core storage and retrieval engine powering Ruvector's
sub-millisecond vector similarity search. It combines HNSW indexing with persistent
storage for fast approximate nearest-neighbor queries.

## 🎯 Overview

Router Core provides the foundation of Ruvector's vector database capabilities:

- **Vector Database**: High-performance storage and retrieval with HNSW indexing
- **HNSW Indexing**: Hierarchical Navigable Small World approximate nearest-neighbor search
- **Multiple Distance Metrics**: Euclidean, Cosine, Dot Product, Manhattan
- **Persistent Storage**: Durable vector storage with metadata
- **Builder API**: Ergonomic configuration via `VectorDB::builder()`

> **Crate name vs. library path:** the package is `ruvector-router-core`, so the
> `use` path in Rust is `ruvector_router_core` (underscores).

## ⚡ Key Features

- **HNSW Indexing**: fast approximate nearest-neighbor search, tunable via `m`,
  `ef_construction`, and `ef_search`.
- **Multiple Distance Metrics**: `DistanceMetric::Euclidean`, `Cosine`, `DotProduct`,
  `Manhattan` (see [`src/distance.rs`](src/distance.rs)).
- **Metadata Filtering**: post-filter search results by exact metadata key/value match.
- **Batch Inserts**: `insert_batch` for bulk loading.
- **Index Rebuild on Open**: persisted vectors are reloaded into the in-memory HNSW
  when a `VectorDB` is reopened against an existing storage path.
- **Thread-Safe**: built on `parking_lot` for concurrent access.

## 📦 Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
ruvector-router-core = "0.1.0"
```

## 🚀 Quick Start

### Basic Vector Database

```rust
use ruvector_router_core::{VectorDB, VectorEntry, SearchQuery, DistanceMetric};
use std::collections::HashMap;

# fn main() -> ruvector_router_core::Result<()> {
// Create database with the builder pattern
let db = VectorDB::builder()
    .dimensions(384)                          // Vector dimensions
    .distance_metric(DistanceMetric::Cosine)
    .hnsw_m(32)                               // HNSW connections per node
    .hnsw_ef_construction(200)                // Construction accuracy
    .storage_path("./vectors.db")
    .build()?;

// Insert a vector
let entry = VectorEntry {
    id: "doc1".to_string(),
    vector: vec![0.1; 384],
    metadata: HashMap::new(),
    timestamp: chrono::Utc::now().timestamp(),
};

db.insert(entry)?;

// Search for similar vectors
let query = SearchQuery {
    vector: vec![0.1; 384],
    k: 10,                     // Top 10 results
    filters: None,
    threshold: Some(0.8),      // Distance threshold (optional)
    ef_search: Some(100),      // Search accuracy (optional)
};

let results = db.search(query)?;
for result in results {
    println!("{}: {}", result.id, result.score);
}
# Ok(())
# }
```

### Batch Operations

```rust
use ruvector_router_core::{VectorDB, VectorEntry};
use std::collections::HashMap;

# fn run(db: &VectorDB) -> ruvector_router_core::Result<()> {
// Insert multiple vectors in one call
let entries: Vec<VectorEntry> = (0..1000)
    .map(|i| VectorEntry {
        id: format!("doc{i}"),
        vector: vec![0.1; 384],
        metadata: HashMap::new(),
        timestamp: chrono::Utc::now().timestamp(),
    })
    .collect();

let ids = db.insert_batch(entries)?;
println!("Inserted {} vectors", ids.len());

// Inspect database statistics
let stats = db.stats();
println!("Total vectors: {}", stats.total_vectors);
println!("Avg latency: {:.2}us", stats.avg_query_latency_us);

// Count vectors / list IDs
println!("count = {}", db.count()?);
let _all_ids = db.get_all_ids()?;
# Ok(())
# }
```

### Configuration

```rust
use ruvector_router_core::{VectorDB, DistanceMetric};

# fn main() -> ruvector_router_core::Result<()> {
let db = VectorDB::builder()
    .dimensions(768)                          // Larger embeddings
    .max_elements(10_000_000)                 // Capacity hint
    .distance_metric(DistanceMetric::Cosine)  // Cosine distance
    .hnsw_m(64)                               // More connections = higher recall
    .hnsw_ef_construction(400)                // Higher accuracy during build
    .hnsw_ef_search(200)                      // Default search-time accuracy
    .mmap_vectors(true)                       // Memory-mapping flag
    .storage_path("./large_db.redb")
    .build()?;
# let _ = db;
# Ok(())
# }
```

## 🎨 Distance Metrics

Router Core supports four distance metrics via the `DistanceMetric` enum. The
free functions in [`src/distance.rs`](src/distance.rs) let you compute distances
directly:

```rust
use ruvector_router_core::DistanceMetric;
use ruvector_router_core::distance::calculate_distance;

# fn main() -> ruvector_router_core::Result<()> {
let a = vec![1.0, 0.0, 0.0];
let b = vec![0.9, 0.1, 0.0];

// Cosine: returns 1 - cosine_similarity (0 = identical)
let _cos = calculate_distance(&a, &b, DistanceMetric::Cosine)?;

// Euclidean (L2): sqrt(sum((a[i] - b[i])^2))
let _l2 = calculate_distance(&a, &b, DistanceMetric::Euclidean)?;

// Dot product: -sum(a[i] * b[i]) (negated so smaller = more similar)
let _dot = calculate_distance(&a, &b, DistanceMetric::DotProduct)?;

// Manhattan (L1): sum(|a[i] - b[i]|)
let _l1 = calculate_distance(&a, &b, DistanceMetric::Manhattan)?;
# Ok(())
# }
```

Additional distance helpers (also in `distance`): `euclidean_distance`,
`cosine_similarity`, `dot_product`, `manhattan_distance`, and `batch_distance`.

## 🔍 Metadata Filtering

`SearchQuery::filters` is an optional `HashMap<String, serde_json::Value>`. When
set, results are retained only when every key/value matches the entry metadata
exactly:

```rust
use ruvector_router_core::SearchQuery;
use std::collections::HashMap;

let mut filters = HashMap::new();
filters.insert("category".to_string(), serde_json::json!("footwear"));
filters.insert("in_stock".to_string(), serde_json::json!(true));

let query = SearchQuery {
    vector: vec![0.1; 384],
    k: 20,
    filters: Some(filters),
    threshold: None,
    ef_search: None,
};
# let _ = query;
```

## 📊 HNSW Index Configuration

Tune the HNSW index via the builder for your performance/accuracy requirements.

### M Parameter (Connections per Node)

```rust
use ruvector_router_core::VectorDB;
# fn build(b: ruvector_router_core::vector_db::VectorDbBuilder) {}
// Lower M = faster build, less memory, lower recall
let _ = VectorDB::builder().hnsw_m(16);
// Default
let _ = VectorDB::builder().hnsw_m(32);
// Higher M = slower build, more memory, higher recall
let _ = VectorDB::builder().hnsw_m(64);
```

### ef_construction (Build-Time Accuracy)

```rust
use ruvector_router_core::VectorDB;
let _ = VectorDB::builder().hnsw_ef_construction(100); // fast build
let _ = VectorDB::builder().hnsw_ef_construction(200); // default
let _ = VectorDB::builder().hnsw_ef_construction(400); // high recall
```

### ef_search (Query-Time Accuracy)

`ef_search` can be overridden per query via `SearchQuery::ef_search`:

```rust
use ruvector_router_core::SearchQuery;
# let query_vec = vec![0.0f32; 384];
let query_fast = SearchQuery {
    vector: query_vec.clone(),
    k: 10,
    filters: None,
    threshold: None,
    ef_search: Some(50),   // Lower = faster, lower recall
};

let query_accurate = SearchQuery {
    vector: query_vec,
    k: 10,
    filters: None,
    threshold: None,
    ef_search: Some(200),  // Higher = more accurate
};
# let _ = (query_fast, query_accurate);
```

## 🎯 Use Cases

### RAG (Retrieval-Augmented Generation)

Fast context retrieval for LLMs:

```rust
use ruvector_router_core::{VectorDB, VectorEntry, SearchQuery};
use std::collections::HashMap;

# fn ingest(db: &VectorDB, documents: Vec<(String, Vec<f32>, HashMap<String, serde_json::Value>)>) -> ruvector_router_core::Result<()> {
// Store document embeddings
for (id, embedding, metadata) in documents {
    db.insert(VectorEntry {
        id,
        vector: embedding,
        metadata,
        timestamp: chrono::Utc::now().timestamp(),
    })?;
}
# Ok(())
# }

# fn retrieve(db: &VectorDB, query_embedding: Vec<f32>) -> ruvector_router_core::Result<()> {
// Retrieve relevant context for a query
let context_docs = db.search(SearchQuery {
    vector: query_embedding,
    k: 5,
    filters: None,
    threshold: Some(0.7),
    ef_search: None,
})?;
# let _ = context_docs;
# Ok(())
# }
```

### Agent Memory Systems

Store and retrieve agent observations:

```rust
use ruvector_router_core::{VectorDB, VectorEntry, SearchQuery};
use std::collections::HashMap;

struct AgentMemory {
    db: VectorDB,
}

impl AgentMemory {
    fn remember(&self, observation: &str, context: Vec<f32>) -> ruvector_router_core::Result<String> {
        let mut metadata = HashMap::new();
        metadata.insert("observation".to_string(), serde_json::json!(observation));
        self.db.insert(VectorEntry {
            id: uuid::Uuid::new_v4().to_string(),
            vector: context,
            metadata,
            timestamp: chrono::Utc::now().timestamp(),
        })
    }

    fn recall(&self, query_context: Vec<f32>, k: usize) -> ruvector_router_core::Result<Vec<String>> {
        let results = self.db.search(SearchQuery {
            vector: query_context,
            k,
            filters: None,
            threshold: None,
            ef_search: None,
        })?;

        Ok(results
            .iter()
            .filter_map(|r| r.metadata.get("observation"))
            .filter_map(|v| v.as_str())
            .map(|s| s.to_string())
            .collect())
    }
}
```

## 🧪 Building and Testing

```bash
# Build
cargo build --release -p ruvector-router-core

# Test
cargo test -p ruvector-router-core

# Benchmark
cargo bench -p ruvector-router-core
```

## 📚 API Reference

### Core Types (re-exported at the crate root)

- **`VectorDB`**: main database interface
- **`VectorEntry`**: vector with `id`, `vector`, `metadata`, `timestamp`
- **`SearchQuery`**: `vector`, `k`, `filters`, `threshold`, `ef_search`
- **`SearchResult`**: `id`, `score`, `metadata`, `vector`
- **`DistanceMetric`**: `Euclidean | Cosine | DotProduct | Manhattan`
- **`Result`** / **`VectorDbError`**: error handling

### Key Methods

```rust
// VectorDB (see src/vector_db.rs)
pub fn new(config: VectorDbConfig) -> Result<Self>
pub fn builder() -> VectorDbBuilder
pub fn insert(&self, entry: VectorEntry) -> Result<String>
pub fn insert_batch(&self, entries: Vec<VectorEntry>) -> Result<Vec<String>>
pub fn search(&self, query: SearchQuery) -> Result<Vec<SearchResult>>
pub fn delete(&self, id: &str) -> Result<bool>
pub fn get(&self, id: &str) -> Result<Option<VectorEntry>>
pub fn stats(&self) -> VectorDbStats
pub fn count(&self) -> Result<usize>
pub fn get_all_ids(&self) -> Result<Vec<String>>

// Distance helpers (module `distance`)
pub fn calculate_distance(a: &[f32], b: &[f32], metric: DistanceMetric) -> Result<f32>
pub fn batch_distance(query: &[f32], vectors: &[Vec<f32>], metric: DistanceMetric) -> Result<Vec<f32>>
```

## 🚧 Planned / WIP

The following are present in the configuration surface but **not yet wired into
storage/search**. They are documented here for transparency and may change:

- **Quantization** — `QuantizationType` (`None | Scalar | Product { subspaces, k } |
  Binary`) and the builder's `.quantization(..)` setter exist, along with the
  `quantization` module's `quantize` / `dequantize` /
  `calculate_compression_ratio` helpers. However, the configured quantization is
  **not** currently applied inside the index or storage layers — inserts and
  searches operate on full-precision `f32` vectors. Treat compression ratios as
  theoretical until this is integrated.
- **Neural / semantic routing strategies** — earlier drafts of this README
  documented a `routing::` module (request routers, load balancers, adaptive
  strategies). **No such module exists in this crate.** Routing is out of scope
  for `ruvector-router-core`, which is purely the vector storage/search engine.

## 🔗 Links

- **Main Repository**: [github.com/ruvnet/ruvector](https://github.com/ruvnet/ruvector)
- **Examples**: [examples/](../../examples/)

## 📊 Related Crates

- **`ruvector-raft`**: Raft consensus for distributed metadata
- **`ruvector-replication`**: Multi-node replication and synchronization

## 📜 License

MIT License - see [LICENSE](../../LICENSE) for details.

## 🙏 Acknowledgments

Built with:

- **HNSW**: Hierarchical Navigable Small World algorithm
- **redb**: Embedded database for persistent storage
- **parking_lot**: High-performance synchronization primitives

---

<div align="center">

**Part of the [Ruvector](https://github.com/ruvnet/ruvector) ecosystem**

Built by [rUv](https://ruv.io) • MIT Licensed

</div>
