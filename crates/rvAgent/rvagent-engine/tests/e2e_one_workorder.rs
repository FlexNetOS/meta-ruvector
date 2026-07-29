//! TEASTASK-007 — LifeOS **TEAS** end-to-end milestone (in-process integration test).
//!
//! Proves that **one** `WorkOrder` flows end-to-end through the unified engine loop,
//! in process, using ONLY the crate's public API:
//!
//! ```text
//! front-door WorkOrder
//!   -> Selector::select_and_claim  (git-kb `ready` -> `hf claim`, faked at the boundary)
//!   -> adapter::workorder_to_taskspec
//!   -> RvAgentEngine::run           (REAL execution via LocalShellBackend)
//!   -> witnessed ProofRecord on the JSONL ledger
//!   -> ProofLedger::verify_witness_chain
//! ```
//!
//! ## What is REAL here vs SIMULATED
//! - **REAL:** [`RvAgentEngine::run`] actually executes the WorkOrder's
//!   `verification_command` through the hardened `LocalShellBackend` (rvagent-backends);
//!   the [`ProofLedger`] blake3 witness chain is really computed and re-verified; the
//!   S3 adapter round-trip (`workorder_to_taskspec`) is the production code path; and the
//!   [`Selector`] claim-safety logic (exit 0 => lease acquired, non-zero => conflict, do
//!   NOT run) is the production code path.
//! - **SIMULATED:** the `git-kb ready --json` output and the `hf claim` exit code, via a
//!   test-local fake [`CommandRunner`] — because `git-kb`/`hf` are external binaries and
//!   prompt_hub is a separate repo. The full cross-repo `gitkb-doc -> WorkOrder` fetch
//!   (turning a claimed slug into its committed WorkOrder envelope) completes at
//!   consolidation and is out of scope here; this test constructs the WorkOrder itself,
//!   as the front door would emit it, and uses the claim only to prove the
//!   selection/lease step. Likewise, network/drift/approval gate refusal is enforced by
//!   handoff's fail-closed gate engine (handoff-drift / handoff-policy) at consolidation,
//!   not in this in-process test.

use rvagent_a2a::executor::TaskRunner;
use rvagent_a2a::types::TaskState;
use rvagent_engine::{
    workorder_to_taskspec, ClaimedTask, CommandOutput, CommandRunner, Priority, ProofLedger,
    ProofStatus, RvAgentEngine, SelectionError, Selector, Status, WorkOrder,
};

/// git-kb slug for the milestone task. Kept coherent with [`WO_ID`] so the wiring
/// reads as one task moving through selection -> adapter -> execution.
const READY_SLUG: &str = "tasks/TEASTASK-007";
/// The WorkOrder id the front door would emit for the slug above.
const WO_ID: &str = "TEASTASK-007";
/// The lease holder identity this engine claims under (fixed for determinism).
const HOLDER: &str = "teas-agent-e2e";

/// The `git-kb ready --json --limit 1` payload the boundary is faked to return: one ready
/// candidate, whose slug is the milestone task, with a high score. Extra `reasons` field
/// present to mirror the real git-kb shape (ignored by the parser).
fn ready_json() -> String {
    format!(
        r#"{{"tasks":[{{"slug":"{READY_SLUG}","doc_type":"task","priority":"high","score":91.4,"reasons":{{"deps":"clear"}}}}]}}"#
    )
}

/// A scripted [`CommandRunner`] that returns a canned [`CommandOutput`] per program name,
/// standing in for the external `git-kb` and `hf` binaries at seam S2.
struct FakeBoundary {
    gitkb: CommandOutput,
    hf: CommandOutput,
}

impl CommandRunner for FakeBoundary {
    fn run(
        &self,
        program: &str,
        _args: &[&str],
        _env: &[(&str, &str)],
    ) -> std::io::Result<CommandOutput> {
        match program {
            "git-kb" => Ok(self.gitkb.clone()),
            "hf" => Ok(self.hf.clone()),
            other => Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("no scripted output for {other}"),
            )),
        }
    }
}

fn out(stdout: &str, stderr: &str, code: Option<i32>) -> CommandOutput {
    CommandOutput {
        stdout: stdout.to_string(),
        stderr: stderr.to_string(),
        exit_code: code,
    }
}

