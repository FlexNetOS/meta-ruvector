//! Criterion benchmarks for the spiking network.
//!
//! Covers the three hot paths: single-neuron updates, a full event-driven
//! network step, and rate encoding.

use criterion::{criterion_group, criterion_main, Criterion};
use spiking_network::{
    encoding::SpikeEncoder,
    network::{ConnectionPattern, NetworkConfig, SpikingNetwork, TopologyConfig},
    neuron::{LIFNeuron, SpikingNeuron},
};
use std::hint::black_box;

fn bench_lif_update(c: &mut Criterion) {
    c.bench_function("lif_update", |b| {
        let mut neuron = LIFNeuron::with_defaults();
        b.iter(|| {
            neuron.receive_input(black_box(1.5));
            black_box(neuron.update(1.0));
        });
    });
}

fn bench_network_step(c: &mut Criterion) {
    c.bench_function("network_step_1000", |b| {
        let config = NetworkConfig {
            num_neurons: 1000,
            topology: TopologyConfig::new(ConnectionPattern::SmallWorld {
                k: 8,
                rewire_prob: 0.1,
            }),
            ..Default::default()
        };
        let mut network = SpikingNetwork::new(config).expect("valid config");
        network.build_topology().expect("topology builds");
        b.iter(|| {
            black_box(network.step());
        });
    });
}

fn bench_rate_encode(c: &mut Criterion) {
    c.bench_function("rate_encode_100ms", |b| {
        b.iter(|| {
            black_box(SpikeEncoder::rate_encode(black_box(0.5), 100.0, 1.0, 100.0));
        });
    });
}

criterion_group!(
    benches,
    bench_lif_update,
    bench_network_step,
    bench_rate_encode
);
criterion_main!(benches);
