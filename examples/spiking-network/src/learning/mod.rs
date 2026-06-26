//! Learning rules for spiking neural networks.
//!
//! Implements Spike-Timing-Dependent Plasticity (STDP) for synaptic weight updates.

use serde::{Deserialize, Serialize};

/// Configuration for STDP learning rule.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct STDPConfig {
    /// Learning rate for weight updates.
    pub lr: f32,
    /// Pre-synaptic decay time constant (ms).
    pub tau_pre: f32,
    /// Post-synaptic decay time constant (ms).
    pub tau_post: f32,
    /// Maximum synaptic weight.
    pub w_max: f32,
}

impl Default for STDPConfig {
    fn default() -> Self { Self { lr: 0.01, tau_pre: 20.0, tau_post: 20.0, w_max: 1.0 } }
}

/// STDP learning engine that computes and applies synaptic weight changes.
pub struct STDPLearning { config: STDPConfig }

impl STDPLearning {
    /// Create a new STDP learner with the given configuration.
    pub fn new(config: STDPConfig) -> Self { Self { config } }
    /// Compute the weight delta for a pre-post spike pair separated by delta_t ms.
    pub fn compute_delta(&self, delta_t: f32) -> f32 {
        let w_max = self.config.w_max; let tau = self.config.tau_pre.min(self.config.tau_post);
        if delta_t > 0.0 { w_max * delta_t / tau } else { -w_max * (-delta_t) / tau }
    }
    /// Apply the computed delta to a weight, clamping within [0, w_max].
    pub fn apply(&self, weight: f32, delta_t: f32) -> f32 { (weight + self.config.lr * self.compute_delta(delta_t)).max(0.0).min(self.config.w_max) }
}

#[cfg(test)] mod tests {
    use super::*;
    #[test] fn test_config() { assert!(STDPConfig::default().lr > 0.0); }
    #[test] fn test_positive_delta() { let l = STDPLearning::new(STDPConfig::default()); assert!(l.compute_delta(5.0) > 0.0); }
    #[test] fn test_negative_delta() { let l = STDPLearning::new(STDPConfig::default()); assert!(l.compute_delta(-5.0) < 0.0); }
}
