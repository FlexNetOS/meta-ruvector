//! TEAS engine operations exposed as MCP tools (TEASTASK-010).
//!
//! This module is the MCP server-layer surface over the LifeOS **TEAS** execution
//! engine ([`rvagent_engine`]). It wraps the engine's task operations as
//! [`McpToolHandler`]s and registers them in an [`McpToolRegistry`], so the exact
//! same registry the existing stdio server ([`crate::server`] / [`crate::transport`])
//! already dispatches through now also carries the engine's `submit -> run -> proof`
//! surface. The engine stays lean; the dependency edge points *from* the MCP layer
//! *to* the engine (cycle-free — see the crate `Cargo.toml` note).
//!
//! ## Tools
//! - `teas_run` — take a `handoff.task.v1` [`WorkOrder`] JSON object, adapt it to a
//!   [`TaskSpec`](rvagent_a2a::types::TaskSpec), and drive
//!   [`RvAgentEngine::run`](rvagent_engine::RvAgentEngine). Returns the resulting
//!   task's terminal state (`completed` / `failed` / `input-required`), its id, and
//!   the artifact or failure message. An optional `ledger_path` string switches the
//!   engine to [`with_ledger`](rvagent_engine::RvAgentEngine::with_ledger) so the run
//!   is witnessed. Bad WorkOrder JSON or an engine error becomes a tool-error result
//!   (`is_error: true`) — never a panic. A *failing* verification is a legitimate
//!   engine outcome (`state: "failed"`), not a tool error.
//! - `teas_verify_ledger` — open the JSONL proof ledger at `ledger_path` and return
//!   its [`verify_witness_chain`](rvagent_engine::ProofLedger::verify_witness_chain)
//!   count (proves the proof surface end-to-end).
//! - `teas_list` — return the set of TEAS tool names + a `ready` status.
//!
//! ## Acceptance mapping (TEASTASK-010)
//! - *"tools/list exposes engine ops"* — after [`register_teas_tools`], the registry's
//!   [`list_tools`](McpToolRegistry::list_tools) /
//!   [`list_mcp_tools`](McpToolRegistry::list_mcp_tools) include the three `teas_*`
//!   tools (covered by tests below).
//! - *"tools/call submit->claim->run->proof works over stdio"* —
//!   [`call_tool`](McpToolRegistry::call_tool) drives run + proof in-process here; the
//!   **existing** stdio server ([`crate::server`]/[`crate::transport`]) carries these
//!   registry entries over stdio unchanged. The transport is deliberately NOT
//!   reimplemented — this module only adds registry entries it already dispatches.
//! - *"parity with hf-mcp"* — naming mirrors the hf-mcp snake_case style; full 38-tool
//!   parity is out of scope for this task.

use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use serde_json::{json, Value};

use rvagent_a2a::{executor::TaskRunner, types::Task};
use rvagent_engine::{workorder_to_taskspec, ProofLedger, RvAgentEngine, WorkOrder};

use crate::protocol::{Content, ToolCallResult};
use crate::registry::{McpToolDefinition, McpToolHandler, McpToolRegistry};
use crate::Result;

/// Canonical names of the TEAS MCP tools (source of truth for `teas_list`).
pub const TEAS_TOOL_NAMES: [&str; 3] = ["teas_run", "teas_verify_ledger", "teas_list"];

/// Build a successful tool result carrying `value` as pretty JSON text.
fn ok_json(value: &Value) -> ToolCallResult {
    let text = serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string());
    ToolCallResult {
        content: vec![Content::text(text)],
        is_error: false,
    }
}

/// Build a tool-error result (`is_error: true`) carrying `msg`. Used for bad input or
/// engine errors so a caller sees a structured error rather than a panic.
fn err_result(msg: impl Into<String>) -> ToolCallResult {
    ToolCallResult {
        content: vec![Content::text(msg.into())],
        is_error: true,
    }
}

// ---------------------------------------------------------------------------
// teas_run
// ---------------------------------------------------------------------------

/// Drives [`RvAgentEngine::run`] from a WorkOrder JSON object.
pub struct TeasRunHandler;

