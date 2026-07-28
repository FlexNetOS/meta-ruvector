//! The rvagent-engine-local `WorkOrder` type — the Rust form of the canonical
//! `handoff.task.v1` task contract (`teas/schemas/task_graph.schema.json`).
//!
//! This is the engine's OWN type, by design (TEAS DOMAIN_MODEL §9.6): rvagent-engine
//! cannot depend on handoff's origin `work-order` struct without a repository cycle
//! (handoff already depends on meta-ruvector's `ruvector-verified`/`rvf-*`/`cognitum`
//! crates), so the two `WorkOrder` types coexist and the [`crate::adapter`] bridges
//! them. TEASTASK-003 regenerates this type's JSON Schema via `schemars` and adds a
//! drift gate proving it stays byte-equal to the canonical schema.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Task lifecycle state. Serde form is lowercase — matches handoff's `Status`
/// (`work-order/src/lib.rs:20`) and the canonical schema `status` enum.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Backlog,
    Active,
    Claimed,
    Blocked,
    Checkpointed,
    Review,
    Done,
}

/// Task priority. Serde form is `P0`..`P3` (matches the canonical schema).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum Priority {
    P0,
    P1,
    P2,
    P3,
}

/// Value object: the five blake3 hashes that pin the immutable contract surface.
/// Each is serialized in `blake3:<hex>` form (canonical schema); kept as opaque
/// strings so the round-trip is byte-exact and the drift sentinel is preserved.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct IntentLock {
    pub objective_hash: String,
    pub path_scope_hash: String,
    pub acceptance_hash: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub constraint_hash: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub northstar_revision: Option<String>,
}

/// The canonical unit-of-work aggregate (`handoff.task.v1`).
///
/// A faithful Rust projection of the canonical schema's load-bearing fields —
/// enough for the S3 adapter round-trip and IntentLock preservation. TEASTASK-003
/// completes the remaining schema fields and adds the schemars drift gate.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct WorkOrder {
    /// Schema tag; canonically the const `"handoff.task.v1"`.
    pub schema: String,
    pub id: String,
    pub title: String,
    pub objective: String,
    pub status: Status,
    pub priority: Priority,
    #[serde(default)]
    pub path_scope: Vec<String>,
    #[serde(default)]
    pub acceptance_criteria: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub correlation_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner_lane: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    #[serde(default)]
    pub dependencies: Vec<String>,
    #[serde(default)]
    pub blocked_by: Vec<String>,
    #[serde(default)]
    pub allows_network: bool,
    #[serde(default)]
    pub allows_dependency_addition: bool,
    #[serde(default)]
    pub human_approval_required: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verification_command: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rollback_plan: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub intent_lock: Option<IntentLock>,
}
