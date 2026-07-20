//! ASIC simulation example: construct a LIF neuron from validated parameters.
use spiking_network::neuron::LIFParams;
use spiking_network::{LIFNeuron, NeuronParams, SpikingNeuron};

fn main() {
    // Default cortical-like LIF parameters.
    let params = LIFParams::default();
    assert!(
        params.validate().is_none(),
        "default LIF parameters must be valid"
    );
    assert!(
        params.tau_m > 0.0,
        "membrane time constant must be positive"
    );

    // Build a neuron and report its configuration and resting state.
    let neuron = LIFNeuron::new(params);
    println!(
        "ASIC simulation: OK (threshold {:.1} mV, resting V {:.1} mV, refractory {:.1} ms)",
        params.threshold(),
        neuron.state().membrane_potential,
        params.refractory_period()
    );
}
