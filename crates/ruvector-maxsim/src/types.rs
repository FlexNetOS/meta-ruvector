//! Core types for multi-vector MaxSim late interaction search.

use serde::{Deserialize, Serialize};

/// Opaque document identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct DocId(pub u64);

/// A single embedding vector stored as f32.
pub type Embedding = Vec<f32>;

/// A document represented by one or more token/chunk embeddings.
///
/// Each entry in `vecs` is a separate embedding: a sentence, a paragraph
/// chunk, or a ColBERT-style token projection. Similarity is computed with
/// MaxSim aggregation rather than averaging.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiVecDoc {
    /// Opaque identifier for this document.
    pub id: DocId,
    /// Token or chunk embeddings that represent this document.
    pub vecs: Vec<Embedding>,
}

/// A query likewise represented by one or more token embeddings.
#[derive(Debug, Clone)]
pub struct MultiVecQuery {
    /// Token embeddings that represent the query.
    pub vecs: Vec<Embedding>,
}

/// One ranked result returned from a MaxSim search.
#[derive(Debug, Clone, PartialEq)]
pub struct SearchResult {
    /// Identifier of the matched document.
    pub doc_id: DocId,
    /// Sum of per-query-token max cosine similarities over all document tokens.
    pub score: f32,
}

impl Eq for SearchResult {}

impl PartialOrd for SearchResult {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for SearchResult {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Higher score = better rank (reverse for BinaryHeap)
        other
            .score
            .partial_cmp(&self.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    }
}

/// Statistics from a benchmark or search run.
#[derive(Debug, Clone, Default)]
pub struct RunStats {
    /// Human-readable name of the index variant (e.g. `"FlatMaxSim"`).
    pub variant: String,
    /// Number of documents in the index.
    pub n_docs: usize,
    /// Total number of token vectors across all indexed documents.
    pub n_token_vecs: usize,
    /// Embedding dimension.
    pub dims: usize,
    /// Number of queries used in this run.
    pub n_queries: usize,
    /// Mean per-query latency in microseconds.
    pub mean_latency_us: f64,
    /// Median (p50) per-query latency in microseconds.
    pub p50_latency_us: f64,
    /// 95th-percentile per-query latency in microseconds.
    pub p95_latency_us: f64,
    /// Queries per second (throughput).
    pub throughput_qps: f64,
    /// Recall@k against the flat (oracle) ground truth.
    pub recall_at_k: f64,
    /// Approximate memory footprint of the index in bytes.
    pub memory_bytes: usize,
}
