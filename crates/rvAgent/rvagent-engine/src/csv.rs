//! CSV projection of [`WorkOrder`]s — the human control surface of the three-surface
//! doctrine (CSV = human, JSON packet = agent, proof = reality). Generated from the
//! `WorkOrder` type, never hand-maintained (TEASTASK-003 acceptance).

use crate::workorder::WorkOrder;

/// CSV columns, in order. List-valued cells are pipe-delimited (execution-framework
/// convention). Only fields present on the engine's `WorkOrder` projection appear.
pub const COLUMNS: &[&str] = &[
    "id",
    "title",
    "objective",
    "status",
    "priority",
    "owner_lane",
    "role",
    "path_scope",
    "acceptance_criteria",
    "dependencies",
    "blocked_by",
    "verification_command",
    "rollback_plan",
    "allows_network",
    "human_approval_required",
];

/// RFC-4180 field quoting: wrap in quotes and double interior quotes iff the field
/// contains a comma, quote, or newline.
fn quote(field: &str) -> String {
    if field.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", field.replace('"', "\"\""))
    } else {
        field.to_string()
    }
}

/// Render one JSON value (a WorkOrder field) as a CSV cell: arrays pipe-join their
/// string items, bools become `true`/`false`, strings pass through, null/absent → "".
fn cell(value: Option<&serde_json::Value>) -> String {
    match value {
        None | Some(serde_json::Value::Null) => String::new(),
        Some(serde_json::Value::Array(items)) => items
            .iter()
            .map(|i| {
                i.as_str()
                    .map(String::from)
                    .unwrap_or_else(|| i.to_string())
            })
            .collect::<Vec<_>>()
            .join("|"),
        Some(serde_json::Value::String(s)) => s.clone(),
        Some(other) => other.to_string(),
    }
}

/// Project a slice of [`WorkOrder`]s to a CSV document (header + one row each).
#[must_use]
pub fn workorders_to_csv(workorders: &[WorkOrder]) -> String {
    let mut out = String::new();
    out.push_str(&COLUMNS.join(","));
    out.push('\n');
    for wo in workorders {
        // Infallible in practice: a WorkOrder always serializes to a JSON object.
        let value = serde_json::to_value(wo).unwrap_or(serde_json::Value::Null);
        let row: Vec<String> = COLUMNS
            .iter()
            .map(|col| quote(&cell(value.get(*col))))
            .collect();
        out.push_str(&row.join(","));
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workorder::{Priority, Status};

    fn wo() -> WorkOrder {
        WorkOrder {
            schema: "handoff.task.v1".to_string(),
            id: "HFTASK-1".to_string(),
            title: "Title, with a comma".to_string(), // forces RFC-4180 quoting
            objective: "obj".to_string(),
            status: Status::Active,
            priority: Priority::P0,
            path_scope: vec!["src/a".to_string(), "src/b".to_string()],
            acceptance_criteria: vec!["ok".to_string()],
            correlation_id: None,
            owner_lane: Some("lane_a".to_string()),
            role: None,
            dependencies: vec!["HFTASK-0".to_string()],
            blocked_by: vec![],
            allows_network: false,
            allows_dependency_addition: false,
            human_approval_required: true,
            verification_command: Some("cargo test".to_string()),
            rollback_plan: Some("revert".to_string()),
            intent_lock: None,
        }
    }

    #[test]
    fn header_matches_columns() {
        let csv = workorders_to_csv(&[]);
        assert_eq!(csv.trim_end(), COLUMNS.join(","));
    }

    #[test]
    fn projects_fields_lists_bools_and_quoting() {
        let csv = workorders_to_csv(&[wo()]);
        let mut lines = csv.lines();
        assert_eq!(lines.next().unwrap(), COLUMNS.join(","));
        let row = lines.next().unwrap();
        assert!(row.starts_with("HFTASK-1,"));
        assert!(
            row.contains("\"Title, with a comma\""),
            "comma field must be quoted"
        );
        assert!(row.contains("active,"), "status enum projected lowercase");
        assert!(row.contains("P0,"), "priority projected");
        assert!(row.contains("src/a|src/b"), "path_scope pipe-joined");
        assert!(
            row.contains("true"),
            "human_approval_required bool projected"
        );
        // empty optional (role=None) yields an empty cell, not the literal "null"
        assert!(
            !row.contains("null"),
            "absent optionals must be empty, not 'null'"
        );
    }

    #[test]
    fn one_row_per_workorder() {
        let csv = workorders_to_csv(&[wo(), wo(), wo()]);
        assert_eq!(csv.lines().count(), 4, "header + 3 rows");
    }
}
