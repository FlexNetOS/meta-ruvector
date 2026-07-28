//! Cryptographic SQL functions backed by the canonical RVF primitives.

use pgrx::prelude::*;

/// Return the 256-bit SHAKE256 digest of an arbitrary PostgreSQL `bytea`.
///
/// `Vec<u8>` maps to `bytea` in pgrx. A non-optional argument also makes the
/// generated SQL function `STRICT`, so PostgreSQL returns `NULL` without
/// invoking Rust when the input is `NULL`.
#[pg_extern(immutable, parallel_safe)]
pub fn ruvector_shake256_256(input: Vec<u8>) -> Vec<u8> {
    rvf_crypto::shake256_256(&input).to_vec()
}

#[cfg(test)]
mod tests {
    use super::ruvector_shake256_256;

    #[test]
    fn shake256_256_matches_nist_empty_input_vector() {
        assert_eq!(
            ruvector_shake256_256(Vec::new()),
            [
                0x46, 0xb9, 0xdd, 0x2b, 0x0b, 0xa8, 0x8d, 0x13, 0x23, 0x3b, 0x3f, 0xeb, 0x74, 0x3e,
                0xeb, 0x24, 0x3f, 0xcd, 0x52, 0xea, 0x62, 0xb8, 0x1b, 0x82, 0xb5, 0x0c, 0x27, 0x64,
                0x6e, 0xd5, 0x76, 0x2f,
            ]
        );
    }

    #[test]
    fn shake256_256_matches_nist_abc_vector() {
        assert_eq!(
            ruvector_shake256_256(b"abc".to_vec()),
            [
                0x48, 0x33, 0x66, 0x60, 0x13, 0x60, 0xa8, 0x77, 0x1c, 0x68, 0x63, 0x08, 0x0c, 0xc4,
                0x11, 0x4d, 0x8d, 0xb4, 0x45, 0x30, 0xf8, 0xf1, 0xe1, 0xee, 0x4f, 0x94, 0xea, 0x37,
                0xe7, 0x8b, 0x57, 0x39,
            ]
        );
    }
}
