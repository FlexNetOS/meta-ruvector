use criterion::{criterion_group, criterion_main, Criterion};
use spiking_network::{SpikeEncoder, SpikingNetwork};
fn bench(c: &mut Criterion) {
    c.bench_function("encode", |b| {
        let i: Vec<f32> = (0..100).map(|x| x as f32).collect();
        b.iter(|| SpikeEncoder::rate_encode(&i, 0.1));
    });
}
criterion_group!(benches, bench);
criterion_main!(benches);
