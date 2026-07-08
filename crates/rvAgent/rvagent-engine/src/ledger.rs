//! Self-contained append-only JSONL proof ledger with a blake3 witness chain
//! (TEASTASK-004).
//!
//! ## Architectural heal — no repo cycle
//! The canonical proof authority is handoff's witnessed **redb** ledger. But
//! `rvagent-engine` must **not** depend on `handoff` (that edge would close a
//! dependency cycle: handoff already reaches into the TEAS engine surface). So the
//! engine writes its proofs to a *local, self-contained* JSONL ledger here, carrying
//! its own blake3 witness chain (`action_hash` links each record to its predecessor).
//! Ingestion of these records into handoff's redb ledger is **handoff-side and
//! deferred to seam S5** — out of scope for this crate. The JSONL chain is designed so
//! that S5 can verify the same `action_hash`/`prev_action_hash` chain it reads here.
//!
//! ## Witness-chain hashing recipe (append and verify agree byte-for-byte)
//! For a record whose predecessor's `action_hash` is `prev` (`""` for the genesis
//! record), the witness hash is computed over **content only**:
//! 1. clone the record and zero its ledger-managed fields:
//!    `action_hash = String::new()`, `prev_action_hash = None`, `ledger_seq = None`;
//! 2. `content = serde_json::to_vec(&clone)` (the canonical serde JSON bytes);
//! 3. `action_hash = blake3::hash([prev.as_bytes(), content].concat()).to_hex()`.
//!
//! Both [`ProofLedger::append`] and [`ProofLedger::verify_witness_chain`] call the
//! same [`content_hash`] helper, so the chain a writer produces always re-verifies.

use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;

use crate::proof::ProofRecord;

