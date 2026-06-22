# rvm-proof

Proof-gated state transitions for the RVM microhypervisor.

Every mutation to partition state requires a valid proof recorded in the
witness trail. This crate defines the proof tiers, the `Proof` payload
structure, verification functions, and the witness-signing layer (ADR-142).
`Hash`-tier proofs are verified by FNV-1a digest comparison and `Witness`-tier
proofs by walking the embedded witness chain (`prev_hash` linkage); `Zk`-tier
verification returns `RvmError::Unsupported` (TEE required, not yet available).

## Proof Tiers

| Tier | Verification | Cost | Use Case |
|------|-------------|------|----------|
| `Hash` | Preimage check | O(1) | Routine transitions |
| `Witness` | Witness chain verification | O(n) | Cross-partition ops |
| `Zk` | Zero-knowledge proof | Expensive | Privacy-preserving |

## Key Types and Functions

- `ProofTier` -- enum: `Hash`, `Witness`, `Zk`
- `Proof` -- proof payload with tier, commitment hash, and up to 64 bytes of data
- `verify(proof, commitment)` -- verify a proof against an expected commitment
- `verify_with_cap(proof, commitment, token)` -- verify with capability gate

## Example

```rust
use rvm_proof::{Proof, ProofTier, verify};
use rvm_types::WitnessHash;

let commitment = WitnessHash::from_bytes([0xAB; 32]);
let proof = Proof::hash_proof(commitment, b"preimage-data");
assert!(verify(&proof, &commitment).is_ok());
```

## Witness Signing (ADR-142)

The `signer` module cryptographically signs the witness records produced by the
proof pipeline. The `WitnessSigner` trait produces a 64-byte signature over a
32-byte digest (`sign`), verifies one (`verify`, returning a typed
`SignatureError`), and exposes a canonical `signer_id`.

### Signers

- `HmacSha256WitnessSigner` -- HMAC-SHA256 signer (default, no heap allocation); `new(key: [u8; 32])`. Requires the `crypto-sha256` feature.
- `DualHmacSigner` -- strong symmetric signer producing 64-byte signatures via dual HMAC-SHA256; `new(key: [u8; 32])`. Requires `crypto-sha256`.
- `Ed25519WitnessSigner` -- Ed25519 signer using `verify_strict`; `new(secret_key, public_key)`. Requires the `ed25519` feature.
- `NullSigner` -- zero-signature signer for testing only (`#[cfg(any(test, feature = "null-signer"))]`).
- `SealSignerAdapter` -- adapter exposed at the crate root.

### Key Derivation (`crypto-sha256`)

- `KeyBundle` -- a derived set of witness keys.
- `derive_witness_key(measurement, partition_id)` -- derive a per-partition 32-byte key.
- `derive_key_bundle(...)` / `dev_measurement()` -- bundle derivation and a development measurement helper.

### TEE Attestation (ADR-142 Phase 3)

The `tee` module defines the `TeePlatform`, `TeeQuoteProvider`, and
`TeeQuoteVerifier` traits. A software implementation is provided behind
`crypto-sha256`: `SoftwareTeeProvider` (`tee_provider`),
`SoftwareTeeVerifier` (`tee_verifier`), and `TeeWitnessSigner` (`tee_signer`),
which wires a TEE quote into the witness-signer pipeline.

## Design Constraints

- **DC-15**: `#![no_std]`, `#![forbid(unsafe_code)]`, `#![deny(missing_docs)]`
- ADR-135: three-tier proof system (P1/P2/P3)

## Workspace Dependencies

- `rvm-types`
- `rvm-cap`
- `rvm-witness`
