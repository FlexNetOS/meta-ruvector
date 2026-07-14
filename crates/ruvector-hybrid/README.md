# ruvector-hybrid

Hybrid sparse-dense search: BM25 + ANN + Reciprocal Rank Fusion for RuVector.

## Overview

`ruvector-hybrid` unifies three retrieval backends — lexical BM25, exact dense cosine ANN, and fused hybrid ranking — behind common traits. A `Document` carries both pre-tokenized text and a dense embedding, so the same corpus can be queried sparsely, densely, or with rank fusion. See `docs/adr/ADR-256-hybrid-sparse-dense-search.md` for rationale. The crate also ships a `hybrid-demo` binary.

## Key API

- `Document` — `{ id, tokens, vector }` carrying text tokens and a dense embedding.
- `SearchResult` — `{ id, score }` ranked result.
- `Bm25Index` — Robertson BM25 lexical sparse retrieval (`SparseSearch`).
- `FlatDenseIndex` — exact cosine ANN via flat exhaustive scan (`DenseSearch`).
- `RrfHybridIndex`, `RsfHybridIndex`, `ScoreFusionIndex` — fusion indexes combining sparse and dense signals (`HybridSearch`).
- Traits: `SparseSearch`, `DenseSearch`, `HybridSearch`.
- `recall_at_k(returned, ground_truth)` — recall metric against a ground-truth set.

## License

MIT OR Apache-2.0
