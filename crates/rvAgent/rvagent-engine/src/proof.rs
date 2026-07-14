//! The rvagent-engine-local `ProofRecord` type — the Rust form of the canonical
//! `teas.proof_record.v1` schema (`teas/schemas/proof_record.schema.json`).
//!
//! The reality plane of TEAS: a `ProofRecord` is evidence-with-checksum that a
//! WorkOrder ran and passed. It is written to the handoff witnessed redb ledger in
//! TEASTASK-004; proof status overrides any card/CSV status (ledger authority).

use std::collections::BTreeMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Canonical schema tag for a v1 ProofRecord.
pub const PROOF_SCHEMA_VERSION: &str = "teas.proof_record.v1";

/// Terminal outcome of a run. Serde form matches the canonical schema `status` enum
/// (`completed`/`passed` = success; `failed`/`error` = failure; `rolled_back` = reverted).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ProofStatus {
    Completed,
    Passed,
    Failed,
    Error,
    RolledBack,
}

impl ProofStatus {
    /// True for the success outcomes (`completed` / `passed`).
    #[must_use]
    pub fn is_success(self) -> bool {
        matches!(self, ProofStatus::Completed | ProofStatus::Passed)
    }
}

/// Evidence-with-checksum that a WorkOrder ran and passed. Reconciles handoff's
/// witnessed `LedgerEvent`, the execution-framework `*.proof.json`, and the
/// planning-spine proof-record shapes into one versioned record (DOMAIN_MODEL §4).
///
/// No `Eq` — `verification_output` is a `serde_json::Value` (may contain floats).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ProofRecord {
    /// Canonically `"teas.proof_record.v1"` (see [`PROOF_SCHEMA_VERSION`]).
    pub proof_schema_version: String,
    pub task_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub correlation_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cell_id: Option<String>,
    pub status: ProofStatus,
    pub started_at: String,
    pub completed_at: String,
    pub actor: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub helper_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_tag: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repo_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub git_head_before: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub git_head_after: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diff_summary: Option<String>,
    #[serde(default)]
    pub files_changed: Vec<String>,
    pub commands_run: Vec<String>,
    /// Raw verification output (string) or a structured result object.
    pub verification_output: serde_json::Value,
    /// `path -> sha256`. `BTreeMap` for deterministic ordering.
    pub checksums: BTreeMap<String, String>,
    /// blake3 witness hash linking this record into the ledger witness chain.
    pub action_hash: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prev_action_hash: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ledger_seq: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub logs_uri: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rollback_point: Option<String>,
    #[serde(default)]
    pub evidence: Vec<String>,
    #[serde(default)]
    pub failed_checks: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failure_reason: Option<String>,
    pub next_action: String,
}
