//! COW cluster map for vector-addressed cluster resolution.
//!
//! Supports three formats: flat array (default), ART tree, and extent list.
//! Currently only flat_array is implemented; ART tree and extent list are
//! reserved for future optimization of sparse mappings.

use rvf_types::cow_map::{CowMapEntry, MapFormat};
use rvf_types::{ErrorCode, RvfError};

/// Adaptive cluster map for cluster_id -> location resolution.
///
/// Each cluster is either local (written to this file), inherited from the
/// parent (ParentRef), or unallocated.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CowMap {
    format: MapFormat,
    entries: Vec<CowMapEntry>,
}

impl CowMap {
    /// Create a new flat-array map with `cluster_count` entries, all Unallocated.
    pub fn new_flat(cluster_count: u32) -> Self {
        Self {
            format: MapFormat::FlatArray,
            entries: vec![CowMapEntry::Unallocated; cluster_count as usize],
        }
    }

    /// Create a new flat-array map with all entries set to ParentRef.
    pub fn new_parent_ref(cluster_count: u32) -> Self {
        Self {
            format: MapFormat::FlatArray,
            entries: vec![CowMapEntry::ParentRef; cluster_count as usize],
        }
    }

    /// Look up a cluster by ID.
    pub fn lookup(&self, cluster_id: u32) -> CowMapEntry {
        self.entries
            .get(cluster_id as usize)
            .copied()
            .unwrap_or(CowMapEntry::Unallocated)
    }

    /// Update a cluster entry.
    pub fn update(&mut self, cluster_id: u32, entry: CowMapEntry) {
        let idx = cluster_id as usize;
        if idx >= self.entries.len() {
            self.entries.resize(idx + 1, CowMapEntry::Unallocated);
        }
        self.entries[idx] = entry;
    }

    /// Construct a map from entries decoded by `rvf-wire`.
    pub fn from_entries(format: MapFormat, entries: Vec<CowMapEntry>) -> Result<Self, RvfError> {
        if format != MapFormat::FlatArray
            || entries
                .iter()
                .any(|entry| matches!(entry, CowMapEntry::LocalOffset(0)))
        {
            return Err(RvfError::Code(ErrorCode::CowMapCorrupt));
        }
        Ok(Self { format, entries })
    }

    /// Borrow entries for the canonical `rvf-wire` encoder.
    pub fn entries(&self) -> &[CowMapEntry] {
        &self.entries
    }

    /// Count of clusters that have local data.
    pub fn local_cluster_count(&self) -> u32 {
        self.entries
            .iter()
            .filter(|e| matches!(e, CowMapEntry::LocalOffset(_)))
            .count() as u32
    }

    /// Total number of clusters in the map.
    pub fn cluster_count(&self) -> u32 {
        self.entries.len() as u32
    }

    /// Get the map format.
    pub fn format(&self) -> MapFormat {
        self.format
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_flat_all_unallocated() {
        let map = CowMap::new_flat(10);
        assert_eq!(map.cluster_count(), 10);
        assert_eq!(map.local_cluster_count(), 0);
        for i in 0..10 {
            assert_eq!(map.lookup(i), CowMapEntry::Unallocated);
        }
    }

    #[test]
    fn new_parent_ref_all_parent() {
        let map = CowMap::new_parent_ref(5);
        assert_eq!(map.cluster_count(), 5);
        for i in 0..5 {
            assert_eq!(map.lookup(i), CowMapEntry::ParentRef);
        }
    }

    #[test]
    fn update_and_lookup() {
        let mut map = CowMap::new_flat(4);
        map.update(1, CowMapEntry::LocalOffset(0x1000));
        map.update(3, CowMapEntry::ParentRef);
        assert_eq!(map.lookup(0), CowMapEntry::Unallocated);
        assert_eq!(map.lookup(1), CowMapEntry::LocalOffset(0x1000));
        assert_eq!(map.lookup(2), CowMapEntry::Unallocated);
        assert_eq!(map.lookup(3), CowMapEntry::ParentRef);
        assert_eq!(map.local_cluster_count(), 1);
    }

    #[test]
    fn update_grows_map() {
        let mut map = CowMap::new_flat(2);
        map.update(5, CowMapEntry::LocalOffset(0x2000));
        assert_eq!(map.cluster_count(), 6);
        assert_eq!(map.lookup(5), CowMapEntry::LocalOffset(0x2000));
    }

    #[test]
    fn out_of_bounds_lookup_returns_unallocated() {
        let map = CowMap::new_flat(2);
        assert_eq!(map.lookup(100), CowMapEntry::Unallocated);
    }

    #[test]
    fn entries_round_trip() {
        let mut map = CowMap::new_flat(4);
        map.update(0, CowMapEntry::LocalOffset(0x100));
        map.update(1, CowMapEntry::ParentRef);
        // 2 stays Unallocated
        map.update(3, CowMapEntry::LocalOffset(0x200));

        let map2 = CowMap::from_entries(map.format(), map.entries().to_vec()).unwrap();

        assert_eq!(map2.cluster_count(), 4);
        assert_eq!(map2.lookup(0), CowMapEntry::LocalOffset(0x100));
        assert_eq!(map2.lookup(1), CowMapEntry::ParentRef);
        assert_eq!(map2.lookup(2), CowMapEntry::Unallocated);
        assert_eq!(map2.lookup(3), CowMapEntry::LocalOffset(0x200));
    }

    #[test]
    fn from_entries_rejects_unsupported_format_and_zero_local_offset() {
        assert!(CowMap::from_entries(MapFormat::ArtTree, Vec::new()).is_err());
        assert!(
            CowMap::from_entries(MapFormat::FlatArray, vec![CowMapEntry::LocalOffset(0)]).is_err()
        );
    }
}
