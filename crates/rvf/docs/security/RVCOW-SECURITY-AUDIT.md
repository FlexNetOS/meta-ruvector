# RVCOW Security Audit Report

| Field | Value |
|-------|-------|
| **Date** | 2026-02-14 |
| **Auditor** | Security Auditor Agent (Claude Opus 4.6) |
| **Scope** | RVCOW copy-on-write branching implementation per ADR-031 |
| **Status** | Complete |
| **Files Reviewed** | 17 source files across rvf-types, rvf-runtime, rvf-cli |

---

## INV-011 Canonical Persistence Resolution (2026-07-27)

The original ADR-031 COW table is internally contradictory: it calls
`CowMapHeader` 64 bytes while assigning fields through offset `0x5F`. The
runtime now treats that incomplete 64-byte shape as **V1, read-only** and uses
an honest **V2 96-byte header** for every new COW map. V2 contains
`map_root_offset`, `cluster_count`, `local_cluster_count`, `extent_support`,
and an explicit monotonic `generation_id`; reserved bytes must be zero.

Canonical COW_MAP and MEMBERSHIP payloads are encoded and validated only by
the strict codecs in `rvf-wire`. Their segment frames use SHAKE-256/128
(`checksum_algo=2`) and zero padding to the 64-byte segment boundary. Their
32-byte payload hashes use `rvf_crypto::shake256_256`.

Compatibility is intentionally one-way:

- historical runtime frames with `checksum_algo=0`, the CRC32-rotation digest,
  and no padding remain readable;
- historical COW V1 payloads are accepted only inside that legacy frame;
- canonical V2 COW and MEMBERSHIP payloads are accepted only in canonical
  SHAKE/padded frames;
- mixed framing/version combinations, zero hashes, non-zero padding, stale
  generations, and malformed counts fail closed;
- no writer emits the historical algo-0/unpadded form.

Both segment types are linked through the manifest directory. The manifest's
versioned generation trailer identifies the current COW and membership
generations while retaining older directory entries as superseded history.
Those counters are strict in-file consistency guards: replaying only one
segment or mixing generations is rejected, but replaying an older
self-consistent file image still requires an external trusted generation root
to detect. They are not described as cryptographic rollback protection.

COW lineage now binds the child to a canonical parent snapshot digest. Clean
manifest-only appends do not change that digest, while every referenced
non-manifest segment is bound by a full SHAKE-256 payload digest. Parent lookup
requires both the exact `file_id` and snapshot digest; payload tampering or a
post-branch parent append fails with `ParentChainBroken`.

Include membership bits mean visible and Exclude membership bits mean
tombstoned. Child deletion therefore clears a bit in Include mode and sets a
bit in Exclude mode. `rvf filter --output` creates a real COW branch before
installing the requested filter, so the closed/reopened output remains a
queryable filtered parent view. Both CLI consumers use public, manifest-linked
store APIs rather than guessed offsets or unmanifested raw segments.

The dated findings below are retained as the historical audit record. Status
and checklist entries updated in this resolution section supersede their
original line references and recommendations.

---

## Executive Summary

The original 2026-02-14 audit identified **2 Critical**, **6 High**, **5
Medium**, and **4 Low** severity findings. This document preserves that
historical inventory; the 2026-07-27 resolution above and per-finding status
updates below describe the current implementation.

### Findings Summary

| Severity | Count | Fixed |
|----------|-------|-------|
| Critical | 2 | 2 |
| High | 6 | 5 |
| Medium | 5 | 0 |
| Low | 4 | 0 |
| Info | 3 | 0 |
| **Total** | **20** | **7** |

---

## Critical Findings

### C-01: Non-Cryptographic Hash Used for Integrity Verification

**Severity**: Critical
**Location**: `/workspaces/ruvector/crates/rvf/rvf-runtime/src/store.rs:1239-1251`
**Status**: **FIXED (2026-07-27)**

