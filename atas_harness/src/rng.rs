//! Deterministic in-crate PRNG (SplitMix64) and FNV-1a fingerprinting.
//!
//! No external `rand` dependency: cross-platform bitwise reproducibility is
//! a gate requirement, so the whole harness draws from this one stream.

/// SplitMix64 (Steele, Lea & Flood, 2014) — tiny, fast, and deterministic.
pub struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    /// Create a generator from an explicit seed.
    pub fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    /// Next raw 64-bit value.
    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform double in `[0, 1)` with 53 bits of precision.
    pub fn next_f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }

    /// Uniform double in `[lo, hi)`.
    pub fn next_uniform(&mut self, lo: f64, hi: f64) -> f64 {
        lo + (hi - lo) * self.next_f64()
    }

    /// Approximately standard-normal draw via Irwin–Hall (sum of 12
    /// uniforms minus 6). Chosen over Box–Muller so determinism never
    /// depends on platform `libm` transcendentals.
    pub fn next_gauss(&mut self) -> f64 {
        let mut s = 0.0;
        for _ in 0..12 {
            s += self.next_f64();
        }
        s - 6.0
    }
}

/// FNV-1a 64-bit hasher for reproducibility fingerprints.
pub struct Fnv1a(u64);

impl Fnv1a {
    /// Start a new hash with the standard FNV offset basis.
    pub fn new() -> Self {
        Self(0xCBF2_9CE4_8422_2325)
    }

    /// Fold one `u64` into the hash byte-by-byte.
    pub fn write_u64(&mut self, v: u64) {
        for b in v.to_le_bytes() {
            self.0 ^= b as u64;
            self.0 = self.0.wrapping_mul(0x0000_0100_0000_01B3);
        }
    }

    /// Fold an `f64` by its exact bit pattern.
    pub fn write_f64(&mut self, v: f64) {
        self.write_u64(v.to_bits());
    }

    /// Finish and return the digest.
    pub fn finish(&self) -> u64 {
        self.0
    }
}

impl Default for Fnv1a {
    fn default() -> Self {
        Self::new()
    }
}