#[async_trait]
impl McpToolHandler for TeasRunHandler {
    async fn execute(&self, arguments: Value) -> Result<ToolCallResult> {
        // Optional ledger switch. WorkOrder has no `deny_unknown_fields`, so an extra
        // `ledger_path` key is ignored by WorkOrder deserialization below.
        let ledger_path = arguments
            .get("ledger_path")
            .and_then(|v| v.as_str())
            .map(PathBuf::from);

        // 1. Deserialize the handoff.task.v1 WorkOrder envelope.
        let wo: WorkOrder = match serde_json::from_value(arguments) {
            Ok(wo) => wo,
            Err(e) => {
                return Ok(err_result(format!(
                    "teas_run: invalid WorkOrder (handoff.task.v1) arguments: {e}"
                )));
            }
        };

        // 2. Adapt to the engine's TaskSpec (never panics on failure).
        let spec = match workorder_to_taskspec(&wo) {
            Ok(spec) => spec,
            Err(e) => {
                return Ok(err_result(format!(
                    "teas_run: failed to build TaskSpec from WorkOrder: {e}"
                )));
            }
        };

        // 3. Construct the engine (witnessed iff a ledger_path was supplied) and run.
        let engine = match ledger_path {
            Some(path) => RvAgentEngine::with_ledger(path),
            None => RvAgentEngine::new(),
        };
        let task: Task = match engine.run(spec).await {
            Ok(task) => task,
            Err(e) => {
                return Ok(err_result(format!("teas_run: engine run failed: {e}")));
            }
        };

        // 4. Map the resulting Task onto a ToolCallResult. Serialize the Task once and
        //    read its terminal state, id, and artifact/failure text via JSON pointers,
        //    so we neither name nor depend on the a2a Part/TaskState surface here.
        let task_json = match serde_json::to_value(&task) {
            Ok(v) => v,
            Err(e) => {
                return Ok(err_result(format!(
                    "teas_run: failed to serialize task result: {e}"
                )));
            }
        };
        let task_id = task_json
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let state = task_json
            .pointer("/status/state")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();
        let message = task_json
            .pointer("/status/message/parts/0/text")
            .and_then(|v| v.as_str())
            .map(str::to_string);
        let artifact = task_json
            .pointer("/artifacts/0/parts/0/text")
            .and_then(|v| v.as_str())
            .map(str::to_string);

        let mut summary = json!({
            "tool": "teas_run",
            "task_id": task_id,
            "state": state,
        });
        if let Some(m) = message {
            summary["message"] = Value::String(m);
        }
        if let Some(a) = artifact {
            summary["artifact"] = Value::String(a);
        }
        // Completed / Failed / InputRequired are all legitimate engine outcomes — not
        // tool errors. `is_error` stays false; the `state` field carries the verdict.
        Ok(ok_json(&summary))
    }
}

// ---------------------------------------------------------------------------
// teas_verify_ledger
// ---------------------------------------------------------------------------

/// Opens a JSONL proof ledger and verifies its blake3 witness chain.
pub struct TeasVerifyLedgerHandler;

#[async_trait]
impl McpToolHandler for TeasVerifyLedgerHandler {
    async fn execute(&self, arguments: Value) -> Result<ToolCallResult> {
        let path = match arguments.get("ledger_path").and_then(|v| v.as_str()) {
            Some(p) => PathBuf::from(p),
            None => {
                return Ok(err_result(
                    "teas_verify_ledger: requires a 'ledger_path' string argument",
                ));
            }
        };
        let ledger = ProofLedger::new(path.clone());
        match ledger.verify_witness_chain() {
            Ok(count) => Ok(ok_json(&json!({
                "tool": "teas_verify_ledger",
                "ledger_path": path.to_string_lossy(),
                "verified": count,
            }))),
            Err(e) => Ok(err_result(format!(
                "teas_verify_ledger: witness chain verification failed: {e}"
            ))),
        }
    }
}

// ---------------------------------------------------------------------------
// teas_list
// ---------------------------------------------------------------------------

/// Lists the TEAS tool names + a small status (proves listing).
pub struct TeasListHandler;

#[async_trait]
impl McpToolHandler for TeasListHandler {
    async fn execute(&self, _arguments: Value) -> Result<ToolCallResult> {
        Ok(ok_json(&json!({
            "tool": "teas_list",
            "status": "ready",
            "tools": TEAS_TOOL_NAMES,
        })))
    }
}

// ---------------------------------------------------------------------------
// Registration
// ---------------------------------------------------------------------------

