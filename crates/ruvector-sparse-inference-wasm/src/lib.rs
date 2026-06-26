use ruvector_sparse_inference::{
    model::{GgufParser, ModelMetadata},
    InferenceConfig, SparseInferenceEngine as SparseCoreEngine,
};
use wasm_bindgen::prelude::*;

/// Initialize panic hook for better error messages
#[wasm_bindgen(start)]
pub fn init() {
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
}

/// Sparse inference engine for WASM.
///
/// Wraps a [`SparseCoreEngine`] (the native Rust sparse-FFN engine) together
/// with the model metadata extracted from the GGUF file.
#[wasm_bindgen]
pub struct SparseInferenceEngine {
    core_engine: SparseCoreEngine,
    model_metadata: ModelMetadata,
}

#[wasm_bindgen]
impl SparseInferenceEngine {
    /// Create a new engine from GGUF model bytes and a JSON config string.
    ///
    /// The JSON config supports the following fields (all optional):
    /// - `sparsity` (f64, default 0.9): fraction of neurons to deactivate
    /// - `sparsity_threshold` (f64, default 0.01)
    /// - `temperature` (f64, default 1.0)
    /// - `top_k` (u64, optional)
    /// - `top_p` (f64, optional)
    /// - `use_sparse_ffn` (bool, default true)
    /// - `active_neurons_per_layer` (u64, optional)
    /// - `output_hidden_states` (bool, default false)
    /// - `output_attentions` (bool, default false)
    #[wasm_bindgen(constructor)]
    pub fn new(model_bytes: &[u8], config_json: &str) -> Result<SparseInferenceEngine, JsError> {
        // InferenceConfig does not derive Deserialize; extract fields from a
        // raw JSON value and construct it explicitly.
        let config = parse_inference_config(config_json);

        // Parse the GGUF file to obtain model metadata (architecture,
        // dimensions, vocabulary size, etc.).
        let gguf_model = GgufParser::parse(model_bytes)
            .map_err(|e| JsError::new(&format!("Failed to parse model: {e}")))?;
        let model_metadata = ModelMetadata::from_gguf(&gguf_model)
            .map_err(|e| JsError::new(&format!("Failed to read model metadata: {e}")))?;

        // Build the sparse-FFN inference engine from the extracted dimensions.
        // sparsity_ratio = fraction of neurons kept active.
        let sparsity_ratio = 1.0_f32 - config.sparsity;
        let intermediate = if model_metadata.intermediate_size == 0 {
            // Fall back to a common 4x heuristic when feed_forward_length
            // is absent from the GGUF metadata.
            model_metadata.hidden_size * 4
        } else {
            model_metadata.intermediate_size
        };

        let core_engine = SparseCoreEngine::new_sparse(
            model_metadata.hidden_size.max(1),
            intermediate.max(1),
            sparsity_ratio,
        )
        .map_err(|e| JsError::new(&format!("Failed to create inference engine: {e}")))?;

        Ok(Self {
            core_engine,
            model_metadata,
        })
    }

    /// Load model from a URL using streaming fetch (for large models).
    #[wasm_bindgen]
    pub async fn load_streaming(
        url: &str,
        config_json: &str,
    ) -> Result<SparseInferenceEngine, JsError> {
        let bytes = fetch_model_bytes(url).await?;
        Self::new(&bytes, config_json)
    }

    /// Run sparse inference on an f32 input vector.
    #[wasm_bindgen]
    pub fn infer(&self, input: &[f32]) -> Result<Vec<f32>, JsError> {
        self.core_engine
            .infer(input)
            .map_err(|e| JsError::new(&format!("Inference failed: {e}")))
    }

