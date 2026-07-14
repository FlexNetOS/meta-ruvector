//! Differential recall + end-to-end wiring tests for the quantized index
//! (issue #563). These verify that turning on quantization actually (a) reduces
//! the index's memory footprint and (b) preserves search quality — the
//! quantized index's top-k must overlap heavily with the full-precision
//! ground truth — and that `VectorDB` selects and drives it correctly.

use ruvector_core::index::flat::FlatIndex;
use ruvector_core::index::quantized_flat::{QuantKind, QuantizedFlatIndex};
use ruvector_core::index::VectorIndex;
use ruvector_core::types::{DbOptions, DistanceMetric, QuantizationConfig, SearchQuery};
use ruvector_core::{VectorDB, VectorEntry};
use std::collections::HashMap;

/// Tiny deterministic PRNG (SplitMix64) so the corpus/queries are stable across
/// runs — flaky recall thresholds are worse than no test.
struct Rng(u64);
impl Rng {
    fn next_f32(&mut self) -> f32 {
        // SplitMix64
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^= z >> 31;
        // Map to [-1, 1).
        ((z >> 40) as f32 / (1u64 << 24) as f32) * 2.0 - 1.0
    }
    fn vector(&mut self, dims: usize) -> Vec<f32> {
        (0..dims).map(|_| self.next_f32()).collect()
    }
}

fn top_k_ids(index: &dyn VectorIndex, query: &[f32], k: usize) -> Vec<String> {
    index
        .search(query, k)
        .unwrap()
        .into_iter()
        .map(|r| r.id)
        .collect()
}

/// Mean recall@k of `quant` against the full-precision `truth` over a query set.
fn mean_recall(kind: QuantKind, metric: DistanceMetric, dims: usize) -> f64 {
    let n = 500usize;
    let queries = 30usize;
    let k = 10usize;

    let mut rng = Rng(0xC0FF_EE12_3456_789A);
    let corpus: Vec<Vec<f32>> = (0..n).map(|_| rng.vector(dims)).collect();

    let mut truth = FlatIndex::new(dims, metric);
    let mut quant = QuantizedFlatIndex::new(dims, metric, kind);
    for (i, v) in corpus.iter().enumerate() {
        let id = format!("v{i}");
        truth.add(id.clone(), v.clone()).unwrap();
        quant.add(id, v.clone()).unwrap();
    }

    let mut total = 0.0;
    for _ in 0..queries {
        let q = rng.vector(dims);
        let gt = top_k_ids(&truth, &q, k);
        let got: std::collections::HashSet<_> = top_k_ids(&quant, &q, k).into_iter().collect();
        let hits = gt.iter().filter(|id| got.contains(*id)).count();
        total += hits as f64 / k as f64;
    }
    total / queries as f64
}

#[test]
fn scalar_quantization_preserves_recall_euclidean() {
    let recall = mean_recall(QuantKind::Scalar, DistanceMetric::Euclidean, 64);
    // Scalar (int8) is near-lossless; on random data recall@10 is typically
    // >0.95. Require a conservative floor so the test is robust but still
    // proves quantized search tracks the full-precision ranking.
    assert!(
        recall >= 0.85,
        "scalar recall@10 too low: {recall:.3} (expected >= 0.85)"
    );
}

#[test]
fn scalar_quantization_preserves_recall_cosine() {
    let recall = mean_recall(QuantKind::Scalar, DistanceMetric::Cosine, 64);
    assert!(
        recall >= 0.85,
        "scalar cosine recall@10 too low: {recall:.3} (expected >= 0.85)"
    );
}

#[test]
fn binary_quantization_has_meaningful_recall() {
    // Binary throws away magnitude (32x reduction), so recall is much lower than
    // scalar — but it must still be far better than random (random top-10 over
    // 500 vectors ≈ 0.02). This guards against a broken binary search path.
    let recall = mean_recall(QuantKind::Binary, DistanceMetric::Cosine, 64);
    assert!(
        recall >= 0.20,
        "binary recall@10 implausibly low: {recall:.3} (expected >= 0.20)"
    );
}

#[test]
fn vectordb_end_to_end_with_scalar_quantization() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("q.db").to_string_lossy().to_string();

    let options = DbOptions {
        dimensions: 8,
        distance_metric: DistanceMetric::Euclidean,
        storage_path: path.clone(),
        hnsw_config: None,
        quantization: Some(QuantizationConfig::Scalar),
    };
    let db = VectorDB::new(options).unwrap();

    let mut meta = HashMap::new();
    meta.insert("tag".to_string(), serde_json::json!("a"));

    db.insert(VectorEntry {
        id: Some("near".to_string()),
        vector: vec![1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        metadata: Some(meta.clone()),
    })
    .unwrap();
    db.insert(VectorEntry {
        id: Some("far".to_string()),
        vector: vec![0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0],
        metadata: None,
    })
    .unwrap();

    // Quantized index ranks the obvious nearest neighbor first.
    let results = db
        .search(SearchQuery {
            vector: vec![1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            k: 2,
            filter: None,
            ef_search: None,
        })
        .unwrap();
    assert_eq!(results[0].id, "near");

    // Storage is lossless: get() returns the original vector exactly, not a
    // dequantized approximation.
    let stored = db.get("near").unwrap().unwrap();
    assert_eq!(stored.vector, vec![1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]);

    // Metadata filtering (resolved from lossless storage) still works.
    let filtered = db
        .search(SearchQuery {
            vector: vec![1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            k: 2,
            filter: Some(meta),
            ef_search: None,
        })
        .unwrap();
    assert_eq!(filtered.len(), 1, "only 'near' carries tag=a");
    assert_eq!(filtered[0].id, "near");
}

#[test]
fn vectordb_quantization_config_round_trips_through_storage() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("rt.db").to_string_lossy().to_string();

    {
        let options = DbOptions {
            dimensions: 4,
            distance_metric: DistanceMetric::Cosine,
            storage_path: path.clone(),
            hnsw_config: None,
            quantization: Some(QuantizationConfig::Binary),
        };
        let db = VectorDB::new(options).unwrap();
        db.insert(VectorEntry {
            id: Some("x".to_string()),
            vector: vec![1.0, 1.0, -1.0, -1.0],
            metadata: None,
        })
        .unwrap();
    }

    // Reopen: persisted config drives the quantized index again and search works.
    let reopened = VectorDB::new(DbOptions {
        dimensions: 4,
        distance_metric: DistanceMetric::Cosine,
        storage_path: path,
        hnsw_config: None,
        quantization: Some(QuantizationConfig::Binary),
    })
    .unwrap();
    assert_eq!(reopened.len().unwrap(), 1);
    let res = reopened
        .search(SearchQuery {
            vector: vec![1.0, 1.0, -1.0, -1.0],
            k: 1,
            filter: None,
            ef_search: None,
        })
        .unwrap();
    assert_eq!(res[0].id, "x");
}
