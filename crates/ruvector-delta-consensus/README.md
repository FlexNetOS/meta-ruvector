# ruvector-delta-consensus

Distributed delta consensus using CRDTs and causal ordering.

## Overview

`ruvector-delta-consensus` enables consistent application of vector deltas across distributed replicas. It layers causal ordering (vector clocks), CRDT-based merging, conflict resolution, and gossip-based dissemination on top of `ruvector-delta-core`'s `VectorDelta`. Within the RuVector delta-CRDT stack in the meta-ruvector workspace, this crate is the replication and coordination layer: deltas produced locally are tagged with causal metadata, exchanged between replicas, and applied in a convergent order.

## Key API

- `DeltaConsensus` — the consensus coordinator; `create_delta` tags a local delta with causal metadata, `receive` ingests a remote delta (returning a `DeliveryStatus`), and `apply_with_consensus` applies a delta to a base vector while resolving concurrent conflicts.
- `ConsensusConfig` — replica id, `ConflictStrategy`, pending-delta limit, and causal-delivery toggle.
- `CausalDelta` — a `VectorDelta` plus its `VectorClock`, origin, timestamp, and dependencies; offers `is_before` and `is_concurrent`.
- `DeliveryStatus` — `Delivered`, `Pending`, `AlreadyApplied`, or `Rejected`.
- `DeltaGossip` — gossip protocol over an `Arc<DeltaConsensus>`: `add_peer`/`remove_peer`, `broadcast`, `get_outbox`, `receive_gossip` (returns `GossipResult`), and `get_summary` (returns `GossipSummary`) for anti-entropy.
- Causal primitives: `VectorClock`, `HybridLogicalClock`, `CausalOrder`.
- Conflict resolution: `ConflictResolver`, `ConflictStrategy`, `MergeResult`.
- CRDT types: `DeltaCrdt`, `GCounter`, `PNCounter`, `LWWRegister`, `ORSet`.
- `ConsensusError` / `Result` — crate error type and result alias.

## Features

- `async` — enables optional async support backed by `tokio` (sync and time features).

## License

Licensed under MIT OR Apache-2.0.
