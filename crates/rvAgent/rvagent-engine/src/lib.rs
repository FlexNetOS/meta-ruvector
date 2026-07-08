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
pub mod csv;
pub mod proof;
pub mod schema;
pub mod workorder;

pub use adapter::{taskspec_to_workorder, workorder_to_taskspec, AdapterError};
pub use csv::workorders_to_csv;
pub use proof::{ProofRecord, ProofStatus, PROOF_SCHEMA_VERSION};
pub use schema::{proof_record_schema, workorder_schema};
pub use workorder::{IntentLock, Priority, Status, WorkOrder};

use std::path::PathBuf;

use async_trait::async_trait;
use chrono::Utc;
use rvagent_a2a::error::A2aError;
use rvagent_a2a::executor::TaskRunner;
use rvagent_a2a::types::{Artifact, Message, Part, Role, Task, TaskSpec, TaskState, TaskStatus};
use rvagent_backends::{LocalShellBackend, LocalShellConfig, SandboxBackend};

/// The unified TEAS execution engine.
///
/// Real form (TEASTASK-011): [`RvAgentEngine::run`] recovers the [`WorkOrder`] from the
/// incoming [`TaskSpec`], executes its `verification_command` through the hardened
/// [`LocalShellBackend`], and maps the exit verdict onto the returned [`Task`]. There
/// is **no paper completion** — a task only reaches [`TaskState::Completed`] when a
/// verification command actually ran and exited 0. Construct with [`RvAgentEngine::new`].
#[derive(Debug, Default, Clone)]
pub struct RvAgentEngine;

