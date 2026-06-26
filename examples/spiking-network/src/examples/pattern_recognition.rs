use spiking_network::{encoding::SpikeEncoder, STDPConfig, STDPLearning, SpikingNetwork};
fn main() {
    let mut network = SpikingNetwork::with_neurons(200).unwrap();
    let config = STDPConfig::default();
    let learning = STDPLearning::new(config);

    // Gaussian pattern across 100 input neurons
    let pattern: Vec<f32> = (0..100)
        .map(|i| f32::exp(-(i as f32 - 50.0).abs() / 20.0))
        .collect();

    // Encode the pattern as a sparse spike train (100-neuron × 1-timestep)
    let spikes = SpikeEncoder::encode_image_patch(&pattern, 100, 1);

    // Inject encoded spikes and run the network for 50 ms
    network.inject_spikes(&spikes);
    let stats = network.run(50.0);

    // Demonstrate STDP: compute the weight update for a pre→post spike pair at Δt = 5 ms
    let updated_weight = learning.apply(0.5, 5.0);

    println!(
        "Pattern recognition complete: {} total spikes, STDP weight update: {:.3}",
        stats.total_spikes, updated_weight
    );
}