**Original Description**: `simple_shake256_256` was a trivially reversible
XOR-fold hash despite its SHAKE name. It was used for:
- `parent_hash` in `FileIdentity` (lineage verification)
- `filter_hash` in `MembershipHeader` (filter integrity)
- COW witness event hashes (`parent_cluster_hash`, `new_cluster_hash`)
- Cluster deduplication in space-reclaim compaction

**Fix Applied**: `rvf-runtime` now depends on `rvf-crypto`, and the compatibility
function delegates to `rvf_crypto::shake256_256`. Membership payloads verify a
full 32-byte SHAKE-256 digest; canonical segment frames use SHAKE-256/128.
Parent resolution additionally recomputes the canonical parent snapshot digest
and requires both that digest and the exact parent ID.

### C-02: KernelBinding from_bytes Does Not Validate Reserved/Padding Fields

**Severity**: Critical
**Location**: `/workspaces/ruvector/crates/rvf/rvf-types/src/kernel_binding.rs:61`
**Status**: **FIXED**

**Description**: `KernelBinding::from_bytes` accepted arbitrary data in `_pad0` and `_reserved` fields. ADR-031 specifies these MUST be zero. Non-zero reserved fields enable:
1. Data smuggling through the KernelBinding structure
2. Future format confusion if reserved fields gain meaning
3. Signature bypass if `signed_data` includes different reserved values

**Fix Applied**: Added `from_bytes_validated()` method that rejects non-zero `_pad0`, non-zero `_reserved`, and `binding_version == 0`. The original `from_bytes` is preserved for backward compatibility with a documentation note.

---

## High Findings

### H-01: Division by Zero in CowEngine with vectors_per_cluster=0

**Severity**: High
**Location**: `/workspaces/ruvector/crates/rvf/rvf-runtime/src/cow.rs:106,164`
**Status**: **FIXED**

**Description**: `CowEngine::read_vector` and `write_vector` compute `cluster_id = vector_id / vectors_per_cluster`. If `vectors_per_cluster` is 0, this causes a panic (integer division by zero). A malicious or corrupted `CowMapHeader` with `vectors_per_cluster=0` would crash the runtime.

**Fix Applied**: Added `assert!(vectors_per_cluster > 0)` to both `CowEngine::new()` and `CowEngine::from_parent()` constructors.

### H-02: Silent Write Drop on Out-of-Bounds Vector Offset

**Severity**: High
**Location**: `/workspaces/ruvector/crates/rvf/rvf-runtime/src/cow.rs:253-258`
**Status**: **FIXED**

**Description**: In `flush_writes`, when `end > cluster_data.len()`, the write was silently skipped (`if end <= cluster_data.len()`). This means data could be silently lost without any error indication, violating write durability guarantees.

**Impact**: An attacker or buggy caller could trigger silent data loss by crafting vector writes where `vector_offset_in_cluster + data.len()` exceeds cluster size.

**Fix Applied**: Changed the condition to return `Err(RvfError::Code(ErrorCode::ClusterNotFound))` when the write would exceed cluster bounds.

### H-03: CowMapHeader Deserialization Missing Critical Validations

**Severity**: High
**Location**: `/workspaces/ruvector/crates/rvf/rvf-types/src/cow_map.rs:97-124`
**Status**: **FIXED**

**Description**: `CowMapHeader::from_bytes` only validated the magic number. It did not validate:
- `map_format` is a known enum value (could be 0xFF)
- `cluster_size_bytes` is non-zero and a power of 2 (spec requirement for SIMD alignment)
- `vectors_per_cluster` is non-zero (prevents division by zero downstream)

**Fix Applied**: Added validation for all three fields, returning appropriate `RvfError` on invalid values.

### H-04: RefcountHeader Deserialization Missing Field Validation

**Severity**: High
**Location**: `/workspaces/ruvector/crates/rvf/rvf-types/src/refcount.rs:59-82`
**Status**: **FIXED**

