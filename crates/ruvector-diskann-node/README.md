# ruvector-diskann-node

NAPI-RS bindings exposing `ruvector-diskann` to Node.js.

## Overview

`ruvector-diskann-node` is a thin native addon that wraps the `ruvector-diskann` Vamana/DiskANN index for use from JavaScript/TypeScript via NAPI-RS. It is built as a `cdylib` and uses an `Arc<RwLock<…>>` around the core index so the addon is safe to share across calls, with both synchronous and async (Tokio `spawn_blocking`) entry points. Within the RuVector stack it is the Node.js delivery surface for the DiskANN index.

## Key API

Exposed through NAPI as a `DiskAnn` class:

- `new(DiskAnnOptions)` — construct an index (`dim`, optional `max_degree`, `build_beam`, `search_beam`, `alpha`, `pq_subspaces`, `pq_iterations`, `storage_path`).
- `insert(id, Float32Array)`, `insert_batch(ids, Float32Array, dim)` — add vectors keyed by string id.
- `build()` / `build_async()` — build the graph after inserts.
- `search(query, k)` / `search_async(query, k)` — return `DiskAnnSearchResult[]` (`{ id, distance }`).
- `delete(id)`, `count()` — mutate and inspect the index.
- `save(dir)`, `load(dir)` — persist and restore an index from disk.

Helper objects `DiskAnnOptions` and `DiskAnnSearchResult` are exported as NAPI object types.

## License

MIT
