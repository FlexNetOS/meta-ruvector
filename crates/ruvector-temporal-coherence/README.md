# ruvector-temporal-coherence

Temporal coherence decay for agent memory retrieval.

Provides three scored retrieval variants, each implementing the `VectorSearch`
trait, that combine cosine similarity with temporal decay and a graph-coherence
gate:

| Variant | Score |
|---------|-------|
| `FlatSearch` | pure cosine similarity, no temporal awareness |
| `TemporalSearch` | cosine × time decay |
| `CoherenceSearch` | cosine × decay × graph-coherence gate |

The coherence gate uses a lightweight adjacency graph where memory vectors that
are mutually similar (above a coherence threshold) form edges. A memory's gate
value is its normalised in-degree — highly connected memories score higher
because the graph has "voted" for their relevance.

## Configuration knobs

- **Decay** (`DecayConfig` / `DecayKind`): `None`, `Linear { half_life }`, or
  `Exponential { lambda }`. Constructors `DecayConfig::none(now)`,
  `::linear(now, half_life)`, `::exponential(now, half_life)`; `factor(memory_ts)`
  returns a multiplier in `[0, 1]`.
- **Coherence weight** (`CoherenceSearch`): blends decay and the graph gate as
  `score = sim * ((1-w)*decay + w*gate)`, where `w` is clamped to `[0, 1]`.

## Usage

```rust
use ruvector_temporal_coherence::{
    CoherenceGraph, CoherenceSearch, DecayConfig, MemoryStore, VectorSearch,
};

let mut store = MemoryStore::new(/* dims */ 32);
// store.insert(vec, MemoryMetadata { timestamp, source, tags });

let decay = DecayConfig::exponential(/* now */ 1_000_000, /* half_life */ 100_000);
let graph = CoherenceGraph::build(&store, /* threshold */ 0.8);
let searcher = CoherenceSearch::new(decay, graph, /* coherence_weight */ 0.3);

let hits = searcher.search(&query, /* k */ 10, &store);
// hits: Vec<SearchResult> of { id, score }
```

## Public API

- Decay: `DecayConfig`, `DecayKind`
- Graph: `CoherenceGraph`
- Search: `VectorSearch` (trait), `FlatSearch`, `TemporalSearch`,
  `CoherenceSearch`, `SearchResult`
- Store: `MemoryStore`, `MemoryRecord`, `MemoryMetadata`, `MemoryId`
- Helpers: `generate_memory_corpus`, `ground_truth_topk`, `recall_at_k`,
  `cosine_sim`, `estimate_memory_bytes`

`tcd-demo` and `tcd-benchmark` binaries are included.

## License

MIT OR Apache-2.0