    /// Return model metadata as a JSON string.
    ///
    /// `ModelMetadata` does not derive `Serialize`, so the fields are
    /// serialised manually via `serde_json::json!`.
    #[wasm_bindgen]
    pub fn metadata(&self) -> String {
        let m = &self.model_metadata;
        serde_json::json!({
            "hidden_size": m.hidden_size,
            "intermediate_size": m.intermediate_size,
            "num_layers": m.num_layers,
            "num_heads": m.num_heads,
            "num_key_value_heads": m.num_key_value_heads,
            "vocab_size": m.vocab_size,
            "max_position_embeddings": m.max_position_embeddings,
        })
        .to_string()
    }

    /// Return sparsity statistics as a JSON string.
    ///
    /// `SparsityStats` does not derive `Serialize`, so the fields are
    /// serialised manually via `serde_json::json!`.
    #[wasm_bindgen]
    pub fn sparsity_stats(&self) -> String {
        let stats = self.core_engine.sparsity_statistics();
        serde_json::json!({
            "average_active_ratio": stats.average_active_ratio,
            "min_active": stats.min_active,
            "max_active": stats.max_active,
        })
        .to_string()
    }

    /// Calibrate the engine with a flat array of sample vectors.
    ///
    /// `samples` is a flat `[f32]` array; `sample_dim` is the length of each
    /// individual sample vector (used to re-chunk the flat slice).
    #[wasm_bindgen]
    pub fn calibrate(&mut self, samples: &[f32], sample_dim: usize) -> Result<(), JsError> {
        let chunked: Vec<Vec<f32>> = samples.chunks(sample_dim).map(|c| c.to_vec()).collect();
        self.core_engine
            .calibrate(&chunked)
            .map_err(|e| JsError::new(&format!("Calibration failed: {e}")))
    }
}

/// Sentence-embedding model wrapper.
///
/// Provides a higher-level API for encoding sequences of token IDs into
/// embedding vectors via the underlying sparse inference engine.
#[wasm_bindgen]
pub struct EmbeddingModel {
    engine: SparseInferenceEngine,
}

#[wasm_bindgen]
impl EmbeddingModel {
    /// Construct an embedding model from raw GGUF bytes using default config.
    #[wasm_bindgen(constructor)]
    pub fn new(model_bytes: &[u8]) -> Result<EmbeddingModel, JsError> {
        // Use a flat-f32 sparsity value to match the current InferenceConfig
        // shape (sparsity: f32, not a nested object).
        let config = r#"{"sparsity": 0.9, "temperature": 1.0, "top_k": 50}"#;
        let engine = SparseInferenceEngine::new(model_bytes, config)?;
        Ok(Self { engine })
    }

    /// Encode a sequence of token IDs to an embedding vector.
    ///
    /// Token IDs are cast to `f32` and passed through the sparse inference
    /// engine.  This uses the same real sparse-FFN computation path as
    /// [`SparseInferenceEngine::infer`]; the model's actual weight matrix
    /// determines the output dimensionality.
    #[wasm_bindgen]
    pub fn encode(&self, input_ids: &[u32]) -> Result<Vec<f32>, JsError> {
        let ids_f32: Vec<f32> = input_ids.iter().map(|&x| x as f32).collect();
        // Call core_engine directly so the error is SparseInferenceError
        // (which implements Display), rather than JsError (which does not in
        // this version of wasm-bindgen).
        self.engine
            .core_engine
            .infer(&ids_f32)
            .map_err(|e| JsError::new(&format!("Encoding failed: {e}")))
    }

    /// Batch-encode multiple sequences packed into a single flat array.
    ///
    /// `input_ids` is a flat array of all token IDs concatenated together;
    /// `lengths` gives the number of tokens in each individual sequence.
    #[wasm_bindgen]
    pub fn encode_batch(&self, input_ids: &[u32], lengths: &[u32]) -> Result<Vec<f32>, JsError> {
        let mut results = Vec::new();
        let mut offset = 0usize;
        for &len in lengths {
            let len = len as usize;
            if offset + len > input_ids.len() {
                return Err(JsError::new("Invalid lengths: exceeds input_ids size"));
            }
            let ids = &input_ids[offset..offset + len];
            let embedding = self.encode(ids)?;
            results.extend(embedding);
            offset += len;
        }
        Ok(results)
    }

