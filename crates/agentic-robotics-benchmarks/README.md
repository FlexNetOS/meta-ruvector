# agentic-robotics-benchmarks

Criterion benchmark suite for the `agentic-robotics-core` and `agentic-robotics-rt` crates.

## Overview

This crate holds the performance benchmarks for the agentic-robotics stack in the
meta-ruvector workspace. It is a non-publishable harness (`publish = false`) that
exercises the public APIs of `agentic-robotics-core` (messages, serialization,
publishers/subscribers) and `agentic-robotics-rt` (the real-time executor and
scheduler). The benchmarks use [Criterion](https://github.com/bheisler/criterion.rs)
with HTML reports enabled, so results are reproducible and trackable over time.

## Benchmarks

Three Criterion targets are defined (each with `harness = false`):

- **`message_serialization`** — CDR and JSON serialization/deserialization of core
  message types (`RobotState`, `Pose`, `PointCloud`), including a JSON-vs-CDR
  comparison and message-size throughput measurements.
- **`pubsub_latency`** — publisher/subscriber creation, publish latency and
  throughput, end-to-end latency, and serializer comparison.
- **`executor_performance`** — `ROS3Executor` creation, task spawning, scheduler
  overhead, task distribution, async task execution, and priority handling.

Run the full suite:

```bash
cargo bench -p agentic-robotics-benchmarks
```

Run a single target:

```bash
cargo bench -p agentic-robotics-benchmarks --bench message_serialization
cargo bench -p agentic-robotics-benchmarks --bench pubsub_latency
cargo bench -p agentic-robotics-benchmarks --bench executor_performance
```

## License

MIT
