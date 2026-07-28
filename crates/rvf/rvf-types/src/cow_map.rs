//! COW_MAP_SEG (0x20) types for the RVF computational container.
//!
//! ADR-031 called its COW header "64 bytes" while assigning fields through
//! offset `0x5f`.  The two statements cannot both be true.  Version 1 is
//! therefore retained as the historical 64-byte, read-only prefix, while
//! version 2 is the canonical 96-byte layout containing the complete field
//! set.  Writers must emit V2.

use crate::error::RvfError;

/// Magic number for COW map headers: "RVCM" in big-endian.
pub const COWMAP_MAGIC: u32 = 0x5256_434D;
/// Historical, incomplete header version.
pub const COW_MAP_V1: u16 = 1;
/// Canonical complete header version.
pub const COW_MAP_V2: u16 = 2;
/// Size of the historical V1 header.
pub const COW_MAP_V1_HEADER_SIZE: usize = 64;
/// Size of the canonical V2 header.
pub const COW_MAP_V2_HEADER_SIZE: usize = 96;

/// Cluster map storage format.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[repr(u8)]
pub enum MapFormat {
    /// Simple flat array of cluster entries.
    FlatArray = 0,
    /// Adaptive Radix Tree for sparse mappings.
    ArtTree = 1,
    /// Extent list for contiguous ranges.
    ExtentList = 2,
}

impl TryFrom<u8> for MapFormat {
    type Error = RvfError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::FlatArray),
            1 => Ok(Self::ArtTree),
            2 => Ok(Self::ExtentList),
            _ => Err(RvfError::InvalidEnumValue {
                type_name: "MapFormat",
                value: value as u64,
            }),
        }
    }
}

/// Entry in the COW cluster map.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum CowMapEntry {
    /// Cluster has been written locally at the given offset.
    LocalOffset(u64),
    /// Cluster data lives in the parent file.
    ParentRef,
    /// Cluster has not been allocated.
    Unallocated,
}

/// Historical 64-byte V1 header.
///
/// V1 omits the map offset, counts, extent flag, and generation.  It exists
/// only so old data can be decoded explicitly; no writer should construct it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(C)]
pub struct CowMapHeaderV1 {
    pub magic: u32,
    pub version: u16,
    pub map_format: u8,
    pub compression_policy: u8,
    pub cluster_size_bytes: u32,
    pub vectors_per_cluster: u32,
    pub base_file_id: [u8; 16],
    pub base_file_hash: [u8; 32],
}

const _: () = assert!(core::mem::size_of::<CowMapHeaderV1>() == COW_MAP_V1_HEADER_SIZE);

impl CowMapHeaderV1 {
    /// Decode and strictly validate a historical header.
    pub fn from_bytes(data: &[u8; COW_MAP_V1_HEADER_SIZE]) -> Result<Self, RvfError> {
        let common = decode_common(data)?;
        if common.version != COW_MAP_V1 {
            return Err(RvfError::Code(crate::ErrorCode::InvalidVersion));
        }
        Ok(Self {
            magic: common.magic,
            version: common.version,
            map_format: common.map_format,
            compression_policy: common.compression_policy,
            cluster_size_bytes: common.cluster_size_bytes,
            vectors_per_cluster: common.vectors_per_cluster,
            base_file_id: common.base_file_id,
            base_file_hash: common.base_file_hash,
        })
    }

    /// Serialize the historical header for byte-exact compatibility tests.
    ///
    /// Production writers must use [`CowMapHeader::to_bytes`].
    pub fn to_bytes(&self) -> [u8; COW_MAP_V1_HEADER_SIZE] {
        encode_common(
            self.magic,
            self.version,
            self.map_format,
            self.compression_policy,
            self.cluster_size_bytes,
            self.vectors_per_cluster,
            &self.base_file_id,
            &self.base_file_hash,
        )
    }
}

/// Canonical 96-byte V2 COW map header.
///
/// The V2 layout makes the ADR's complete `0x00..0x5f` field map honest.
/// V2 assigns four formerly-reserved bytes at `0x54` to `generation_id` so
/// manifests can reject replayed maps without overloading the format version.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(C)]
pub struct CowMapHeader {
    pub magic: u32,
    pub version: u16,
    pub map_format: u8,
    pub compression_policy: u8,
    pub cluster_size_bytes: u32,
    pub vectors_per_cluster: u32,
    pub base_file_id: [u8; 16],
    pub base_file_hash: [u8; 32],
    /// Offset from the start of the segment payload to map entries.
    pub map_root_offset: u64,
    pub cluster_count: u32,
    pub local_cluster_count: u32,
    /// Whether extent entries are enabled (0 or 1).
    pub extent_support: u8,
    /// Reserved, must be zero.
    pub reserved: [u8; 3],
    /// Monotonic anti-replay generation.
    pub generation_id: u32,
    /// Reserved, must be zero.
    pub reserved2: [u8; 8],
}

