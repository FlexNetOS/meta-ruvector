//! ASIC spike-routing fabric.
//!
//! On a neuromorphic ASIC, neurons are partitioned across physical cores and
//! spikes travel between cores as tiny packets. This module models that fabric:
//! it maps neurons to cores, buffers packets per core, accounts for inter-core
//! hop cost (an energy proxy), and applies back-pressure when a core's input
//! buffer is full.

use crate::error::{Result, SpikingError};
use serde::{Deserialize, Serialize};

/// A compact spike packet routed across the fabric.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SpikePacket {
    /// Source neuron id.
    pub source: u32,
    /// Destination neuron id.
    pub target: u32,
    /// Emission timestamp (ms).
    pub timestamp: f32,
    /// Small payload (e.g. a graded value or plasticity tag).
    pub payload: u8,
}

impl SpikePacket {
    /// Create a new spike packet with an empty payload.
    pub fn new(source: u32, target: u32, timestamp: f32) -> Self {
        Self {
            source,
            target,
            timestamp,
            payload: 0,
        }
    }

    /// Create a spike packet carrying a payload byte.
    pub fn with_payload(source: u32, target: u32, timestamp: f32, payload: u8) -> Self {
        Self {
            source,
            target,
            timestamp,
            payload,
        }
    }

    /// On-wire size of a packet in bits (source + target + timestamp + payload).
    pub const fn bit_size() -> usize {
        32 + 32 + 32 + 8
    }
}

/// Configuration for the [`AsicRouter`].
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct RouterConfig {
    /// Number of physical cores on the fabric.
    pub num_cores: usize,
    /// Per-core input buffer capacity, in packets.
    pub buffer_size: usize,
    /// Energy cost of a single inter-core hop (pJ).
    pub hop_energy_pj: f32,
}

impl Default for RouterConfig {
    fn default() -> Self {
        Self {
            num_cores: 16,
            buffer_size: 256,
            hop_energy_pj: 2.0,
        }
    }
}

/// Ring-mesh router that maps neurons to cores and accounts for routing cost.
#[derive(Debug, Clone)]
pub struct AsicRouter {
    config: RouterConfig,
    buffers: Vec<Vec<SpikePacket>>,
    total_hops: u64,
    dropped: u64,
}

impl AsicRouter {
    /// Create a new router.
    ///
    /// # Errors
    /// Returns an error if `num_cores` is zero.
    pub fn new(config: RouterConfig) -> Result<Self> {
        if config.num_cores == 0 {
            return Err(SpikingError::RouterError(
                "num_cores must be greater than 0".into(),
            ));
        }
        let buffers = vec![Vec::new(); config.num_cores];
        Ok(Self {
            config,
            buffers,
            total_hops: 0,
            dropped: 0,
        })
    }

    /// Create a router with default configuration.
    pub fn with_defaults() -> Self {
        // num_cores is non-zero in the default, so this cannot fail.
        Self::new(RouterConfig::default()).expect("default config has non-zero cores")
    }

    /// Map a neuron id to its home core (modulo block mapping).
    pub fn core_of(&self, neuron: u32) -> usize {
        (neuron as usize) % self.config.num_cores
    }

    /// Number of hops between two cores on a bidirectional ring.
    fn hops_between(&self, a: usize, b: usize) -> usize {
        let n = self.config.num_cores;
        let d = a.abs_diff(b);
        d.min(n - d)
    }

    /// Route a packet to its destination core's input buffer.
    ///
    /// Returns the number of mesh hops taken.
    ///
    /// # Errors
    /// Returns an error and increments the drop counter if the destination
    /// buffer is full (back-pressure).
    pub fn route(&mut self, packet: SpikePacket) -> Result<usize> {
        let src_core = self.core_of(packet.source);
        let dst_core = self.core_of(packet.target);

        if self.buffers[dst_core].len() >= self.config.buffer_size {
            self.dropped += 1;
            return Err(SpikingError::ResourceExhausted(format!(
                "core {dst_core} input buffer is full"
            )));
        }

        let hops = self.hops_between(src_core, dst_core);
        self.total_hops += hops as u64;
        self.buffers[dst_core].push(packet);
        Ok(hops)
    }

    /// Drain and return all packets buffered at a core.
    pub fn drain_core(&mut self, core: usize) -> Vec<SpikePacket> {
        match self.buffers.get_mut(core) {
            Some(buf) => std::mem::take(buf),
            None => Vec::new(),
        }
    }

    /// Total inter-core hops routed so far.
    pub fn total_hops(&self) -> u64 {
        self.total_hops
    }

    /// Total packets dropped due to back-pressure.
    pub fn dropped(&self) -> u64 {
        self.dropped
    }

    /// Estimated routing energy (pJ) accumulated from hops.
    pub fn routing_energy_pj(&self) -> f32 {
        self.total_hops as f32 * self.config.hop_energy_pj
    }

    /// Borrow the router configuration.
    pub fn config(&self) -> &RouterConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_zero_cores() {
        let cfg = RouterConfig {
            num_cores: 0,
            ..Default::default()
        };
        assert!(AsicRouter::new(cfg).is_err());
    }

    #[test]
    fn maps_neurons_to_cores() {
        let router = AsicRouter::with_defaults();
        assert_eq!(router.core_of(0), 0);
        assert_eq!(router.core_of(16), 0);
        assert_eq!(router.core_of(17), 1);
    }

    #[test]
    fn routes_and_counts_hops() {
        let mut router = AsicRouter::with_defaults();
        // neuron 0 -> core 0, neuron 3 -> core 3, ring distance = 3.
        let hops = router.route(SpikePacket::new(0, 3, 1.0)).unwrap();
        assert_eq!(hops, 3);
        assert_eq!(router.total_hops(), 3);
        assert!(router.routing_energy_pj() > 0.0);
    }

    #[test]
    fn ring_distance_wraps() {
        let mut router = AsicRouter::with_defaults();
        // core 0 -> core 15: direct distance 15, wrap distance 1.
        let hops = router.route(SpikePacket::new(0, 15, 1.0)).unwrap();
        assert_eq!(hops, 1);
    }

    #[test]
    fn back_pressure_drops_when_full() {
        let cfg = RouterConfig {
            num_cores: 2,
            buffer_size: 1,
            hop_energy_pj: 1.0,
        };
        let mut router = AsicRouter::new(cfg).unwrap();
        router.route(SpikePacket::new(0, 1, 0.0)).unwrap();
        assert!(router.route(SpikePacket::new(0, 1, 0.0)).is_err());
        assert_eq!(router.dropped(), 1);
        let drained = router.drain_core(1);
        assert_eq!(drained.len(), 1);
    }
}