    /// Return the embedding dimension (hidden_size from the model metadata).
    #[wasm_bindgen]
    pub fn dimension(&self) -> usize {
        self.engine.model_metadata.hidden_size
    }
}

/// Measure the average inference time for a given number of iterations.
///
/// Uses the browser Performance API; throws if that API is not available.
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

    (end - start) / f64::from(iterations)
}

/// Return the crate version string.
#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Build an [`InferenceConfig`] from an arbitrary JSON string.
///
/// `InferenceConfig` does not derive `serde::Deserialize`, so we parse the
/// JSON into a raw `serde_json::Value` and extract each field individually,
/// falling back to sensible defaults when a key is absent or mis-typed.
fn parse_inference_config(json: &str) -> InferenceConfig {
    let val: serde_json::Value = serde_json::from_str(json).unwrap_or_default();

    InferenceConfig {
        sparsity: val
            .get("sparsity")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.9) as f32,
        sparsity_threshold: val
            .get("sparsity_threshold")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.01) as f32,
        temperature: val
            .get("temperature")
            .and_then(|v| v.as_f64())
            .unwrap_or(1.0) as f32,
        top_k: val
            .get("top_k")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize),
        top_p: val
            .get("top_p")
            .and_then(|v| v.as_f64())
            .map(|v| v as f32),
        use_sparse_ffn: val
            .get("use_sparse_ffn")
            .and_then(|v| v.as_bool())
            .unwrap_or(true),
        active_neurons_per_layer: val
            .get("active_neurons_per_layer")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize),
        output_hidden_states: val
            .get("output_hidden_states")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        output_attentions: val
            .get("output_attentions")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
    }
}

/// Fetch model bytes from a URL using the browser Fetch API.
///
/// `JsFuture::await` yields `Result<JsValue, JsValue>`.  `JsValue` does not
/// implement `std::error::Error`, so we cannot use `?` directly when the
/// function returns `Result<_, JsError>`.  Instead we convert explicitly via
/// `.map_err()` using the `Debug` representation (Display is not available on
/// `JsValue` either).
async fn fetch_model_bytes(url: &str) -> Result<Vec<u8>, JsError> {
    use wasm_bindgen_futures::JsFuture;

    let window = web_sys::window().ok_or_else(|| JsError::new("No window"))?;
    let response = JsFuture::from(window.fetch_with_str(url))
        .await
        .map_err(|e| JsError::new(&format!("Fetch request failed: {e:?}")))?;
    let response: web_sys::Response = response
        .dyn_into()
        .map_err(|_| JsError::new("Failed to cast fetch response to Response"))?;
    let buffer = JsFuture::from(
        response
            .array_buffer()
            .map_err(|_| JsError::new("Failed to start array_buffer read"))?,
    )
    .await
    .map_err(|e| JsError::new(&format!("Failed to read array buffer: {e:?}")))?;
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

    #[test]
    fn test_parse_inference_config_defaults() {
        let config = parse_inference_config("{}");
        assert!((config.sparsity - 0.9).abs() < f32::EPSILON);
        assert!((config.temperature - 1.0).abs() < f32::EPSILON);
        assert_eq!(config.top_k, None);
        assert!(config.use_sparse_ffn);
    }

    #[test]
    fn test_parse_inference_config_custom() {
        let config = parse_inference_config(
            r#"{"sparsity": 0.5, "temperature": 0.8, "top_k": 40, "use_sparse_ffn": false}"#,
        );
        assert!((config.sparsity - 0.5).abs() < f32::EPSILON);
        assert!((config.temperature - 0.8).abs() < 1e-5);
        assert_eq!(config.top_k, Some(40));
        assert!(!config.use_sparse_ffn);
    }
}