const _: () = assert!(core::mem::size_of::<CowMapHeader>() == COW_MAP_V2_HEADER_SIZE);

impl CowMapHeader {
    /// Serialize the canonical V2 header.
    pub fn to_bytes(&self) -> [u8; COW_MAP_V2_HEADER_SIZE] {
        let common = encode_common(
            self.magic,
            self.version,
            self.map_format,
            self.compression_policy,
            self.cluster_size_bytes,
            self.vectors_per_cluster,
            &self.base_file_id,
            &self.base_file_hash,
        );
        let mut buf = [0u8; COW_MAP_V2_HEADER_SIZE];
        buf[..COW_MAP_V1_HEADER_SIZE].copy_from_slice(&common);
        buf[0x40..0x48].copy_from_slice(&self.map_root_offset.to_le_bytes());
        buf[0x48..0x4C].copy_from_slice(&self.cluster_count.to_le_bytes());
        buf[0x4C..0x50].copy_from_slice(&self.local_cluster_count.to_le_bytes());
        buf[0x50] = self.extent_support;
        buf[0x51..0x54].copy_from_slice(&self.reserved);
        buf[0x54..0x58].copy_from_slice(&self.generation_id.to_le_bytes());
        buf[0x58..0x60].copy_from_slice(&self.reserved2);
        buf
    }

    /// Decode and strictly validate a canonical V2 header.
    pub fn from_bytes(data: &[u8; COW_MAP_V2_HEADER_SIZE]) -> Result<Self, RvfError> {
        let common = decode_common(data)?;
        if common.version != COW_MAP_V2 {
            return Err(RvfError::Code(crate::ErrorCode::InvalidVersion));
        }
        let header = Self {
            magic: common.magic,
            version: common.version,
            map_format: common.map_format,
            compression_policy: common.compression_policy,
            cluster_size_bytes: common.cluster_size_bytes,
            vectors_per_cluster: common.vectors_per_cluster,
            base_file_id: common.base_file_id,
            base_file_hash: common.base_file_hash,
            map_root_offset: u64::from_le_bytes(data[0x40..0x48].try_into().unwrap()),
            cluster_count: u32::from_le_bytes(data[0x48..0x4C].try_into().unwrap()),
            local_cluster_count: u32::from_le_bytes(data[0x4C..0x50].try_into().unwrap()),
            extent_support: data[0x50],
            reserved: data[0x51..0x54].try_into().unwrap(),
            generation_id: u32::from_le_bytes(data[0x54..0x58].try_into().unwrap()),
            reserved2: data[0x58..0x60].try_into().unwrap(),
        };
        if header.map_root_offset != COW_MAP_V2_HEADER_SIZE as u64
            || header.local_cluster_count > header.cluster_count
            || header.extent_support > 1
            || header.reserved != [0; 3]
            || header.reserved2 != [0; 8]
            || header.generation_id == 0
        {
            return Err(RvfError::Code(crate::ErrorCode::CowMapCorrupt));
        }
        Ok(header)
    }
}

struct CommonHeader {
    magic: u32,
    version: u16,
    map_format: u8,
    compression_policy: u8,
    cluster_size_bytes: u32,
    vectors_per_cluster: u32,
    base_file_id: [u8; 16],
    base_file_hash: [u8; 32],
}

fn decode_common(data: &[u8]) -> Result<CommonHeader, RvfError> {
    let magic = u32::from_le_bytes(data[0x00..0x04].try_into().unwrap());
    if magic != COWMAP_MAGIC {
        return Err(RvfError::BadMagic {
            expected: COWMAP_MAGIC,
            got: magic,
        });
    }
    let map_format = data[0x06];
    MapFormat::try_from(map_format)?;
    let compression_policy = data[0x07];
    if compression_policy != 0 {
        return Err(RvfError::Code(crate::ErrorCode::CowMapCorrupt));
    }
    let cluster_size_bytes = u32::from_le_bytes(data[0x08..0x0C].try_into().unwrap());
    let vectors_per_cluster = u32::from_le_bytes(data[0x0C..0x10].try_into().unwrap());
    if cluster_size_bytes == 0 || !cluster_size_bytes.is_power_of_two() || vectors_per_cluster == 0
    {
        return Err(RvfError::Code(crate::ErrorCode::CowMapCorrupt));
    }
    Ok(CommonHeader {
        magic,
        version: u16::from_le_bytes(data[0x04..0x06].try_into().unwrap()),
        map_format,
        compression_policy,
        cluster_size_bytes,
        vectors_per_cluster,
        base_file_id: data[0x10..0x20].try_into().unwrap(),
        base_file_hash: data[0x20..0x40].try_into().unwrap(),
    })
}

