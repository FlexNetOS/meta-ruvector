# Ruvector Metrics

[![Crates.io](https://img.shields.io/crates/v/ruvector-metrics.svg)](https://crates.io/crates/ruvector-metrics)
[![Documentation](https://docs.rs/ruvector-metrics/badge.svg)](https://docs.rs/ruvector-metrics)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-1.77%2B-orange.svg)](https://www.rust-lang.org)

**Prometheus-compatible metrics collection for Ruvector vector databases.**

`ruvector-metrics` provides Prometheus-backed observability with counters, gauges, and histograms for monitoring Ruvector performance and health. Part of the [Ruvector](https://github.com/ruvnet/ruvector) ecosystem.

## Why Ruvector Metrics?

- **Prometheus Native**: Built on the `prometheus` crate with a global registry
- **Zero Overhead**: `lazy_static` metric handles, minimal impact
- **Operation Coverage**: Search / insert / delete counts, latencies, vector counts
- **Standard Format**: Prometheus text exposition via `gather_metrics()`
- **Health Checks**: Liveness / readiness helpers in the `health` module

## Features

### Core Metrics

- **Operation Counters**: Search, insert, delete counts (labelled by collection + status)
- **Latency Histograms**: Search and insert latency in seconds
- **Vector / Collection Gauges**: `set_vectors_count`, `set_collections_count`
- **Memory Gauge**: `set_memory_usage`
- **Health**: `HealthChecker`, `HealthResponse`, `ReadinessResponse`

### Planned / Not Yet Implemented

These are **not** present in the current code:

- **Custom labels / metric groups** beyond the built-in collection/status labels
- **JSON export** (only Prometheus text format via `gather_metrics()`)
- **Built-in metrics HTTP server** (expose `gather_metrics()` from your own server)
- **HNSW / quantization index metrics**

## Installation

Add `ruvector-metrics` to your `Cargo.toml`:

```toml
[dependencies]
ruvector-metrics = "0.1.1"
```

## Quick Start

Metrics are global. There is no config object or handle to construct — record via the
`MetricsRecorder` unit struct's associated functions, then export with `gather_metrics()`.

### Record Metrics

```rust
use ruvector_metrics::MetricsRecorder;

// Record a search: collection, latency in seconds, success
MetricsRecorder::record_search("documents", 0.0012, true);

// Record an insert: collection, latency in seconds, count, success
MetricsRecorder::record_insert("documents", 0.0030, 100, true);

// Record a delete: collection, success
MetricsRecorder::record_delete("documents", true);

// Update gauges
MetricsRecorder::set_vectors_count("documents", 10_000);
MetricsRecorder::set_collections_count(3);
MetricsRecorder::set_memory_usage(1024 * 1024 * 500); // 500 MB

// Record a batch in one call: collection, searches, inserts, deletes
MetricsRecorder::record_batch("documents", 100, 50, 10);
```

### Export Metrics

```rust
use ruvector_metrics::gather_metrics;

// Prometheus text-exposition format for the global registry
let output: String = gather_metrics();
println!("{output}");
```

Expose this from your own HTTP handler (e.g. a `/metrics` route) — this crate does not
ship a metrics server.

## Available Metrics

```text
# Counters
ruvector_search_requests_total      # Total search requests   (labels: collection, status)
ruvector_insert_requests_total      # Total insert requests   (labels: collection, status)
ruvector_delete_requests_total      # Total delete requests   (labels: collection, status)
ruvector_vectors_inserted_total     # Total vectors inserted  (label: collection)
ruvector_uptime_seconds             # Uptime counter

# Histograms
ruvector_search_latency_seconds     # Search latency  (label: collection)
ruvector_insert_latency_seconds     # Insert latency  (label: collection)

# Gauges
ruvector_vectors_total              # Current vector count    (label: collection)
ruvector_collections_total          # Number of collections
ruvector_memory_usage_bytes         # Memory usage in bytes
```

## API Overview

### Recorder

```rust
// Unit struct — all functions are associated (no instance/config needed)
pub struct MetricsRecorder;

impl MetricsRecorder {
    pub fn record_search(collection: &str, latency_secs: f64, success: bool);
    pub fn record_insert(collection: &str, latency_secs: f64, count: usize, success: bool);
    pub fn record_delete(collection: &str, success: bool);
    pub fn set_vectors_count(collection: &str, count: usize);
    pub fn set_collections_count(count: usize);
    pub fn set_memory_usage(bytes: usize);
    pub fn record_batch(collection: &str, searches: usize, inserts: usize, deletes: usize);
}
```

### Export & Health

```rust
// Prometheus text-format dump of the global registry
pub fn gather_metrics() -> String;

// Health module exports
pub use ruvector_metrics::{
    HealthChecker, HealthResponse, HealthStatus,
    ReadinessResponse, CollectionHealth,
};
```

## Grafana Dashboard

Example Grafana queries:

```promql
# Search request rate
rate(ruvector_search_requests_total[5m])

# p99 search latency
histogram_quantile(0.99, rate(ruvector_search_latency_seconds_bucket[5m]))

# Memory usage (MB)
ruvector_memory_usage_bytes / 1024 / 1024

# Search error rate
rate(ruvector_search_requests_total{status="error"}[5m])
  / rate(ruvector_search_requests_total[5m])
```

## Related Crates

- **[ruvector-core](../ruvector-core/)** - Core vector database engine
- **[ruvector-server](../ruvector-server/)** - REST API server

## Documentation

- **[Main README](../../README.md)** - Complete project overview
- **[API Documentation](https://docs.rs/ruvector-metrics)** - Full API reference
- **[GitHub Repository](https://github.com/ruvnet/ruvector)** - Source code

## License

**MIT License** - see [LICENSE](../../LICENSE) for details.

---

<div align="center">

**Part of [Ruvector](https://github.com/ruvnet/ruvector) - Built by [rUv](https://ruv.io)**

[![Star on GitHub](https://img.shields.io/github/stars/ruvnet/ruvector?style=social)](https://github.com/ruvnet/ruvector)

[Documentation](https://docs.rs/ruvector-metrics) | [Crates.io](https://crates.io/crates/ruvector-metrics) | [GitHub](https://github.com/ruvnet/ruvector)

</div>
