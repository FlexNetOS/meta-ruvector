//! Quantized flat (brute-force) index — trades HNSW's sublinear search for
//! quantized in-memory storage (4x–32x reduction of the index's RAM footprint).
//!
//! This is the index path that makes [`crate::types::QuantizationConfig`]
//! actually reduce memory (issue #563). Where [`crate::index::flat::FlatIndex`]
//! holds a full-precision `Vec<f32>` per vector, this index holds only the
//! quantized codes:
//!
//! | Quantizer | Stored per dim | Index reduction vs `f32` |
//! |-----------|----------------|--------------------------|
//! | Scalar    | 1 byte (u8)    | ~4x                      |
//! | Binary    | 1 bit          | ~32x                     |
//!
//! ## Asymmetric distance
//!
//! Search keeps the **query** in full precision and dequantizes each stored
//! vector on the fly, then evaluates the database's configured
//! [`DistanceMetric`]. This (a) preserves the metric exactly — cosine stays
//! cosine, not the L2/Hamming baked into the quantizers' own `distance()` — and
//! (b) gives higher recall than symmetric quantized-to-quantized comparison.
//! The storage layer (redb / memory) is untouched, so `VectorDB::get` still
//! returns the original lossless vector; only the in-RAM search structure is
//! compressed.
//!
//! Product quantization is intentionally **not** handled here: it requires
//! training a codebook over the full corpus up front, which the streaming
//! `add()` interface cannot supply. See
//! [`crate::advanced_features::product_quantization`] for that path.

use crate::distance::distance;
use crate::error::{Result, RuvectorError};
use crate::index::VectorIndex;
use crate::quantization::{BinaryQuantized, QuantizedVector, ScalarQuantized};
use crate::types::{DistanceMetric, SearchResult, VectorId};
use dashmap::DashMap;

#[cfg(all(feature = "parallel", not(target_arch = "wasm32")))]
use rayon::prelude::*;

/// Per-vector quantizer used by [`QuantizedFlatIndex`].
///
/// Only the quantizers that need no corpus-wide training are supported, so they
/// can be applied in a streaming `add()`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuantKind {
    /// Scalar (int8) quantization — ~4x reduction, highest recall.
    Scalar,
    /// Binary (sign) quantization — ~32x reduction, lossiest.
    Binary,
}

/// A stored quantized vector.
enum Stored {
    Scalar(ScalarQuantized),
    Binary(BinaryQuantized),
}

impl Stored {
    #[inline]
    fn reconstruct(&self) -> Vec<f32> {
        match self {
            Stored::Scalar(q) => q.reconstruct(),
            Stored::Binary(q) => q.reconstruct(),
        }
    }

    /// Heap bytes used by the quantized payload (excludes per-entry map overhead).
    #[inline]
    fn payload_bytes(&self) -> usize {
        match self {
            // u8 codes + min/scale (2 × f32).
            Stored::Scalar(q) => q.data.len() + 2 * std::mem::size_of::<f32>(),
            // packed bits + dimensions (usize).
            Stored::Binary(q) => q.bits.len() + std::mem::size_of::<usize>(),
        }
    }
}

/// Brute-force index over quantized vectors.
pub struct QuantizedFlatIndex {
    vectors: DashMap<VectorId, Stored>,
    metric: DistanceMetric,
    dimensions: usize,
    kind: QuantKind,
}

impl QuantizedFlatIndex {
    /// Create a new quantized flat index.
    pub fn new(dimensions: usize, metric: DistanceMetric, kind: QuantKind) -> Self {
        Self {
            vectors: DashMap::new(),
            metric,
            dimensions,
            kind,
        }
    }

    /// The quantizer this index stores with.
    pub fn kind(&self) -> QuantKind {
        self.kind
    }

    fn quantize(&self, vector: &[f32]) -> Stored {
        match self.kind {
            QuantKind::Scalar => Stored::Scalar(ScalarQuantized::quantize(vector)),
            QuantKind::Binary => Stored::Binary(BinaryQuantized::quantize(vector)),
        }
    }

    /// Total heap bytes used by the quantized payloads.
    ///
    /// Useful for verifying the compression ratio in tests/benchmarks. Excludes
    /// `DashMap`/key overhead so it reflects the vector payload only — compare
    /// against `count * dimensions * size_of::<f32>()` for the full-precision
    /// baseline.
    pub fn quantized_bytes(&self) -> usize {
        self.vectors.iter().map(|e| e.value().payload_bytes()).sum()
    }

    fn check_dim(&self, vector: &[f32]) -> Result<()> {
        if vector.len() != self.dimensions {
            return Err(RuvectorError::DimensionMismatch {
                expected: self.dimensions,
                actual: vector.len(),
            });
        }
        Ok(())
    }
}

impl VectorIndex for QuantizedFlatIndex {
    fn add(&mut self, id: VectorId, vector: Vec<f32>) -> Result<()> {
        self.check_dim(&vector)?;
        self.vectors.insert(id, self.quantize(&vector));
        Ok(())
    }

