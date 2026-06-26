//! Edge-detection demo.
//!
//! Encodes a small image patch containing a vertical edge into a sparse spike
//! train, feeds it into an event-driven spiking network, and reports the
//! resulting activity. Demonstrates [`SpikeEncoder::encode_image_patch`] and the
//! sparse, event-driven `SpikingNetwork` pipeline.

use spiking_network::{
    encoding::SpikeEncoder,
    network::{ConnectionPattern, NetworkConfig, SpikingNetwork, Synapse, TopologyConfig},
};

fn main() {
    // 4x4 patch with a vertical edge: dark left, bright right.
    let (width, height) = (4, 4);
    #[rustfmt::skip]
    let patch = vec![
        0.0, 0.0, 1.0, 1.0,
        0.0, 0.0, 1.0, 1.0,
        0.0, 0.0, 1.0, 1.0,
        0.0, 0.0, 1.0, 1.0,
    ];

    let spikes = SpikeEncoder::encode_image_patch(&patch, width, height);
    println!(
        "Encoded {} spikes from a {width}x{height} patch (sparsity {:.2})",
        spikes.spike_count(),
        spikes.sparsity()
    );

    // Build a network whose pixels feed a single "edge" readout neuron.
    let num_pixels = width * height;
    let config = NetworkConfig {
        num_neurons: num_pixels + 1,
        topology: TopologyConfig::new(ConnectionPattern::Custom),
        ..Default::default()
    };
    let mut network = SpikingNetwork::new(config).expect("valid network config");

    let readout = num_pixels;
    for pixel in 0..num_pixels {
        network
            .connect(pixel, readout, Synapse::excitatory(0.6))
            .expect("connect pixel to readout");
    }

    network.inject_spikes(&spikes);
    let stats = network.run(50.0);

    println!(
        "Network fired {} spikes total; readout received input from {} bright pixels",
        stats.total_spikes,
        spikes.spike_count()
    );
    println!(
        "Estimated energy: {:.1} pJ over {:.0} ms",
        stats.energy_consumed, stats.simulation_time
    );
}
