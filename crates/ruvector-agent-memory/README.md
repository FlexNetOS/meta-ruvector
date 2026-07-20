# ruvector-agent-memory

Coherence-weighted agent memory compaction for ruvector.

Agent memories decay in relevance over time. When a memory store grows past a
target capacity, this crate selects which entries to retain. Three compaction
policies are provided behind the `CompactionPolicy` trait:

| Policy | Signal | Novel? |
|--------|--------|--------|
| `LruPolicy` | Recency (`last_accessed_at`) | No — classical |
| `LfuPolicy` | Frequency (`access_count`) | No — classical |
| `CoherencePolicy` | Weighted score: `α·recency + β·frequency + γ·coherence` | **Yes** |

The `CoherencePolicy` is the research contribution: it scores each stored memory
vector against a *context window* (the embeddings of recent agent queries) and
preferentially retains memories semantically aligned with the agent's current
reasoning thread. Weights default to `α=0.25, β=0.35, γ=0.40`
(`CoherenceWeights::default`).

## Usage

```rust
use ruvector_agent_memory::{compact, CoherencePolicy, MemoryStore};

let mut store = MemoryStore::new(/* dims */ 4);
for _ in 0..1000 {
    store.insert(vec![1.0, 0.0, 0.0, 0.0]);
}

// Recent query embeddings drive the coherence component.
let context_window: Vec<Vec<f32>> = vec![vec![1.0, 0.0, 0.0, 0.0]];

// Retain the 256 most important entries in-place.
compact(&mut store, &CoherencePolicy::default(), 256, &context_window);
assert_eq!(store.len(), 256);
```

`compact(store, policy, target_size, context_window)` retains `target_size`
entries in-place (panics if `target_size > store.len()`). `LruPolicy` and
`LfuPolicy` ignore `context_window`; pass an empty slice when context is
unavailable.

## Public API

- Compaction: `CompactionPolicy` (trait), `LruPolicy`, `LfuPolicy`,
  `CoherencePolicy`, `CoherenceWeights`
- Store: `MemoryStore`, `MemoryEntry`, `SearchResult`
- Scoring: `coherence_score`, `cosine_sim`, `normalize`
- Free functions: `compact`, `recall_at_k`

An `agent-memory-bench` binary is included.

## License

MIT OR Apache-2.0
