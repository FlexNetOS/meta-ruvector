//! Spike-timing-dependent plasticity (STDP).
//!
//! STDP is the canonical unsupervised learning rule for spiking networks: a
//! synapse is strengthened when the presynaptic spike precedes the
//! postsynaptic one (causal, "pre-before-post") and weakened when the order is
//! reversed. The magnitude of the change decays exponentially with the spike
//! timing difference.
//!
//! This implementation is the classic pair-based additive rule, which is cheap
//! enough for on-chip learning: one exponential and one clamp per update.

use serde::{Deserialize, Serialize};

/// Configuration for the pair-based STDP rule.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct STDPConfig {
    /// Potentiation amplitude (weight increase for a causal pre→post pairing).
    pub a_plus: f32,
    /// Depression amplitude (weight decrease for an acausal post→pre pairing).
    pub a_minus: f32,
    /// Potentiation time constant (ms).
    pub tau_plus: f32,
    /// Depression time constant (ms).
    pub tau_minus: f32,
    /// Minimum allowed weight (clamp floor).
    pub w_min: f32,
    /// Maximum allowed weight (clamp ceiling).
    pub w_max: f32,
}

impl Default for STDPConfig {
    fn default() -> Self {
        // Biologically-plausible defaults: slightly stronger depression than
        // potentiation keeps weights from saturating (Bi & Poo, 1998).
        Self {
            a_plus: 0.010,
            a_minus: 0.012,
            tau_plus: 20.0,
            tau_minus: 20.0,
            w_min: 0.0,
            w_max: 1.0,
        }
    }
}

/// Online pair-based STDP learner.
#[derive(Debug, Clone)]
pub struct STDPLearning {
    config: STDPConfig,
}

impl STDPLearning {
    /// Create a learner with the given configuration.
    pub fn new(config: STDPConfig) -> Self {
        Self { config }
    }

    /// Create a learner with default biologically-plausible parameters.
    pub fn with_defaults() -> Self {
        Self::new(STDPConfig::default())
    }

    /// Borrow the configuration.
    pub fn config(&self) -> &STDPConfig {
        &self.config
    }

    /// Unbounded weight change for a pre/post spike pair.
    ///
    /// `dt = t_post - t_pre` (ms). A positive `dt` (presynaptic spike first)
    /// potentiates; a negative `dt` (postsynaptic spike first) depresses; an
    /// exactly-coincident pair produces no change.
    pub fn weight_delta(&self, dt: f32) -> f32 {
        if dt > 0.0 {
            self.config.a_plus * (-dt / self.config.tau_plus).exp()
        } else if dt < 0.0 {
            -self.config.a_minus * (dt / self.config.tau_minus).exp()
        } else {
            0.0
        }
    }

    /// Apply STDP to a weight given a pre/post spike timing pair.
    ///
    /// Returns the new weight, clamped to `[w_min, w_max]`.
    pub fn update_weight(&self, weight: f32, t_pre: f32, t_post: f32) -> f32 {
        let updated = weight + self.weight_delta(t_post - t_pre);
        updated.clamp(self.config.w_min, self.config.w_max)
    }
}

impl Default for STDPLearning {
    fn default() -> Self {
        Self::with_defaults()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn causal_pairing_potentiates() {
        let stdp = STDPLearning::with_defaults();
        // pre at t=0, post at t=5 -> potentiation.
        let w = stdp.update_weight(0.5, 0.0, 5.0);
        assert!(w > 0.5, "causal pairing should strengthen the synapse");
    }

    #[test]
    fn acausal_pairing_depresses() {
        let stdp = STDPLearning::with_defaults();
        // post at t=0, pre at t=5 -> depression.
        let w = stdp.update_weight(0.5, 5.0, 0.0);
        assert!(w < 0.5, "acausal pairing should weaken the synapse");
    }

    #[test]
    fn delta_decays_with_timing() {
        let stdp = STDPLearning::with_defaults();
        let near = stdp.weight_delta(1.0);
        let far = stdp.weight_delta(40.0);
        assert!(
            near > far,
            "potentiation should decay with timing difference"
        );
        assert!(far > 0.0);
    }

    #[test]
    fn weights_are_clamped() {
        let stdp = STDPLearning::with_defaults();
        assert_eq!(stdp.update_weight(1.0, 0.0, 1.0), 1.0); // clamped at w_max
        assert_eq!(stdp.update_weight(0.0, 1.0, 0.0), 0.0); // clamped at w_min
    }

    #[test]
    fn coincident_pairing_is_neutral() {
        let stdp = STDPLearning::with_defaults();
        assert_eq!(stdp.update_weight(0.5, 3.0, 3.0), 0.5);
    }
}
