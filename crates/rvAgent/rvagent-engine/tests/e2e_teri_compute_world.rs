//! DOGFOOD — drive the **real** teri ComputeWorld build through the unified TEAS loop.
//!
//! Unlike `e2e_one_workorder.rs` (which verifies `true`/`false`), this runs an actual
//! cross-repo build: it points one `WorkOrder` at the local teri worktree and executes
//! its build+test+clippy gate through the whole engine, end to end:
//!
//! ```text
//! WorkOrder (teri gate) -> WorkOrderStore (SQLite task_graph, status Active)
//!   -> workorder_to_taskspec (S3 adapter)
//!   -> RvAgentEngine::run (REAL cargo build/test/clippy via LocalShellBackend)
//!   -> witnessed ProofRecord (blake3 chain) + learning trajectory (reward 1.0)
//!   -> store.upsert(status = Done)
//! ```
//!
//! ## Why `#[ignore]`
//! It is deliberately path-coupled to `/home/flexnetos/lifeos/worktrees/teri--compute-world`
//! and runs a real (warm) cargo build, so it must never run in CI. Invoke explicitly:
//! ```text
//! cargo test -p rvagent-engine --test e2e_teri_compute_world -- --ignored --nocapture
//! ```
//!
//! ## What the dogfood surfaces about the automation flow (real seams)
//! 1. The hardened `LocalShellBackend` does `env_clear()` and passes only `SAFE_ENV_VARS`
//!    (PATH, HOME, …) — it DROPS `LD_LIBRARY_PATH`. A nix-toolchain build therefore only
//!    runs if the WorkOrder's `verification_command` re-establishes its own toolchain env.
//!    That is exactly what [`gate_command`] does; this test is the proof it is necessary.
//! 2. `RvAgentEngine::run` executes with the backend's **default 30s timeout** (it calls
//!    `execute(cmd, None)`), so the gate must be a *warm* incremental build, not cold.

use std::sync::Arc;

use rvagent_a2a::executor::TaskRunner;
use rvagent_a2a::types::TaskState;
use rvagent_engine::{
    workorder_to_taskspec, Priority, ProofLedger, ProofStatus, RvAgentEngine, Status,
    TrajectoryRecorder, WorkOrder, WorkOrderStore,
};

/// The local teri worktree that hosts `src/sim/compute_world.rs`.
const TERI_WORKTREE: &str = "/home/flexnetos/lifeos/worktrees/teri--compute-world";

/// The verification gate for ComputeWorld. It re-establishes the nix toolchain env inside
/// the sanitized shell (seam #1 above): rust-mixed on PATH (cargo + clippy), zlib/zstd on
/// LD_LIBRARY_PATH (so the teri test binary links), and the real HOME (so cargo finds its
/// registry cache). Then it runs the ComputeWorld unit tests and the `-D warnings` clippy
/// gate — the same two commands proven by hand.
fn gate_command() -> String {
    [
        "export PATH=/nix/store/62iid59rpbhpgkm0882yxa3b2rpl1fci-rust-mixed/bin:$PATH:/usr/bin:/bin",
        "export LD_LIBRARY_PATH=/nix/store/dbz6pb9g67kpgpl95k8d85kzpxm1c32p-zlib-1.3.2/lib:/nix/store/fsvb5zrsm1n7m5wshm570imspffi7i8f-zstd-1.5.7/lib",
        "export HOME=/home/flexnetos",
        "cargo test -p teri --lib sim::compute_world",
        "cargo clippy -p teri --lib -- -D warnings",
    ]
    .join(" && ")
}

