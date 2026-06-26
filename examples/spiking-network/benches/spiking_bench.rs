use criterion::{black_box, criterion_group, criterion_main, Criterion};
use spiking_network::SpikeEncoder;

fn bench(c: &mut Criterion) {
    c.bench_function("encode", |b| {
        // rate_encode(value, duration_ms, dt, max_rate_hz)
        b.iter(|| SpikeEncoder::rate_encode(black_box(0.7), 100.0, 1.0, 200.0));
    });
}

criterion_group!(benches, bench);
criterion_main!(benches);
