//! agentic-robotics-embedded - placeholder for future embedded support
//!
//! This crate is currently a small `std` skeleton. It exports two
//! configuration types — [`EmbeddedPriority`] and [`EmbeddedConfig`] — that
//! sketch the intended embedded API surface. Real embedded support (no-std,
//! Embassy, RTIC, target-specific HALs) is **not** implemented yet: the
//! relevant dependencies are commented out in `Cargo.toml` and the `embassy`
//! / `rtic` Cargo features are present but empty (they enable nothing today).


/// Embedded task priority
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmbeddedPriority {
    Low = 0,
    Normal = 1,
    High = 2,
    Critical = 3,
}

/// Embedded system configuration
#[derive(Debug, Clone)]
pub struct EmbeddedConfig {
    pub tick_rate_hz: u32,
    pub stack_size: usize,
}

impl Default for EmbeddedConfig {
    fn default() -> Self {
        Self {
            tick_rate_hz: 1000,
            stack_size: 4096,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_embedded_config() {
        let config = EmbeddedConfig::default();
        assert_eq!(config.tick_rate_hz, 1000);
        assert_eq!(config.stack_size, 4096);
    }
}
