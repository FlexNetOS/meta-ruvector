//! Router for ASIC deployment of spiking neural networks.

#![allow(dead_code)]

use serde::{Deserialize, Serialize};

/// Configuration for the ASIC router.
#[derive(Debug, Clone, Serialize, Deserialize)] pub struct RouterConfig {
    /// Maximum number of spike packs to buffer.
    pub max_packs: usize,
    /// Whether to use priority scheduling.
    pub use_priority: bool,
}
impl Default for RouterConfig { fn default() -> Self { Self { max_packs: 1024, use_priority: true } } }

/// A spike packet routed between neuron groups.
#[derive(Debug, Clone)] pub struct SpikePacket {
    /// Source neuron group ID.
    pub source_group: usize,
    /// Target neuron group ID.
    pub target_group: usize,
    /// List of spiking neuron indices within the target group.
    pub neurons: Vec<usize>,
}

/// ASIC-compatible spike router for spiking neural networks.
pub struct AsicRouter { config: RouterConfig }
impl AsicRouter {
    /// Create a new router with the given configuration.
    pub fn new(config: RouterConfig) -> Self { Self { config } }
    /// Route spike packets from source group to target group.
    pub fn route(&self, sg: usize, tg: usize, n: Vec<usize>) -> SpikePacket { SpikePacket { source_group: sg, target_group: tg, neurons: n } } }

#[cfg(test)] mod tests {
    use super::*;
    #[test] fn test_router() { let r = AsicRouter::new(RouterConfig::default()); assert_eq!(r.config.max_packs, 1024); }
}
