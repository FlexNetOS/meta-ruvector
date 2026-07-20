//! Edge-detection example: encode an image patch into sparse spikes and run the network.
use spiking_network::encoding::SparseSpikes;
use spiking_network::{SpikeEncoder, SpikingNetwork};

fn main() {
    // A small 8x8 intensity patch containing a vertical edge (dark left, bright right).
    let width = 8;
    let height = 8;
    let patch: Vec<f32> = (0..width * height)
        .map(|i| if (i % width) < width / 2 { 0.1 } else { 0.9 })
        .collect();

    // Encode the patch into a sparse spike representation (one neuron per pixel).
    let spikes: SparseSpikes = SpikeEncoder::encode_image_patch(&patch, width, height);

    // Build a network with one neuron per pixel and inject the encoded spikes.
    let mut net = SpikingNetwork::with_neurons(width * height)
        .expect("network construction should succeed for a non-empty patch");
    net.inject_spikes(&spikes);

    // Run for 100 ms of simulated time and report activity.
    let stats = net.run(100.0);
    println!(
        "Edge detection complete: {} input spikes, {} output spikes ({:.4} sparsity)",
        spikes.spike_count(),
        net.output_spikes().len(),
        stats.sparsity
    );
}
