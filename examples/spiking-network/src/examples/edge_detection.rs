use spiking_network::{encoding::SpikeEncoder, SpikingNetwork};
fn main() {
    let mut network = SpikingNetwork::with_neurons(100).unwrap();
    // Sinusoidal luminance values — normalise to [0, 1] for spike encoding
    let inputs: Vec<f32> = (0..50)
        .map(|i| (f32::sin(i as f32 * 0.2) + 1.0) / 2.0)
        .collect();
    // Encode the patch as a sparse spike train (50-neuron × 1-timestep input)
    let spikes = SpikeEncoder::encode_image_patch(&inputs, 50, 1);
    network.inject_spikes(&spikes);
    let n_spikes = network.step();
    println!("Edge detection complete: {n_spikes} network spikes");
}
