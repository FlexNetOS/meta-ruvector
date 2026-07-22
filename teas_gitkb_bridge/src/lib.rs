//! ARCHBP-023 — RED STUB. Contract surface only; the TEAS -> GitKB projection
//! is unimplemented so the write-back-seam gate fails closed before the real
//! bridge lands.

#![forbid(unsafe_code)]

use std::collections::BTreeMap;

pub const SCHEMA_VERSION: &str = "teas-gitkb-bridge.v0";
pub const AUTHORITY: &str = "unimplemented";

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

/// Whether a GitKB -> TEAS write-back path exists (dual authority). Always false.
pub fn has_writeback() -> bool {
    // RED: unimplemented — no bridge yet.
    true
}

pub fn project(_events: &[TeasEvent]) -> GitKbViews {
    // RED: unimplemented.
    GitKbViews::default()
}