fn teri_work_order() -> WorkOrder {
    WorkOrder {
        schema: "handoff.task.v1".to_string(),
        id: "TERI-COMPUTE-WORLD".to_string(),
        title: "teri ComputeWorld gate".to_string(),
        objective: "build+test+clippy the teri ComputeWorld execution-effect twin".to_string(),
        status: Status::Active,
        priority: Priority::P1,
        // choose_cwd runs the gate here (first existing path_scope dir).
        path_scope: vec![TERI_WORKTREE.to_string()],
        acceptance_criteria: vec![
            "compute_world unit tests pass".to_string(),
            "clippy -D warnings clean".to_string(),
        ],
        correlation_id: None,
        owner_lane: None,
        role: None,
        dependencies: Vec::new(),
        blocked_by: Vec::new(),
        allows_network: false,
        allows_dependency_addition: false,
        human_approval_required: false,
        verification_command: Some(gate_command()),
        rollback_plan: None,
        intent_lock: None,
    }
}

#[tokio::test]
#[ignore = "path-coupled to a local teri worktree; runs a real cargo build — run with --ignored"]
async fn teri_compute_world_flows_through_teas_end_to_end() {
    assert!(
        std::path::Path::new(TERI_WORKTREE).is_dir(),
        "teri worktree {TERI_WORKTREE} must exist for the dogfood run"
    );

    let wo = teri_work_order();

    // 1. Persist to the TEAS task DATABASE (SQLite tables), status Active.
    let dbdir = tempfile::tempdir().expect("temp db dir");
    let store = WorkOrderStore::new(dbdir.path().join("teas.db"));
    store.upsert(&wo).expect("persist WorkOrder to task_graph");
    assert_eq!(store.count().expect("count"), 1);
    assert_eq!(
        store.list_by_status(Status::Active).expect("active").len(),
        1,
        "the WorkOrder is Active in the task DB before execution"
    );

    // 2. Engine with the witnessed proof ledger + the learning-trajectory recorder.
    let ledger_file = tempfile::NamedTempFile::new().expect("temp ledger");
    let recorder = Arc::new(TrajectoryRecorder::new());
    let engine = RvAgentEngine::with_ledger(ledger_file.path().to_path_buf())
        .with_recorder(recorder.clone());

    // 3. S3 adapter -> REAL execution through the hardened backend.
    let spec = workorder_to_taskspec(&wo).expect("adapt WorkOrder -> TaskSpec");
    let task = engine.run(spec).await.expect("engine.run must return a verdict");

    // 4. Verdict: proof-backed Completed, never paper.
    assert_eq!(
        task.status.state,
        TaskState::Completed,
        "the teri gate must pass through TEAS; verdict message: {:?}",
        task.status.message
    );
    assert_eq!(
        task.artifacts.len(),
        1,
        "a real exit-0 run carries its captured build/test/clippy output"
    );

    // 5. Witnessed proof + blake3 chain re-verifies.
    let ledger = ProofLedger::new(ledger_file.path().to_path_buf());
    let records = ledger.read_all().expect("read the ledger");
    assert_eq!(records.len(), 1, "exactly one ProofRecord witnessed");
    assert_eq!(records[0].status, ProofStatus::Completed);
    assert_eq!(records[0].task_id, "TERI-COMPUTE-WORLD");
    assert!(records[0].failure_reason.is_none());
    assert_eq!(ledger.verify_witness_chain().expect("verify"), 1);

    // 6. Learning trajectory captured for the real Completed outcome (reward 1.0).
    assert_eq!(recorder.trajectory_count(), 1);
    assert_eq!(recorder.recorded()[0].reward, 1.0);

    // 7. Close the loop: mark the WorkOrder Done in the task DB.
    let mut done = wo;
    done.status = Status::Done;
    store.upsert(&done).expect("mark Done");
    assert_eq!(store.count().expect("count still one"), 1, "same id updates, no dupe");
    assert_eq!(store.list_by_status(Status::Done).expect("done").len(), 1);

    eprintln!(
        "\n=== TEAS witnessed proof for TERI-COMPUTE-WORLD ===\n{}",
        serde_json::to_string_pretty(&records[0]).expect("pretty proof")
    );
}
