//! Strict, versioned COW_MAP payload codec.
//!
//! V1 is decoded only for compatibility.  V2 is the sole emitted format.

use rvf_types::{
    CowMapEntry, CowMapHeader, CowMapHeaderV1, ErrorCode, MapFormat, RvfError,
    COW_MAP_V1_HEADER_SIZE, COW_MAP_V2_HEADER_SIZE,
};

const ENTRY_SIZE: usize = 9;

/// Header returned by the versioned decoder.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DecodedCowMapHeader {
    /// Historical incomplete 64-byte header.
    V1(CowMapHeaderV1),
    /// Canonical complete 96-byte header.
    V2(CowMapHeader),
}

/// Fully decoded COW map payload.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DecodedCowMap {
    pub header: DecodedCowMapHeader,
    pub entries: Vec<CowMapEntry>,
}

/// Encode the canonical V2 payload.
pub fn encode_cow_map(header: &CowMapHeader, entries: &[CowMapEntry]) -> Result<Vec<u8>, RvfError> {
    // Round-trip through the strict header decoder so callers cannot emit
    // noncanonical reserved fields or inconsistent geometry.
    CowMapHeader::from_bytes(&header.to_bytes())?;
    if MapFormat::try_from(header.map_format)? != MapFormat::FlatArray
        || usize::try_from(header.cluster_count).ok() != Some(entries.len())
        || header.local_cluster_count != count_local(entries)
    {
        return Err(RvfError::Code(ErrorCode::CowMapCorrupt));
    }
    let entry_bytes = entries
        .len()
        .checked_mul(ENTRY_SIZE)
        .ok_or(RvfError::Code(ErrorCode::CowMapCorrupt))?;
    let mut payload = Vec::with_capacity(
        COW_MAP_V2_HEADER_SIZE
            .checked_add(entry_bytes)
            .ok_or(RvfError::Code(ErrorCode::CowMapCorrupt))?,
    );
    payload.extend_from_slice(&header.to_bytes());
    encode_entries(&mut payload, entries)?;
    Ok(payload)
}

/// Decode either historical V1 or canonical V2.
pub fn decode_cow_map(payload: &[u8]) -> Result<DecodedCowMap, RvfError> {
    if payload.len() < COW_MAP_V1_HEADER_SIZE {
        return Err(RvfError::Code(ErrorCode::CowMapCorrupt));
    }
    let version = u16::from_le_bytes([payload[0x04], payload[0x05]]);
    match version {
        1 => decode_v1(payload),
        2 => decode_v2(payload),
        _ => Err(RvfError::Code(ErrorCode::InvalidVersion)),
    }
}

fn decode_v1(payload: &[u8]) -> Result<DecodedCowMap, RvfError> {
    let bytes: &[u8; COW_MAP_V1_HEADER_SIZE] = payload[..COW_MAP_V1_HEADER_SIZE]
        .try_into()
        .map_err(|_| RvfError::Code(ErrorCode::CowMapCorrupt))?;
    let header = CowMapHeaderV1::from_bytes(bytes)?;
    if MapFormat::try_from(header.map_format)? != MapFormat::FlatArray {
        return Err(RvfError::Code(ErrorCode::CowMapCorrupt));
    }

    // The historical runtime map blob redundantly stored format + count.
    let map = &payload[COW_MAP_V1_HEADER_SIZE..];
    if map.len() < 5 || map[0] != header.map_format {
        return Err(RvfError::Code(ErrorCode::CowMapCorrupt));
    }
    let count = u32::from_le_bytes(map[1..5].try_into().unwrap());
    let entries = decode_entries(&map[5..], count, false)?;
    Ok(DecodedCowMap {
        header: DecodedCowMapHeader::V1(header),
        entries,
    })
}

fn decode_v2(payload: &[u8]) -> Result<DecodedCowMap, RvfError> {
    if payload.len() < COW_MAP_V2_HEADER_SIZE {
        return Err(RvfError::Code(ErrorCode::CowMapCorrupt));
    }
    let bytes: &[u8; COW_MAP_V2_HEADER_SIZE] = payload[..COW_MAP_V2_HEADER_SIZE]
        .try_into()
        .map_err(|_| RvfError::Code(ErrorCode::CowMapCorrupt))?;
    let header = CowMapHeader::from_bytes(bytes)?;
    if MapFormat::try_from(header.map_format)? != MapFormat::FlatArray {
        return Err(RvfError::Code(ErrorCode::CowMapCorrupt));
    }
    let root = usize::try_from(header.map_root_offset)
        .map_err(|_| RvfError::Code(ErrorCode::CowMapCorrupt))?;
    let entries = decode_entries(&payload[root..], header.cluster_count, true)?;
    if count_local(&entries) != header.local_cluster_count {
        return Err(RvfError::Code(ErrorCode::CowMapCorrupt));
    }
    Ok(DecodedCowMap {
        header: DecodedCowMapHeader::V2(header),
        entries,
    })
}

