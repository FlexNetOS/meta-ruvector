# ruvector-coherence-hnsw

Coherence-gated beam search on a flat proximity graph.

Standard beam search expands every candidate's neighbors unconditionally. This
crate adds a **traversal-direction coherence gate**: before expanding a
candidate's neighbors, it checks whether the candidate lies roughly *toward* the
query from the search entry point. If not, its neighborhood is skipped while the
candidate is still considered as a result. This prunes off-direction expansion
to speed up beam search while maintaining recall.

The crate operates on a single-layer flat k-NN proximity graph — exactly what
HNSW's layer-0 looks like — keeping the proof-of-concept self-contained. The
gating innovation applies unchanged to multi-layer HNSW.

## Variants

| Type | Threshold | Description |
|------|-----------|-------------|
| `BaselineSearch` | N/A | Standard beam search — all neighbors expanded |
| `CoherenceGatedSearch` | fixed | Skip neighbors when coherence < threshold |
| `AdaptiveCoherenceSearch` | dynamic | Raise threshold as the best result improves |

All implement the `Searcher` trait.

## Usage

```rust
use ruvector_coherence_hnsw::{CoherenceGatedSearch, FlatGraph, GraphConfig, Searcher};

let graph = FlatGraph::build(&vectors, GraphConfig::default());
let searcher = CoherenceGatedSearch::new(/* threshold */ 0.2);
let results = searcher.search(&graph, &query, /* k */ 10);
// results: Vec<SearchResult>
```

## Public API

- Graph: `FlatGraph`, `GraphConfig`
- Searchers: `BaselineSearch`, `CoherenceGatedSearch`, `AdaptiveCoherenceSearch`
  (trait `Searcher`), `SearchResult`

A `benchmark` binary is included.

## License

MIT OR Apache-2.0