/// The WorkOrder the front door would emit for the claimed slug. `verification` sets the
/// `verification_command`; all other fields are schema-valid and fixed.
fn work_order(verification: &str) -> WorkOrder {
    WorkOrder {
        schema: "handoff.task.v1".to_string(),
        id: WO_ID.to_string(),
        title: "TEAS end-to-end milestone".to_string(),
        objective: "prove one WorkOrder flows end-to-end through the unified engine".to_string(),
        status: Status::Active,
        priority: Priority::P1,
        path_scope: Vec::new(),
        acceptance_criteria: vec!["one WorkOrder completes via a real, witnessed run".to_string()],
        correlation_id: None,
        owner_lane: None,
        role: None,
        dependencies: Vec::new(),
        blocked_by: Vec::new(),
        allows_network: false,
        allows_dependency_addition: false,
        human_approval_required: false,
        verification_command: Some(verification.to_string()),
        rollback_plan: None,
        intent_lock: None,
    }
}

/// **Assertion 1 — HAPPY PATH.**
///
/// git-kb reports the milestone task ready, `hf claim` succeeds (exit 0), so
/// `select_and_claim` returns a [`ClaimedTask`] for our slug under our holder. The same
/// WorkOrder is then adapted and run through [`RvAgentEngine::run`] with a live ledger:
/// the `true` verification really executes and exits 0, so the task reaches
/// [`TaskState::Completed`] carrying a `verification` artifact (produced only on a real
/// exit-0 run). Exactly one [`ProofStatus::Completed`] ProofRecord is witnessed, and the
/// blake3 witness chain re-verifies to length 1.
#[tokio::test]
async fn happy_path_one_workorder_flows_end_to_end() {
    // --- select_and_claim: git-kb ready (0) -> hf claim (0) ------------------------------
    let boundary = FakeBoundary {
        gitkb: out(&ready_json(), "", Some(0)),
        hf: out("hf claim: acquired lease", "", Some(0)),
    };
    let selector = Selector::new(boundary).with_holder(HOLDER);
    let claimed: ClaimedTask = selector
        .select_and_claim()
        .expect("happy-path claim must acquire the lease");
    assert_eq!(
        claimed.slug, READY_SLUG,
        "we claimed the milestone task slug"
    );
    assert_eq!(claimed.holder, HOLDER, "claim recorded under our holder");

    // --- adapter -> engine.run (REAL execution) with a live ledger ----------------------
    let tmp = tempfile::NamedTempFile::new().expect("temp ledger file");
    let engine = RvAgentEngine::with_ledger(tmp.path().to_path_buf());
    let spec = workorder_to_taskspec(&work_order("true")).expect("adapt WorkOrder -> TaskSpec");

    let task = engine.run(spec).await.expect("engine.run must succeed");

    // Completed via a REAL run: the exit-0 path is the only one that both sets Completed
    // AND emits the verification artifact (the backend really ran the command).
    assert_eq!(task.status.state, TaskState::Completed);
    assert_ne!(task.status.state, TaskState::Failed);
    assert_eq!(
        task.artifacts.len(),
        1,
        "a real exit-0 run must carry its captured verification artifact"
    );
    assert_eq!(task.artifacts[0].name.as_deref(), Some("verification"));

    // --- witnessed proof + chain verification -------------------------------------------
    let ledger = ProofLedger::new(tmp.path().to_path_buf());
    let records = ledger.read_all().expect("read the ledger");
    assert_eq!(records.len(), 1, "exactly one ProofRecord witnessed");
    assert_eq!(records[0].status, ProofStatus::Completed);
    assert_eq!(records[0].task_id, WO_ID);
    assert_eq!(records[0].commands_run, vec!["true".to_string()]);
    assert!(
        records[0].failure_reason.is_none(),
        "a completed proof carries no failure_reason"
    );
    assert_eq!(
        ledger.verify_witness_chain().expect("verify witness chain"),
        1,
        "the blake3 witness chain must re-verify to exactly one record"
    );
}