**Description**: `RefcountHeader::from_bytes` did not validate:
- `refcount_width` must be 1, 2, or 4 (spec requirement)
- `_pad` must be zero (spec requirement)
- `_reserved` must be zero (spec requirement)

Invalid `refcount_width` could cause incorrect array indexing when reading the refcount array.

**Fix Applied**: Added validation for all three constraints.

### H-05: CowMap Deserialize Integer Overflow

**Severity**: High
**Location**: `/workspaces/ruvector/crates/rvf/rvf-runtime/src/cow_map.rs:93-94`
**Status**: **FIXED**

**Description**: `CowMap::deserialize` computed `expected_len = 5 + count * 9` without checked arithmetic. With a crafted `count` value near `usize::MAX / 9`, the multiplication could overflow, causing `expected_len` to wrap to a small value. This would pass the length check and then cause out-of-bounds reads in the deserialization loop.

**Fix Applied**: Replaced with `count.checked_mul(9).and_then(|v| v.checked_add(5))`, returning `CowMapCorrupt` on overflow.

### H-06: verify_attestation Does Not Verify manifest_root_hash

**Severity**: High
**Location**: `/workspaces/ruvector/crates/rvf/rvf-cli/src/cmd/verify_attestation.rs:49-66`
**Status**: Documented (requires architecture decision)

**Description**: The `verify_attestation` CLI command extracts and displays the `KernelBinding`, but does NOT actually verify that `manifest_root_hash` matches the current file's manifest. Per ADR-031 Section 7.5, the launcher verification sequence requires:
1. Compute SHAKE-256-256 of current Level0Root
2. Compare to `KernelBinding.manifest_root_hash`
3. Refuse to boot on mismatch

The current implementation skips steps 1-3, merely displaying the hash values. This completely defeats the anti-segment-swap protection that KernelBinding is designed to provide.

**Impact**: An attacker can take a signed kernel from file A, embed it into file B (different vectors, different manifest), and `verify-attestation` will report "valid" because it only checks magic bytes, not the binding.

**Recommendation**: Implement the full verification sequence. This requires either:
- Computing the real manifest hash (needs crypto dependency)
- At minimum, extracting the manifest and comparing hashes using available tools

---

## Medium Findings

### M-01: MembershipHeader Deserialization Does Not Validate Reserved Fields

**Severity**: Medium
**Location**: `/workspaces/ruvector/crates/rvf/rvf-types/src/membership.rs:126-171`

**Description**: `MembershipHeader::from_bytes` does not validate that `_reserved` and `_reserved2` are zero. While not as critical as KernelBinding (no signing is involved), non-zero reserved fields violate the spec and could cause future compatibility issues.

**Recommendation**: Add zero-check for `_reserved` and `_reserved2` fields.

### M-02: DeltaHeader Deserialization Does Not Validate Reserved Fields

**Severity**: Medium
**Location**: `/workspaces/ruvector/crates/rvf/rvf-types/src/delta.rs:88-119`

**Description**: `DeltaHeader::from_bytes` does not validate that `_pad` and `_reserved` are zero.

**Recommendation**: Add zero-check for both fields.

### M-03: Freeze CLI Bypasses Store API

**Severity**: Medium
**Location**: `/workspaces/ruvector/crates/rvf/rvf-cli/src/cmd/freeze.rs:43-54`

**Description**: The `freeze` CLI command opens the store, but then directly opens the file again and writes raw segment bytes, bypassing the `RvfStore::freeze()` API. This means:
1. The segment header hash is not computed/validated
2. The segment is not recorded in the manifest
3. The writer lock from `RvfStore::open()` is held while another file handle writes

**Impact**: The REFCOUNT_SEG written by the CLI is effectively invisible to the runtime -- it won't be in the manifest's segment directory. The store's freeze state is not actually recorded in any way the runtime can detect on next open.

**Recommendation**: Use `store.freeze()` instead of raw file writes, or update the manifest after writing the raw segment.

### M-04: Filter CLI Bypasses Store API

