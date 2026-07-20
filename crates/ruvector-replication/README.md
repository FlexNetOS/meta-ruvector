# ruvector-replication

[![Crates.io](https://img.shields.io/crates/v/ruvector-replication.svg)](https://crates.io/crates/ruvector-replication)
[![docs.rs](https://docs.rs/ruvector-replication/badge.svg)](https://docs.rs/ruvector-replication)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-1.77%2B-orange.svg)](https://www.rust-lang.org)

**Primary/secondary vector replication with vector clocks, conflict resolution, and automatic failover.**

```toml
ruvector-replication = "0.1.1"
```

When your vector database runs on more than one node, you need a way to keep data
in sync without losing writes. `ruvector-replication` provides the building blocks:
a replica set with role/health tracking (`ReplicaSet`), a synchronization manager
with configurable sync modes (`SyncManager` / `SyncMode`), vector-clock-based
conflict resolution (`VectorClock`), and automatic failover (`FailoverManager`). It
plugs into the [RuVector](https://github.com/ruvnet/ruvector) ecosystem alongside
Raft consensus.

| | Single-node vector DB | ruvector-replication |
|---|---|---|
| **Availability** | One node goes down, everything stops | Secondaries serve reads; primary can be promoted |
| **Topology** | One node | Primary + Secondary + Witness roles |
| **Conflict handling** | N/A | Vector clocks + last-write-wins / merge resolvers |
| **Sync control** | N/A | Per `SyncManager`: `Sync`, `Async`, or `SemiSync` |
| **Recovery** | Manual restore from backup | `FailoverManager` promotes a secondary |

## Quick Start

```rust
use std::sync::Arc;
use ruvector_replication::{ReplicaSet, ReplicaRole, SyncMode, SyncManager, ReplicationLog};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Build a replica set and register replicas with roles.
    let mut replica_set = ReplicaSet::new("cluster-1");
    replica_set.add_replica("replica-1", "192.168.1.10:9001", ReplicaRole::Primary)?;
    replica_set.add_replica("replica-2", "192.168.1.11:9001", ReplicaRole::Secondary)?;

    // Wire up a sync manager over the replica set and a replication log.
    let log = Arc::new(ReplicationLog::new("replica-1"));
    let manager = SyncManager::new(Arc::new(replica_set), log);
    manager.set_sync_mode(SyncMode::SemiSync { min_replicas: 1 });

    Ok(())
}
```

## Key Features

| Feature | What It Does | Where |
|---------|-------------|-------|
| **Replica set management** | Add/remove replicas, track role & health, promote a secondary to primary | `ReplicaSet`, `Replica` |
| **Sync modes** | `Sync` (wait for all), `Async` (fire-and-forget), `SemiSync { min_replicas }` | `SyncMode`, `SyncManager` |
| **Replication log** | Append-and-verify ordered log entries with checksums | `ReplicationLog`, `LogEntry` |
| **Vector-clock conflict resolution** | Track causal ordering; detect concurrent writes | `VectorClock`, `ClockOrdering`, `Versioned<T>` |
| **Pluggable resolvers** | Last-write-wins or a custom merge function | `ConflictResolver`, `LastWriteWins`, `MergeFunction` |
| **Change streams** | Observe replication change events | `ReplicationStream`, `ChangeEvent`, `ChangeOperation` |
| **Automatic failover** | Health monitoring + secondary promotion with split-brain prevention | `FailoverManager`, `FailoverPolicy`, `HealthStatus` |

### Manage the replica set

```rust
use ruvector_replication::{ReplicaSet, ReplicaRole};

# fn run() -> Result<(), Box<dyn std::error::Error>> {
let mut set = ReplicaSet::new("cluster-1");
set.add_replica("r1", "10.0.0.1:9001", ReplicaRole::Primary)?;
set.add_replica("r2", "10.0.0.2:9001", ReplicaRole::Secondary)?;
set.add_replica("r3", "10.0.0.3:9001", ReplicaRole::Witness)?;

// Inspect topology and health.
let _primary = set.get_primary();            // Option<Replica>
let _secondaries = set.get_secondaries();     // Vec<Replica>
let _healthy = set.get_healthy_replicas();    // Vec<Replica>
println!("replicas: {}", set.replica_count());
println!("has quorum: {}", set.has_quorum());

// Promote a secondary if the primary fails.
set.promote_to_primary("r2")?;
# Ok(())
# }
```

### Configure synchronization

```rust
use std::sync::Arc;
use ruvector_replication::{ReplicaSet, ReplicaRole, ReplicationLog, SyncManager, SyncMode};

# fn run() -> Result<(), Box<dyn std::error::Error>> {
let mut set = ReplicaSet::new("cluster-1");
set.add_replica("r1", "10.0.0.1:9001", ReplicaRole::Primary)?;

let log = Arc::new(ReplicationLog::new("r1"));
let manager = SyncManager::new(Arc::new(set), log);

// Choose a sync mode:
manager.set_sync_mode(SyncMode::Sync);                         // wait for all replicas
manager.set_sync_mode(SyncMode::Async);                        // don't wait
manager.set_sync_mode(SyncMode::SemiSync { min_replicas: 1 }); // wait for N

let _mode = manager.sync_mode();
let _pos = manager.current_position();
# Ok(())
# }
```

### Conflict resolution with vector clocks

```rust
use ruvector_replication::{VectorClock, ClockOrdering};

let mut a = VectorClock::new();
let mut b = VectorClock::new();

a.increment("r1");
b.increment("r2");

match a.compare(&b) {
    ClockOrdering::Concurrent => println!("concurrent writes — resolve conflict"),
    ClockOrdering::Before => println!("a happened before b"),
    ClockOrdering::After => println!("a happened after b"),
    ClockOrdering::Equal => println!("equal"),
}
```

### Automatic failover

```rust
use std::sync::Arc;
use parking_lot::RwLock;
use ruvector_replication::{ReplicaSet, ReplicaRole, FailoverManager, FailoverPolicy};

# fn run() -> Result<(), Box<dyn std::error::Error>> {
let mut set = ReplicaSet::new("cluster-1");
set.add_replica("r1", "10.0.0.1:9001", ReplicaRole::Primary)?;
set.add_replica("r2", "10.0.0.2:9001", ReplicaRole::Secondary)?;

let failover = FailoverManager::with_policy(
    Arc::new(RwLock::new(set)),
    FailoverPolicy {
        auto_failover: true,
        failure_threshold: 3,
        prevent_split_brain: true,
        ..Default::default()
    },
);

println!("failover in progress: {}", failover.is_failover_in_progress());
let _history = failover.health_history();
# Ok(())
# }
```

## API Overview

### Re-exported types (crate root)

```rust
// Replica management (src/replica.rs)
pub struct ReplicaSet;
pub struct Replica;             // id, address, role, status, lag_ms, log_position, priority
pub enum ReplicaRole { Primary, Secondary, Witness }
pub enum ReplicaStatus { Healthy, Lagging, Offline, Recovering }

// Synchronization (src/sync.rs)
pub struct SyncManager;
pub struct ReplicationLog;
pub struct LogEntry;
pub enum SyncMode { Sync, Async, SemiSync { min_replicas: usize } }

// Conflict resolution (src/conflict.rs)
pub struct VectorClock;
pub enum ClockOrdering { Equal, Before, After, Concurrent }
pub trait ConflictResolver<T: Clone>;
pub struct LastWriteWins;
pub struct MergeFunction<T, F>;

// Change streaming (src/stream.rs)
pub struct ReplicationStream;
pub struct ChangeEvent;
pub enum ChangeOperation;

// Failover (src/failover.rs)
pub struct FailoverManager;
pub struct FailoverPolicy;      // auto_failover, failure_threshold, min_quorum, prevent_split_brain, …
pub enum HealthStatus;

// Errors
pub enum ReplicationError { ReplicaNotFound(String), NoPrimary, QuorumNotMet { needed, available }, SplitBrain, /* … */ }
pub type Result<T> = std::result::Result<T, ReplicationError>;
```

### Key operations

```rust
// ReplicaSet
pub fn new(cluster_id: impl Into<String>) -> Self;
pub fn add_replica(&mut self, id: impl Into<String>, address: impl Into<String>, role: ReplicaRole) -> Result<()>;
pub fn remove_replica(&mut self, id: &str) -> Result<()>;
pub fn get_primary(&self) -> Option<Replica>;
pub fn get_secondaries(&self) -> Vec<Replica>;
pub fn get_healthy_replicas(&self) -> Vec<Replica>;
pub fn promote_to_primary(&mut self, id: &str) -> Result<()>;
pub fn replica_count(&self) -> usize;
pub fn has_quorum(&self) -> bool;

// SyncManager
pub fn new(replica_set: Arc<ReplicaSet>, log: Arc<ReplicationLog>) -> Self;
pub fn set_sync_mode(&self, mode: SyncMode);
pub fn sync_mode(&self) -> SyncMode;
pub fn current_position(&self) -> u64;
pub fn verify_entry(&self, sequence: u64) -> Result<bool>;

// FailoverManager
pub fn new(replica_set: Arc<RwLock<ReplicaSet>>) -> Self;
pub fn with_policy(replica_set: Arc<RwLock<ReplicaSet>>, policy: FailoverPolicy) -> Self;
pub fn is_failover_in_progress(&self) -> bool;
pub fn health_history(&self) -> Vec<HealthCheck>;
```

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│                   Replication Flow                       │
│                                                         │
│  Client                                                 │
│    │                                                    │
│    ▼                                                    │
│  ┌──────────┐     Quorum Write    ┌──────────┐         │
│  │ Primary  │────────────────────▶│ Replica 1│         │
│  │          │                     │          │         │
│  │ Vectors  │────────────────────▶│ Vectors  │         │
│  └──────────┘                     └──────────┘         │
│       │                                                 │
│       │        Async Replication                        │
│       └──────────────────────────▶┌──────────┐         │
│                                   │ Replica 2│         │
│                                   │          │         │
│                                   │ Vectors  │         │
│                                   └──────────┘         │
└─────────────────────────────────────────────────────────┘
```

## Planned / WIP

Earlier drafts of this README documented a high-level `Replicator` façade that does
not exist in the code. The following types/methods are **not** present — use the
building blocks above instead:

- **`Replicator` / `ReplicationConfig`** — there is no top-level `Replicator` type
  nor a `ReplicationConfig`. Compose `ReplicaSet` + `SyncManager` (+
  `FailoverManager`) directly.
- **`ConsistencyLevel` (One / Quorum / All) and `WriteOptions`** — consistency is
  expressed through `SyncMode` (`Sync` / `Async` / `SemiSync { min_replicas }`) on
  the `SyncManager`, not per-write `WriteOptions`.
- **`ReplicationEvent` / `ReplicaInfo`** — change streaming is via
  `ReplicationStream` + `ChangeEvent` / `ChangeOperation`; replica metadata is the
  `Replica` struct.
- **`replicator.write()` / `.lag()` / `.replicas()` / `.force_sync()`** — these
  methods do not exist. Append through `ReplicationLog`, inspect replicas via
  `ReplicaSet`, and drive recovery via `FailoverManager`.

## Related Crates

- **[ruvector-router-core](../ruvector-router-core/)** - Core vector database engine
- **[ruvector-raft](../ruvector-raft/)** - Raft consensus

## Documentation

- **[Main README](../../README.md)** - Complete project overview
- **[GitHub Repository](https://github.com/ruvnet/ruvector)** - Source code

## License

**MIT License** - see [LICENSE](../../LICENSE) for details.

---

<div align="center">

**Part of [Ruvector](https://github.com/ruvnet/ruvector) - Built by [rUv](https://ruv.io)**

[![Star on GitHub](https://img.shields.io/github/stars/ruvnet/ruvector?style=social)](https://github.com/ruvnet/ruvector)

[Documentation](https://docs.rs/ruvector-replication) | [Crates.io](https://crates.io/crates/ruvector-replication) | [GitHub](https://github.com/ruvnet/ruvector)

</div>
