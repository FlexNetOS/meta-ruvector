//! Error types for ruvector-maxsim.

use thiserror::Error;

/// Errors that can occur in MaxSim index operations.
#[derive(Debug, Error)]
pub enum MaxSimError {
    /// The vector dimension of an added document or query token does not match
    /// the dimension the index was built with.
    #[error("dimension mismatch: index expects {expected}, got {got}")]
    DimensionMismatch {
        /// The dimension the index expects.
        expected: usize,
        /// The dimension that was actually provided.
        got: usize,
    },

    /// A document with no token vectors was passed to [`crate::MultiVecIndex::add`].
    #[error("empty document: at least one token vector is required")]
    EmptyDocument,

    /// A search was attempted on an index with no documents.
    #[error("index is empty: no documents have been added")]
    EmptyIndex,
}
