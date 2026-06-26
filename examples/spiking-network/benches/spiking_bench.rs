use criterion::{criterion_group, criterion_main, Criterion};
use spiking_network::encoding::SpikeEncoder;

fn bench(c: &mut Criterion) {
    c.bench_function("encode_image_patch", |b| {
        // 100 normalised pixel values in [0, 1]
        let pixels: Vec<f32> = (0..100).map(|x| x as f32 / 100.0).collect();
        b.iter(|| SpikeEncoder::encode_image_patch(&pixels, 100, 1));
    });

    c.bench_function("rate_encode_single", |b| {
        b.iter(|| SpikeEncoder::rate_encode(0.5, 100.0, 1.0, 100.0));
    });
}
criterion_group!(benches, bench);
criterion_main!(benches);
