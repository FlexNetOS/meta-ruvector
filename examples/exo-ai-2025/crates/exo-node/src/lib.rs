//! Node.js bindings for EXO-AI cognitive substrate via NAPI-RS
//!
//! High-performance Rust-based cognitive substrate with async/await support,
//! hypergraph queries, and temporal memory.

#![deny(clippy::all)]
#![warn(clippy::pedantic)]

use napi::bindgen_prelude::*;
use napi_derive::napi;

use exo_backend_classical::ClassicalBackend;
use exo_core::{Pattern, SubstrateBackend};
use std::sync::Arc;

mod types;
use types::{JsPattern, JsSearchResult};

/// EXO-AI cognitive substrate for Node.js
///
/// Provides vector similarity search, hypergraph queries, and temporal memory
/// backed by the high-performance ruvector database.
#[napi]
pub struct ExoSubstrateNode {
    backend: Arc<ClassicalBackend>,
}

#[napi]
impl ExoSubstrateNode {
    /// Create a new substrate instance
    ///
    /// # Example
    /// ```javascript
    /// const substrate = new ExoSubstrateNode({
    ///   dimensions: 384,
    ///   distanceMetric: 'Cosine'
    /// });
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if the backend cannot be initialized for the requested
    /// number of dimensions.
    #[napi(constructor)]
    pub fn new(dimensions: u32) -> Result<Self> {
        let backend = ClassicalBackend::with_dimensions(dimensions as usize)
            .map_err(|e| Error::from_reason(format!("Failed to create backend: {e}")))?;

        Ok(Self {
            backend: Arc::new(backend),
        })
    }

    /// Create a substrate with default configuration (768 dimensions)
    ///
    /// # Example
    /// ```javascript
    /// const substrate = ExoSubstrateNode.withDimensions(384);
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if the backend cannot be initialized for the requested
    /// number of dimensions.
    #[napi(factory)]
    pub fn with_dimensions(dimensions: u32) -> Result<Self> {
        Self::new(dimensions)
    }

    /// Store a pattern in the substrate
    ///
    /// Returns the ID of the stored pattern
    ///
    /// # Example
    /// ```javascript
    /// const id = await substrate.store({
    ///   embedding: new Float32Array([1.0, 2.0, 3.0, ...]),
    ///   metadata: '{"text": "example", "category": "demo"}',
    ///   salience: 1.0
    /// });
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if the pattern cannot be converted to the core
    /// representation or if storing it in the backend fails.
    #[napi]
    pub fn store(&self, pattern: JsPattern) -> Result<String> {
        let core_pattern: Pattern = pattern.try_into()?;
        let pattern_id = core_pattern.id;

        self.backend
            .manifold_deform(&core_pattern, 0.0)
            .map_err(|e| Error::from_reason(format!("Failed to store pattern: {e}")))?;

        Ok(pattern_id.to_string())
    }

    /// Search for similar patterns
    ///
    /// Returns an array of search results sorted by similarity
    ///
    /// # Example
    /// ```javascript
    /// const results = await substrate.search(
    ///   new Float32Array([1.0, 2.0, 3.0, ...]),
    ///   10  // top-k
    /// );
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if the similarity search fails in the backend.
    // napi-rs requires `Float32Array` arguments to be taken by value (it does not
    // implement `FromNapiRef`), so a reference cannot be used here.
    #[allow(clippy::needless_pass_by_value)]
    #[napi]
    pub fn search(&self, embedding: Float32Array, k: u32) -> Result<Vec<JsSearchResult>> {
        let results = self
            .backend
            .similarity_search(&embedding, k as usize, None)
            .map_err(|e| Error::from_reason(format!("Failed to search: {e}")))?;

        Ok(results.into_iter().map(Into::into).collect())
    }

    /// Query hypergraph topology
    ///
    /// Performs topological data analysis queries on the substrate
    /// Note: This feature is not yet fully implemented in the classical backend
    ///
    /// # Example
    /// ```javascript
    /// const result = await substrate.hypergraphQuery('{"BettiNumbers":{"max_dimension":3}}');
    /// ```
    ///
    /// # Errors
    ///
    /// Currently never returns an error; hypergraph queries are not yet
    /// supported by the classical backend and a `NotSupported` response is
    /// returned instead. The `Result` is retained for forward compatibility.
    #[napi]
    pub fn hypergraph_query(&self, _query: String) -> Result<String> {
        // Hypergraph queries are not supported in the classical backend yet
        // Return a NotSupported response
        Ok(r#"{"NotSupported":null}"#.to_string())
    }

    /// Get substrate dimensions
    ///
    /// # Example
    /// ```javascript
    /// const dims = substrate.dimensions();
    /// console.log(`Dimensions: ${dims}`);
    /// ```
    #[napi]
    #[must_use]
    pub fn dimensions(&self) -> u32 {
        // The dimension is set from a `u32` in the constructor, so it always
        // fits back into a `u32` — the round-trip cast cannot truncate.
        #[allow(clippy::cast_possible_truncation)]
        let dims = self.backend.dimension() as u32;
        dims
    }
}

/// Get the version of the EXO-AI library
#[napi]
#[must_use]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Test function to verify the bindings are working
#[napi]
#[must_use]
pub fn hello() -> String {
    "Hello from EXO-AI cognitive substrate!".to_string()
}
