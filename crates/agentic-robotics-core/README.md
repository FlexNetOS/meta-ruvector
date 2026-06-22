# agentic-robotics-core

[![Crates.io](https://img.shields.io/crates/v/agentic-robotics-core.svg)](https://crates.io/crates/agentic-robotics-core)
[![Documentation](https://docs.rs/agentic-robotics-core/badge.svg)](https://docs.rs/agentic-robotics-core)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](../../LICENSE)

**Rust robotics middleware — typed pub/sub messaging, services, and serialization**

Part of the [Agentic Robotics](https://github.com/ruvnet/vibecast) framework. The
crate is internally branded "ROS3" (its message type names use the `ros3_msgs/`
prefix and `init()` logs "ROS3 Core").

---

## 🎯 What is agentic-robotics-core?

`agentic-robotics-core` is a Rust library that provides the building blocks for
robot messaging:

- Typed **publishers** and **subscribers** (`Publisher<T>` / `Subscriber<T>`)
- Request/response **services** (`Service<Req, Res>` / `Queryable<Req, Res>`)
- Pluggable **serialization** (CDR, JSON, with an rkyv path stubbed out)
- A **Zenoh** session wrapper for future middleware integration
- A small set of built-in message types (`RobotState`, `PointCloud`, `Pose`, `Point3D`)

> ⚠️ **Maturity / status.** This is an early-stage crate. The networking layer
> is not wired up yet: `Publisher::publish` serializes the message and updates
> stats but does not yet send it over a live transport, the `Zenoh` type is a
> placeholder, `Subscriber` is an in-process channel, `Service::call` returns an
> error ("not implemented"), and the rkyv serialization path is stubbed. The
> typed API surface below is real and tested; the distributed transport behind
> it is planned (see [Planned / WIP](#-planned--wip)).

---

## 📦 Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
agentic-robotics-core = "0.1"
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
```

Or use `cargo add`:

```bash
cargo add agentic-robotics-core
cargo add tokio --features full
cargo add serde --features derive
```

---

## 🚀 Quick Start

```rust
use agentic_robotics_core::{Publisher, Subscriber, RobotState};

#[tokio::main]
async fn main() -> agentic_robotics_core::Result<()> {
    // Optional: install the tracing subscriber and log the version.
    agentic_robotics_core::init()?;

    // Create a typed publisher for a topic (CDR serialization by default).
    let publisher = Publisher::<RobotState>::new("robot/state");

    // Publish a message (async).
    let state = RobotState::default();
    publisher.publish(&state).await?;

    // Inspect publisher stats: (messages_sent, bytes_sent).
    let (count, bytes) = publisher.stats();
    println!("sent {count} messages, {bytes} bytes");

    Ok(())
}
```

---

## 🧩 Core API

### Publisher

`Publisher<T: Message>` serializes and (eventually) sends messages on a topic.

```rust
use agentic_robotics_core::Publisher;
use agentic_robotics_core::serialization::Format;
use agentic_robotics_core::RobotState;

// Default constructor uses CDR serialization.
let pub_cdr = Publisher::<RobotState>::new("robot/state");

// Or choose a serialization format explicitly.
let pub_json = Publisher::<RobotState>::with_format("robot/state", Format::Json);

// Topic name accessor.
assert_eq!(pub_cdr.topic(), "robot/state");
```

| Method | Signature | Notes |
|--------|-----------|-------|
| `Publisher::<T>::new(topic)` | `(impl Into<String>) -> Publisher<T>` | CDR format |
| `Publisher::<T>::with_format(topic, format)` | `(impl Into<String>, Format) -> Publisher<T>` | explicit format |
| `publish(&msg)` | `async (&T) -> Result<()>` | serializes + updates stats |
| `topic()` | `() -> &str` | topic name |
| `stats()` | `() -> (u64, u64)` | `(messages_sent, bytes_sent)` |

### Subscriber

`Subscriber<T: Message>` receives messages from an in-process channel. It is
`Clone`.

```rust
use agentic_robotics_core::{Subscriber, RobotState};

let sub = Subscriber::<RobotState>::new("robot/state");

// Non-blocking receive: Ok(None) when nothing is available.
if let Some(msg) = sub.try_recv()? {
    println!("got {:?}", msg);
}
```

| Method | Signature | Notes |
|--------|-----------|-------|
| `Subscriber::<T>::new(topic)` | `(impl Into<String>) -> Subscriber<T>` | unbounded channel |
| `recv()` | `(&self) -> Result<T>` | **blocking** receive |
| `recv_async()` | `async (&self) -> Result<T>` | async receive (via `spawn_blocking`) |
| `try_recv()` | `(&self) -> Result<Option<T>>` | non-blocking; `Ok(None)` if empty |
| `topic()` | `() -> &str` | topic name |

> Note: `recv()` returns `Result<T>` and **blocks** the current thread. For
> async contexts use `recv_async().await`. `try_recv()` returns
> `Result<Option<T>>` (`Ok(None)` means no message is currently queued).

### Services (RPC)

`Queryable<Req, Res>` hosts a handler; `Service<Req, Res>` is the client.

```rust
use agentic_robotics_core::{Queryable, Service, RobotState};

#[tokio::main]
async fn main() -> agentic_robotics_core::Result<()> {
    // Server side: register a handler.
    let queryable = Queryable::new("compute", |req: RobotState| {
        Ok(RobotState { timestamp: req.timestamp + 1, ..req })
    });

    let response = queryable.handle(RobotState::default()).await?;
    let (handled, errors) = queryable.stats(); // (requests_handled, errors)

    // Client side: Service::call currently returns an error (see WIP).
    let client = Service::<RobotState, RobotState>::new("compute");
    assert_eq!(client.name(), "compute");
    let _ = response;
    let _ = (handled, errors);
    Ok(())
}
```

| Type | Method | Notes |
|------|--------|-------|
| `Queryable<Req, Res>` | `new(name, handler)` | handler: `Fn(Req) -> Result<Res>` |
| `Queryable<Req, Res>` | `handle(req).await -> Result<Res>` | runs handler, updates stats |
| `Queryable<Req, Res>` | `name() -> &str`, `stats() -> (u64, u64)` | `(requests_handled, errors)` |
| `Service<Req, Res>` | `new(name)`, `name() -> &str` | client |
| `Service<Req, Res>` | `call(req).await -> Result<Res>` | **not implemented yet** (returns `Err`) |

### Messages

Implement the `Message` trait (it is auto-derivable for any
`Serialize + Deserialize + Send + Sync + 'static` type via the trait's
requirements). Built-in messages:

```rust
use agentic_robotics_core::{RobotState, PointCloud};
use agentic_robotics_core::message::{Pose, Point3D, Message};

let state = RobotState::default();          // position/velocity: [f64; 3], timestamp: i64
assert_eq!(RobotState::type_name(), "ros3_msgs/RobotState");

let cloud = PointCloud::default();          // points: Vec<Point3D>, intensities: Vec<f32>
let pose = Pose::default();                 // position: [f64; 3], orientation: [f64; 4]
let _ = (cloud, pose);
```

`serde_json::Value` also implements `Message` for generic JSON messages.

### Serialization

```rust
use agentic_robotics_core::serialization::{
    Format, Serializer, serialize_cdr, deserialize_cdr, serialize_json, deserialize_json,
};
use agentic_robotics_core::RobotState;

let state = RobotState::default();

// CDR (DDS-compatible binary).
let bytes = serialize_cdr(&state).unwrap();
let recovered: RobotState = deserialize_cdr(&bytes).unwrap();

// JSON (human-readable, debugging).
let json = serialize_json(&state).unwrap();
let from_json: RobotState = deserialize_json(&json).unwrap();

// A Serializer bundles a Format.
let ser = Serializer::new(Format::Cdr);
let _ = (recovered, from_json, ser);
```

Available formats: `Format::Cdr`, `Format::Json`, `Format::Rkyv`.

> `serialize_rkyv` exists but currently returns an error
> (`"rkyv serialization not fully implemented"`). Use `Cdr` or `Json`.

### Errors

All fallible operations return `agentic_robotics_core::Result<T>`, an alias for
`std::result::Result<T, agentic_robotics_core::Error>`. `Error` variants:
`Zenoh`, `Serialization`, `Connection`, `Timeout`, `Configuration`, `Io`,
`Other`.

### Zenoh middleware

```rust
use agentic_robotics_core::Zenoh;
use agentic_robotics_core::middleware::ZenohConfig;

#[tokio::main]
async fn main() -> agentic_robotics_core::Result<()> {
    let zenoh = Zenoh::open().await?;            // default config
    let _custom = Zenoh::new(ZenohConfig::default()).await?;
    println!("mode = {}", zenoh.config().mode);
    Ok(())
}
```

> The `Zenoh` type is a placeholder wrapper today (it stores config but does not
> yet open a real Zenoh session). See [Planned / WIP](#-planned--wip).

---

## 🚧 Planned / WIP

The following are intended but **not implemented in the current code**:

- **Live network transport.** `Publisher::publish` currently serializes and
  updates stats only; `Subscriber` is an in-process `crossbeam` channel. Sending
  over Zenoh/DDS is planned.
- **Real Zenoh session.** `Zenoh` is a placeholder around `ZenohConfig`.
- **Service client calls.** `Service::call` returns
  `Err("Service call not implemented")`.
- **rkyv zero-copy serialization.** `serialize_rkyv` returns an error today.
- **A node abstraction, QoS configuration, topic discovery, and ROS2 bridging.**
  There is no `Node` type, QoS API, or DDS bridge in this crate yet.

---

## 🧪 Testing

```rust
#[cfg(test)]
mod tests {
    use agentic_robotics_core::{Publisher, RobotState};

    #[tokio::test]
    async fn test_publish() {
        let publisher = Publisher::<RobotState>::new("robot/state");
        publisher.publish(&RobotState::default()).await.unwrap();
        let (count, bytes) = publisher.stats();
        assert_eq!(count, 1);
        assert!(bytes > 0);
    }
}
```

---

## 📄 License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](../../LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT License ([LICENSE-MIT](../../LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

---

## 🔗 Links

- **Homepage**: [ruv.io](https://ruv.io)
- **Documentation**: [docs.rs/agentic-robotics-core](https://docs.rs/agentic-robotics-core)
- **Repository**: [github.com/ruvnet/vibecast](https://github.com/ruvnet/vibecast)
