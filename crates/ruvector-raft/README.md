# Ruvector Raft

[![Crates.io](https://img.shields.io/crates/v/ruvector-raft.svg)](https://crates.io/crates/ruvector-raft)
[![Documentation](https://docs.rs/ruvector-raft/badge.svg)](https://docs.rs/ruvector-raft)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-1.77%2B-orange.svg)](https://www.rust-lang.org)

**Raft consensus implementation for Ruvector distributed metadata coordination.**

`ruvector-raft` provides a Raft consensus core for coordinating distributed
Ruvector deployments — leader election, log replication, and the Raft RPC message
types for cluster metadata. Part of the
[Ruvector](https://github.com/ruvnet/ruvector) ecosystem.

## Why Ruvector Raft?

- **Leader Election**: randomized election timeouts and majority-vote leadership
- **Log Replication**: AppendEntries-based replication with conflict backtracking
- **Commit Management**: majority-based commit index tracking
- **Raft RPC Types**: `AppendEntries`, `RequestVote`, and `InstallSnapshot` messages

## Implemented vs. Planned

This crate implements the in-process Raft state machine, election logic, log, and
RPC message types. **Network transport is not included** — the node computes RPC
responses internally but does not send them over the wire (the `start` loop and
handlers contain `TODO: Send …` points where a transport must be plugged in). See
[Planned / WIP](#planned--wip).

## Installation

Add `ruvector-raft` to your `Cargo.toml`:

```toml
[dependencies]
ruvector-raft = "0.1.1"
```

## Quick Start

### Create and run a Raft node

`RaftNode::new` is synchronous and takes a `RaftNodeConfig`. The node is driven by
`start`, which takes `Arc<Self>` and runs the message loop.

```rust
use std::sync::Arc;
use ruvector_raft::{RaftNode, RaftNodeConfig};

#[tokio::main]
async fn main() {
    // Configure the node: this node's ID + all cluster member IDs (including self).
    // NodeId is a String.
    let config = RaftNodeConfig::new(
        "node1".to_string(),
        vec!["node1".to_string(), "node2".to_string(), "node3".to_string()],
    );
    // RaftNodeConfig also exposes tunable fields:
    //   election_timeout_min / election_timeout_max (ms), heartbeat_interval (ms),
    //   max_entries_per_message, snapshot_chunk_size.

    let node = Arc::new(RaftNode::new(config));

    // Inspect state before starting.
    println!("state = {:?}", node.current_state()); // RaftState::Follower
    println!("term  = {}", node.current_term());      // 0

    // Drive the node (runs the internal message loop; does not return).
    // node.clone().start().await;
}
```

### Submit a command

Commands are opaque byte payloads. Only the leader accepts them; submitting on a
follower returns `RaftError::NotLeader`.

```rust
use std::sync::Arc;
use ruvector_raft::{RaftNode, RaftError};

# async fn submit(node: Arc<RaftNode>) {
match node.submit_command(b"set foo=bar".to_vec()).await {
    Ok(result) => {
        // CommandResult { index, term }
        println!("appended at index {} (term {})", result.index, result.term);
    }
    Err(RaftError::NotLeader) => println!("not the leader; redirect to current leader"),
    Err(e) => eprintln!("submit failed: {e}"),
}
# }
```

### Inspect node state

```rust
use std::sync::Arc;
use ruvector_raft::{RaftNode, RaftState};

# fn inspect(node: Arc<RaftNode>) {
let state = node.current_state();      // RaftState: Follower | Candidate | Leader
let term = node.current_term();        // Term (u64)
let leader = node.current_leader();    // Option<NodeId>

if state.is_leader() {
    println!("this node is the leader for term {term}");
}
if let Some(leader_id) = leader {
    println!("current leader: {leader_id}");
}
# }
```

## API Overview

### Re-exported types (crate root)

```rust
// Node and configuration
pub struct RaftNode;          // the consensus node (see src/node.rs)
pub struct RaftNodeConfig {   // node configuration
    pub node_id: NodeId,
    pub cluster_members: Vec<NodeId>,
    pub election_timeout_min: u64,   // milliseconds
    pub election_timeout_max: u64,   // milliseconds
    pub heartbeat_interval: u64,     // milliseconds
    pub max_entries_per_message: usize,
    pub snapshot_chunk_size: usize,
}

// Raft node state (NOTE: there is no Learner variant)
pub enum RaftState {
    Follower,
    Candidate,
    Leader,
}

// State storage (src/state.rs)
pub struct PersistentState;   // current_term, voted_for, log
pub struct VolatileState;     // commit_index, last_applied
pub struct LeaderState;       // next_index / match_index per follower

// RPC message types (src/rpc.rs)
pub struct AppendEntriesRequest;
pub struct AppendEntriesResponse;
pub struct RequestVoteRequest;
pub struct RequestVoteResponse;
pub struct InstallSnapshotRequest;
pub struct InstallSnapshotResponse;

// Errors and aliases
pub enum RaftError { NotLeader, NoLeader, InvalidTerm(u64), /* … */ }
pub type RaftResult<T> = Result<T, RaftError>;
pub type NodeId = String;
pub type Term = u64;
pub type LogIndex = u64;
```

### Node operations (`impl RaftNode`)

```rust
impl RaftNode {
    pub fn new(config: RaftNodeConfig) -> Self;
    pub async fn start(self: std::sync::Arc<Self>);   // runs the message loop

    // Client commands (opaque bytes)
    pub async fn submit_command(&self, data: Vec<u8>) -> RaftResult<CommandResult>;

    // Introspection
    pub fn current_state(&self) -> RaftState;
    pub fn current_term(&self) -> Term;
    pub fn current_leader(&self) -> Option<NodeId>;
}

// Command payload and apply result (src/node.rs)
pub struct Command { pub data: Vec<u8> }
pub struct CommandResult { pub index: LogIndex, pub term: Term }
```

The `Command` / `CommandResult` types use raw `Vec<u8>` payloads — this crate does
**not** define a generic `StateMachine` trait with associated `Command` / `Response`
types. Applying committed commands to your own state machine is left to the caller.

## Architecture

```
┌────────────────────────────────────────────────────────┐
│                     Raft Cluster                        │
│                                                        │
│   ┌──────────┐    ┌──────────┐    ┌──────────┐        │
│   │  Node 1  │    │  Node 2  │    │  Node 3  │        │
│   │ (Leader) │───▶│(Follower)│    │(Follower)│        │
│   │          │    │          │    │          │        │
│   │ Log:     │    │ Log:     │    │ Log:     │        │
│   │ [1,2,3]  │───▶│ [1,2,3]  │    │ [1,2,3]  │        │
│   └──────────┘    └──────────┘    └──────────┘        │
│         │                               ▲              │
│         └───────────────────────────────┘              │
│                  AppendEntries RPC                     │
└────────────────────────────────────────────────────────┘
```

## Planned / WIP

The following are **not** implemented in the current code. They were described in
earlier drafts of this README but no corresponding API exists:

- **Network transport** — `RaftNode` computes RPC responses but does not deliver
  them between nodes. The election, heartbeat, and replication paths contain
  `TODO: Send …` placeholders. You must supply a transport to form a real cluster.
- **Snapshot installation** — `InstallSnapshotRequest` / `InstallSnapshotResponse`
  message types exist, but the handler currently only acknowledges; log
  compaction / snapshot transfer is not yet implemented.
- **Learner (non-voting) nodes** — `RaftState` has only `Follower`, `Candidate`,
  and `Leader`. There is no `Learner` variant.
- **Dynamic membership changes** — there is no `add_node` / `remove_node` /
  `transfer_leadership` API. Cluster membership is fixed at construction via
  `RaftNodeConfig::cluster_members`.
- **Linearizable read index / leadership transfer** — no `read_index`,
  `wait_for_leader`, or `transfer_leadership` methods exist. Use `current_state` /
  `current_leader` for polling instead.
- **Pre-vote protocol** — not implemented.

## Related Crates

- **[ruvector-router-core](../ruvector-router-core/)** - Core vector database engine
- **[ruvector-replication](../ruvector-replication/)** - Data replication

## Documentation

- **[Main README](../../README.md)** - Complete project overview
- **[GitHub Repository](https://github.com/ruvnet/ruvector)** - Source code

## License

**MIT License** - see [LICENSE](../../LICENSE) for details.

---

<div align="center">

**Part of [Ruvector](https://github.com/ruvnet/ruvector) - Built by [rUv](https://ruv.io)**

[![Star on GitHub](https://img.shields.io/github/stars/ruvnet/ruvector?style=social)](https://github.com/ruvnet/ruvector)

[Documentation](https://docs.rs/ruvector-raft) | [Crates.io](https://crates.io/crates/ruvector-raft) | [GitHub](https://github.com/ruvnet/ruvector)

</div>
