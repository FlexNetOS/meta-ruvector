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
pub mod learning;
pub mod ledger;
pub mod proof;
pub mod schema;
pub mod selection;
pub mod workorder;

pub use adapter::{taskspec_to_workorder, workorder_to_taskspec, AdapterError};
pub use csv::workorders_to_csv;
pub use learning::{LearningError, RecordedRun, TrajectoryRecorder};
pub use ledger::{LedgerError, ProofLedger};
pub use proof::{ProofRecord, ProofStatus, PROOF_SCHEMA_VERSION};
pub use schema::{proof_record_schema, workorder_schema};
pub use selection::{
    parse_ready, top_ready, ClaimedTask, CommandOutput, CommandRunner, ReadyTask, SelectionError,
    Selector, SystemRunner,
};
pub use workorder::{IntentLock, Priority, Status, WorkOrder};

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

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
///
/// TEASTASK-004: when constructed with [`RvAgentEngine::with_ledger`], every *real* run
/// outcome (Completed / Failed — not InputRequired, not a missing envelope) is witnessed
/// by appending a [`ProofRecord`] to the self-contained JSONL proof ledger. Under the
/// "no proof, no done" doctrine, if a ledger is configured and the append fails, [`run`]
/// returns an error rather than a completion that could never be witnessed.
///
/// TEASTASK-008 (capability-gain law): attach a [`TrajectoryRecorder`] with
/// [`RvAgentEngine::with_recorder`] to capture every *real* run outcome as a learning
/// trajectory in a sona `ReasoningBank`, so each cycle can inform the next.
///
/// ## Proof is mandatory; trajectory is best-effort (deliberate asymmetry)
/// The two seams are intentionally *not* symmetric. Proof is load-bearing evidence: a run
/// that cannot be witnessed is not a completion, so a ledger append failure aborts
/// [`run`]. Learning is an optimization: a run's *outcome* stands on its own whether or
/// not it was recorded, so a recorder failure is swallowed and never fails the task.
#[derive(Debug, Default, Clone)]
pub struct RvAgentEngine {
    /// Optional witnessed proof ledger. `None` reproduces the exact TEASTASK-011
    /// behavior (no proof written). `Arc` so the engine stays cheaply `Clone`.
    ledger: Option<Arc<ProofLedger>>,
    /// Optional learning-trajectory recorder (TEASTASK-008). `None` reproduces the exact
    /// pre-TEASTASK-008 behavior (no trajectory recorded). `Arc` keeps the engine cheaply
    /// `Clone` and lets a caller hold a handle to inspect what was recorded.
    recorder: Option<Arc<TrajectoryRecorder>>,
}

impl RvAgentEngine {
    /// Construct a new engine with no proof ledger and no learning recorder
    /// (TEASTASK-011 behavior — no proof written, no trajectory recorded).
    #[must_use]
    pub fn new() -> Self {
        Self {
            ledger: None,
            recorder: None,
        }
    }

    /// Construct an engine that witnesses every real run outcome to the JSONL proof
    /// ledger at `path` (created on first append).
    #[must_use]
    pub fn with_ledger(path: PathBuf) -> Self {
        Self {
            ledger: Some(Arc::new(ProofLedger::new(path))),
            recorder: None,
        }
    }

