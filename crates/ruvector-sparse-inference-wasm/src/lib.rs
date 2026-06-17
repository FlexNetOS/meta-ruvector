//! WASM bindings for ruvector-sparse-inference.
//!
//! Wraps the crate's `SparseEmbeddingProvider` (sparse feature-vector → embedding).
//! NOTE: the underlying crate's embedding API is feature-vector based (`embed(&[f32])`);
//! the older token-id `encode`/`forward_embedding`/`sparsity_statistics` low-level model
//! methods no longer exist upstream, so this binding reconciles to the current
//! `SparseEmbeddingProvider` surface (GGUF load, embed, batch, dim, sparsity stats, calibrate).

use ruvector_sparse_inference::SparseEmbeddingProvider;
use wasm_bindgen::prelude::*;

/// Initialize panic hook for better error messages
#[wasm_bindgen(start)]
pub fn init() {
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
}

/// Sparse inference (embedding) engine for WASM
#[wasm_bindgen]
pub struct SparseInferenceEngine {
    provider: SparseEmbeddingProvider,
}

#[wasm_bindgen]
impl SparseInferenceEngine {
    /// Create a new engine from GGUF model bytes.
    /// `config_json` is accepted for API compatibility; sparsity can be tuned via
    /// `set_sparsity_threshold`.
    #[wasm_bindgen(constructor)]
    pub fn new(model_bytes: &[u8], _config_json: &str) -> Result<SparseInferenceEngine, JsError> {
        let provider = SparseEmbeddingProvider::from_gguf_bytes(model_bytes)
            .map_err(|e| JsError::new(&format!("Failed to load model: {}", e)))?;
        Ok(Self { provider })
    }

    /// Load model with streaming fetch (for large models)
    #[wasm_bindgen]
    pub async fn load_streaming(
        url: &str,
        config_json: &str,
    ) -> Result<SparseInferenceEngine, JsError> {
        let bytes = fetch_model_bytes(url).await?;
        Self::new(&bytes, config_json)
    }

    /// Run sparse embedding inference on a feature vector
    #[wasm_bindgen]
    pub fn infer(&self, input: &[f32]) -> Result<Vec<f32>, JsError> {
        self.provider
            .embed(input)
            .map_err(|e| JsError::new(&format!("Inference failed: {}", e)))
    }

    /// Embedding dimension
    #[wasm_bindgen]
    pub fn dimension(&self) -> usize {
        self.provider.embedding_dim()
    }

    /// Model metadata as JSON (embedding dimension)
    #[wasm_bindgen]
    pub fn metadata(&self) -> String {
        format!("{{\"embedding_dim\":{}}}", self.provider.embedding_dim())
    }

    /// Get sparsity statistics as JSON
    #[wasm_bindgen]
    pub fn sparsity_stats(&self) -> String {
        serde_json::to_string(self.provider.sparsity_stats()).unwrap_or_default()
    }

    /// Set the sparsity threshold
    #[wasm_bindgen]
    pub fn set_sparsity_threshold(&mut self, threshold: f32) {
        self.provider.set_sparsity_threshold(threshold);
    }

    /// Calibrate with sample feature vectors (flattened; each row is `sample_dim` long)
    #[wasm_bindgen]
    pub fn calibrate(&mut self, samples: &[f32], sample_dim: usize) -> Result<(), JsError> {
        let dim = sample_dim.max(1);
        let samples: Vec<Vec<f32>> = samples.chunks(dim).map(|c| c.to_vec()).collect();
        self.provider
            .calibrate(&samples)
            .map_err(|e| JsError::new(&format!("Calibration failed: {}", e)))
    }
}

/// Embedding model wrapper (sparse feature-vector → embedding)
#[wasm_bindgen]
pub struct EmbeddingModel {
    engine: SparseInferenceEngine,
}

#[wasm_bindgen]
impl EmbeddingModel {
    #[wasm_bindgen(constructor)]
    pub fn new(model_bytes: &[u8]) -> Result<EmbeddingModel, JsError> {
        let engine = SparseInferenceEngine::new(model_bytes, "{}")?;
        Ok(Self { engine })
    }

    /// Encode a feature vector to a sparse embedding
    #[wasm_bindgen]
    pub fn encode(&self, features: &[f32]) -> Result<Vec<f32>, JsError> {
        self.engine.infer(features)
    }

    /// Batch encode: flattened feature vectors, each of length `dim`; returns
    /// the concatenated embeddings.
    #[wasm_bindgen]
    pub fn encode_batch(&self, features: &[f32], dim: usize) -> Result<Vec<f32>, JsError> {
        let dim = dim.max(1);
        let mut out = Vec::new();
        for chunk in features.chunks(dim) {
            out.extend(self.engine.infer(chunk)?);
        }
        Ok(out)
    }

    /// Get embedding dimension
    #[wasm_bindgen]
    pub fn dimension(&self) -> usize {
        self.engine.dimension()
    }
}

/// Performance measurement utility
#[wasm_bindgen]
pub fn measure_inference_time(
    engine: &SparseInferenceEngine,
    input: &[f32],
    iterations: u32,
) -> f64 {
    let performance = web_sys::window()
        .and_then(|w| w.performance())
        .expect("Performance API not available");

    let start = performance.now();
    for _ in 0..iterations {
        let _ = engine.infer(input);
    }
    let end = performance.now();

    (end - start) / iterations.max(1) as f64
}

/// Get library version
#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

// Helper for streaming fetch
async fn fetch_model_bytes(url: &str) -> Result<Vec<u8>, JsError> {
    use wasm_bindgen_futures::JsFuture;

    let window = web_sys::window().ok_or_else(|| JsError::new("No window"))?;
    let response = JsFuture::from(window.fetch_with_str(url))
        .await
        .map_err(|_| JsError::new("Fetch failed"))?;
    let response: web_sys::Response = response
        .dyn_into()
        .map_err(|_| JsError::new("Failed to cast to Response"))?;
    let buffer = JsFuture::from(
        response
            .array_buffer()
            .map_err(|_| JsError::new("Failed to get array buffer"))?,
    )
    .await
    .map_err(|_| JsError::new("Failed to read array buffer"))?;
    let array = js_sys::Uint8Array::new(&buffer);
    Ok(array.to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert!(!version().is_empty());
    }
}
