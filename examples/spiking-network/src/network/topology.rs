//! Network connectivity topologies.
//!
//! The topology describes how neurons are wired before simulation. Patterns are
//! chosen to be ASIC-friendly: local grids minimize routing, small-world
//! lattices keep path lengths short, and feedforward stacks map onto pipelined
//! layers.

use serde::{Deserialize, Serialize};

/// Connectivity pattern used to wire a [`super::SpikingNetwork`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ConnectionPattern {
    /// Random connectivity; each ordered pair connects with `probability`.
    AllToAll {
        /// Per-pair connection probability in `[0, 1]`.
        probability: f32,
    },
    /// Local 2D-grid connectivity within a Chebyshev `radius`.
    LocalGrid {
        /// Grid width (height is inferred from the neuron count).
        width: usize,
        /// Neighborhood radius (Chebyshev distance).
        radius: usize,
    },
    /// Watts–Strogatz small-world ring lattice with rewiring.
    SmallWorld {
        /// Each node connects to its `k` nearest ring neighbors.
        k: usize,
        /// Probability of rewiring an edge to a random target.
        rewire_prob: f32,
    },
    /// Layered feedforward connectivity.
    Feedforward {
        /// Number of neurons in each successive layer.
        layer_sizes: Vec<usize>,
    },
    /// No automatic wiring; connections are added manually via `connect`.
    Custom,
}

impl Default for ConnectionPattern {
    fn default() -> Self {
        Self::AllToAll { probability: 0.1 }
    }
}

/// Configuration describing how a network should be wired.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct TopologyConfig {
    /// The connectivity pattern to build.
    pub pattern: ConnectionPattern,
}

impl TopologyConfig {
    /// Create a config for the given pattern.
    pub fn new(pattern: ConnectionPattern) -> Self {
        Self { pattern }
    }
}

/// Descriptor for local (neighborhood) connectivity on a 2D grid.
///
/// Useful for sizing on-chip routing fabric: [`Self::max_degree`] bounds the
/// fan-out of any neuron in the grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalConnectivity {
    /// Grid width in neurons.
    pub width: usize,
    /// Grid height in neurons.
    pub height: usize,
    /// Connection radius (Chebyshev distance).
    pub radius: usize,
}

impl LocalConnectivity {
    /// Create a new local-connectivity descriptor.
    pub fn new(width: usize, height: usize, radius: usize) -> Self {
        Self {
            width,
            height,
            radius,
        }
    }

    /// Maximum out-degree: neighbors within the radius, excluding self.
    pub fn max_degree(&self) -> usize {
        let side = 2 * self.radius + 1;
        side * side - 1
    }

    /// Total neuron count of the grid.
    pub fn len(&self) -> usize {
        self.width * self.height
    }

    /// Whether the grid contains no neurons.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_sparse_all_to_all() {
        let cfg = TopologyConfig::default();
        assert_eq!(
            cfg.pattern,
            ConnectionPattern::AllToAll { probability: 0.1 }
        );
    }

    #[test]
    fn local_connectivity_degree() {
        let lc = LocalConnectivity::new(8, 8, 1);
        // 3x3 neighborhood minus self = 8.
        assert_eq!(lc.max_degree(), 8);
        assert_eq!(lc.len(), 64);
        assert!(!lc.is_empty());
    }
}
