# rvf-manifest

Two-level manifest system for tracking RVF segments and coordinating compaction.

## Overview

`rvf-manifest` is the two-level manifest system that enables **progressive boot**: a reader needs only the fixed end-of-file block to start answering approximate queries, then loads the full directory asynchronously for full-quality results.

- **Level 0** (`level0`) -- a fixed **4096-byte block at EOF** holding hotset pointers (entrypoint, top-layer, centroid, quant-dict, hot-cache, prefetch-map) plus file metadata, signed and CRC32C-protected. This is all a reader needs for an instant approximate query.
- **Level 1** (`level1`) -- variable-size **TLV records** (`TlvRecord`, `ManifestTag`, `Level1Manifest`) forming the full segment directory, loaded after boot for full-quality results.
- **Two-phase boot** (`boot`) -- `boot_phase1` parses the Level 0 root; `boot_phase2` loads the Level 1 manifest; `extract_hotset_offsets` returns the `HotsetPointers`, with `BootState` tracking progress.
- **Segment directory** (`directory`) -- `SegmentDirectory` / `SegmentDirEntry` describe the active segments.
- **Overlay chain** (`chain`) -- `OverlayChain` for layered manifest composition.

### Public API

- `boot`: `boot_phase1`, `boot_phase2`, `extract_hotset_offsets`, `BootState`, `HotsetPointers`
- `level0`: `read_level0`, `write_level0`, `validate_level0`
- `level1`: `read_tlv_records`, `write_tlv_records`, `Level1Manifest`, `ManifestTag`, `TlvRecord`
- `directory`: `SegmentDirectory`, `SegmentDirEntry`
- `chain`: `OverlayChain`
- `writer`: `build_manifest`, `build_manifest_at`, and `commit_manifest` (under `std`)

## Usage

```toml
[dependencies]
rvf-manifest = "0.1"
```

## Features

- `std` (default) -- enable `std` I/O support

## FileIdentity Storage

The `FileIdentity` struct (68 bytes) is stored at offset `0xF00` within the Level0Root reserved area (252 bytes starting at the end of the signature region). This placement is backward compatible: old readers that ignore the reserved area see zeros and continue working normally.

| Offset | Size | Field |
|--------|------|-------|
| `0xF00` | 16 | `file_id` |
| `0xF10` | 16 | `parent_id` |
| `0xF20` | 32 | `parent_hash` |
| `0xF40` | 4 | `lineage_depth` |

The `read_level0()` and `write_level0()` functions in this crate transparently read and write the `FileIdentity` at these offsets. The CRC32C checksum at offset `0xFFC` covers the entire 4092-byte region including the FileIdentity bytes, ensuring integrity.

## License

MIT OR Apache-2.0
