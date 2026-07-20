# ruvector-acorn

ACORN: predicate-agnostic filtered approximate-nearest-neighbor search for ruvector.

Implements the ACORN algorithm (Patel et al., *"ACORN: Performant and
Predicate-Agnostic Search Over Vector Embeddings and Structured Data"*, SIGMOD
2024, arXiv:2403.04871). Standard filtered search runs the ANN graph traversal
first and discards results that fail the predicate — at low selectivity the beam
exhausts before finding `k` valid candidates and recall collapses. ACORN fixes
this with two changes to standard HNSW: a **denser graph** (γ·M neighbors per
node instead of M) and **predicate-agnostic traversal** (expand all neighbors
regardless of whether the current node passes the predicate; failing nodes are
skipped in results but their neighborhood is still explored).

## Index variants

| Struct | γ | M | Edge budget | Use when |
|--------|---|---|-------------|----------|
| `FlatFilteredIndex` | N/A | N/A | 0 | Baseline, high selectivity |
| `AcornIndex1` | 1 | 16 | 16/node | Moderate selectivity (≥10%) |
| `AcornIndexGamma` | 2 | 16 | 32/node | Low selectivity (<10%) |

All three implement the `FilteredIndex` trait.

## Usage

```rust
use ruvector_acorn::{AcornIndexGamma, FilteredIndex, recall_at_k};

// Build a γ=2 index over your vectors (Vec<Vec<f32>>).
let data: Vec<Vec<f32>> = /* n vectors of equal dimension */;
let index = AcornIndexGamma::new_with_gamma(data, 2)?;

// Search for the 10 nearest neighbors whose id passes the predicate.
let query: Vec<f32> = /* ... */;
let hits = index.search(&query, 10, &|id: u32| id % 2 == 0)?;
// hits: Vec<(u32, f32)> of (id, distance) in ascending distance
# Ok::<(), ruvector_acorn::AcornError>(())
```

## Public API

- Indexes: `FlatFilteredIndex`, `AcornIndex1`, `AcornIndexGamma` (trait `FilteredIndex`)
- Graph: `AcornGraph`
- Errors: `AcornError`
- Helper: `recall_at_k`

A runnable `acorn-demo` binary and an `acorn_bench` Criterion benchmark are
included.

## License

MIT OR Apache-2.0
