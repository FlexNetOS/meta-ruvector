//! rvAgent Engine — the unified LifeOS **TEAS** `TaskRunner` over `handoff.task.v1`.
//!
//! This crate is the build home of the Task Execution Automation System's execution
//! engine. It implements the *existing* [`rvagent_a2a::executor::TaskRunner`] trait so
//! the A2A `Router`, budget enforcer, and recursion guard compose around it unchanged —
//! there is deliberately **no parallel task type** (ADR-159; TEAS DOMAIN_MODEL §8).
//!
//! ## Build stage
//! - **TEASTASK-001 (this crate):** scaffold + a *stub* `TaskRunner`. The stub
//!   acknowledges a [`TaskSpec`] in the `Submitted` state and never returns
//!   `Completed`, so a stub run can never be mistaken for a real, proof-backed
//!   completion ("no paper completion").
//! - **TEASTASK-011:** replace [`RvAgentEngine::run`] with real execution — dispatch
//!   the WorkOrder via `rvagent-backends`, run its `verification_command`, and capture
//!   the pass/fail verdict.
//! - **TEASTASK-004:** write the resulting `ProofRecord` to the handoff witnessed ledger.

pub mod adapter;
pub mod workorder;

pub use adapter::{taskspec_to_workorder, workorder_to_taskspec, AdapterError};
pub use workorder::{IntentLock, Priority, Status, WorkOrder};

use async_trait::async_trait;
use chrono::Utc;
use rvagent_a2a::error::A2aError;
use rvagent_a2a::executor::TaskRunner;
use rvagent_a2a::types::{Task, TaskSpec, TaskState, TaskStatus};

/// The unified TEAS execution engine.
///
/// Stub form (TEASTASK-001): it wires the trait and its type identity into the rvAgent
/// A2A machinery without yet executing work. Construct with [`RvAgentEngine::new`].
#[derive(Debug, Default, Clone)]
pub struct RvAgentEngine;

impl RvAgentEngine {
    /// Construct a new engine.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl TaskRunner for RvAgentEngine {
    /// **Stub:** acknowledge the spec in `Submitted` state without executing it.
    ///
    /// Deliberately never returns `TaskState::Completed` — real dispatch and verdict
    /// capture land in TEASTASK-011, so nothing here can be read as a proven completion.
    async fn run(&self, spec: TaskSpec) -> Result<Task, A2aError> {
        Ok(Task {
            id: spec.id,
            session_id: None,
            status: TaskStatus {
                state: TaskState::Submitted,
                timestamp: Utc::now(),
                message: None,
            },
            history: vec![spec.message],
            artifacts: Vec::new(),
            metadata: spec.metadata,
        })
    }

    /// Synchronous stub: nothing is dispatched, so cancellation is a successful noop.
    async fn cancel(&self, _task_id: &str) -> Result<(), A2aError> {
        Ok(())
    }
}
