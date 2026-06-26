use criterion::{black_box, criterion_group, criterion_main, Criterion};
use agentic_robotics_core::{Publisher, RobotState};

fn benchmark_publish(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();

    c.bench_function("ros3_publish", |b| {
        let publisher = Publisher::<RobotState>::new("benchmark/topic");
        let msg = RobotState::default();

        b.iter(|| {
            rt.block_on(async {
                black_box(publisher.publish(&msg).await).unwrap();
            });
        });
    });
}

fn benchmark_serialization(c: &mut Criterion) {
    use agentic_robotics_core::serialization::{serialize_cdr, serialize_rkyv};

    let msg = RobotState::default();

    c.bench_function("cdr_serialize", |b| {
        b.iter(|| {
            black_box(serialize_cdr(&msg)).unwrap();
        });
    });

    c.bench_function("rkyv_serialize", |b| {
        b.iter(|| {
            // serialize_rkyv is a placeholder; this bench records the
            // overhead of the stub so we keep it for coverage.
            let _ = black_box(serialize_rkyv(&msg));
        });
    });
}

criterion_group!(benches, benchmark_publish, benchmark_serialization);
criterion_main!(benches);