    /// Attach a learning-trajectory recorder (TEASTASK-008), composably with any existing
    /// ledger. Every *real* run outcome is then recorded as a trajectory — best-effort, so
    /// a recorder failure never fails the task (see the type-level asymmetry note).
    #[must_use]
    pub fn with_recorder(mut self, recorder: Arc<TrajectoryRecorder>) -> Self {
        self.recorder = Some(recorder);
        self
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

impl RvAgentEngine {
    /// Witness a *real* run outcome by appending a [`ProofRecord`] to the ledger.
    ///
    /// A no-op when no ledger is configured (reproducing TEASTASK-011 exactly). When a
    /// ledger IS configured, "no proof, no done" applies: if the append fails, this
    /// returns [`A2aError::Internal`] so the caller aborts the completion — a run that
    /// cannot be witnessed is not a real completion.
    fn record_proof(
        &self,
        task_id: &str,
        status: ProofStatus,
        started_at: String,
        command: String,
        output: String,
        failure_reason: Option<String>,
    ) -> Result<(), A2aError> {
        let Some(ledger) = self.ledger.as_ref() else {
            return Ok(());
        };
        let record = ProofRecord {
            proof_schema_version: PROOF_SCHEMA_VERSION.to_string(),
            task_id: task_id.to_string(),
            correlation_id: None,
            cell_id: None,
            status,
            started_at,
            completed_at: Utc::now().to_rfc3339(),
            actor: "rvagent-engine".to_string(),
            helper_id: None,
            model_tag: None,
            repo_path: None,
            git_head_before: None,
            git_head_after: None,
            diff_summary: None,
            files_changed: Vec::new(),
            commands_run: vec![command],
            verification_output: serde_json::Value::String(output),
            checksums: BTreeMap::new(),
            action_hash: String::new(),
            prev_action_hash: None,
            ledger_seq: None,
            logs_uri: None,
            rollback_point: None,
            evidence: Vec::new(),
            failed_checks: Vec::new(),
            failure_reason,
            next_action: "select-next".to_string(),
        };
        ledger.append(record).map_err(|e| {
            A2aError::Internal(format!(
                "no proof, no done: failed to witness ProofRecord for {task_id}: {e}"
            ))
        })?;
        Ok(())
    }

    /// Record a *real* run outcome as a learning trajectory (TEASTASK-008).
    ///
    /// A no-op when no recorder is configured. Unlike [`record_proof`](Self::record_proof),
    /// this is **best-effort**: a recording error is intentionally swallowed and never
    /// propagated, so learning can never turn a real Completed/Failed outcome into a task
    /// failure. `reward` is the honest signal — `1.0` Completed, `0.0` Failed.
    fn record_trajectory(&self, task_id: &str, objective: &str, reward: f32) {
        if let Some(recorder) = self.recorder.as_ref() {
            // Best-effort: swallow the result. Proof is mandatory; trajectory is not.
            let _ = recorder.record_run(task_id, objective, reward);
        }
    }
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
        // Capture the run start BEFORE any execution, for the ProofRecord's started_at.
        let started_at = Utc::now().to_rfc3339();

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

        // 4. Map the exit verdict onto the returned Task, witnessing a ProofRecord for
        //    the real run outcome first (no-op when no ledger is configured). Under
        //    "no proof, no done", a failed witness aborts the completion.
        match result.exit_code {
            Some(0) => {
                self.record_proof(
                    &spec.id,
                    ProofStatus::Completed,
                    started_at,
                    command.clone(),
                    result.output.clone(),
                    None,
                )?;
                // TEASTASK-008: capture the real Completed outcome as a learning
                // trajectory (best-effort — see record_trajectory). reward = 1.0.
                self.record_trajectory(&spec.id, &wo.objective, 1.0);
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
                self.record_proof(
                    &spec.id,
                    ProofStatus::Failed,
                    started_at,
                    command,
                    result.output,
                    Some(reason.clone()),
                )?;
                // TEASTASK-008: capture the real Failed outcome as a learning trajectory
                // (best-effort — see record_trajectory). reward = 0.0.
                self.record_trajectory(&spec.id, &wo.objective, 0.0);
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

    // ---- TEASTASK-004: witnessed proof ledger integration ----

    #[tokio::test]
    async fn with_ledger_passing_run_witnesses_a_completed_proof() {
        let tmp = tempfile::NamedTempFile::new().expect("temp file");
        let engine = RvAgentEngine::with_ledger(tmp.path().to_path_buf());
        let spec = crate::adapter::workorder_to_taskspec(&work_order(Some("true")))
            .expect("to taskspec");

        let task = engine.run(spec).await.expect("run ok");
        assert_eq!(task.status.state, TaskState::Completed);

        let ledger = crate::ledger::ProofLedger::new(tmp.path().to_path_buf());
        let records = ledger.read_all().expect("read_all");
        assert_eq!(records.len(), 1, "a passing run must witness exactly one proof");
        assert_eq!(records[0].status, ProofStatus::Completed);
        assert_eq!(records[0].task_id, "TEASTASK-011-test");
        assert_eq!(records[0].actor, "rvagent-engine");
        assert_eq!(records[0].commands_run, vec!["true".to_string()]);
        assert_eq!(ledger.verify_witness_chain().expect("verify"), 1);
    }

    #[tokio::test]
    async fn with_ledger_failing_run_witnesses_a_failed_proof() {
        let tmp = tempfile::NamedTempFile::new().expect("temp file");
        let engine = RvAgentEngine::with_ledger(tmp.path().to_path_buf());
        let spec = crate::adapter::workorder_to_taskspec(&work_order(Some("false")))
            .expect("to taskspec");

        let task = engine.run(spec).await.expect("run ok");
        assert_eq!(task.status.state, TaskState::Failed);

        let ledger = crate::ledger::ProofLedger::new(tmp.path().to_path_buf());
        let records = ledger.read_all().expect("read_all");
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].status, ProofStatus::Failed);
        assert!(
            records[0].failure_reason.is_some(),
            "a failed run's proof must carry a failure_reason"
        );
        assert_eq!(ledger.verify_witness_chain().expect("verify"), 1);
    }

    #[tokio::test]
    async fn without_ledger_completes_and_writes_nothing() {
        // Unchanged TEASTASK-011 behavior: no ledger → Completed, no proof written.
        let task = run_wo(&work_order(Some("true"))).await;
        assert_eq!(task.status.state, TaskState::Completed);
        // (RvAgentEngine::new() has no ledger; nothing to read — asserted by the fact
        // that the run succeeded without any ledger path configured.)
    }

    // ---- TEASTASK-008: learning-trajectory recorder integration ----

    #[tokio::test]
    async fn with_recorder_passing_run_records_reward_one() {
        let recorder = Arc::new(crate::learning::TrajectoryRecorder::new());
        let engine = RvAgentEngine::new().with_recorder(recorder.clone());
        let spec = crate::adapter::workorder_to_taskspec(&work_order(Some("true")))
            .expect("to taskspec");

        let task = engine.run(spec).await.expect("run ok");
        assert_eq!(task.status.state, TaskState::Completed);

        assert_eq!(
            recorder.trajectory_count(),
            1,
            "a Completed run must record exactly one trajectory"
        );
        let recorded = recorder.recorded();
        assert_eq!(recorded.len(), 1);
        assert_eq!(
            recorded[0].reward, 1.0,
            "a Completed run's trajectory carries reward 1.0"
        );
    }

    #[tokio::test]
    async fn with_recorder_failing_run_records_reward_zero() {
        let recorder = Arc::new(crate::learning::TrajectoryRecorder::new());
        let engine = RvAgentEngine::new().with_recorder(recorder.clone());
        let spec = crate::adapter::workorder_to_taskspec(&work_order(Some("false")))
            .expect("to taskspec");

        let task = engine.run(spec).await.expect("run ok");
        assert_eq!(task.status.state, TaskState::Failed);

        assert_eq!(recorder.trajectory_count(), 1);
        let recorded = recorder.recorded();
        assert_eq!(recorded.len(), 1);
        assert_eq!(
            recorded[0].reward, 0.0,
            "a Failed run's trajectory carries reward 0.0"
        );
    }

    #[tokio::test]
    async fn with_recorder_input_required_records_nothing() {
        // No verification_command → InputRequired is NOT a real outcome, so nothing is
        // recorded (matches the proof-ledger seam: only Completed/Failed are witnessed).
        let recorder = Arc::new(crate::learning::TrajectoryRecorder::new());
        let engine = RvAgentEngine::new().with_recorder(recorder.clone());
        let spec =
            crate::adapter::workorder_to_taskspec(&work_order(None)).expect("to taskspec");

        let task = engine.run(spec).await.expect("run ok");
        assert_eq!(task.status.state, TaskState::InputRequired);
        assert_eq!(
            recorder.trajectory_count(),
            0,
            "InputRequired is not a real outcome — record nothing"
        );
    }

    #[tokio::test]
    async fn without_recorder_run_is_unaffected() {
        // No recorder → run behaves exactly as before; nothing is recorded and the
        // outcome is unchanged.
        let task = run_wo(&work_order(Some("true"))).await;
        assert_eq!(task.status.state, TaskState::Completed);
        let failing = run_wo(&work_order(Some("false"))).await;
        assert_eq!(failing.status.state, TaskState::Failed);
    }

    #[tokio::test]
    async fn recorder_and_ledger_compose() {
        // Proof (mandatory) and trajectory (best-effort) coexist on one engine.
        let tmp = tempfile::NamedTempFile::new().expect("temp file");
        let recorder = Arc::new(crate::learning::TrajectoryRecorder::new());
        let engine =
            RvAgentEngine::with_ledger(tmp.path().to_path_buf()).with_recorder(recorder.clone());
        let spec = crate::adapter::workorder_to_taskspec(&work_order(Some("true")))
            .expect("to taskspec");

        let task = engine.run(spec).await.expect("run ok");
        assert_eq!(task.status.state, TaskState::Completed);

        let ledger = crate::ledger::ProofLedger::new(tmp.path().to_path_buf());
        assert_eq!(ledger.read_all().expect("read_all").len(), 1);
        assert_eq!(recorder.trajectory_count(), 1);
    }
}
