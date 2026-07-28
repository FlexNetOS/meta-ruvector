//! # ATAS/ESN future-timeline research harness (ARCHBP-011)
//!
//! Bounded **Agentic Temporal Attractor Studio** research instrument:
//! echo-state-network (ESN) reservoirs, ensemble simulations, and structured
//! outputs for state diffs, resource burn, complexity spikes, and failure
//! nodes.
//!
//! ## Non-authority contract (binding)
//!
//! This is a **research layer only**. Every timeline and report is labeled
//! `authority: "research-only"` / `non_authoritative: true`. Forecasts
//! produced here must **never** be used as a completion gate, a proof, or a
//! production task-routing authority. The blueprint classifies ATAS/ESN as
//! *planned/unproven*; this harness is the instrument that tests those
//! claims — it is not evidence that they hold.
//!
//! ## Primary research (no invented algorithm APIs)
//!
//! The math implemented here is the standard, citable ESN formulation:
//! - H. Jaeger, "The 'echo state' approach to analysing and training
//!   recurrent neural networks", GMD Report 148 (2001).
//! - M. Lukoševičius & H. Jaeger, "Reservoir computing approaches to
//!   recurrent neural network training", Computer Science Review 3 (2009).
//! - M. Lukoševičius, "A Practical Guide to Applying Echo State Networks",
//!   Neural Networks: Tricks of the Trade (2012).
//! - Leaky-integrator update: `x(t+1) = (1-a)·x(t) + a·tanh(W_in·u(t+1) +
//!   W·x(t) + b)`, fixed random reservoir `W` rescaled to a target spectral
//!   radius, linear readout trained by ridge regression on collected states.
//! - Uncertainty: split-conformal calibration (Vovk et al., *Algorithmic
//!   Learning in a Random World*, 2005; Papadopoulos et al., inductive
//!   conformal prediction, 2002) over ensemble-mean residuals.
//!
//! ## Determinism
//!
//! Zero external dependencies. All randomness flows from an in-crate
//! SplitMix64 stream seeded explicitly; fixed seeds reproduce whole studio
//! runs bit-for-bit (see `tests/harness_gate.rs`, gate 1).

#![forbid(unsafe_code)]

pub mod atas;
pub mod attractor_study;
pub mod baseline;
pub mod dataset;
pub mod ensemble;
pub mod esn;
pub mod json;
pub mod linalg;
pub mod rng;
pub mod schema;
