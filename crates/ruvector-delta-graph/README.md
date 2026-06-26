# ruvector-delta-graph

Delta operations for graph structures — incremental edge and node changes.

## Overview

`ruvector-delta-graph` extends the delta model to property graphs, representing incremental changes to nodes, edges, and their properties (including vector-valued properties). Built on `ruvector-delta-core`, it lets graph state evolve through composable, invertible deltas and supports delta streaming for event sourcing. It fits into the RuVector delta-CRDT stack in the meta-ruvector workspace as the graph-aware change-tracking layer.

## Key API

- `GraphDelta` — a batch of node/edge additions, removals, and updates; supports `compose`, `inverse`, `is_empty`, `operation_count`, `affected_nodes`, and `affected_edges`, plus mutators (`add_node`, `remove_node`, `update_node`, `add_edge`, `remove_edge`, `update_edge`).
- `GraphDeltaBuilder` — fluent builder for constructing a `GraphDelta`.
- `GraphState` — an in-memory graph (nodes and edges) that applies a `GraphDelta` via `apply_delta`, with `node_count` / `edge_count`.
- `GraphDeltaStream` — a stream of graph deltas for event sourcing, wrapping `DeltaStream`.
- `EdgeDelta` / `EdgeOp` — edge-level change operations.
- `NodeDelta` / `PropertyDelta` — node property change operations.
- `PropertyValue` — typed property values (null, bool, int, float, string, vector, list, map).
- `PropertyOp` — `Set`, `Remove`, or `VectorDelta` applied to a property.
- `EdgeAddition` — full specification of an edge being added (id, source, target, type, properties).
- `DeltaAwareTraversal` / `TraversalMode` — delta-aware graph traversal.
- `NodeId` / `EdgeId` — string id aliases; `GraphDeltaError` / `Result` — error type and result alias.

## Features

- `parallel` — enables parallel processing via `rayon`.
- `serde` — derives `serde` serialization for graph delta types (also pulls in `serde_json`).

## License

Licensed under MIT OR Apache-2.0.
