//! Witnessed proof ledger backed by a **SQLite database** (TEASTASK-004).
//!
//! Durable TEAS state lives in tables, not flat files — consistent with gitkb (SQLite)
//! and the envctl execution-framework DB. Proof records are rows in a `proof_records`
//! table, each carrying a blake3 witness hash (`action_hash`) chained to its
//! predecessor. Appends run inside an `IMMEDIATE` transaction, so concurrent writers
//! are serialized by SQLite (no torn chain).
//!
//! ## Architectural heal — no repo cycle
//! The canonical proof authority is handoff's witnessed ledger, but `rvagent-engine`
//! must not depend on `handoff` (that edge would close a cycle). So the engine owns
//! this self-contained SQLite ledger; ingestion into handoff at seam S5 reads the same
//! `action_hash`/`prev_action_hash` chain.
//!
//! ## Witness-chain hashing recipe (append and verify agree byte-for-byte)
//! For a record whose predecessor's `action_hash` is `prev` (`""` for genesis), the
//! witness hash is computed over **content only**: clone the record, zero its
//! ledger-managed fields (`action_hash=""`, `prev_action_hash=None`, `ledger_seq=None`),
//! serialize to canonical serde JSON, and `blake3::hash(prev.bytes ++ content)`. Both
//! [`ProofLedger::append`] and [`ProofLedger::verify_witness_chain`] call [`content_hash`].

use std::path::PathBuf;

use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};

use crate::proof::ProofRecord;