/// JSON Schema for a `teas_run` argument object (a `handoff.task.v1` WorkOrder plus an
/// optional `ledger_path`). Mirrors the WorkOrder shape's load-bearing fields.
fn teas_run_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "schema": { "type": "string", "description": "Schema tag; canonically \"handoff.task.v1\"" },
            "id": { "type": "string", "description": "WorkOrder / task id" },
            "title": { "type": "string", "description": "Short human title" },
            "objective": { "type": "string", "description": "What the work order accomplishes" },
            "status": {
                "type": "string",
                "enum": ["backlog", "active", "claimed", "blocked", "checkpointed", "review", "done"],
                "description": "WorkOrder status"
            },
            "priority": {
                "type": "string",
                "enum": ["P0", "P1", "P2", "P3"],
                "description": "Task priority"
            },
            "path_scope": {
                "type": "array",
                "items": { "type": "string" },
                "description": "Directories the work is scoped to (first existing dir is the run cwd)"
            },
            "acceptance_criteria": {
                "type": "array",
                "items": { "type": "string" }
            },
            "verification_command": {
                "type": ["string", "null"],
                "description": "Shell command whose exit-0 proves completion; absent -> input-required"
            },
            "ledger_path": {
                "type": "string",
                "description": "Optional: witness the run's ProofRecord to this JSONL ledger path"
            }
        },
        "required": ["schema", "id", "title", "objective", "status", "priority"]
    })
}

/// JSON Schema for `teas_verify_ledger` arguments.
fn teas_verify_ledger_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "ledger_path": {
                "type": "string",
                "description": "Path to the JSONL proof ledger to verify"
            }
        },
        "required": ["ledger_path"]
    })
}

