# rvf-runtime

RuVector Format runtime providing the RvfStore API, background compaction, and streaming I/O.

## Overview

`rvf-runtime` is the main entry point for applications that want to read and write RVF files:

- **`RvfStore`** -- high-level API for storing and retrieving vectors (`store` module)
- **HNSW query path** -- queries route through the persisted HNSW index when one is available
- **Compaction** -- background merge of segments to reclaim space
- **Streaming I/O** -- append-only writes with configurable flush policy

### DoS hardening (`dos`, ADR-033 §3.3.1)

- **`BudgetTokenBucket`** -- per-connection token bucket rate-limiting distance ops (`try_consume`, `remaining`, `refill`).
- **`NegativeCache`** -- caches degenerate query signatures and blacklists repeat offenders (`record_degenerate`, `is_blacklisted`).
- **`ProofOfWork`** -- optional proof-of-work challenge for public endpoints.
- **`QuerySignature`** -- compact fingerprint of a query (`from_query`) used as the negative-cache key.

### Adversarial safety (`adversarial`, `safety_net`)

- **`is_degenerate_distribution`** / **`centroid_distance_cv`** -- detect degenerate (adversarial) distance distributions via coefficient-of-variation, with `DEGENERATE_CV_THRESHOLD`.
- **`adaptive_n_probe`**, **`effective_n_probe_with_drift`**, **`combined_effective_n_probe`** -- adapt the number of probes when the distribution is degenerate.
- **`should_activate_safety_net`** / **`selective_safety_net_scan`** -- fall back to a bounded exact scan (`Candidate`, `SafetyNetResult`) when HNSW returns too few candidates.

### AGI cognitive container (`agi_container`, ADR-036)

- **`AgiContainerBuilder`** / **`ParsedAgiManifest`** -- assemble and parse the META segment that pins an intelligence runtime (model id, governance policy, orchestrator/tool/agent configs, eval suite, skill library) into a single RVF artifact. Builder uses a fluent `with_*` API (`with_model_id`, `with_policy`, `with_orchestrator`, ...). The other AGI parts (`agi_authority`, `agi_coherence`) provide authority and coherence-gate support.

### Seed, witness, and crypto

- **`SeedBuilder`** / **`ParsedSeed`** / **`DownloadManifest`** -- bootstrap-seed assembly (`qr_seed`).
- **`WitnessBuilder`** / **`ParsedWitness`** / **`ScorecardBuilder`** / **`GovernancePolicy`** -- witness records and governance (`witness`, ADR-035).
- **`sign_seed`** / **`verify_seed`** / **`seed_content_hash`** (HMAC-SHA256; Ed25519 variants under the `ed25519` feature) -- seed signing/verification (`seed_crypto`).
- **`CowEngine`** / **`CowMap`** / **`CowCompactor`** -- copy-on-write segment management.

## Usage

```toml
[dependencies]
rvf-runtime = "0.3"
```

## Features

- `std` (default) -- enable `std` I/O support
- `wasm` -- enable WASM-compatible runtime paths

## Query Path

`RvfStore::query` routes through the persisted HNSW index when an INDEX_SEG
is present in the file; otherwise it falls back to an exact brute-force scan.

- The index is persisted as an INDEX_SEG with a self-delimiting ID-mapping
  trailer (`"RVIX"` magic). Readers that only understand the plain INDEX_SEG
  codec ignore the trailer.
- Index rebuilds after ingest/delete are **non-blocking**: queries serve from
  the exact scan until the new index commits.
- `QueryOptions::force_exact` forces the exact scan even when an index is
  available (ground-truth comparison, benchmarking).
- Result ordering uses deterministic `(distance, id)` tie-breaking.

### RaBitQ opt-in (`QueryOptions::rabitq`)

Setting `rabitq: true` enables a two-stage path: a 1-bit-code candidate scan
(~32x smaller than f32) followed by an exact f32 rescore of the oversampled
candidates (`rabitq_oversample`, default 4x). v1 serves the L2 metric only;
other metrics and filtered/COW queries fall back to the default routing.

### In-Memory Vector Slab

In-memory vectors are stored in one contiguous row-major slab with an
id -> ordinal map (no per-vector heap allocation). Removals tombstone in
place; slots are reclaimed during compaction.

### Measured Performance

Environment: Windows x64, criterion release builds, 100k vectors x 64 dims, k=10.

| Benchmark | Baseline | Measured | Quality |
|-----------|----------|----------|---------|
| k-NN query via HNSW index | 21.7 ms (brute force) | **1.51 ms** | recall@10 0.968 |
| Brute-force scan (contiguous slab) | 24.5 ms (per-vector heap allocs) | **3.8 ms** | exact |
| Cold open (slab layout) | — | **-21.5%** open time | — |
| RaBitQ two-stage query (opt-in) | f32 codes | 32x code compression | recall@10 0.972 |

