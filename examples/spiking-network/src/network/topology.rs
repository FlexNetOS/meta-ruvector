//! Network topologies.
//!
//! Defines connection patterns for ASIC-friendly network layouts.

use serde::{Deserialize, Serialize};

/// Pattern used to generate network connectivity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConnectionPattern {
    /// Connect every neuron to every other with given probability.
    AllToAll { probability: f32 },
    /// Grid-based local connectivity with wraparound (toroidal).
    LocalGrid { width: usize, radius: usize },
    /// Watts-Strogatz small-world network.
    SmallWorld { k: usize, rewire_prob: f32 },
    /// Layered feedforward with specified layer sizes.
    Feedforward { layer_sizes: Vec<usize> },
    /// Connections added manually by the caller.
    Custom,
}

/// Configuration for network topology generation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopologyConfig {
    /// The connection pattern to use.
    pub pattern: ConnectionPattern,
    /// Enable random weight initialization (ignored when false).
    pub random: bool,
}
impl Default for TopologyConfig {
    fn default() -> Self {
        Self {
            pattern: ConnectionPattern::AllToAll { probability: 0.1 },
            random: true,
        }
    }
}

/// Local grid connectivity parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalConnectivity {
    /// Width of the grid (neurons are arranged in a square).
    pub grid_width: usize,
    /// Radius of connectivity within the grid.
    pub radius: usize,
    /// Whether grid wraps around edges (toroidal topology).
    pub toroidal: bool,
}
