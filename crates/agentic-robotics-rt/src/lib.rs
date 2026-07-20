//! ROS3 Real-Time Execution
//!
//! Dual-runtime architecture: two Tokio multi-threaded runtimes (a 2-thread
//! high-priority runtime and a 4-thread low-priority runtime) with tasks routed
//! by deadline. Hard-real-time RTIC integration is planned but not yet
//! implemented; "priority" is currently expressed by runtime routing, not by
//! OS-level thread priorities.

pub mod executor;
pub mod latency;
pub mod scheduler;

pub use executor::{Deadline, Priority, ROS3Executor};
pub use latency::LatencyTracker;
pub use scheduler::PriorityScheduler;

/// Real-time task priority levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RTPriority {
    /// Lowest priority (background tasks)
    Background = 0,
    /// Low priority
    Low = 1,
    /// Normal priority
    Normal = 2,
    /// High priority
    High = 3,
    /// Critical priority (hard real-time)
    Critical = 4,
}

impl From<u8> for RTPriority {
    fn from(value: u8) -> Self {
        match value {
            0 => RTPriority::Background,
            1 => RTPriority::Low,
            2 => RTPriority::Normal,
            3 => RTPriority::High,
            _ => RTPriority::Critical,
        }
    }
}

impl From<RTPriority> for u8 {
    fn from(priority: RTPriority) -> Self {
        priority as u8
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_priority_conversion() {
        let priority = RTPriority::High;
        let value: u8 = priority.into();
        assert_eq!(value, 3);

        let converted: RTPriority = value.into();
        assert_eq!(converted, RTPriority::High);
    }
}
