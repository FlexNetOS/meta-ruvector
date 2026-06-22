# rvf-wire

Zero-copy wire format reader and writer for RuVector Format (RVF) segments.

## Overview

`rvf-wire` handles serialization and deserialization of RVF binary segments. It exposes free functions over byte buffers rather than reader/writer objects:

- **`write_segment`** -- encode a segment (with `calculate_padded_size` for sizing)
- **`read_segment`** / **`read_segment_header`** -- decode a segment or just its header
- **`validate_segment`** -- check a segment's structure and integrity
- **`find_latest_manifest`** -- tail-scan a buffer for the most recent manifest segment

Lower-level building blocks live in submodules: `varint`, `delta` (delta coding), `hash`, and the per-segment-type codecs (`hot_seg_codec`, `index_seg_codec`, `manifest_codec`, `vec_seg_codec`).

## Usage

```toml
[dependencies]
rvf-wire = "0.1"
```

```rust
use rvf_wire::{write_segment, read_segment, find_latest_manifest};
```

## Features

- `std` (default) -- enable `std` I/O support

## License

MIT OR Apache-2.0
