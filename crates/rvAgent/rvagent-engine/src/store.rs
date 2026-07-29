//! SQLite-backed task-graph store — the TEAS **task database**.
//!
//! WorkOrders are durable rows in a `task_graph` table (indexed by status), not flat
//! CSV/JSON — consistent with gitkb (SQLite task docs) and the envctl execution-framework
//! DB. Pairs with [`crate::ledger::ProofLedger`] (the `proof_records` table); both can
//! share one SQLite database file so a TEAS store is a single database with tables.

use std::path::PathBuf;

use rusqlite::{params, Connection, OptionalExtension};

use crate::workorder::{Status, WorkOrder};

/// Errors from task-store DB access or (de)serialization.
#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    /// SQLite error opening, migrating, or querying the task database.
    #[error("store db error: {0}")]
    Db(#[from] rusqlite::Error),
    /// A stored row failed to (de)serialize as a `WorkOrder`.
    #[error("store JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

/// A SQLite-backed store of canonical [`WorkOrder`]s (the task graph).
#[derive(Debug, Clone)]
pub struct WorkOrderStore {
    path: PathBuf,
}

/// Serialize an enum to its canonical serde token for an indexed column.
fn token<T: serde::Serialize>(value: &T) -> String {
    match serde_json::to_value(value) {
        Ok(serde_json::Value::String(s)) => s,
        _ => String::new(),
    }
}

impl WorkOrderStore {
    /// Construct a store backed by the SQLite database at `path` (created on first use).
    #[must_use]
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    /// The backing database file path.
    #[must_use]
    pub fn path(&self) -> &PathBuf {
        &self.path
    }

    fn open(&self) -> Result<Connection, StoreError> {
        let conn = Connection::open(&self.path)?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS task_graph (
                 id             TEXT PRIMARY KEY,
                 status         TEXT NOT NULL,
                 priority       TEXT NOT NULL,
                 workorder_json TEXT NOT NULL
             );
             CREATE INDEX IF NOT EXISTS idx_task_graph_status ON task_graph(status);",
        )?;
        Ok(conn)
    }

    /// Insert a WorkOrder, or replace the existing row with the same `id`.
    pub fn upsert(&self, wo: &WorkOrder) -> Result<(), StoreError> {
        let conn = self.open()?;
        let json = serde_json::to_string(wo)?;
        conn.execute(
            "INSERT INTO task_graph (id, status, priority, workorder_json)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(id) DO UPDATE SET
                 status = excluded.status,
                 priority = excluded.priority,
                 workorder_json = excluded.workorder_json",
            params![wo.id, token(&wo.status), token(&wo.priority), json],
        )?;
        Ok(())
    }

    /// Fetch a WorkOrder by id, if present.
    pub fn get(&self, id: &str) -> Result<Option<WorkOrder>, StoreError> {
        let conn = self.open()?;
        let json: Option<String> = conn
            .query_row(
                "SELECT workorder_json FROM task_graph WHERE id = ?1",
                params![id],
                |row| row.get(0),
            )
            .optional()?;
        match json {
            Some(j) => Ok(Some(serde_json::from_str(&j)?)),
            None => Ok(None),
        }
    }

    /// All WorkOrders with the given status, id-ordered.
    pub fn list_by_status(&self, status: Status) -> Result<Vec<WorkOrder>, StoreError> {
        self.query(
            "SELECT workorder_json FROM task_graph WHERE status = ?1 ORDER BY id ASC",
            params![token(&status)],
        )
    }

    /// All WorkOrders, id-ordered.
    pub fn all(&self) -> Result<Vec<WorkOrder>, StoreError> {
        self.query("SELECT workorder_json FROM task_graph ORDER BY id ASC", [])
    }

    /// Number of stored WorkOrders.
    pub fn count(&self) -> Result<usize, StoreError> {
        let conn = self.open()?;
        let n: u64 = conn.query_row("SELECT COUNT(*) FROM task_graph", [], |row| row.get(0))?;
        Ok(n as usize)
    }

    fn query<P: rusqlite::Params>(&self, sql: &str, p: P) -> Result<Vec<WorkOrder>, StoreError> {
        let conn = self.open()?;
        let mut stmt = conn.prepare(sql)?;
        let rows = stmt.query_map(p, |row| row.get::<_, String>(0))?;
        let mut out = Vec::new();
        for row in rows {
            out.push(serde_json::from_str(&row?)?);
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workorder::Priority;

    fn wo(id: &str, status: Status) -> WorkOrder {
        WorkOrder {
            schema: "handoff.task.v1".to_string(),
            id: id.to_string(),
            title: "t".to_string(),
            objective: "o".to_string(),
            status,
            priority: Priority::P1,
            path_scope: Vec::new(),
            acceptance_criteria: vec!["ok".to_string()],
            correlation_id: None,
            owner_lane: None,
            role: None,
            dependencies: Vec::new(),
            blocked_by: Vec::new(),
            allows_network: false,
            allows_dependency_addition: false,
            human_approval_required: false,
            verification_command: None,
            rollback_plan: None,
            intent_lock: None,
        }
    }

    fn store() -> (tempfile::TempDir, WorkOrderStore) {
        let dir = tempfile::tempdir().expect("temp dir");
        let s = WorkOrderStore::new(dir.path().join("teas.db"));
        (dir, s)
    }

    #[test]
    fn upsert_get_and_count() {
        let (_dir, s) = store();
        s.upsert(&wo("TASK-1", Status::Backlog)).expect("upsert 1");
        s.upsert(&wo("TASK-2", Status::Active)).expect("upsert 2");
        assert_eq!(s.count().expect("count"), 2);
        assert_eq!(s.get("TASK-1").expect("get").unwrap().id, "TASK-1");
        assert!(s.get("MISSING").expect("get missing").is_none());
    }

    #[test]
    fn upsert_replaces_same_id() {
        let (_dir, s) = store();
        s.upsert(&wo("TASK-1", Status::Backlog)).expect("insert");
        s.upsert(&wo("TASK-1", Status::Done)).expect("update");
        assert_eq!(
            s.count().expect("count"),
            1,
            "same id updates, not duplicates"
        );
        assert_eq!(s.get("TASK-1").expect("get").unwrap().status, Status::Done);
    }

    #[test]
    fn list_by_status_filters() {
        let (_dir, s) = store();
        s.upsert(&wo("TASK-1", Status::Backlog)).expect("1");
        s.upsert(&wo("TASK-2", Status::Backlog)).expect("2");
        s.upsert(&wo("TASK-3", Status::Done)).expect("3");
        assert_eq!(s.list_by_status(Status::Backlog).expect("backlog").len(), 2);
        assert_eq!(s.list_by_status(Status::Done).expect("done").len(), 1);
        assert_eq!(s.all().expect("all").len(), 3);
    }
}
