//! ARCHBP-023 — TEAS -> GitKB single-authority projection bridge.
//!
//! TEAS is the SOLE task authority. The GitKB board/graph/ready/assign/context/
//! incident views are a deterministic ONE-DIRECTIONAL projection of the TEAS
//! event log — there is no GitKB -> TEAS write-back, so there is no dual
//! authority and no write-back seam. Replaying the same events yields
//! byte-identical views (GitKB agrees with TEAS after replay). Contradictions
//! (completing without a proof reference, invalidated proofs, explicit
//! conflicts) FAIL CLOSED: the task is never silently marked complete and the
//! contradiction remains visible in the incident view.

#![forbid(unsafe_code)]

use std::collections::BTreeMap;

pub const SCHEMA_VERSION: &str = "teas-gitkb-bridge.v0";
/// The single, explicit task authority.
pub const AUTHORITY: &str = "TEAS";

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EventKind {
    Create,
    Assign,
    Run,
    Block,
    Complete,
    ProofInvalidate,
    Retry,
    Conflict,
    Restart,
}

#[derive(Clone, Debug)]
pub struct TeasEvent {
    pub seq: u64,
    pub task_id: String,
    pub kind: EventKind,
    pub actor: String,
    pub body: Option<String>,
    pub assignee: Option<String>,
    pub proof_ref: Option<String>,
    pub deps: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Incident {
    pub task_id: String,
    pub reason: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GitKbViews {
    pub board: BTreeMap<String, String>,
    pub ready: Vec<String>,
    pub assign: BTreeMap<String, String>,
    pub context: BTreeMap<String, String>,
    pub graph: Vec<(String, String)>,
    pub incident: Vec<Incident>,
}

/// Whether a GitKB -> TEAS write-back path exists (dual authority). Always
/// false — the bridge only projects TEAS forward into GitKB views.
pub fn has_writeback() -> bool {
    false
}

#[derive(Clone, Default)]
struct TaskState {
    status: String,
    body: Option<String>,
    assignee: Option<String>,
    proof_ref: Option<String>,
}

/// Deterministically project the TEAS event log into the GitKB views.
pub fn project(events: &[TeasEvent]) -> GitKbViews {
    let mut ordered: Vec<&TeasEvent> = events.iter().collect();
    ordered.sort_by_key(|e| e.seq);

    let mut states: BTreeMap<String, TaskState> = BTreeMap::new();
    let mut order: Vec<String> = Vec::new();
    let mut graph: Vec<(String, String)> = Vec::new();
    let mut incidents: Vec<Incident> = Vec::new();

    for e in ordered {
        if !states.contains_key(&e.task_id) {
            states.insert(e.task_id.clone(), TaskState::default());
            order.push(e.task_id.clone());
        }
        let st = states.get_mut(&e.task_id).unwrap();
        let incident = |incidents: &mut Vec<Incident>, task: &str, reason: &str| {
            incidents.push(Incident { task_id: task.to_string(), reason: reason.to_string() });
        };
        match e.kind {
            EventKind::Create => {
                st.status = "created".to_string();
                if let Some(b) = &e.body {
                    st.body = Some(b.clone());
                }
                for dep in &e.deps {
                    graph.push((dep.clone(), e.task_id.clone()));
                }
            }
            EventKind::Assign => {
                if let Some(a) = &e.assignee {
                    st.assignee = Some(a.clone());
                }
                st.status = "assigned".to_string();
            }
            EventKind::Run => st.status = "running".to_string(),
            EventKind::Block => st.status = "blocked".to_string(),
            EventKind::Complete => {
                if e.proof_ref.is_none() {
                    incident(&mut incidents, &e.task_id, "complete without proof reference — failed closed");
                } else if st.status == "running" || st.status == "assigned" {
                    st.status = "complete".to_string();
                    st.proof_ref = e.proof_ref.clone();
                } else {
                    incident(
                        &mut incidents,
                        &e.task_id,
                        &format!("complete from invalid state '{}' — failed closed", st.status),
                    );
                }
            }
            EventKind::ProofInvalidate => {
                if st.status == "complete" {
                    st.status = "proof-invalidated".to_string();
                    st.proof_ref = None;
                    incident(&mut incidents, &e.task_id, "proof invalidated — completion revoked");
                } else {
                    incident(&mut incidents, &e.task_id, "proof invalidate on a non-complete task");
                }
            }
            EventKind::Retry => {
                if st.status == "blocked" || st.status == "proof-invalidated" || st.status == "restarted" {
                    st.status = "running".to_string();
                } else {
                    incident(
                        &mut incidents,
                        &e.task_id,
                        &format!("retry from non-retryable state '{}'", st.status),
                    );
                }
            }
            EventKind::Conflict => {
                incident(&mut incidents, &e.task_id, "explicit conflict — failed closed, status unchanged");
            }
            EventKind::Restart => st.status = "restarted".to_string(),
        }
    }

    let mut board = BTreeMap::new();
    let mut assign = BTreeMap::new();
    let mut context = BTreeMap::new();
    let mut ready = Vec::new();
    for task in &order {
        let st = &states[task];
        board.insert(task.clone(), st.status.clone());
        if let Some(a) = &st.assignee {
            assign.insert(task.clone(), a.clone());
        }
        if let Some(b) = &st.body {
            context.insert(task.clone(), b.clone());
        }
        if st.status == "assigned" {
            ready.push(task.clone());
        }
    }
    ready.sort();

    GitKbViews { board, ready, assign, context, graph, incident: incidents }
}
