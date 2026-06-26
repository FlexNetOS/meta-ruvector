//! Synaptic connection model.
//!
//! Synapses are the routed edges of the network. Each carries a non-negative
//! weight magnitude, a transmission delay, and a sign (excitatory or
//! inhibitory). Keeping the weight as a magnitude plus an explicit sign maps
//! cleanly onto fixed-point ASIC hardware where the sign is a single bit.

use serde::{Deserialize, Serialize};

/// Whether a synapse excites or inhibits its postsynaptic target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SynapseType {
    /// Positive contribution to the postsynaptic membrane potential.
    Excitatory,
    /// Negative contribution to the postsynaptic membrane potential.
    Inhibitory,
}

impl SynapseType {
    /// Sign multiplier for this synapse type (`+1.0` / `-1.0`).
    pub fn sign(self) -> f32 {
        match self {
            SynapseType::Excitatory => 1.0,
            SynapseType::Inhibitory => -1.0,
        }
    }
}

/// A directed synaptic connection between two neurons.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Synapse {
    /// Connection weight, stored as a non-negative magnitude.
    pub weight: f32,
    /// Axonal/synaptic transmission delay in milliseconds.
    pub delay: f32,
    /// Whether the synapse excites or inhibits the postsynaptic neuron.
    pub kind: SynapseType,
}

impl Synapse {
    /// Create a synapse with an explicit type and delay.
    ///
    /// The weight is stored as a magnitude (its absolute value); polarity is
    /// carried by `kind`.
    pub fn new(weight: f32, delay: f32, kind: SynapseType) -> Self {
        Self {
            weight: weight.abs(),
            delay,
            kind,
        }
    }

    /// Create an excitatory synapse with a unit (1 ms) delay.
    pub fn excitatory(weight: f32) -> Self {
        Self::new(weight, 1.0, SynapseType::Excitatory)
    }

    /// Create an inhibitory synapse with a unit (1 ms) delay.
    pub fn inhibitory(weight: f32) -> Self {
        Self::new(weight, 1.0, SynapseType::Inhibitory)
    }

    /// Sign of this synapse's contribution: `+1.0` excitatory, `-1.0` inhibitory.
    pub fn sign(&self) -> f32 {
        self.kind.sign()
    }

    /// Signed effective weight (`weight * sign`).
    pub fn effective_weight(&self) -> f32 {
        self.weight * self.sign()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn excitatory_is_positive() {
        let s = Synapse::excitatory(0.5);
        assert_eq!(s.sign(), 1.0);
        assert_eq!(s.weight, 0.5);
        assert_eq!(s.delay, 1.0);
        assert_eq!(s.effective_weight(), 0.5);
    }

    #[test]
    fn inhibitory_is_negative() {
        let s = Synapse::inhibitory(0.5);
        assert_eq!(s.sign(), -1.0);
        assert_eq!(s.effective_weight(), -0.5);
    }

    #[test]
    fn weight_is_stored_as_magnitude() {
        let s = Synapse::new(-0.75, 2.0, SynapseType::Excitatory);
        assert_eq!(s.weight, 0.75);
        assert_eq!(s.delay, 2.0);
    }
}
