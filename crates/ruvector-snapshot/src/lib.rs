//! Snapshot and restore functionality for rUvector collections
//!
//! This crate provides full backup and restore capabilities for vector collections,
//! including GZIP compression and SHA-256 checksums. Storage is pluggable via the
//! [`SnapshotStorage`] trait; a filesystem backend ([`LocalStorage`]) is included.

mod error;
mod manager;
mod snapshot;
mod storage;

pub use error::{Result, SnapshotError};
pub use manager::SnapshotManager;
pub use snapshot::{Snapshot, SnapshotData, SnapshotMetadata, VectorRecord};
pub use storage::{LocalStorage, SnapshotStorage};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_exports() {
        // Verify all public exports are accessible
        let _: Option<SnapshotError> = None;
        let _: Option<SnapshotManager> = None;
        let _: Option<Snapshot> = None;
    }
}
