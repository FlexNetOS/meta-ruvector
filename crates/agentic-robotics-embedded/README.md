# agentic-robotics-embedded

[![Crates.io](https://img.shields.io/crates/v/agentic-robotics-embedded.svg)](https://crates.io/crates/agentic-robotics-embedded)
[![Documentation](https://docs.rs/agentic-robotics-embedded/badge.svg)](https://docs.rs/agentic-robotics-embedded)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](../../LICENSE)

**Placeholder crate for future embedded support in Agentic Robotics**

Part of the [Agentic Robotics](https://github.com/ruvnet/vibecast) framework.

> ⚠️ **Status: placeholder / skeleton.** This crate does **not** yet provide
> embedded runtime support. Today it is a small `std` crate that exports two
> configuration types sketching the intended API. The embedded dependencies
> (Embassy, RTIC) are commented out in `Cargo.toml`, and the `embassy` / `rtic`
> Cargo features exist but are empty (they enable no code). There is currently
> no no-std build, no real-time executor integration, and no target HAL.

## What it provides today

The entire public API is two types:

```rust
use agentic_robotics_embedded::{EmbeddedPriority, EmbeddedConfig};

// Task priority enum: Low = 0, Normal = 1, High = 2, Critical = 3.
let priority = EmbeddedPriority::High;

// Configuration struct with defaults (tick_rate_hz = 1000, stack_size = 4096).
let config = EmbeddedConfig::default();
assert_eq!(config.tick_rate_hz, 1000);
assert_eq!(config.stack_size, 4096);

// Or construct explicitly.
let custom = EmbeddedConfig { tick_rate_hz: 500, stack_size: 8192 };
let _ = (priority, config, custom);
```

| Type | Kind | Fields / variants |
|------|------|-------------------|
| `EmbeddedPriority` | enum (`Copy`) | `Low`, `Normal`, `High`, `Critical` |
| `EmbeddedConfig` | struct (`Clone`, `Default`) | `tick_rate_hz: u32`, `stack_size: usize` |

## Installation

```toml
[dependencies]
agentic-robotics-embedded = "0.1"
```

This crate currently depends on `agentic-robotics-core` plus `serde`, `anyhow`,
and `thiserror`. It builds as a normal `std` crate.

## Planned / WIP

The following are intended directions but are **not** implemented yet:

- **no-std support** for bare-metal targets.
- **Embassy** async executor integration (the `embassy` feature is a stub).
- **RTIC** real-time concurrency integration (the `rtic` feature is a stub).
- **Target support** for boards such as STM32, ESP32, nRF, and RP2040.
- A minimal, allocation-conscious footprint suitable for microcontrollers.

The optional Embassy/RTIC dependencies are present (commented out) in
`Cargo.toml` to mark this direction; they are not yet wired in.

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](../../LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT License ([LICENSE-MIT](../../LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Links

- **Homepage**: [ruv.io](https://ruv.io)
- **Documentation**: [docs.rs/agentic-robotics-embedded](https://docs.rs/agentic-robotics-embedded)
- **Repository**: [github.com/ruvnet/vibecast](https://github.com/ruvnet/vibecast)

---

**Part of the Agentic Robotics framework**