/// Register every TEAS tool into `registry`.
///
/// Returns an error if any tool name is already registered (the registry rejects
/// duplicates). Additive: leaves all previously-registered tools untouched.
pub fn register_teas_tools(registry: &McpToolRegistry) -> anyhow::Result<()> {
    registry.register_tool(McpToolDefinition {
        name: "teas_run".into(),
        description: "Run a handoff.task.v1 WorkOrder through the TEAS engine \
                      (verification -> proof); returns the task's terminal state, id, \
                      and artifact/failure message"
            .into(),
        input_schema: teas_run_schema(),
        handler: Arc::new(TeasRunHandler),
    })?;

    registry.register_tool(McpToolDefinition {
        name: "teas_verify_ledger".into(),
        description: "Verify the blake3 witness chain of a TEAS JSONL proof ledger; \
                      returns the number of witnessed ProofRecords"
            .into(),
        input_schema: teas_verify_ledger_schema(),
        handler: Arc::new(TeasVerifyLedgerHandler),
    })?;

    registry.register_tool(McpToolDefinition {
        name: "teas_list".into(),
        description: "List the TEAS MCP tool names and engine status".into(),
        input_schema: json!({ "type": "object", "properties": {} }),
        handler: Arc::new(TeasListHandler),
    })?;

    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rvagent_engine::{Priority, Status, WorkOrder};

    /// A minimal, schema-valid WorkOrder JSON object with the given verification command.
    fn work_order_json(verification: Option<&str>) -> Value {
        let wo = WorkOrder {
            schema: "handoff.task.v1".to_string(),
            id: "TEASTASK-010-test".to_string(),
            title: "trivial verification".to_string(),
            objective: "prove the MCP tool drives the engine".to_string(),
            status: Status::Active,
            priority: Priority::P2,
            path_scope: Vec::new(),
            acceptance_criteria: vec!["verification runs".to_string()],
            correlation_id: None,
            owner_lane: None,
            role: None,
            dependencies: Vec::new(),
            blocked_by: Vec::new(),
            allows_network: false,
            allows_dependency_addition: false,
            human_approval_required: false,
            verification_command: verification.map(str::to_string),
            rollback_plan: None,
            intent_lock: None,
        };
        serde_json::to_value(&wo).expect("workorder to json")
    }

    /// Read the first text content block of a result as parsed JSON.
    fn result_json(result: &ToolCallResult) -> Value {
        match &result.content[0] {
            Content::Text { text } => serde_json::from_str(text).expect("result text is json"),
            other => panic!("expected text content, got {other:?}"),
        }
    }

    #[test]
    fn register_adds_all_teas_tools() {
        let reg = McpToolRegistry::new();
        let before = reg.len();
        register_teas_tools(&reg).expect("register");
        assert_eq!(reg.len(), before + 3, "three TEAS tools registered");

        let names: Vec<String> = reg.list_tools().iter().map(|t| t.name.clone()).collect();
        assert!(names.contains(&"teas_run".to_string()));
        assert!(names.contains(&"teas_verify_ledger".to_string()));
        assert!(names.contains(&"teas_list".to_string()));

        // list_mcp_tools (wire form) also exposes them.
        let wire: Vec<String> = reg
            .list_mcp_tools()
            .iter()
            .map(|t| t.name.clone())
            .collect();
        assert!(wire.contains(&"teas_run".to_string()));
    }

    #[test]
    fn register_is_idempotent_error_on_duplicate() {
        let reg = McpToolRegistry::new();
        register_teas_tools(&reg).expect("first register");
        assert!(
            register_teas_tools(&reg).is_err(),
            "re-registering the same tools must error (duplicate)"
        );
    }

    #[tokio::test]
    async fn teas_run_passing_verification_completes() {
        let reg = McpToolRegistry::new();
        register_teas_tools(&reg).expect("register");
        let result = reg
            .call_tool("teas_run", work_order_json(Some("true")))
            .await
            .expect("call ok");
        assert!(!result.is_error, "a passing run is not a tool error");
        let body = result_json(&result);
        assert_eq!(body["state"], "completed");
        assert_eq!(body["task_id"], "TEASTASK-010-test");
        assert!(
            body.get("artifact").is_some(),
            "completed run carries artifact"
        );
    }

    #[tokio::test]
    async fn teas_run_failing_verification_fails_not_completed() {
        let reg = McpToolRegistry::new();
        register_teas_tools(&reg).expect("register");
        let result = reg
            .call_tool("teas_run", work_order_json(Some("false")))
            .await
            .expect("call ok");
        assert!(
            !result.is_error,
            "a failing verification is an outcome, not a tool error"
        );
        let body = result_json(&result);
        assert_eq!(body["state"], "failed");
        assert_ne!(body["state"], "completed");
        assert!(
            body.get("message").is_some(),
            "failed run carries a failure message"
        );
    }

    #[tokio::test]
    async fn teas_run_malformed_args_is_error_result_not_panic() {
        let reg = McpToolRegistry::new();
        register_teas_tools(&reg).expect("register");
        // Missing every required WorkOrder field.
        let result = reg
            .call_tool("teas_run", json!({ "not": "a workorder" }))
            .await
            .expect("call returns a result (no panic)");
        assert!(
            result.is_error,
            "malformed WorkOrder must be a tool-error result"
        );
    }

    #[tokio::test]
    async fn teas_run_with_ledger_then_verify_counts_one() {
        let tmp = tempfile::NamedTempFile::new().expect("temp file");
        let ledger_path = tmp.path().to_string_lossy().to_string();

        let reg = McpToolRegistry::new();
        register_teas_tools(&reg).expect("register");

        // Run with a ledger_path so the completion is witnessed.
        let mut args = work_order_json(Some("true"));
        args["ledger_path"] = Value::String(ledger_path.clone());
        let run = reg.call_tool("teas_run", args).await.expect("run ok");
        assert!(!run.is_error);
        assert_eq!(result_json(&run)["state"], "completed");

        // Verify the witness chain via the tool.
        let verified = reg
            .call_tool("teas_verify_ledger", json!({ "ledger_path": ledger_path }))
            .await
            .expect("verify ok");
        assert!(!verified.is_error);
        assert_eq!(result_json(&verified)["verified"], 1);
    }

    #[tokio::test]
    async fn teas_verify_ledger_missing_arg_is_error() {
        let reg = McpToolRegistry::new();
        register_teas_tools(&reg).expect("register");
        let result = reg
            .call_tool("teas_verify_ledger", json!({}))
            .await
            .expect("call ok");
        assert!(result.is_error, "missing ledger_path must be a tool error");
    }

    #[tokio::test]
    async fn teas_list_returns_the_tool_names() {
        let reg = McpToolRegistry::new();
        register_teas_tools(&reg).expect("register");
        let result = reg
            .call_tool("teas_list", Value::Null)
            .await
            .expect("call ok");
        assert!(!result.is_error);
        let body = result_json(&result);
        assert_eq!(body["status"], "ready");
        let tools = body["tools"].as_array().expect("tools array");
        assert_eq!(tools.len(), 3);
    }
}