    fn search(&self, query: &[f32], k: usize) -> Result<Vec<SearchResult>> {
        if k == 0 {
            return Ok(vec![]);
        }
        self.check_dim(query)?;

        // Asymmetric distance: query stays f32, stored vectors are dequantized,
        // then the configured metric is evaluated.
        #[cfg(all(feature = "parallel", not(target_arch = "wasm32")))]
        let mut results: Vec<_> = self
            .vectors
            .iter()
            .par_bridge()
            .map(|entry| {
                let recon = entry.value().reconstruct();
                let dist = distance(query, &recon, self.metric)?;
                Ok((entry.key().clone(), dist))
            })
            .collect::<Result<Vec<_>>>()?;

        #[cfg(any(not(feature = "parallel"), target_arch = "wasm32"))]
        let mut results: Vec<_> = self
            .vectors
            .iter()
            .map(|entry| {
                let recon = entry.value().reconstruct();
                let dist = distance(query, &recon, self.metric)?;
                Ok((entry.key().clone(), dist))
            })
            .collect::<Result<Vec<_>>>()?;

        results.sort_unstable_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(k);

        Ok(results
            .into_iter()
            .map(|(id, score)| SearchResult {
                id,
                score,
                vector: None,
                metadata: None,
            })
            .collect())
    }

    fn remove(&mut self, id: &VectorId) -> Result<bool> {
        Ok(self.vectors.remove(id).is_some())
    }

    fn len(&self) -> usize {
        self.vectors.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scalar_index_basic_search() -> Result<()> {
        let mut index = QuantizedFlatIndex::new(3, DistanceMetric::Euclidean, QuantKind::Scalar);
        index.add("v1".to_string(), vec![1.0, 0.0, 0.0])?;
        index.add("v2".to_string(), vec![0.0, 1.0, 0.0])?;
        index.add("v3".to_string(), vec![0.0, 0.0, 1.0])?;

        let results = index.search(&[1.0, 0.0, 0.0], 2)?;
        assert_eq!(results.len(), 2);
        assert_eq!(
            results[0].id, "v1",
            "scalar-quantized search must rank exact match first"
        );
        Ok(())
    }

    #[test]
    fn test_k_zero_returns_empty() -> Result<()> {
        let mut index = QuantizedFlatIndex::new(3, DistanceMetric::Euclidean, QuantKind::Scalar);
        index.add("v1".to_string(), vec![1.0, 0.0, 0.0])?;
        assert!(index.search(&[1.0, 0.0, 0.0], 0)?.is_empty());
        Ok(())
    }

    #[test]
    fn test_dimension_mismatch_rejected() {
        let mut index = QuantizedFlatIndex::new(3, DistanceMetric::Cosine, QuantKind::Scalar);
        assert!(index.add("bad".to_string(), vec![1.0, 2.0]).is_err());
        index.add("ok".to_string(), vec![1.0, 2.0, 3.0]).unwrap();
        assert!(index.search(&[1.0, 2.0], 1).is_err());
    }

    #[test]
    fn test_remove_and_len() -> Result<()> {
        let mut index = QuantizedFlatIndex::new(2, DistanceMetric::Euclidean, QuantKind::Binary);
        index.add("a".to_string(), vec![1.0, 1.0])?;
        index.add("b".to_string(), vec![-1.0, -1.0])?;
        assert_eq!(index.len(), 2);
        assert!(index.remove(&"a".to_string())?);
        assert!(!index.remove(&"a".to_string())?);
        assert_eq!(index.len(), 1);
        Ok(())
    }

    #[test]
    fn test_scalar_quantized_bytes_are_compressed() -> Result<()> {
        let dims = 128;
        let count = 50;
        let mut index = QuantizedFlatIndex::new(dims, DistanceMetric::Cosine, QuantKind::Scalar);
        for i in 0..count {
            let v: Vec<f32> = (0..dims).map(|d| ((i + d) % 7) as f32).collect();
            index.add(format!("v{i}"), v)?;
        }
        let f32_baseline = count * dims * std::mem::size_of::<f32>();
        let quantized = index.quantized_bytes();
        // Scalar: ~1 byte/dim vs 4 → expect well under half the baseline.
        assert!(
            quantized * 2 < f32_baseline,
            "scalar index payload {quantized} should be far below f32 baseline {f32_baseline}"
        );
        Ok(())
    }

    #[test]
    fn test_binary_is_more_compressed_than_scalar() -> Result<()> {
        let dims = 256;
        let build = |kind| -> Result<usize> {
            let mut idx = QuantizedFlatIndex::new(dims, DistanceMetric::Cosine, kind);
            for i in 0..20 {
                let v: Vec<f32> = (0..dims).map(|d| ((i + d) % 5) as f32 - 2.0).collect();
                idx.add(format!("v{i}"), v)?;
            }
            Ok(idx.quantized_bytes())
        };
        let scalar = build(QuantKind::Scalar)?;
        let binary = build(QuantKind::Binary)?;
        assert!(
            binary < scalar,
            "binary payload {binary} should be smaller than scalar payload {scalar}"
        );
        Ok(())
    }
}