impl RvAgentEngine {
    /// Construct a new engine.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

/// Build an `agent`-role [`Message`] carrying a single text note (status messages).
fn agent_note(text: impl Into<String>) -> Message {
    Message {
        role: Role::Agent,
        parts: vec![Part::Text { text: text.into() }],
        metadata: serde_json::Value::Null,
    }
}

/// Assemble the returned [`Task`], consuming `spec` for its `id`/`message`/`metadata`.
fn make_task(
    spec: TaskSpec,
    state: TaskState,
    message: Option<Message>,
    artifacts: Vec<Artifact>,
) -> Task {
    Task {
        id: spec.id,
        session_id: None,
        status: TaskStatus {
            state,
            timestamp: Utc::now(),
            message,
        },
        history: vec![spec.message],
        artifacts,
        metadata: spec.metadata,
    }
}

/// Choose the working directory for verification.
///
/// Prefer the WorkOrder's first `path_scope` entry that resolves to an existing
/// directory (so the command runs where its subject lives); otherwise fall back to
/// the process' current directory, and finally to `.` if that is unreadable. Never
/// panics — a non-existent scope would only make the spawn fail, which we still map
/// to a `Failed` verdict rather than a crash.
fn choose_cwd(wo: &WorkOrder) -> PathBuf {
    for entry in &wo.path_scope {
        let candidate = PathBuf::from(entry);
        if candidate.is_dir() {
            return candidate;
        }
    }
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

#[async_trait]
impl TaskRunner for RvAgentEngine {
    /// Execute the WorkOrder's `verification_command` and map the verdict to a [`Task`].
    ///
    /// Verdict mapping (TEASTASK-011 acceptance criteria):
    /// - no WorkOrder envelope on the spec → [`A2aError::Internal`] (the dispatch input
    ///   is malformed; `error.rs` has no dedicated bad-input variant, so the catch-all
    ///   applies).
    /// - `verification_command == None` → [`TaskState::InputRequired`] (non-terminal):
    ///   there is no way to *prove* completion, so the engine refuses to fabricate one.
    /// - command exits `0` → [`TaskState::Completed`], with an [`Artifact`] carrying the
    ///   combined stdout/stderr the backend captured.
    /// - command exits non-zero (or times out) → [`TaskState::Failed`], with a status
    ///   [`Message`] describing the failure. Never `Completed` on a failing proof.
    ///
    /// Execution goes through [`LocalShellBackend`] (rvagent-backends) — not a raw
    /// `tokio::process::Command` — to inherit its environment sanitization (secrets
    /// stripped), timeout, and output-truncation hardening (ADR-103 C2).
    async fn run(&self, spec: TaskSpec) -> Result<Task, A2aError> {
        // 1. Recover the WorkOrder envelope carried in TaskSpec.metadata.
        let wo = crate::adapter::taskspec_to_workorder(&spec)
            .map_err(|e| A2aError::Internal(format!("no WorkOrder envelope on TaskSpec: {e}")))?;

        // 2. No verification_command → cannot prove completion (no paper completion).
        let Some(command) = wo.verification_command.clone() else {
            return Ok(make_task(
                spec,
                TaskState::InputRequired,
                Some(agent_note("no verification_command to prove completion")),
                Vec::new(),
            ));
        };

        // 3. Run the verification through the hardened shell backend.
        let cwd = choose_cwd(&wo);
        let backend = LocalShellBackend::new(cwd, LocalShellConfig::default());
        let result = backend.execute(&command, None).await;

        // 4. Map the exit verdict onto the returned Task.
        match result.exit_code {
            Some(0) => {
                let artifact = Artifact {
                    name: Some("verification".to_string()),
                    description: Some(format!("stdout/stderr of verification_command `{command}`")),
                    parts: vec![Part::Text {
                        text: result.output,
                    }],
                    index: 0,
                    append: false,
                    last_chunk: true,
                    metadata: serde_json::Value::Null,
                };
                Ok(make_task(spec, TaskState::Completed, None, vec![artifact]))
            }
            other => {
                let verdict = match other {
                    Some(code) => format!("exit {code}"),
                    None => "timed out / no exit code".to_string(),
                };
                let reason = format!(
                    "verification_command `{command}` failed ({verdict}): {}",
                    result.output
                );
                Ok(make_task(
                    spec,
                    TaskState::Failed,
                    Some(agent_note(reason)),
                    Vec::new(),
                ))
            }
        }
    }

    /// No detached execution yet: `run` awaits the verification inline, so there is
    /// nothing to interrupt and cancellation is a successful noop.
    async fn cancel(&self, _task_id: &str) -> Result<(), A2aError> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workorder::{Priority, Status, WorkOrder};

    /// A minimal WorkOrder whose `verification_command` the test can set.
    fn work_order(verification: Option<&str>) -> WorkOrder {
        WorkOrder {
            schema: "handoff.task.v1".to_string(),
            id: "TEASTASK-011-test".to_string(),
            title: "trivial verification".to_string(),
            objective: "prove the runner executes real work".to_string(),
            status: Status::Active,
            priority: Priority::P2,
            path_scope: Vec::new(),
            acceptance_criteria: vec!["verification runs".to_string()],
            correlation_id: None,
            owner_lane: None,
            role: None,
            dependencies: Vec::new(),
            blocked_by: Vec::new(),
            allows_network: false,
            allows_dependency_addition: false,
            human_approval_required: false,
            verification_command: verification.map(str::to_string),
            rollback_plan: None,
            intent_lock: None,
        }
    }

    async fn run_wo(wo: &WorkOrder) -> Task {
        let spec = crate::adapter::workorder_to_taskspec(wo).expect("to taskspec");
        RvAgentEngine::new().run(spec).await.expect("run ok")
    }

    #[tokio::test]
    async fn passing_verification_completes_with_artifact() {
        // `true` exits 0 → a real, proof-backed Completed.
        let task = run_wo(&work_order(Some("true"))).await;
        assert_eq!(task.status.state, TaskState::Completed);
        assert_eq!(
            task.artifacts.len(),
            1,
            "a passing verification must carry its stdout/stderr artifact"
        );
        assert_eq!(task.artifacts[0].name.as_deref(), Some("verification"));
    }

    #[tokio::test]
    async fn failing_verification_fails_never_completes() {
        // `false` exits 1 → Failed, and NEVER Completed.
        let task = run_wo(&work_order(Some("false"))).await;
        assert_eq!(task.status.state, TaskState::Failed);
        assert_ne!(
            task.status.state,
            TaskState::Completed,
            "a failing verification must never read as a completion"
        );
        let message = task.status.message.expect("failure message present");
        match &message.parts[0] {
            Part::Text { text } => assert!(
                text.contains("failed"),
                "failure message should describe the failure, got: {text}"
            ),
            other => panic!("expected a Text failure message, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn missing_verification_is_input_required_not_completed() {
        // No verification_command → non-terminal InputRequired (no paper completion).
        let task = run_wo(&work_order(None)).await;
        assert_eq!(task.status.state, TaskState::InputRequired);
        assert!(
            !task.status.state.is_terminal(),
            "InputRequired must stay non-terminal"
        );
        let message = task.status.message.expect("explanatory message present");
        match &message.parts[0] {
            Part::Text { text } => {
                assert_eq!(text, "no verification_command to prove completion");
            }
            other => panic!("expected a Text message, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn no_workorder_envelope_is_internal_error() {
        // A TaskSpec with no WorkOrder envelope → A2aError (not a panic, not Completed).
        let wo = work_order(Some("true"));
        let mut spec = crate::adapter::workorder_to_taskspec(&wo).expect("to taskspec");
        spec.metadata = serde_json::Value::Null;
        let err = RvAgentEngine::new().run(spec).await.expect_err("must error");
        assert!(matches!(err, A2aError::Internal(_)));
    }
}
