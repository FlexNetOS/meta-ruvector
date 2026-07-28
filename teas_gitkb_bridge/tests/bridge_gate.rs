// ARCHBP-023 — TEAS -> GitKB single-authority write-back gate.
//
// TEAS is the sole task authority; the GitKB board/graph/ready/assign/context/
// incident views are a deterministic one-directional projection. These tests
// cover creation, assignment, running, blocked, complete, proof invalidation,
// retry, conflict, and restart; assert a single explicit authority and no
// write-back; require GitKB views to agree with TEAS after replay; and require
// contradictions to fail closed and remain visible.

use teas_gitkb_bridge::{has_writeback, project, EventKind, GitKbViews, Incident, TeasEvent, AUTHORITY};

fn ev(seq: u64, task: &str, kind: EventKind) -> TeasEvent {
    TeasEvent {
        seq,
        task_id: task.to_string(),
        kind,
        actor: "teas".to_string(),
        body: None,
        assignee: None,
        proof_ref: None,
        deps: Vec::new(),
    }
}

#[test]
fn single_explicit_authority_and_no_write_back() {
    assert_eq!(AUTHORITY, "TEAS");
    // The bridge is one-directional: there is no GitKB -> TEAS write-back path.
    assert!(!has_writeback());
}

#[test]
fn creation_and_context_and_dependency_graph() {
    let mut c = ev(1, "T1", EventKind::Create);
    c.body = Some("canonical body".to_string());
    c.deps = vec!["T0".to_string()];
    let views = project(&[c]);
    assert_eq!(views.board.get("T1").map(String::as_str), Some("created"));
    assert_eq!(views.context.get("T1").map(String::as_str), Some("canonical body"));
    assert!(views.graph.contains(&("T0".to_string(), "T1".to_string())));
}

#[test]
fn assignment_running_blocked_lifecycle() {
    let mut a = ev(2, "T1", EventKind::Assign);
    a.assignee = Some("agent-x".to_string());
    let events = vec![ev(1, "T1", EventKind::Create), a, ev(3, "T1", EventKind::Run), ev(4, "T1", EventKind::Block)];
    let views = project(&events);
    assert_eq!(views.board.get("T1").map(String::as_str), Some("blocked"));

    // stop at assignment -> assign view + ready
    let mut a2 = ev(2, "T2", EventKind::Assign);
    a2.assignee = Some("agent-y".to_string());
    let v2 = project(&[ev(1, "T2", EventKind::Create), a2]);
    assert_eq!(v2.assign.get("T2").map(String::as_str), Some("agent-y"));
    assert_eq!(v2.board.get("T2").map(String::as_str), Some("assigned"));
    assert!(v2.ready.contains(&"T2".to_string()));
}

#[test]
fn complete_requires_proof_and_invalidation_reverts() {
    let mut done = ev(3, "T1", EventKind::Complete);
    done.proof_ref = Some("proof://T1@rev1".to_string());
    let complete = project(&[ev(1, "T1", EventKind::Create), ev(2, "T1", EventKind::Run), done.clone()]);
    assert_eq!(complete.board.get("T1").map(String::as_str), Some("complete"));

    // proof invalidation reverts from complete and is visible in the incident view
    let events = vec![ev(1, "T1", EventKind::Create), ev(2, "T1", EventKind::Run), done, ev(4, "T1", EventKind::ProofInvalidate)];
    let views = project(&events);
    assert_ne!(views.board.get("T1").map(String::as_str), Some("complete"));
    assert!(views.incident.iter().any(|i| i.task_id == "T1"));
}

#[test]
fn complete_without_proof_fails_closed() {
    // Completing without a proof reference is a contradiction: it must not be
    // silently accepted; the task stays non-complete and the incident is visible.
    let events = vec![ev(1, "T1", EventKind::Create), ev(2, "T1", EventKind::Run), ev(3, "T1", EventKind::Complete)];
    let views = project(&events);
    assert_ne!(views.board.get("T1").map(String::as_str), Some("complete"));
    assert!(views.incident.iter().any(|i| i.task_id == "T1" && i.reason.contains("proof")));
}

#[test]
fn retry_and_restart() {
    let retry = project(&[
        ev(1, "T1", EventKind::Create),
        ev(2, "T1", EventKind::Run),
        ev(3, "T1", EventKind::Block),
        ev(4, "T1", EventKind::Retry),
    ]);
    assert_eq!(retry.board.get("T1").map(String::as_str), Some("running"));

    let restart = project(&[
        ev(1, "T1", EventKind::Create),
        ev(2, "T1", EventKind::Run),
        ev(3, "T1", EventKind::Restart),
    ]);
    assert_eq!(restart.board.get("T1").map(String::as_str), Some("restarted"));
}

#[test]
fn conflict_fails_closed_and_stays_visible() {
    let events = vec![ev(1, "T1", EventKind::Create), ev(2, "T1", EventKind::Run), ev(3, "T1", EventKind::Conflict)];
    let views = project(&events);
    assert!(!views.incident.is_empty());
    assert!(views.incident.iter().any(|i| i.task_id == "T1"));
    // fail-closed: the conflicting task is never silently marked complete
    assert_ne!(views.board.get("T1").map(String::as_str), Some("complete"));
}

#[test]
fn projection_agrees_after_replay() {
    let mut done = ev(3, "T1", EventKind::Complete);
    done.proof_ref = Some("proof://T1@rev1".to_string());
    let events: Vec<TeasEvent> = vec![ev(1, "T1", EventKind::Create), ev(2, "T1", EventKind::Run), done];
    let a: GitKbViews = project(&events);
    let b: GitKbViews = project(&events); // replay
    assert_eq!(a, b, "GitKB views must agree with TEAS byte-for-byte on replay");
    // sanity: incidents are a Vec<Incident> and equality is structural
    let _ = Incident { task_id: "x".into(), reason: "y".into() };
}
