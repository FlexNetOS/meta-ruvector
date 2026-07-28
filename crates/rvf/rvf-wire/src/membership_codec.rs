//! Strict canonical MEMBERSHIP payload codec.

use rvf_types::{ErrorCode, FilterType, MembershipHeader, RvfError, MEMBERSHIP_MAGIC};

/// Fully decoded canonical membership payload.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DecodedMembership {
    pub header: MembershipHeader,
    pub filter: Vec<u8>,
}

/// Encode a validated membership header and dense bitmap.
pub fn encode_membership(header: &MembershipHeader, filter: &[u8]) -> Result<Vec<u8>, RvfError> {
    validate(header, filter)?;
    let mut payload = Vec::with_capacity(96 + filter.len());
    payload.extend_from_slice(&header.to_bytes());
    payload.extend_from_slice(filter);
    Ok(payload)
}

/// Decode and validate an entire membership payload.
pub fn decode_membership(payload: &[u8]) -> Result<DecodedMembership, RvfError> {
    if payload.len() < 96 {
        return Err(RvfError::Code(ErrorCode::MembershipInvalid));
    }
    let bytes: &[u8; 96] = payload[..96]
        .try_into()
        .map_err(|_| RvfError::Code(ErrorCode::MembershipInvalid))?;
    let header = MembershipHeader::from_bytes(bytes)?;
    let filter = &payload[96..];
    validate(&header, filter)?;
    Ok(DecodedMembership {
        header,
        filter: filter.to_vec(),
    })
}

fn validate(header: &MembershipHeader, filter: &[u8]) -> Result<(), RvfError> {
    // Reparse bytes to enforce all header-level canonical constraints even
    // when a caller constructed the Rust struct directly.
    MembershipHeader::from_bytes(&header.to_bytes())?;
    if header.magic != MEMBERSHIP_MAGIC
        || header.version != 1
        || FilterType::try_from(header.filter_type) != Ok(FilterType::Bitmap)
        || header.filter_offset != 96
        || header.bloom_offset != 0
        || header.bloom_size != 0
        || header.generation_id == 0
    {
        return Err(RvfError::Code(ErrorCode::MembershipInvalid));
    }
    let words = header.vector_count.div_ceil(64);
    let expected = usize::try_from(words)
        .ok()
        .and_then(|words| words.checked_mul(8))
        .ok_or(RvfError::Code(ErrorCode::MembershipInvalid))?;
    if filter.len() != expected
        || usize::try_from(header.filter_size).ok() != Some(expected)
        || rvf_crypto::shake256_256(filter) != header.filter_hash
    {
        return Err(RvfError::Code(ErrorCode::MembershipInvalid));
    }
    let mut count = 0u64;
    for word in filter.chunks_exact(8) {
        count = count
            .checked_add(u64::from_le_bytes(word.try_into().unwrap()).count_ones() as u64)
            .ok_or(RvfError::Code(ErrorCode::MembershipInvalid))?;
    }
    if count != header.member_count {
        return Err(RvfError::Code(ErrorCode::MembershipInvalid));
    }
    if let Some(last) = filter.chunks_exact(8).last() {
        let used = header.vector_count % 64;
        let word = u64::from_le_bytes(last.try_into().unwrap());
        if used != 0 && word >> used != 0 {
            return Err(RvfError::Code(ErrorCode::MembershipInvalid));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rvf_types::FilterMode;

    fn fixture() -> (MembershipHeader, Vec<u8>) {
        let mut filter = vec![0u8; 16];
        filter[0] = 1;
        filter[8] = 1;
        let header = MembershipHeader {
            magic: MEMBERSHIP_MAGIC,
            version: 1,
            filter_type: FilterType::Bitmap as u8,
            filter_mode: FilterMode::Include as u8,
            vector_count: 65,
            member_count: 2,
            filter_offset: 96,
            filter_size: 16,
            generation_id: 4,
            filter_hash: rvf_crypto::shake256_256(&filter),
            bloom_offset: 0,
            bloom_size: 0,
            _reserved: 0,
            _reserved2: [0; 8],
        };
        (header, filter)
    }

    #[test]
    fn byte_exact_round_trip() {
        let (header, filter) = fixture();
        let payload = encode_membership(&header, &filter).unwrap();
        assert_eq!(&payload[..96], &header.to_bytes());
        assert_eq!(decode_membership(&payload).unwrap().filter, filter);
        assert_eq!(encode_membership(&header, &filter).unwrap(), payload);
    }

    #[test]
    fn rejects_truncation_checksum_count_padding_and_generation() {
        let (header, filter) = fixture();
        let payload = encode_membership(&header, &filter).unwrap();
        assert!(decode_membership(&payload[..payload.len() - 1]).is_err());

        let mut corrupt = payload.clone();
        corrupt[96] ^= 2;
        assert!(decode_membership(&corrupt).is_err());

        let mut bad_count = payload.clone();
        bad_count[0x10..0x18].copy_from_slice(&3u64.to_le_bytes());
        assert!(decode_membership(&bad_count).is_err());

        let mut bad_unused = payload.clone();
        bad_unused[111] = 0x80;
        let bad_unused_hash = rvf_crypto::shake256_256(&bad_unused[96..]);
        bad_unused[0x28..0x48].copy_from_slice(&bad_unused_hash);
        bad_unused[0x10..0x18].copy_from_slice(&3u64.to_le_bytes());
        assert!(decode_membership(&bad_unused).is_err());

        let mut generation_zero = payload;
        generation_zero[0x24..0x28].copy_from_slice(&0u32.to_le_bytes());
        assert!(decode_membership(&generation_zero).is_err());
    }
}
