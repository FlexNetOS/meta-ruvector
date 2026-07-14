# photonlayer-bench

PhotonLayer reproducible benchmarks plus the in-Rust mask learner and digital decoder.

## Overview

`photonlayer-bench` turns the PhotonLayer optical core (`photonlayer-core`) into an
end-to-end, trainable hybrid system (ADR-260 Phase 2 & 4). It provides reproducible
benchmark harnesses and the mask-learning / decoding logic as a library, so the
`photonlayer-cli` studio and example apps reuse the learner without duplicating it.
Benchmarks compare three variants — a digital baseline, a random optical mask, and a
learned optical mask — to demonstrate that a learned optical frontend preserves
task-useful information while shrinking the sensor and decoder versus a direct pixel
pipeline. It is part of the PhotonLayer stack within the meta-ruvector workspace.

## Key API

- `learn_mask`, `LearnConfig`, `LearnOutcome` — learn an optical phase mask plus its decoder.
- `run_classification`, `run_compression`, `BenchReport`, `VariantResult` — benchmark drivers and result types.
- `frame_features`, `NearestCentroid` — detector-frame feature extraction and the digital decoder.
- `run_mnist_differential`, `MnistBenchConfig`, `MnistBenchResult` — MNIST differential benchmark.
- `load_train`, `load_test`, `subset`, `RawMnist`, `MnistError`, `MNIST_CLASSES` — MNIST data loading.
- `make_dataset`, `Sample`, `class_names`, `NUM_CLASSES` — synthetic dataset generation.
- `verify_eer`, `VerificationReport` — 1:1 verification equal-error-rate evaluation.
- `privacy_leakage`, `PrivacyReport` — reconstruction-attack privacy scoring.
- `DiffDetector`, `Region` — differential detection helpers.

The crate also ships a `photonlayer-bench` binary (`src/bin/bench.rs`):

```bash
cargo run -p photonlayer-bench
```

## License

MIT