**Severity**: Medium
**Location**: `/workspaces/ruvector/crates/rvf/rvf-cli/src/cmd/filter.rs:97-109`
**Status**: **FIXED (2026-07-27)**

**Original Description**: The `filter` CLI command wrote a raw MEMBERSHIP_SEG
directly to the file, bypassing the store API.

**Fix Applied**: The CLI uses `append_membership_filter`, and `--output`
creates a persisted COW branch rather than an empty derived file. Its
close/reopen/query behavior is covered by an end-to-end CLI test.

### M-05: No Parent Chain Depth Limit Enforced

**Severity**: Medium
**Location**: `/workspaces/ruvector/crates/rvf/rvf-runtime/src/cow.rs:137-140`

**Description**: ADR-031 Section 8.1 specifies a 64-level depth limit for parent chain traversal to prevent cycles and unbounded recursion. The current `CowEngine::read_cluster` follows `ParentRef` to the parent file, but there is no depth counter or cycle detection. A malicious chain of files referencing each other could cause stack overflow or infinite loops.

**Recommendation**: Add a depth counter to parent chain resolution. The `lineage_depth` field in `FileIdentity` should be checked against the 64-level limit.

---

## Low Findings

### L-01: generation_id Not Validated Monotonically

**Severity**: Low
**Location**: `/workspaces/ruvector/crates/rvf/rvf-runtime/src/membership.rs:133-135`
**Status**: **FIXED (2026-07-27)**

**Fix Applied**: Reopen requires the decoded COW/MEMBERSHIP generation to equal
the manifest's corresponding generation and rejects mismatches with
`GenerationStale`. This detects mixed in-file generations, not replay of an
older self-consistent whole-file image.

### L-02: No Overflow Check on generation_id Increment

**Severity**: Low
**Location**: `/workspaces/ruvector/crates/rvf/rvf-runtime/src/membership.rs:134`
**Status**: **FIXED (2026-07-27)**

**Fix Applied**: Generation increments use `checked_add` and return
`GenerationStale` on overflow.

### L-03: Cluster ID Multiplication Overflow in Parent Read

**Severity**: Low
**Location**: `/workspaces/ruvector/crates/rvf/rvf-runtime/src/cow.rs:139`

**Description**: `let parent_offset = cluster_id as u64 * self.cluster_size as u64;` could theoretically overflow for very large cluster IDs combined with large cluster sizes, though this requires `cluster_id * cluster_size > u64::MAX` which is unlikely.

**Recommendation**: Use `checked_mul` for defense-in-depth.

### L-04: Bitmap Filter Allows Inconsistent member_count on Deserialization

**Severity**: Low
**Location**: `/workspaces/ruvector/crates/rvf/rvf-runtime/src/membership.rs:147-182`
**Status**: **FIXED (2026-07-27)**

**Fix Applied**: The strict `rvf-wire` codec rejects a header count that does
not equal the bitmap popcount, as well as non-zero unused tail bits.

---

## Informational Findings

### I-01: simple_hash Duplicated in CLI

**Severity**: Info
**Location**: `/workspaces/ruvector/crates/rvf/rvf-cli/src/cmd/filter.rs:132-140`
**Status**: **FIXED (2026-07-27)**

**Fix Applied**: The duplicate CLI hash was removed; canonical MEMBERSHIP
encoding and hashing route through `RvfStore` and `rvf-wire`.

### I-02: KernelBinding Version 0 Used as "Not Present" Sentinel

**Severity**: Info
**Location**: `/workspaces/ruvector/crates/rvf/rvf-runtime/src/store.rs:773`

**Description**: `extract_kernel_binding` uses `binding_version == 0` to detect "no binding present" (line 773). This means version 0 can never be a valid binding version. This should be documented as a format invariant.

### I-03: Witness Event Hash Placeholder

**Severity**: Info
**Location**: `/workspaces/ruvector/crates/rvf/rvf-runtime/src/cow.rs:232`