fn encode_entries(out: &mut Vec<u8>, entries: &[CowMapEntry]) -> Result<(), RvfError> {
    for entry in entries {
        match *entry {
            CowMapEntry::Unallocated => {
                out.push(0);
                out.extend_from_slice(&0u64.to_le_bytes());
            }
            CowMapEntry::ParentRef => {
                out.push(1);
                out.extend_from_slice(&0u64.to_le_bytes());
            }
            CowMapEntry::LocalOffset(offset) if offset != 0 => {
                out.push(2);
                out.extend_from_slice(&offset.to_le_bytes());
            }
            CowMapEntry::LocalOffset(_) => {
                return Err(RvfError::Code(ErrorCode::CowMapCorrupt));
            }
        }
    }
    Ok(())
}

fn decode_entries(
    data: &[u8],
    count: u32,
    reject_zero_local: bool,
) -> Result<Vec<CowMapEntry>, RvfError> {
    let count = usize::try_from(count).map_err(|_| RvfError::Code(ErrorCode::CowMapCorrupt))?;
    let expected = count
        .checked_mul(ENTRY_SIZE)
        .ok_or(RvfError::Code(ErrorCode::CowMapCorrupt))?;
    if data.len() != expected {
        return Err(RvfError::Code(ErrorCode::CowMapCorrupt));
    }
    let mut entries = Vec::with_capacity(count);
    for chunk in data.chunks_exact(ENTRY_SIZE) {
        let value = u64::from_le_bytes(chunk[1..9].try_into().unwrap());
        let entry = match (chunk[0], value) {
            (0, 0) => CowMapEntry::Unallocated,
            (1, 0) => CowMapEntry::ParentRef,
            (2, 0) if !reject_zero_local => CowMapEntry::LocalOffset(0),
            (2, value) if value != 0 => CowMapEntry::LocalOffset(value),
            _ => return Err(RvfError::Code(ErrorCode::CowMapCorrupt)),
        };
        entries.push(entry);
    }
    Ok(entries)
}

fn count_local(entries: &[CowMapEntry]) -> u32 {
    u32::try_from(
        entries
            .iter()
            .filter(|entry| matches!(entry, CowMapEntry::LocalOffset(_)))
            .count(),
    )
    .unwrap_or(u32::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rvf_types::{COWMAP_MAGIC, COW_MAP_V2};

    fn header() -> CowMapHeader {
        CowMapHeader {
            magic: COWMAP_MAGIC,
            version: COW_MAP_V2,
            map_format: MapFormat::FlatArray as u8,
            compression_policy: 0,
            cluster_size_bytes: 4096,
            vectors_per_cluster: 16,
            base_file_id: [1; 16],
            base_file_hash: [2; 32],
            map_root_offset: 96,
            cluster_count: 3,
            local_cluster_count: 1,
            extent_support: 0,
            reserved: [0; 3],
            generation_id: 1,
            reserved2: [0; 8],
        }
    }

    #[test]
    fn v2_byte_exact_round_trip() {
        let entries = [
            CowMapEntry::ParentRef,
            CowMapEntry::LocalOffset(0x1234),
            CowMapEntry::Unallocated,
        ];
        let payload = encode_cow_map(&header(), &entries).unwrap();
        assert_eq!(payload.len(), 96 + 27);
        assert_eq!(&payload[96..105], &[1, 0, 0, 0, 0, 0, 0, 0, 0]);
        assert_eq!(decode_cow_map(&payload).unwrap().entries, entries);
        assert_eq!(encode_cow_map(&header(), &entries).unwrap(), payload);
    }

    #[test]
    fn rejects_truncation_trailing_bytes_counts_and_tags() {
        let entries = [
            CowMapEntry::ParentRef,
            CowMapEntry::LocalOffset(0x1234),
            CowMapEntry::Unallocated,
        ];
        let payload = encode_cow_map(&header(), &entries).unwrap();
        assert!(decode_cow_map(&payload[..payload.len() - 1]).is_err());

        let mut trailing = payload.clone();
        trailing.push(0);
        assert!(decode_cow_map(&trailing).is_err());

        let mut bad_count = payload.clone();
        bad_count[0x48..0x4C].copy_from_slice(&4u32.to_le_bytes());
        assert!(decode_cow_map(&bad_count).is_err());

        let mut bad_tag = payload;
        bad_tag[96] = 9;
        assert!(decode_cow_map(&bad_tag).is_err());
    }
}