/// Errors from ledger DB access, (de)serialization, or witness-chain verification.
#[derive(Debug, thiserror::Error)]
pub enum LedgerError {
    /// SQLite error opening, migrating, or querying the ledger database.
    #[error("ledger db error: {0}")]
    Db(#[from] rusqlite::Error),
    /// A stored record failed to (de)serialize as a `ProofRecord`.
    #[error("ledger JSON error: {0}")]
    Json(#[from] serde_json::Error),
    /// The witness chain does not check out (tamper detected).
    #[error("witness chain broken: {0}")]
    ChainBroken(String),
}

/// A witnessed proof ledger backed by a SQLite database file.
#[derive(Debug, Clone)]
pub struct ProofLedger {
    path: PathBuf,
}

/// Compute the content-only witness hash for `record` chained onto `prev`.
///
/// Single source of truth for the recipe; [`ProofLedger::append`] and
/// [`ProofLedger::verify_witness_chain`] both call it so they agree exactly.
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

/// Serialize a ProofStatus to its canonical lowercase token for the indexed column.
fn status_token(record: &ProofRecord) -> String {
    match serde_json::to_value(record.status) {
        Ok(serde_json::Value::String(s)) => s,
        _ => String::new(),
    }
}

impl ProofLedger {
    /// Construct a ledger backed by the SQLite database at `path` (created on first use).
    #[must_use]
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    /// The backing database file path.
    #[must_use]
    pub fn path(&self) -> &PathBuf {
        &self.path
    }

    /// Open a connection and ensure the schema exists.
    fn open(&self) -> Result<Connection, LedgerError> {
        let conn = Connection::open(&self.path)?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS proof_records (
                 ledger_seq       INTEGER PRIMARY KEY,
                 task_id          TEXT NOT NULL,
                 status           TEXT NOT NULL,
                 action_hash      TEXT NOT NULL,
                 prev_action_hash TEXT,
                 record_json      TEXT NOT NULL
             );
             CREATE INDEX IF NOT EXISTS idx_proof_records_task ON proof_records(task_id);",
        )?;
        Ok(conn)
    }

    /// Append `record`, finalizing its witness-chain fields, inside an IMMEDIATE
    /// transaction (concurrent writers are serialized by SQLite). Returns the
    /// finalized record.
    pub fn append(&self, record: ProofRecord) -> Result<ProofRecord, LedgerError> {
        let mut conn = self.open()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;

        let prev: String = tx
            .query_row(
                "SELECT action_hash FROM proof_records ORDER BY ledger_seq DESC LIMIT 1",
                [],
                |row| row.get::<_, String>(0),
            )
            .optional()?
            .unwrap_or_default();
        let next_seq: u64 =
            tx.query_row("SELECT COUNT(*) FROM proof_records", [], |row| row.get(0))?;

        let action_hash = content_hash(&prev, &record)?;
        let mut finalized = record;
        finalized.prev_action_hash = if next_seq == 0 { None } else { Some(prev) };
        finalized.ledger_seq = Some(next_seq);
        finalized.action_hash = action_hash;

        let json = serde_json::to_string(&finalized)?;
        tx.execute(
            "INSERT INTO proof_records
                 (ledger_seq, task_id, status, action_hash, prev_action_hash, record_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                finalized.ledger_seq,
                finalized.task_id,
                status_token(&finalized),
                finalized.action_hash,
                finalized.prev_action_hash,
                json,
            ],
        )?;
        tx.commit()?;
        Ok(finalized)
    }

    /// Read every record in ledger order. Returns an empty vec if the ledger is empty.
    pub fn read_all(&self) -> Result<Vec<ProofRecord>, LedgerError> {
        let conn = self.open()?;
        let mut stmt =
            conn.prepare("SELECT record_json FROM proof_records ORDER BY ledger_seq ASC")?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
        let mut records = Vec::new();
        for row in rows {
            records.push(serde_json::from_str::<ProofRecord>(&row?)?);
        }
        Ok(records)
    }

    /// Re-walk the ledger and verify the witness chain end to end.
    ///
    /// For each record: assert `ledger_seq` is contiguous from 0; assert
    /// `prev_action_hash` links to the predecessor's `action_hash`; and recompute the
    /// content-only witness hash and assert it equals the stored `action_hash`. Returns
    /// the verified count, or [`LedgerError::ChainBroken`] on the first mismatch.
    pub fn verify_witness_chain(&self) -> Result<usize, LedgerError> {
        let records = self.read_all()?;
        let mut prev_hash = String::new();
        for (i, record) in records.iter().enumerate() {
            if record.ledger_seq != Some(i as u64) {
                return Err(LedgerError::ChainBroken(format!(
                    "record {i}: ledger_seq {:?} is not contiguous (expected {i})",
                    record.ledger_seq
                )));
            }
            let expected_prev = if i == 0 {
                None
            } else {
                Some(prev_hash.clone())
            };
            if record.prev_action_hash != expected_prev {
                return Err(LedgerError::ChainBroken(format!(
                    "record {i}: prev_action_hash {:?} does not link to predecessor {expected_prev:?}",
                    record.prev_action_hash
                )));
            }
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

    fn db_path(dir: &tempfile::TempDir) -> PathBuf {
        dir.path().join("teas.db")
    }

    #[test]
    fn append_two_records_chains_and_verifies() {
        let dir = tempfile::tempdir().expect("temp dir");
        let ledger = ProofLedger::new(db_path(&dir));

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
        let dir = tempfile::tempdir().expect("temp dir");
        let ledger = ProofLedger::new(db_path(&dir));
        ledger.append(sample_record("TASK-1")).expect("append 1");
        ledger.append(sample_record("TASK-2")).expect("append 2");

        let all = ledger.read_all().expect("read_all");
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].task_id, "TASK-1");
        assert_eq!(all[1].task_id, "TASK-2");
    }

    #[test]
    fn read_all_on_absent_db_is_empty() {
        let dir = tempfile::tempdir().expect("temp dir");
        let ledger = ProofLedger::new(dir.path().join("does-not-exist.db"));
        assert!(ledger.read_all().expect("read_all").is_empty());
        assert_eq!(ledger.verify_witness_chain().expect("verify empty"), 0);
    }

    #[test]
    fn tampering_a_row_breaks_the_chain() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = db_path(&dir);
        let ledger = ProofLedger::new(path.clone());
        ledger.append(sample_record("TASK-1")).expect("append 1");
        ledger.append(sample_record("TASK-2")).expect("append 2");

        // Mutate a content field of the genesis row's stored JSON while leaving its
        // stored action_hash column untouched — the recompute must catch it.
        let conn = Connection::open(&path).expect("open");
        let json: String = conn
            .query_row(
                "SELECT record_json FROM proof_records WHERE ledger_seq = 0",
                [],
                |r| r.get(0),
            )
            .expect("read row");
        let mut first: ProofRecord = serde_json::from_str(&json).expect("parse");
        first.task_id = "TAMPERED".to_string();
        let tampered = serde_json::to_string(&first).expect("reserialize");
        conn.execute(
            "UPDATE proof_records SET record_json = ?1 WHERE ledger_seq = 0",
            params![tampered],
        )
        .expect("update");

        match ledger.verify_witness_chain() {
            Err(LedgerError::ChainBroken(_)) => {}
            other => panic!("tamper must break the chain, got {other:?}"),
        }
    }
}