**Description**: When emitting a `CLUSTER_COW` witness event, the `new_cluster_hash` is initially set to `[0u8; 32]` and updated later (line 270). If the update loop doesn't find the matching event (e.g., due to a logic bug), the witness event would contain an all-zeros hash. Consider using a sentinel value that is explicitly invalid (e.g., `[0xFF; 32]`).

---

## Security Checklist Results

### 1. KernelBinding Verification

- [x] **manifest_root_hash verified before kernel boot?** -- NO. `verify_attestation` CLI does not verify this (H-06). The runtime does not enforce this either.
- [x] **KernelBinding strippable without detection?** -- PARTIALLY. If the binding is removed, `extract_kernel_binding` returns `None` (backward-compatible). Signature verification would detect removal if signatures are present, but unsigned kernels have no protection.
- [x] **signed_data correctly constructed?** -- YES. `embed_kernel_with_binding` includes `KernelHeader || KernelBinding || cmdline || image` in the correct order per ADR-031.
- [x] **binding_version validated?** -- YES (after fix C-02). `from_bytes_validated` rejects version 0.
- [x] **Reserved fields checked?** -- YES (after fix C-02). `from_bytes_validated` rejects non-zero reserved.

### 2. COW Map Security

- [x] **Malicious redirect possible?** -- Parent lookup requires the exact
  `file_id` and canonical SHAKE-256 snapshot digest; payload tampering and
  post-branch parent appends are rejected.
- [x] **cluster_id range validated?** -- YES. Out-of-bounds lookup returns `Unallocated`.
- [x] **Parent chain cycle prevention?** -- NO. No depth limit enforced (M-05).
- [x] **Offsets validated before dereferencing?** -- YES. File I/O will return errors on invalid offsets.
- [x] **Map deterministic?** -- YES. Flat array is inherently ordered by cluster_id.

### 3. Membership Filter Security

- [x] **Empty include filter blocks all access?** -- YES. Verified by test `include_mode_empty_is_empty_view`.
- [x] **generation_id validated monotonically?** -- YES for in-file
  COW/MEMBERSHIP-to-manifest consistency. Whole-file rollback requires an
  external trusted generation root.
- [x] **Filter bitmap bounds checked?** -- YES. `bitmap_contains` checks `vector_id >= vector_count`.
- [x] **filter_hash verified on load?** -- YES. The strict codec recomputes the
  full `rvf_crypto` SHAKE-256 digest.

### 4. Crash Recovery

- [x] **Double-root scheme implemented?** -- NOT YET. The runtime code does not implement the double-root scheme described in ADR-031 Section 8.3. Current implementation uses append-only manifests.
- [x] **Orphaned data accessible after failed writes?** -- NO. Orphaned appended data has no manifest reference and is invisible.
- [x] **Generation counters validated?** -- PARTIALLY. Increment, overflow,
  and current-manifest equality are enforced, but a trusted external anchor is
  still required to detect a self-consistent whole-file rollback.

### 5. Input Validation

- [x] **Deserialization safe with arbitrary input?** -- YES (after fixes H-03, H-04, H-05). All headers validate magic, enum values, and bounds.
- [x] **Magic numbers checked?** -- YES. All four new headers check magic on deserialization.
- [x] **Sizes validated before allocation?** -- YES (after fix H-05). Checked arithmetic prevents overflow.
- [x] **Offset+length bounds checked?** -- YES. File I/O operations use `read_exact` which fails on short reads.

### 6. Integer Overflow

- [x] **cluster_id * cluster_size overflow?** -- LOW RISK. Uses `u64` arithmetic (L-03).
- [x] **vector_id / vectors_per_cluster panic on zero?** -- FIXED (H-01). Constructors now assert > 0.
- [x] **Capacity calculations safe?** -- YES (after fix H-05). Deserialization uses checked arithmetic.

### 7. Downgrade Prevention

