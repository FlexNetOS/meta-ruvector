# ruvector-rabitq

RaBitQ: rotation-based 1-bit quantization for ultra-fast approximate nearest-neighbor search with theoretical error bounds.

## Overview

`ruvector-rabitq` implements rotation-based 1-bit vector quantization motivated by the SIGMOD 2024 RaBitQ algorithm (Gao & Long). It ships two estimators — a symmetric Charikar-style estimator (both query and database 1-bit, cheapest per candidate) and an asymmetric RaBitQ-2024-style estimator (f32 query against 1-bit database, tighter variance) — across several index variants. The crate is deterministic (a `(dim, seed, data)` triple yields bit-identical builds and search output), uses no `unsafe` and no external BLAS/LAPACK, and is one of the ANN index families in the RuVector stack. A `rabitq-demo` binary measures recall and throughput.

## Key API

- `AnnIndex` — common index trait implemented by all variants.
- `FlatF32Index` — f32 originals with exact L2 (baseline).
- `RabitqIndex` — rotation + 1-bit codes, symmetric estimator.
- `RabitqPlusIndex` — codes plus stored originals with exact rerank.
- `RabitqAsymIndex` — asymmetric estimator with optional rerank.
- `SearchResult` — ranked result type.
- `CpuKernel`, `VectorKernel`, `KernelCaps`, `ScanRequest`, `ScanResponse` — distance-scan kernel interface.
- `RandomRotation`, `RandomRotationKind` — deterministic Haar-style rotations.
- `pack_bits`, `unpack_bits`, `BinaryCode` — padding-safe 1-bit code packing.
- `RabitqError` — crate error type.

## License

MIT
