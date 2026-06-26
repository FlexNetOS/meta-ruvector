//! Synapse models.
//!
//! Defines excitatory and inhibitory synaptic connection types.

use serde::{Deserialize, Serialize};

/// Type of synaptic connection (excitatory or inhibitory).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SynapseType {
    /// Increases post-synaptic membrane potential.
    Excitatory,
    /// Decreases post-synaptic membrane potential.
    Inhibitory,
}

/// A single synaptic connection between two neurons.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Synapse {
    /// Synaptic weight (strength of connection).
    pub weight: f32,
    /// Conduction delay in milliseconds.
    pub delay: f32,
    /// Whether this synapse is excitatory or inhibitory.
    pub syn_type: SynapseType,
}
impl Synapse {
    /// Create an excitatory synapse with the given weight.
    pub fn excitatory(w: f32) -> Self {
        Self {
            weight: w,
            delay: 1.0,
            syn_type: SynapseType::Excitatory,
        }
    }
    /// Create an inhibitory synapse with the given weight magnitude.
    pub fn inhibitory(w: f32) -> Self {
        Self {
            weight: -w,
            delay: 1.0,
            syn_type: SynapseType::Inhibitory,
        }
    }
    /// Return +1.0 for excitatory, -1.0 for inhibitory.
    pub fn sign(&self) -> f32 {
        match self.syn_type {
            SynapseType::Excitatory => 1.0,
            _ => -1.0,
        }
    }
    /// Returns true if this synapse is inhibitory.
    pub fn is_inhibitory(&self) -> bool {
        self.syn_type == SynapseType::Inhibitory
    }
}

#[cfg(test)]
mod tests {
    use super::Synapse;
    #[test]
    fn test_excitatory() {
        let s = Synapse::excitatory(0.5);
        assert!(!s.is_inhibitory());
    }
}
