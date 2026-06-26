use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use ros3_core::message::{Point3D, PointCloud, Pose, RobotState};
use ros3_core::serialization::{deserialize_cdr, deserialize_json, serialize_cdr, serialize_json};

fn benchmark_cdr_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("CDR Serialization");

    // Small message (RobotState)
    let robot_state = RobotState {
        position: [1.0, 2.0, 3.0],
        velocity: [0.1, 0.2, 0.3],
        timestamp: 123456789,
    };

    group.throughput(Throughput::Bytes(std::mem::size_of::<RobotState>() as u64));
    group.bench_function("RobotState", |b| {
        b.iter(|| {
            let serialized = serialize_cdr(black_box(&robot_state)).unwrap();
            black_box(serialized)
        })
    });

    // Medium message (Pose)
    let pose = Pose {
        position: [1.0, 2.0, 3.0],
        orientation: [0.0, 0.0, 0.0, 1.0],
    };

    group.throughput(Throughput::Bytes(std::mem::size_of::<Pose>() as u64));
    group.bench_function("Pose", |b| {
        b.iter(|| {
            let serialized = serialize_cdr(black_box(&pose)).unwrap();
            black_box(serialized)
        })
    });

    // Large message (PointCloud with 1000 points)
    let points: Vec<Point3D> = (0..1000)
        .map(|i| Point3D {
            x: i as f32 * 0.01,
            y: i as f32 * 0.02,
            z: i as f32 * 0.03,
        })
        .collect();

    let pointcloud = PointCloud {
        points,
        intensities: vec![],
        timestamp: 123456789,
    };

    let size_bytes = pointcloud.points.len() * std::mem::size_of::<Point3D>();
    group.throughput(Throughput::Bytes(size_bytes as u64));
    group.bench_function("PointCloud_1k", |b| {
        b.iter(|| {
            let serialized = serialize_cdr(black_box(&pointcloud)).unwrap();
            black_box(serialized)
        })
    });

    group.finish();
}

fn benchmark_cdr_deserialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("CDR Deserialization");

    // Pre-serialize messages for deserialization benchmarks
    let robot_state = RobotState {
        position: [1.0, 2.0, 3.0],
        velocity: [0.1, 0.2, 0.3],
        timestamp: 123456789,
    };
    let robot_state_bytes = serialize_cdr(&robot_state).unwrap();

    group.throughput(Throughput::Bytes(robot_state_bytes.len() as u64));
    group.bench_function("RobotState", |b| {
        b.iter(|| {
            let deserialized: RobotState = deserialize_cdr(black_box(&robot_state_bytes)).unwrap();
            black_box(deserialized)
        })
    });

    let pose = Pose {
        position: [1.0, 2.0, 3.0],
        orientation: [0.0, 0.0, 0.0, 1.0],
    };
    let pose_bytes = serialize_cdr(&pose).unwrap();

    group.throughput(Throughput::Bytes(pose_bytes.len() as u64));
    group.bench_function("Pose", |b| {
        b.iter(|| {
            let deserialized: Pose = deserialize_cdr(black_box(&pose_bytes)).unwrap();
            black_box(deserialized)
        })
    });

    group.finish();
}

fn benchmark_json_vs_cdr(c: &mut Criterion) {
    let mut group = c.benchmark_group("JSON vs CDR");

    let robot_state = RobotState {
        position: [1.0, 2.0, 3.0],
        velocity: [0.1, 0.2, 0.3],
        timestamp: 123456789,
    };

    group.bench_function("CDR_serialize", |b| {
        b.iter(|| {
            let serialized = serialize_cdr(black_box(&robot_state)).unwrap();
            black_box(serialized)
        })
    });

    group.bench_function("JSON_serialize", |b| {
        b.iter(|| {
            let serialized = serialize_json(black_box(&robot_state)).unwrap();
            black_box(serialized)
        })
    });

    let cdr_bytes = serialize_cdr(&robot_state).unwrap();
    let json_bytes = serialize_json(&robot_state).unwrap();

    group.bench_function("CDR_deserialize", |b| {
        b.iter(|| {
            let deserialized: RobotState = deserialize_cdr(black_box(&cdr_bytes)).unwrap();
            black_box(deserialized)
        })
    });

    group.bench_function("JSON_deserialize", |b| {
        b.iter(|| {
            let deserialized: RobotState =
                deserialize_json(black_box(json_bytes.as_str())).unwrap();
            black_box(deserialized)
        })
    });

    // Report size comparison
    println!("\nSerialization size comparison for RobotState:");
    println!("  CDR:  {} bytes", cdr_bytes.len());
    println!("  JSON: {} bytes", json_bytes.len());
    println!(
        "  Ratio: {:.2}x",
        json_bytes.len() as f64 / cdr_bytes.len() as f64
    );

    group.finish();
}

fn benchmark_message_sizes(c: &mut Criterion) {
    let mut group = c.benchmark_group("Message Size Scaling");

    // Benchmark serialization with different point cloud sizes
    for size in [100, 1000, 10000, 100000].iter() {
        let points: Vec<Point3D> = (0..*size)
            .map(|i| Point3D {
                x: i as f32 * 0.01,
                y: i as f32 * 0.02,
                z: i as f32 * 0.03,
            })
            .collect();

        let pointcloud = PointCloud {
            points,
            intensities: vec![],
            timestamp: 123456789,
        };

        let size_bytes = pointcloud.points.len() * std::mem::size_of::<Point3D>();
        group.throughput(Throughput::Bytes(size_bytes as u64));

        group.bench_with_input(
            BenchmarkId::new("PointCloud", size),
            &pointcloud,
            |b, pc| {
                b.iter(|| {
                    let serialized = serialize_cdr(black_box(pc)).unwrap();
                    black_box(serialized)
                })
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    benchmark_cdr_serialization,
    benchmark_cdr_deserialization,
    benchmark_json_vs_cdr,
    benchmark_message_sizes
);
criterion_main!(benches);
