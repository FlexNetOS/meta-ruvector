//! Quantized index tradeoff benchmark: memory vs recall vs search latency for
//! full-precision `f32` (FlatIndex) vs Scalar (~4x) vs Binary (~32x) quantized
//! indexes over the SAME corpus (issue #563 follow-up to the quantized-index
//! wiring).
//!
//! Criterion only times code, so the three axes are reported as follows:
//!   * latency  — measured by criterion (the `quantized_index_search` group)
//!   * memory   — computed exactly (index payload bytes) and printed once
//!   * recall@k — measured against the f32 ground truth and printed once
//!
//! Run: `cargo bench -p ruvector-core --bench quantized_index_tradeoff`
//! The memory/recall table is printed to stderr before the timed runs.

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use ruvector_core::index::flat::FlatIndex;
use ruvector_core::index::quantized_flat::{QuantKind, QuantizedFlatIndex};
use ruvector_core::index::VectorIndex;
use ruvector_core::types::DistanceMetric;
use std::collections::HashSet;

const DIMS: usize = 128;
const N: usize = 5_000;
const K: usize = 10;
const RECALL_QUERIES: usize = 50;
const METRIC: DistanceMetric = DistanceMetric::Cosine;

/// Deterministic SplitMix64 PRNG so the corpus/queries (and therefore the
/// reported numbers) are stable across runs.
struct Rng(u64);
impl Rng {
    fn next_f32(&mut self) -> f32 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^= z >> 31;
        ((z >> 40) as f32 / (1u64 << 24) as f32) * 2.0 - 1.0
    }
    fn vector(&mut self, dims: usize) -> Vec<f32> {
        (0..dims).map(|_| self.next_f32()).collect()
    }
}

fn build_indexes() -> (
    FlatIndex,
    QuantizedFlatIndex,
    QuantizedFlatIndex,
    Vec<Vec<f32>>,
) {
    let mut rng = Rng(0x5EED_1234_ABCD_0001);
    let corpus: Vec<Vec<f32>> = (0..N).map(|_| rng.vector(DIMS)).collect();

    let mut f32_idx = FlatIndex::new(DIMS, METRIC);
    let mut scalar = QuantizedFlatIndex::new(DIMS, METRIC, QuantKind::Scalar);
    let mut binary = QuantizedFlatIndex::new(DIMS, METRIC, QuantKind::Binary);
    for (i, v) in corpus.iter().enumerate() {
        let id = format!("v{i}");
        f32_idx.add(id.clone(), v.clone()).unwrap();
        scalar.add(id.clone(), v.clone()).unwrap();
        binary.add(id, v.clone()).unwrap();
    }

    // Queries for the recall measurement (separate from the corpus).
    let queries: Vec<Vec<f32>> = (0..RECALL_QUERIES).map(|_| rng.vector(DIMS)).collect();
    (f32_idx, scalar, binary, queries)
}

fn recall_at_k(truth: &FlatIndex, quant: &dyn VectorIndex, queries: &[Vec<f32>]) -> f64 {
    let mut total = 0.0;
    for q in queries {
        let gt: HashSet<String> = truth
            .search(q, K)
            .unwrap()
            .into_iter()
            .map(|r| r.id)
            .collect();
        let got = quant.search(q, K).unwrap();
        let hits = got.iter().filter(|r| gt.contains(&r.id)).count();
        total += hits as f64 / K as f64;
    }
    total / queries.len() as f64
}

fn report_memory_and_recall(
    f32_idx: &FlatIndex,
    scalar: &QuantizedFlatIndex,
    binary: &QuantizedFlatIndex,
    queries: &[Vec<f32>],
) {
    // f32 FlatIndex stores a Vec<f32> per vector: N * dims * 4 bytes.
    let f32_bytes = N * DIMS * std::mem::size_of::<f32>();
    let scalar_bytes = scalar.quantized_bytes();
    let binary_bytes = binary.quantized_bytes();

    let scalar_recall = recall_at_k(f32_idx, scalar, queries);
    let binary_recall = recall_at_k(f32_idx, binary, queries);

    eprintln!("\n=== Quantized index tradeoff (N={N}, dims={DIMS}, k={K}, metric={METRIC:?}) ===");
    eprintln!(
        "{:<8} | {:>14} | {:>9} | {:>10}",
        "index", "payload bytes", "vs f32", "recall@10"
    );
    eprintln!("{:-<8}-+-{:-<14}-+-{:-<9}-+-{:-<10}", "", "", "", "");
    eprintln!(
        "{:<8} | {:>14} | {:>9} | {:>10}",
        "f32", f32_bytes, "1.00x", "1.000 (gt)"
    );
    eprintln!(
        "{:<8} | {:>14} | {:>8.1}x | {:>10.3}",
        "scalar",
        scalar_bytes,
        f32_bytes as f64 / scalar_bytes as f64,
        scalar_recall
    );
    eprintln!(
        "{:<8} | {:>14} | {:>8.1}x | {:>10.3}",
        "binary",
        binary_bytes,
        f32_bytes as f64 / binary_bytes as f64,
        binary_recall
    );
    eprintln!("(latency for each is measured by the 'quantized_index_search' group below)\n");
}

fn bench_search(c: &mut Criterion) {
    let (f32_idx, scalar, binary, queries) = build_indexes();
    report_memory_and_recall(&f32_idx, &scalar, &binary, &queries);

    // A fixed representative query for the timed comparison.
    let query = queries[0].clone();

    let mut group = c.benchmark_group("quantized_index_search");
    group.bench_function("f32_flat", |b| {
        b.iter(|| black_box(f32_idx.search(black_box(&query), K).unwrap()))
    });
    group.bench_function("scalar", |b| {
        b.iter(|| black_box(scalar.search(black_box(&query), K).unwrap()))
    });
    group.bench_function("binary", |b| {
        b.iter(|| black_box(binary.search(black_box(&query), K).unwrap()))
    });
    group.finish();
}

criterion_group!(benches, bench_search);
criterion_main!(benches);
