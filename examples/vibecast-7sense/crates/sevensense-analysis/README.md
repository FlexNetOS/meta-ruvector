# sevensense-analysis

[![Crate](https://img.shields.io/badge/crates.io-sevensense--analysis-orange.svg)](https://crates.io/crates/sevensense-analysis)
[![Docs](https://img.shields.io/badge/docs-sevensense--analysis-blue.svg)](https://docs.rs/sevensense-analysis)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](../../LICENSE)

> Advanced acoustic analysis algorithms for bioacoustic pattern discovery.

**sevensense-analysis** provides analysis services for understanding bird vocalizations at scale. From clustering calls into groups, detecting recurring motifs, to modeling temporal patterns, it transforms raw embeddings into actionable ecological insights.

The crate follows Domain-Driven Design (DDD): domain entities and value objects, application services that orchestrate them, and infrastructure implementations. The public surface is built around four application services:

- `ClusteringService` — HDBSCAN / K-means clustering of embeddings into call types
- `MotifDetectionService` — recurring-pattern discovery over cluster-id sequences
- `SequenceAnalysisService` — transition metrics and entropy over cluster sequences
- `AnomalyDetectionService` — k-NN distance-based outlier detection

## Features

- **HDBSCAN & K-means Clustering**: Group similar vocalizations via `ClusteringService`
- **Motif Detection**: Find recurring patterns in cluster-id sequences via `MotifDetectionService`
- **Sequence Analysis**: Transition metrics and entropy via `SequenceAnalysisService`
- **Anomaly Detection**: k-NN distance-based outlier detection via `AnomalyDetectionService`
- **Clustering Metrics**: Silhouette score and V-measure via the `metrics` module

## Use Cases

| Use Case | Description | Key API |
|----------|-------------|---------|
| Call-Type Clustering | Group similar vocalizations | `ClusteringService::run_hdbscan()` |
| K-means Clustering | Cluster into a fixed number of groups | `ClusteringService::run_kmeans()` |
| Motif Discovery | Find repeated cluster-id patterns | `MotifDetectionService::detect_motifs()` |
| Sequence Analysis | Transition metrics & entropy | `SequenceAnalysisService::analyze_sequence()` |
| Anomaly Detection | Find unusual calls | `AnomalyDetectionService::detect_anomalies()` |

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
sevensense-analysis = "0.1"
```

## Quick Start

`ClusteringService` operates on `EmbeddingWithId` values, which are simply
`(EmbeddingId, Vec<f32>)` tuples, and returns `Vec<Cluster>`.

```rust,ignore
use sevensense_analysis::{ClusteringService, ClusteringConfig, EmbeddingId};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Configure HDBSCAN: min_cluster_size = 5, min_samples = 3
    let service = ClusteringService::new(ClusteringConfig::hdbscan(5, 3));

    // Embeddings are (EmbeddingId, Vec<f32>) tuples
    let embeddings: Vec<(EmbeddingId, Vec<f32>)> = load_embeddings();

    let clusters = service.run_hdbscan(&embeddings).await?;
    println!("Found {} call types", clusters.len());

    Ok(())
}
```

`ClusteringConfig` also exposes `::kmeans(k)`, `::default()`, and a `.with_seed(seed)`
builder method. The default service can be created with `ClusteringService::default_service()`.

---

<details>
<summary><b>Tutorial: HDBSCAN & K-means Clustering</b></summary>

### HDBSCAN Clustering

```rust,ignore
use sevensense_analysis::{ClusteringService, ClusteringConfig};

// HDBSCAN with min_cluster_size = 5, min_samples = 3
let service = ClusteringService::new(ClusteringConfig::hdbscan(5, 3));

let clusters = service.run_hdbscan(&embeddings).await?;

for cluster in &clusters {
    println!("Cluster {:?}: {} members", cluster.id, cluster.member_ids.len());
}
```

### K-means Clustering

```rust,ignore
use sevensense_analysis::{ClusteringService, ClusteringConfig};

// Configure for K-means, then ask for k = 8 clusters at call time
let service = ClusteringService::new(ClusteringConfig::kmeans(8));

let clusters = service.run_kmeans(&embeddings, 8).await?;
println!("Created {} clusters", clusters.len());
```

### Reproducible Runs

```rust,ignore
use sevensense_analysis::{ClusteringService, ClusteringConfig};

// Seed the configuration for reproducible clustering
let config = ClusteringConfig::hdbscan(5, 3).with_seed(42);
let service = ClusteringService::new(config);
```

</details>

<details>
<summary><b>Tutorial: Motif Detection</b></summary>

`MotifDetectionService` discovers recurring patterns in sequences of `ClusterId`
values (e.g. the cluster ids assigned to a temporally ordered set of segments).

```rust,ignore
use sevensense_analysis::{MotifDetectionService, MotifConfig, ClusterId};

let service = MotifDetectionService::new(MotifConfig::default());
// or: MotifDetectionService::default_service()

// Sequences of cluster ids (call types) per recording
let sequences: Vec<Vec<ClusterId>> = load_cluster_sequences();

// Detect motifs with a minimum length of 3
let motifs = service.detect_motifs(&sequences, 3).await?;

for motif in &motifs {
    println!("Motif with {} occurrences", motif.occurrences);
}
```

</details>

<details>
<summary><b>Tutorial: Sequence Analysis</b></summary>

`SequenceAnalysisService` builds a `SequenceAnalysis` from an ordered set of
segments and their cluster assignments, and can compute entropy and
`SequenceMetrics`.

```rust,ignore
use std::collections::HashMap;
use sevensense_analysis::{SequenceAnalysisService, SegmentId, ClusterId, RecordingId};

let service = SequenceAnalysisService::new();
// (also available via `SequenceAnalysisService::default()`)

let segment_ids: Vec<SegmentId> = load_ordered_segments();
let assignments: HashMap<SegmentId, ClusterId> = load_assignments();
let recording_id: RecordingId = current_recording();

let analysis = service.analyze_sequence(&segment_ids, &assignments, recording_id).await?;

// Entropy over a list of (from, to, weight) transitions
let entropy = service.compute_entropy(&transitions);
println!("Sequence entropy: {:.3}", entropy);

// Aggregate metrics over a cluster sequence
let metrics = service.compute_metrics(&cluster_sequence).await?;
```

</details>

<details>
<summary><b>Tutorial: Anomaly Detection</b></summary>

`AnomalyDetectionService` flags embeddings that lie far from existing clusters
using a distance threshold and a k-nearest-neighbors count.

```rust,ignore
use sevensense_analysis::AnomalyDetectionService;

// threshold = 0.8, k_neighbors = 5
let service = AnomalyDetectionService::new(0.8, 5);
// or: AnomalyDetectionService::default_service()

let anomalies = service.detect_anomalies(&embeddings, &clusters).await?;

for anomaly in &anomalies {
    // Classify the anomaly given the size of its nearest cluster
    let kind = service.classify_anomaly(&anomaly, cluster_member_count);
    println!("Anomaly {:?}: {:?}", anomaly, kind);
}
```

</details>

<details>
<summary><b>Tutorial: Clustering Metrics</b></summary>

The `metrics` module exposes scoring helpers re-exported at the crate root.

```rust,ignore
use sevensense_analysis::{ClusteringMetrics, SilhouetteScore, VMeasure, SequenceEntropy};

// `ClusteringService::cluster_with_metrics` returns clusters alongside
// `ClusteringMetrics` (silhouette score, V-measure, etc.).
let (clusters, metrics) = service.cluster_with_metrics(&embeddings).await?;
```

</details>

---

## Public API

Re-exported at the crate root (`sevensense_analysis::`):

| Category | Items |
|----------|-------|
| Services | `ClusteringService`, `MotifDetectionService`, `SequenceAnalysisService`, `AnomalyDetectionService` |
| Entities | `Anomaly`, `AnomalyType`, `Cluster`, `ClusterId`, `EmbeddingId`, `Motif`, `MotifOccurrence`, `Prototype`, `RecordingId`, `SegmentId`, `SequenceAnalysis` |
| Value Objects | `ClusteringConfig`, `ClusteringMethod`, `ClusteringParameters`, `MotifConfig`, `SequenceMetrics`, `TransitionMatrix` |
| Events | `AnalysisEvent`, `ClusterAssigned`, `ClustersDiscovered`, `MotifDetected`, `SequenceAnalyzed` |
| Repositories | `ClusterRepository`, `MotifRepository`, `SequenceRepository` |
| Metrics | `ClusteringMetrics`, `SequenceEntropy`, `SilhouetteScore`, `VMeasure` |

A `prelude` module is provided for convenient bulk imports.

### `ClusteringConfig` constructors

| Constructor | Description |
|-------------|-------------|
| `ClusteringConfig::hdbscan(min_cluster_size, min_samples)` | HDBSCAN configuration |
| `ClusteringConfig::kmeans(k)` | K-means configuration |
| `ClusteringConfig::default()` | HDBSCAN defaults |
| `.with_seed(seed)` | Set a random seed (builder) |

## Links

- **Homepage**: [ruv.io](https://ruv.io)
- **Repository**: [github.com/ruvnet/ruvector](https://github.com/ruvnet/ruvector)
- **Crates.io**: [crates.io/crates/sevensense-analysis](https://crates.io/crates/sevensense-analysis)
- **Documentation**: [docs.rs/sevensense-analysis](https://docs.rs/sevensense-analysis)

## License

MIT License - see [LICENSE](../../LICENSE) for details.

---

*Part of the [7sense Bioacoustic Intelligence Platform](https://ruv.io) by rUv*
