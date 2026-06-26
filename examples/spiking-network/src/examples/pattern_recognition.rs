//! Pattern-recognition demo.
//!
//! Uses population coding to represent a scalar stimulus across a bank of
//! neurons, then shows how spike-timing-dependent plasticity ([`STDPLearning`])
//! strengthens a synapse for a causal spike pairing and weakens it for an
//! acausal one.

use spiking_network::{encoding::SpikeEncoder, learning::STDPLearning};

fn main() {
    // Population-code three stimuli across 16 tuning-curve neurons.
    let neurons = 16;
    let sigma = 0.12;
    for stimulus in [0.2_f32, 0.5, 0.8] {
        let activity = SpikeEncoder::population_encode(stimulus, neurons, sigma);
        let peak = activity
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.total_cmp(b))
            .map(|(i, _)| i)
            .unwrap_or(0);
        println!("stimulus {stimulus:.1} -> peak neuron {peak}/{neurons}");
    }

    // Demonstrate STDP weight adaptation.
    let stdp = STDPLearning::with_defaults();
    let mut weight = 0.5_f32;
    println!("\nSTDP adaptation (initial weight {weight:.3}):");

    // Repeated causal pairings (pre at t=0, post at t=4) potentiate the synapse.
    for epoch in 0..5 {
        weight = stdp.update_weight(weight, 0.0, 4.0);
        println!("  epoch {epoch}: causal pairing -> weight {weight:.3}");
    }

    // A burst of acausal pairings (post before pre) then depresses it.
    for epoch in 0..5 {
        weight = stdp.update_weight(weight, 4.0, 0.0);
        println!("  epoch {epoch}: acausal pairing -> weight {weight:.3}");
    }
}