## Lineage Derivation

`RvfStore` supports DNA-style derivation chains where a parent store produces child stores with provenance linkage.

### `derive()` Method

Creates a child store that records this store as its parent. The child gets a new `file_id`, inherits dimensions and options, and records the parent's manifest hash for later verification:

```rust
use rvf_runtime::{RvfStore, options::RvfOptions};
use rvf_types::DerivationType;
use std::path::Path;

let parent = RvfStore::create(Path::new("parent.rvf"), options)?;
let child = parent.derive(
    Path::new("child.rvf"),
    DerivationType::Filter,
    None, // inherit parent options
)?;
assert_eq!(child.lineage_depth(), 1);
```

### FileIdentity Accessors

| Method | Return | Description |
|--------|--------|-------------|
| `file_id()` | `&[u8; 16]` | This file's unique identifier |
| `parent_id()` | `&[u8; 16]` | Parent file's identifier (zeros if root) |
| `lineage_depth()` | `u32` | Derivation depth (0 for root files) |
| `file_identity()` | `&FileIdentity` | Full 68-byte identity struct |

### Extension Aliasing and Domain Profiles

RVF files can use domain-specific extensions that are automatically detected on `create()` and `open()`:

| Extension | Domain Profile | Optimized For |
|-----------|---------------|---------------|
| `.rvf` | Generic | General-purpose vectors |
| `.rvdna` | RVDNA | Genomic sequence embeddings |
| `.rvtext` | RVText | Language model embeddings |
| `.rvgraph` | RVGraph | Graph/network node embeddings |
| `.rvvis` | RVVision | Image/vision model embeddings |

When a child is derived with `derive()`, the child's extension also controls its domain profile. For example, deriving a `.rvdna` child from a `.rvf` parent automatically sets the child's profile to RVDNA.

### FIDI Magic Marker

When `FileIdentity` is present (non-zero `file_id`), the manifest segment includes a 4-byte FIDI magic marker trailer followed by the 68-byte `FileIdentity`. This ensures backward compatibility: old readers that do not recognize the FIDI marker simply stop parsing the manifest payload at the expected end and ignore the trailing bytes.

## Computational Container

`rvf-runtime` provides low-level write-path support for the two computational container segment types defined in [ADR-030](../../../docs/adr/ADR-030-rvf-computational-container.md): KERNEL_SEG (`0x0E`) and EBPF_SEG (`0x0F`).

### Public `RvfStore` API

`RvfStore` exposes public methods for embedding and extracting computational container segments:

- `embed_kernel(arch, kernel_type, kernel_flags, kernel_image, api_port, cmdline)` -- writes a KERNEL_SEG (128-byte `KernelHeader` + kernel image + optional command line); returns the new segment id.
- `embed_kernel_with_binding(...)` / `extract_kernel_binding()` -- kernel embedding with a `KernelBinding` record.
- `extract_kernel()` -- reads the KERNEL_SEG back, returning `Option<(header_bytes, image_bytes)>`.
- `embed_ebpf(program_type, attach_type, max_dimension, program_bytecode, btf_data)` -- writes an EBPF_SEG (64-byte `EbpfHeader` + bytecode + optional BTF); returns the new segment id.
- `extract_ebpf()` -- reads the EBPF_SEG back, returning `Option<(header_bytes, program_bytes)>`.

Under the hood these delegate to the `pub(crate)` `SegmentWriter::write_kernel_seg()` / `write_ebpf_seg()` methods in the write-path layer.

### Unknown Segment Preservation

During compaction, `rvf-runtime` preserves segments with unknown or unrecognized types. This means KERNEL_SEG and EBPF_SEG payloads written by newer tooling are retained even when compaction is performed by a runtime version that predates the computational container feature. The compactor copies unknown segments verbatim to the compacted output.

### Example: Embed and Extract a Kernel Segment

```rust
// Embed a kernel image into the store's .rvf file.
// arch=0x00 (x86_64), kernel_type=0xFE (test stub),
// kernel_flags=0x0050 (HAS_QUERY_API | HAS_ADMIN_API), api_port=8080.
let seg_id = store.embed_kernel(
    0x00,
    0xFE,
    0x0050,
    b"test-kernel-stub",
    8080,
    Some("console=ttyS0"),
)?;

// Extract it back (None if no KERNEL_SEG is present).
if let Some((header_bytes, image_bytes)) = store.extract_kernel()? {
    // header_bytes is the 128-byte KernelHeader; image_bytes is the kernel image.
}

// eBPF programs use the analogous embed_ebpf() / extract_ebpf() pair.
let ebpf_seg = store.embed_ebpf(0, 0, 64, program_bytecode, None)?;
let _ = store.extract_ebpf()?;
```

## License

MIT OR Apache-2.0