/// Errors from ledger I/O, (de)serialization, or witness-chain verification.
#[derive(Debug, thiserror::Error)]
pub enum LedgerError {
    /// Filesystem error reading or appending to the ledger file.
    #[error("ledger I/O error: {0}")]
    Io(#[from] std::io::Error),
    /// A ledger line failed to (de)serialize as a `ProofRecord`.
    #[error("ledger JSON error: {0}")]
    Json(#[from] serde_json::Error),
    /// The witness chain does not check out (tamper detected).
    #[error("witness chain broken: {0}")]
    ChainBroken(String),
}

/// An append-only JSONL proof ledger with a blake3 witness chain.
#[derive(Debug, Clone)]
pub struct ProofLedger {
    path: PathBuf,
}

/// Compute the content-only witness hash for `record` chained onto `prev`.
///
/// This is the single source of truth for the hashing recipe; both [`ProofLedger::append`]
/// and [`ProofLedger::verify_witness_chain`] use it so append and verify agree exactly.
fn content_hash(prev: &str, record: &ProofRecord) -> Result<String, LedgerError> {
    let mut clone = record.clone();
    clone.action_hash = String::new();
    clone.prev_action_hash = None;
    clone.ledger_seq = None;
    let content = serde_json::to_vec(&clone)?;
    let mut buf = Vec::with_capacity(prev.len() + content.len());
    buf.extend_from_slice(prev.as_bytes());
    buf.extend_from_slice(&content);
    Ok(blake3::hash(&buf).to_hex().to_string())
}

impl ProofLedger {
    /// Construct a ledger backed by the JSONL file at `path` (created on first append).
    #[must_use]
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    /// The backing file path.
    #[must_use]
    pub fn path(&self) -> &PathBuf {
        &self.path
    }

    /// Append `record` to the ledger, finalizing its witness-chain fields.
    ///
    /// Reads the last line's `action_hash` as `prev` (`""` if the file is empty or
    /// absent), computes the content-only witness hash, sets `prev_action_hash`
    /// (`None` for the genesis record), `ledger_seq` (contiguous from 0), and
    /// `action_hash`, then appends the record as one JSON line. Returns the finalized
    /// record.
    pub fn append(&self, record: ProofRecord) -> Result<ProofRecord, LedgerError> {
        let existing = self.read_all()?;
        let prev = existing
            .last()
            .map(|r| r.action_hash.clone())
            .unwrap_or_default();
        let next_seq = existing.len() as u64;

        let action_hash = content_hash(&prev, &record)?;

        let mut finalized = record;
        finalized.prev_action_hash = if existing.is_empty() {
            None
        } else {
            Some(prev)
        };
        finalized.ledger_seq = Some(next_seq);
        finalized.action_hash = action_hash;

        let line = format!("{}\n", serde_json::to_string(&finalized)?);
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;
        file.write_all(line.as_bytes())?;
        file.flush()?;

        Ok(finalized)
    }

    /// Read and parse every record in the ledger. Returns an empty vec if absent.
    pub fn read_all(&self) -> Result<Vec<ProofRecord>, LedgerError> {
        let raw = match std::fs::read_to_string(&self.path) {
            Ok(s) => s,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(e) => return Err(LedgerError::Io(e)),
        };
        let mut records = Vec::new();
        for line in raw.lines() {
            if line.trim().is_empty() {
                continue;
            }
            records.push(serde_json::from_str::<ProofRecord>(line)?);
        }
        Ok(records)
    }

    /// Re-walk the ledger and verify the witness chain end to end.
    ///
    /// For each record: recompute the content-only witness hash from its stored
    /// `prev_action_hash` (`""` when `None`) and assert it equals the stored
    /// `action_hash`; assert `record[i].prev_action_hash == record[i-1].action_hash`;
    /// and assert `ledger_seq` is contiguous from 0. Returns the verified count, or a
    /// [`LedgerError::ChainBroken`] on the first mismatch.
    pub fn verify_witness_chain(&self) -> Result<usize, LedgerError> {
        let records = self.read_all()?;
        let mut prev_hash = String::new();
        for (i, record) in records.iter().enumerate() {
            // 1. ledger_seq is contiguous from 0.
            if record.ledger_seq != Some(i as u64) {
                return Err(LedgerError::ChainBroken(format!(
                    "record {i}: ledger_seq {:?} is not contiguous (expected {i})",
                    record.ledger_seq
                )));
            }

            // 2. prev_action_hash links to the previous record's action_hash.
            let expected_prev = if i == 0 { None } else { Some(prev_hash.clone()) };
            if record.prev_action_hash != expected_prev {
                return Err(LedgerError::ChainBroken(format!(
                    "record {i}: prev_action_hash {:?} does not link to predecessor {expected_prev:?}",
                    record.prev_action_hash
                )));
            }

            // 3. action_hash matches the content-only witness recompute.
            let prev = record.prev_action_hash.clone().unwrap_or_default();
            let recomputed = content_hash(&prev, record)?;
            if recomputed != record.action_hash {
                return Err(LedgerError::ChainBroken(format!(
                    "record {i}: action_hash mismatch (stored {}, recomputed {recomputed}) — content tampered",
                    record.action_hash
                )));
            }

            prev_hash = record.action_hash.clone();
        }
        Ok(records.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::proof::{ProofStatus, PROOF_SCHEMA_VERSION};
    use std::collections::BTreeMap;

    fn sample_record(task_id: &str) -> ProofRecord {
        ProofRecord {
            proof_schema_version: PROOF_SCHEMA_VERSION.to_string(),
            task_id: task_id.to_string(),
            correlation_id: None,
            cell_id: None,
            status: ProofStatus::Completed,
            started_at: "2026-07-07T00:00:00Z".to_string(),
            completed_at: "2026-07-07T00:00:01Z".to_string(),
            actor: "rvagent-engine".to_string(),
            helper_id: None,
            model_tag: None,
            repo_path: None,
            git_head_before: None,
            git_head_after: None,
            diff_summary: None,
            files_changed: Vec::new(),
            commands_run: vec!["true".to_string()],
            verification_output: serde_json::Value::String("ok".to_string()),
            checksums: BTreeMap::new(),
            action_hash: String::new(),
            prev_action_hash: None,
            ledger_seq: None,
            logs_uri: None,
            rollback_point: None,
            evidence: Vec::new(),
            failed_checks: Vec::new(),
            failure_reason: None,
            next_action: "select-next".to_string(),
        }
    }

    #[test]
    fn append_two_records_chains_and_verifies() {
        let tmp = tempfile::NamedTempFile::new().expect("temp file");
        let ledger = ProofLedger::new(tmp.path().to_path_buf());

        let r1 = ledger.append(sample_record("TASK-1")).expect("append 1");
        let r2 = ledger.append(sample_record("TASK-2")).expect("append 2");

        assert_eq!(r1.ledger_seq, Some(0));
        assert_eq!(r2.ledger_seq, Some(1));
        assert_eq!(r1.prev_action_hash, None, "genesis has no predecessor");
        assert_eq!(
            r2.prev_action_hash,
            Some(r1.action_hash.clone()),
            "record 2 must link to record 1's action_hash"
        );
        assert!(!r1.action_hash.is_empty());
        assert_ne!(r1.action_hash, r2.action_hash);

        assert_eq!(ledger.verify_witness_chain().expect("verify"), 2);
    }

    #[test]
    fn read_all_returns_appended_records() {
        let tmp = tempfile::NamedTempFile::new().expect("temp file");
        let ledger = ProofLedger::new(tmp.path().to_path_buf());
        ledger.append(sample_record("TASK-1")).expect("append 1");
        ledger.append(sample_record("TASK-2")).expect("append 2");

        let all = ledger.read_all().expect("read_all");
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].task_id, "TASK-1");
        assert_eq!(all[1].task_id, "TASK-2");
    }

    #[test]
    fn read_all_on_absent_file_is_empty() {
        let dir = tempfile::tempdir().expect("temp dir");
        let ledger = ProofLedger::new(dir.path().join("does-not-exist.jsonl"));
        assert!(ledger.read_all().expect("read_all").is_empty());
        assert_eq!(ledger.verify_witness_chain().expect("verify empty"), 0);
    }

    #[test]
    fn tampering_a_field_breaks_the_chain() {
        let tmp = tempfile::NamedTempFile::new().expect("temp file");
        let ledger = ProofLedger::new(tmp.path().to_path_buf());
        ledger.append(sample_record("TASK-1")).expect("append 1");
        ledger.append(sample_record("TASK-2")).expect("append 2");

        // Rewrite the file, mutating a content field of the first record's line
        // while leaving its stored action_hash untouched.
        let raw = std::fs::read_to_string(tmp.path()).expect("read");
        let mut lines: Vec<String> = raw.lines().map(str::to_string).collect();
        let mut first: ProofRecord = serde_json::from_str(&lines[0]).expect("parse first");
        first.task_id = "TAMPERED".to_string();
        lines[0] = serde_json::to_string(&first).expect("reserialize");
        std::fs::write(tmp.path(), format!("{}\n", lines.join("\n"))).expect("write");

        match ledger.verify_witness_chain() {
            Err(LedgerError::ChainBroken(_)) => {}
            other => panic!("tamper must break the chain, got {other:?}"),
        }
    }
}