#[allow(clippy::too_many_arguments)]
fn encode_common(
    magic: u32,
    version: u16,
    map_format: u8,
    compression_policy: u8,
    cluster_size_bytes: u32,
    vectors_per_cluster: u32,
    base_file_id: &[u8; 16],
    base_file_hash: &[u8; 32],
) -> [u8; COW_MAP_V1_HEADER_SIZE] {
    let mut buf = [0u8; COW_MAP_V1_HEADER_SIZE];
    buf[0x00..0x04].copy_from_slice(&magic.to_le_bytes());
    buf[0x04..0x06].copy_from_slice(&version.to_le_bytes());
    buf[0x06] = map_format;
    buf[0x07] = compression_policy;
    buf[0x08..0x0C].copy_from_slice(&cluster_size_bytes.to_le_bytes());
    buf[0x0C..0x10].copy_from_slice(&vectors_per_cluster.to_le_bytes());
    buf[0x10..0x20].copy_from_slice(base_file_id);
    buf[0x20..0x40].copy_from_slice(base_file_hash);
    buf
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_v2() -> CowMapHeader {
        CowMapHeader {
            magic: COWMAP_MAGIC,
            version: COW_MAP_V2,
            map_format: MapFormat::FlatArray as u8,
            compression_policy: 0,
            cluster_size_bytes: 4096,
            vectors_per_cluster: 64,
            base_file_id: [0xAA; 16],
            base_file_hash: [0xBB; 32],
            map_root_offset: 96,
            cluster_count: 3,
            local_cluster_count: 1,
            extent_support: 0,
            reserved: [0; 3],
            generation_id: 7,
            reserved2: [0; 8],
        }
    }

    #[test]
    fn header_sizes_are_honest() {
        assert_eq!(core::mem::size_of::<CowMapHeaderV1>(), 64);
        assert_eq!(core::mem::size_of::<CowMapHeader>(), 96);
    }

    #[test]
    fn v2_byte_exact_round_trip_and_offsets() {
        let header = sample_v2();
        let bytes = header.to_bytes();
        assert_eq!(&bytes[0x40..0x48], &96u64.to_le_bytes());
        assert_eq!(&bytes[0x48..0x4C], &3u32.to_le_bytes());
        assert_eq!(&bytes[0x4C..0x50], &1u32.to_le_bytes());
        assert_eq!(&bytes[0x54..0x58], &7u32.to_le_bytes());
        assert_eq!(CowMapHeader::from_bytes(&bytes).unwrap(), header);
    }

    #[test]
    fn v1_is_readable_but_not_v2() {
        let v2 = sample_v2();
        let v1 = CowMapHeaderV1 {
            magic: v2.magic,
            version: COW_MAP_V1,
            map_format: v2.map_format,
            compression_policy: v2.compression_policy,
            cluster_size_bytes: v2.cluster_size_bytes,
            vectors_per_cluster: v2.vectors_per_cluster,
            base_file_id: v2.base_file_id,
            base_file_hash: v2.base_file_hash,
        };
        let bytes = v1.to_bytes();
        assert_eq!(CowMapHeaderV1::from_bytes(&bytes).unwrap(), v1);
    }

    #[test]
    fn v2_rejects_reserved_counts_generation_and_offset() {
        let mut bytes = sample_v2().to_bytes();
        bytes[0x51] = 1;
        assert!(CowMapHeader::from_bytes(&bytes).is_err());

        let mut bytes = sample_v2().to_bytes();
        bytes[0x4C..0x50].copy_from_slice(&4u32.to_le_bytes());
        assert!(CowMapHeader::from_bytes(&bytes).is_err());

        let mut bytes = sample_v2().to_bytes();
        bytes[0x54..0x58].copy_from_slice(&0u32.to_le_bytes());
        assert!(CowMapHeader::from_bytes(&bytes).is_err());

        let mut bytes = sample_v2().to_bytes();
        bytes[0x40..0x48].copy_from_slice(&64u64.to_le_bytes());
        assert!(CowMapHeader::from_bytes(&bytes).is_err());
    }

    #[test]
    fn magic_bytes_match_ascii() {
        assert_eq!(&COWMAP_MAGIC.to_be_bytes(), b"RVCM");
    }
}
