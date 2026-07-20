//! Pattern-recognition example: build a network, encode a feature, and adapt synapses with STDP.
use spiking_network::{STDPConfig, STDPLearning, SpikeEncoder, SpikingNetwork};

fn main() {
    // A network whose synapses we will adapt with spike-timing-dependent plasticity.
    let mut net = SpikingNetwork::with_neurons(200)
        .expect("network construction should succeed for 200 neurons");

    // Encode one analog feature into a spike train: (value, duration_ms, dt, max_rate_hz).
    let feature = 0.8;
    let train = SpikeEncoder::rate_encode(feature, 100.0, 1.0, 200.0);
    println!(
        "Encoded feature into {} active spike timesteps",
        train.count_ones()
    );

    // Configure STDP and adapt a synapse for causal vs. anti-causal spike pairings.
    let learner = STDPLearning::new(STDPConfig::default());
    let initial_weight = 0.5;
    let potentiated = learner.apply(initial_weight, 5.0); // post fires 5 ms after pre
    let depressed = learner.apply(initial_weight, -5.0); // post fires 5 ms before pre
    println!(
        "STDP: weight {initial_weight:.3} -> {potentiated:.3} (potentiation), {depressed:.3} (depression)"
    );

    // Run the network so the example exercises the event-driven simulation path.
    let stats = net.run(100.0);
    println!(
        "Pattern recognition complete: {} total spikes over {:.0} ms",
        stats.total_spikes, stats.simulation_time
    );
}
