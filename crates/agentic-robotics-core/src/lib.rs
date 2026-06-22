//! agentic-robotics-core - Rust robotics middleware (internally branded "ROS3")
//!
//! A Rust publish/subscribe and service (RPC) messaging library for building
//! robot systems. It provides typed [`Publisher`]/[`Subscriber`] channels,
//! request/response [`Service`]/[`Queryable`] services, pluggable
//! serialization ([`Format::Cdr`], [`Format::Json`], [`Format::Rkyv`]), and a
//! [`Zenoh`] session wrapper.
//!
//! Note: this is an early-stage crate. The [`Zenoh`] middleware and the rkyv
//! serialization path are placeholders today (see each type's docs), and the
//! `Publisher`/`Subscriber` are in-process channels rather than a live network
//! transport. Internal message type names use the `ros3_msgs/` prefix.

pub mod middleware;
pub mod serialization;
pub mod message;
pub mod publisher;
pub mod subscriber;
pub mod service;
pub mod error;

pub use middleware::Zenoh;
pub use message::{Message, RobotState, PointCloud};
pub use publisher::Publisher;
pub use subscriber::Subscriber;
pub use service::{Service, Queryable};
pub use error::{Result, Error};

/// ROS3 Core version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Initialize ROS3 runtime
pub fn init() -> Result<()> {
    tracing_subscriber::fmt()
        .with_target(false)
        .with_thread_ids(true)
        .with_level(true)
        .init();

    tracing::info!("ROS3 Core v{} initialized", VERSION);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init() {
        let result = init();
        assert!(result.is_ok());
    }
}
