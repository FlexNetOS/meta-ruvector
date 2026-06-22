# Ruvector Cluster

[![Crates.io](https://img.shields.io/crates/v/ruvector-cluster.svg)](https://crates.io/crates/ruvector-cluster)
[![Documentation](https://docs.rs/ruvector-cluster/badge.svg)](https://docs.rs/ruvector-cluster)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-1.77%2B-orange.svg)](https://www.rust-lang.org)

**Distributed clustering and sharding for Ruvector vector databases.**

`ruvector-cluster` provides horizontal scaling capabilities with consistent hashing, shard management, and cluster coordination. Enables Ruvector to scale to billions of vectors across multiple nodes. Part of the [Ruvector](https://github.com/FlexNetOS/ruvector) ecosystem.

## Why Ruvector Cluster?

- **Horizontal Scaling**: Distribute data across multiple nodes
- **Consistent Hashing**: Minimal rebalancing on cluster changes
- **Auto-Sharding**: Automatic shard distribution and balancing
- **Fault Tolerant**: Handle node failures gracefully
- **Async-First**: Built on Tokio for high-performance networking

## Features

### Core Capabilities

- **Cluster Membership**: Node discovery (`StaticDiscovery`, `GossipDiscovery`) and health monitoring
- **Consistent Hashing**: Virtual-node consistent hashing (`ConsistentHashRing`, 150 virtual nodes per real node) for shard placement
- **Shard Management**: Assign and rebalance shards (`ShardInfo`, `ShardRouter`)
- **Node Coordination**: DAG-based consensus (`DagConsensus`)
- **Failure Detection**: Heartbeat-based health checks (`run_health_checks`)
- **Dynamic Rebalancing**: Auto-rebalance on node add/remove

### Planned / Not Yet Implemented

These are roadmap items and are **not** present in the current code:

- **Rack Awareness**: Place replicas across failure domains
- **Hot Spot Detection**: Identify and redistribute hot shards
- **Gradual / Zero-downtime Migration**: `ShardStatus::Migrating` exists, but online migration is not yet wired up
- **Cluster Metrics**: Prometheus-compatible metrics (see the `ruvector-metrics` crate for metrics today)

## Installation

Add `ruvector-cluster` to your `Cargo.toml`:

```toml
[dependencies]
ruvector-cluster = "0.1.1"
```

## Quick Start

### Initialize a Cluster Manager

```rust
use ruvector_cluster::{ClusterManager, ClusterConfig, ClusterNode, StaticDiscovery};
use std::net::SocketAddr;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Configure the cluster
    let config = ClusterConfig {
        replication_factor: 3,
        shard_count: 64,
        ..Default::default()
    };

    // A discovery service supplies the initial node set.
    // Implementations: StaticDiscovery, GossipDiscovery.
    let discovery = Box::new(StaticDiscovery::new(vec![]));

    // Create the manager (config, this node's id, discovery)
    let manager = ClusterManager::new(config, "node-1".to_string(), discovery)?;

    // Start: discover peers, then assign shards
    manager.start().await?;

    // Add a node manually
    let node = ClusterNode::new(
        "node-2".to_string(),
        "127.0.0.1:7000".parse::<SocketAddr>()?,
    );
    manager.add_node(node).await?;

    println!("Cluster has {} nodes", manager.list_nodes().len());

    Ok(())
}
```

### Shard Operations

```rust
// Assign a shard to nodes via consistent hashing
let shard = manager.assign_shard(0)?;
println!(
    "Shard {} -> primary {}, replicas {:?}",
    shard.shard_id, shard.primary_node, shard.replica_nodes
);

// Look up an existing shard
if let Some(info) = manager.get_shard(0) {
    println!("Shard 0 status: {:?}", info.status);
}

// List all shards
for s in manager.list_shards() {
    println!("shard {} on {}", s.shard_id, s.primary_node);
}
```

### Cluster Health

```rust
// Run a health-check pass (marks unresponsive nodes Offline)
manager.run_health_checks().await?;

// Healthy nodes only
let healthy = manager.healthy_nodes();
println!("{} healthy nodes", healthy.len());

// Aggregate statistics
let stats = manager.get_stats();
println!(
    "{}/{} nodes healthy, {} shards, {} vectors",
    stats.healthy_nodes, stats.total_nodes, stats.total_shards, stats.total_vectors
);

// Inspect individual nodes
for node in manager.list_nodes() {
    println!("{}: {:?} (last seen: {})", node.node_id, node.status, node.last_seen);
}
```

## API Overview

### Core Types

```rust
// Cluster configuration (Default available)
pub struct ClusterConfig {
    pub replication_factor: usize,
    pub shard_count: u32,
    pub heartbeat_interval: Duration,
    pub node_timeout: Duration,
    pub enable_consensus: bool,
    pub min_quorum_size: usize,
}

// Node information
pub struct ClusterNode {
    pub node_id: String,
    pub address: SocketAddr,
    pub status: NodeStatus,        // Leader | Follower | Candidate | Offline
    pub last_seen: DateTime<Utc>,
    pub metadata: HashMap<String, String>,
    pub capacity: f64,
}

// Shard information
pub struct ShardInfo {
    pub shard_id: u32,
    pub primary_node: String,
    pub replica_nodes: Vec<String>,
    pub vector_count: usize,
    pub status: ShardStatus,       // Active | Migrating | Replicating | Offline
    pub created_at: DateTime<Utc>,
    pub modified_at: DateTime<Utc>,
}
```

### ClusterManager Operations

```rust
impl ClusterManager {
    pub fn new(
        config: ClusterConfig,
        node_id: String,
        discovery: Box<dyn DiscoveryService>,
    ) -> Result<Self>;

    pub async fn start(&self) -> Result<()>;

    // Membership
    pub async fn add_node(&self, node: ClusterNode) -> Result<()>;
    pub async fn remove_node(&self, node_id: &str) -> Result<()>;
    pub fn get_node(&self, node_id: &str) -> Option<ClusterNode>;
    pub fn list_nodes(&self) -> Vec<ClusterNode>;
    pub fn healthy_nodes(&self) -> Vec<ClusterNode>;

    // Sharding
    pub fn assign_shard(&self, shard_id: u32) -> Result<ShardInfo>;
    pub fn get_shard(&self, shard_id: u32) -> Option<ShardInfo>;
    pub fn list_shards(&self) -> Vec<ShardInfo>;
    pub fn router(&self) -> Arc<ShardRouter>;

    // Health & consensus
    pub async fn run_health_checks(&self) -> Result<()>;
    pub fn get_stats(&self) -> ClusterStats;
    pub fn consensus(&self) -> Option<Arc<DagConsensus>>;
}
```

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                        Cluster                               │
│  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────┐        │
│  │ Node 1  │  │ Node 2  │  │ Node 3  │  │ Node 4  │        │
│  │ Shards: │  │ Shards: │  │ Shards: │  │ Shards: │        │
│  │ 0,4,8   │  │ 1,5,9   │  │ 2,6,10  │  │ 3,7,11  │        │
│  └────┬────┘  └────┬────┘  └────┬────┘  └────┬────┘        │
│       │            │            │            │              │
│       └────────────┴────────────┴────────────┘              │
│                    Gossip Protocol                          │
└─────────────────────────────────────────────────────────────┘
```

## Related Crates

- **[ruvector-core](../ruvector-core/)** - Core vector database engine
- **[ruvector-raft](../ruvector-raft/)** - RAFT consensus
- **[ruvector-replication](../ruvector-replication/)** - Data replication

## Documentation

- **[Main README](../../README.md)** - Complete project overview
- **[API Documentation](https://docs.rs/ruvector-cluster)** - Full API reference
- **[GitHub Repository](https://github.com/FlexNetOS/ruvector)** - Source code

## License

**MIT License** - see [LICENSE](../../LICENSE) for details.

---

<div align="center">

**Part of [Ruvector](https://github.com/FlexNetOS/ruvector) - Built by [rUv](https://ruv.io)**

[![Star on GitHub](https://img.shields.io/github/stars/FlexNetOS/ruvector?style=social)](https://github.com/FlexNetOS/ruvector)

[Documentation](https://docs.rs/ruvector-cluster) | [Crates.io](https://crates.io/crates/ruvector-cluster) | [GitHub](https://github.com/FlexNetOS/ruvector)

</div>