- [x] **Signed kernel replaceable with unsigned?** -- YES. No enforcement prevents replacing a signed KERNEL_SEG with an unsigned one. ADR-031 Section 9 specifies signed-required downgrade prevention, but this is not implemented.
- [x] **Older api_version forceable?** -- YES. No version pinning in KernelBinding currently enforced.
- [x] **Filter mode switchable?** -- YES. No mechanism prevents changing filter_mode from Include to Exclude, which could expose all vectors in a branch.

---

## Threat Model Alignment

| ADR-031 Threat | Implementation Status | Assessment |
|----------------|----------------------|------------|
| Host compromise | VMM not implemented (launcher is stub) | NOT TESTABLE |
| Guest compromise | Kernel is stub; eBPF verifier not implemented | NOT TESTABLE |
| TEE integrity | Not implemented | NOT TESTABLE |
| Supply chain | Signatures supported in type system | PARTIAL |
| Replay attack | generation equality enforced in-file; no external trusted root | PARTIAL |
| Data swap | KernelBinding exists but verification not enforced | INCOMPLETE |
| Malicious alt kernel | Deterministic selection not implemented | NOT IMPLEMENTED |
| COW map poisoning | strict codec, SHAKE frame, parent snapshot binding | IMPLEMENTED for file integrity |
| Stale membership filter | strict hash/count/generation validation | IMPLEMENTED in-file; external rollback anchor pending |

---

## Positive Observations

1. **Compile-time size assertions** on all headers prevent ABI drift.
2. **Field offset tests** verify `repr(C)` layout matches spec.
3. **Magic number validation** on all `from_bytes` paths.
4. **Round-trip serialization tests** catch encoding bugs.
5. **Frozen snapshot enforcement** correctly prevents writes via `SnapshotFrozen` error.
6. **Write coalescing** correctly batches multiple writes to same cluster.
7. **Membership filter** correctly implements fail-safe (empty include = empty view).
8. **Bitmap bounds checking** prevents out-of-bounds bit access.
9. **Write buffer drain before freeze** prevents data loss.
10. **Checked arithmetic in scan_preservable_segments** prevents overflow on crafted payloads.

---

## Recommendations (Priority Order)

1. **P0**: Implement `manifest_root_hash` verification in
   `verify_attestation` and in the kernel boot path.
2. **P0**: Anchor COW/MEMBERSHIP generations in a trusted external or Level0
   root so rollback of an older self-consistent whole-file image is detectable.
3. **P1**: Enforce the 64-level parent-chain limit and cycle prevention from
   ADR-031.
4. **P1**: Implement signed-required downgrade prevention per ADR-031 Section
   9.
5. **P2**: Fix the freeze CLI command to use the store API instead of raw
   segment writes.
6. **P2**: Add reserved-field validation to `MembershipHeader` and
   `DeltaHeader` deserialization where not already enforced by the strict wire
   codec.

---

## Files Modified by This Audit

| File | Change |
|------|--------|
| `rvf-types/src/kernel_binding.rs` | Added `from_bytes_validated()` with reserved/pad/version checks |
| `rvf-types/src/cow_map.rs` | Added `map_format`, `cluster_size_bytes`, `vectors_per_cluster` validation |
| `rvf-types/src/refcount.rs` | Added `refcount_width`, `_pad`, `_reserved` validation |
| `rvf-runtime/src/cow.rs` | Added `vectors_per_cluster > 0` assertion; changed silent write drop to error |
| `rvf-runtime/src/cow_map.rs` | Added checked arithmetic for `count * 9` overflow |

## Historical 2026-02-14 Test Results

```
rvf-types:   122 passed, 0 failed
rvf-runtime:  65 passed, 0 failed
rvf-cli:       0 passed, 0 failed (no unit tests)
integration:   6 passed, 2 failed (pre-existing failures in cow_branching.rs)
```

These counts are retained only as the original audit receipt. Current
verification includes `rvf-integration-tests`; the canonical INV-011 artifact
records the current command, counts, and source digest.