/// **Assertion 2 — GATE REFUSAL.**
///
/// git-kb reports the task ready, but `hf claim` exits 1 (the lease is held by another
/// peer). `select_and_claim` must surface [`SelectionError::ClaimConflict`] — never a
/// silent success — so the WorkOrder is NEVER run. We prove non-execution by pointing an
/// engine at a fresh ledger, never calling `run` in the refused branch, and asserting the
/// ledger stays empty. You cannot run a task you do not own.
#[tokio::test]
async fn gate_refusal_claim_conflict_never_runs_the_workorder() {
    let boundary = FakeBoundary {
        gitkb: out(&ready_json(), "", Some(0)),
        hf: out(
            "",
            "hf claim: TEASTASK-007 BLOCKED — conflict: held by other peer",
            Some(1),
        ),
    };
    let selector = Selector::new(boundary).with_holder(HOLDER);

    // A ledger the engine WOULD write to — if it ever ran. It must not.
    let tmp = tempfile::NamedTempFile::new().expect("temp ledger file");
    let engine = RvAgentEngine::with_ledger(tmp.path().to_path_buf());

    match selector.select_and_claim() {
        Err(SelectionError::ClaimConflict { slug, code, stderr }) => {
            assert_eq!(
                slug, READY_SLUG,
                "the conflict names the task we could not claim"
            );
            assert_eq!(code, Some(1));
            assert!(stderr.contains("conflict"), "conflict surfaced: {stderr}");
            // Refused: the WorkOrder is NOT ours, so `engine.run` is deliberately never
            // invoked here. `engine` is intentionally left unused in this branch.
            let _ = &engine;
        }
        other => {
            panic!("expected ClaimConflict; a refused lease must not read as success: {other:?}")
        }
    }

    // No claim => no run => no proof. The ledger was never written.
    let ledger = ProofLedger::new(tmp.path().to_path_buf());
    assert!(
        ledger.read_all().expect("read the ledger").is_empty(),
        "a task we never claimed must never produce a ProofRecord"
    );
    assert_eq!(
        ledger.verify_witness_chain().expect("verify empty chain"),
        0,
        "an empty ledger verifies to zero records"
    );
}

/// **Assertion 3 — NO PAPER COMPLETION.**
///
/// The claim succeeds, but the WorkOrder's `verification_command` is `false` (exit 1).
/// A real run through the engine must yield [`TaskState::Failed`] — never
/// [`TaskState::Completed`] — and witness a [`ProofStatus::Failed`] ProofRecord carrying
/// a `failure_reason`. A failing proof can never be laundered into a completion.
#[tokio::test]
async fn no_paper_completion_failing_verification_is_failed_not_completed() {
    // Selection succeeds (exit 0) to show the lease step is orthogonal to the verdict.
    let boundary = FakeBoundary {
        gitkb: out(&ready_json(), "", Some(0)),
        hf: out("hf claim: acquired lease", "", Some(0)),
    };
    let selector = Selector::new(boundary).with_holder(HOLDER);
    let claimed = selector
        .select_and_claim()
        .expect("claim acquires the lease");
    assert_eq!(claimed.slug, READY_SLUG);

    let tmp = tempfile::NamedTempFile::new().expect("temp ledger file");
    let engine = RvAgentEngine::with_ledger(tmp.path().to_path_buf());
    let spec = workorder_to_taskspec(&work_order("false")).expect("adapt WorkOrder -> TaskSpec");

    let task = engine
        .run(spec)
        .await
        .expect("engine.run returns a verdict");

    // A failing verification is Failed, and NEVER Completed.
    assert_eq!(task.status.state, TaskState::Failed);
    assert_ne!(
        task.status.state,
        TaskState::Completed,
        "a failing verification must never read as a completion (no paper completion)"
    );

    let ledger = ProofLedger::new(tmp.path().to_path_buf());
    let records = ledger.read_all().expect("read the ledger");
    assert_eq!(records.len(), 1, "the failed run is still witnessed once");
    assert_eq!(
        records[0].status,
        ProofStatus::Failed,
        "the proof is Failed, not Completed"
    );
    assert_ne!(records[0].status, ProofStatus::Completed);
    assert!(
        records[0].failure_reason.is_some(),
        "a failed run's proof must carry a failure_reason"
    );
    assert_eq!(
        ledger.verify_witness_chain().expect("verify witness chain"),
        1,
        "even a failed run's proof joins the witness chain"
    );
}
