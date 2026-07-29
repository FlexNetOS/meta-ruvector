//! Anti-corruption adapter (TEAS DOMAIN_MODEL seam **S3**): canonical
//! [`WorkOrder`] ⇄ rvagent-a2a [`TaskSpec`].
//!
//! **Projection + envelope.** The routing-relevant fields are projected into the
//! native `TaskSpec` shape — `id → id`, `role`/`owner_lane → skill` (the A2A routing
//! key; `role` is a capability so it is preferred over the concurrency `owner_lane`),
//! `objective → user Message` — while the *whole* `WorkOrder` is carried in
//! `TaskSpec.metadata["workorder"]`. `TaskSpec.metadata` is a `serde_json::Value`
//! made for exactly this, so the mapping is **lossless**, including the blake3
//! `IntentLock`.
//!
//! `TaskSpec`-only fields (`policy`, `context`/trace) are execution concerns, not
//! part of the task contract, so they are freshly generated on the way out and do
//! not round-trip back into the `WorkOrder`.

use rvagent_a2a::context::{default_current_agent, TaskContext};
use rvagent_a2a::types::{Message, Part, Role, TaskSpec};
use serde_json::json;

use crate::workorder::WorkOrder;

/// Routing skill used when a `WorkOrder` names neither an `owner_lane` nor a `role`.
pub const DEFAULT_SKILL: &str = "teas.execute";

/// Metadata key under which the full `WorkOrder` envelope is carried.
pub const WORKORDER_META_KEY: &str = "workorder";

/// Errors recovering a [`WorkOrder`] from a [`TaskSpec`].
#[derive(Debug, thiserror::Error)]
pub enum AdapterError {
    /// The `TaskSpec.metadata` carried no `workorder` envelope.
    #[error("TaskSpec.metadata has no '{WORKORDER_META_KEY}' key")]
    MissingWorkOrder,
    /// The envelope was present but did not decode to a `WorkOrder`.
    #[error("failed to decode WorkOrder from TaskSpec.metadata: {0}")]
    Decode(#[from] serde_json::Error),
}

/// Map a canonical [`WorkOrder`] to a [`TaskSpec`]. Lossless: the `WorkOrder` is
/// serialized into `TaskSpec.metadata["workorder"]`.
pub fn workorder_to_taskspec(wo: &WorkOrder) -> Result<TaskSpec, serde_json::Error> {
    // `skill` is an A2A capability id: prefer `role` (a capability, e.g. backend-dev)
    // over `owner_lane` (a concurrency lane, e.g. lane_d_filesystem).
    let skill = wo
        .role
        .clone()
        .or_else(|| wo.owner_lane.clone())
        .unwrap_or_else(|| DEFAULT_SKILL.to_string());
    let message = Message {
        role: Role::User,
        parts: vec![Part::Text {
            text: wo.objective.clone(),
        }],
        metadata: serde_json::Value::Null,
    };
    let metadata = json!({ WORKORDER_META_KEY: serde_json::to_value(wo)? });
    Ok(TaskSpec {
        id: wo.id.clone(),
        skill,
        message,
        policy: None,
        context: TaskContext::new_root(default_current_agent()),
        metadata,
    })
}

/// Recover the [`WorkOrder`] carried in a [`TaskSpec`]'s metadata envelope.
pub fn taskspec_to_workorder(spec: &TaskSpec) -> Result<WorkOrder, AdapterError> {
    let raw = spec
        .metadata
        .get(WORKORDER_META_KEY)
        .ok_or(AdapterError::MissingWorkOrder)?;
    Ok(serde_json::from_value(raw.clone())?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workorder::{IntentLock, Priority, Status};

    fn h(seed: char) -> String {
        format!("blake3:{}", seed.to_string().repeat(64))
    }

    fn sample() -> WorkOrder {
        WorkOrder {
            schema: "handoff.task.v1".to_string(),
            id: "HFTASK-0058".to_string(),
            title: "Wire the front door".to_string(),
            objective: "emit WorkOrders from ExecutionPlan".to_string(),
            status: Status::Backlog,
            priority: Priority::P1,
            path_scope: vec!["src/prompt_hub/prompt-hub/src".to_string()],
            acceptance_criteria: vec!["emits >=1 valid WorkOrder".to_string()],
            correlation_id: Some("wf-1234".to_string()),
            owner_lane: Some("lane_d_frontdoor".to_string()),
            role: Some("backend-dev".to_string()),
            dependencies: vec!["TEASTASK-003".to_string()],
            blocked_by: vec!["TEASTASK-003".to_string()],
            allows_network: false,
            allows_dependency_addition: true,
            human_approval_required: false,
            verification_command: Some("cargo test -p prompt-hub".to_string()),
            rollback_plan: Some("revert the taskgraph module".to_string()),
            intent_lock: Some(IntentLock {
                objective_hash: h('a'),
                path_scope_hash: h('b'),
                acceptance_hash: h('c'),
                constraint_hash: Some(h('d')),
                northstar_revision: Some("blake3:northstar-rev-1".to_string()),
            }),
        }
    }

    #[test]
    fn round_trips_losslessly() {
        let wo = sample();
        let spec = workorder_to_taskspec(&wo).expect("to taskspec");
        let back = taskspec_to_workorder(&spec).expect("from taskspec");
        assert_eq!(wo, back, "WorkOrder must survive the round-trip unchanged");
    }

    #[test]
    fn preserves_intent_lock() {
        let wo = sample();
        let spec = workorder_to_taskspec(&wo).unwrap();
        let back = taskspec_to_workorder(&spec).unwrap();
        assert_eq!(
            wo.intent_lock, back.intent_lock,
            "the blake3 IntentLock is the drift sentinel — it must be preserved exactly"
        );
    }

    #[test]
    fn projects_routing_fields() {
        let wo = sample();
        let spec = workorder_to_taskspec(&wo).unwrap();
        assert_eq!(spec.id, "HFTASK-0058");
        assert_eq!(
            spec.skill, "backend-dev",
            "role is preferred as the skill/capability key"
        );
        match &spec.message.parts[0] {
            Part::Text { text } => assert_eq!(text, "emit WorkOrders from ExecutionPlan"),
            other => panic!("objective must project to a Text part, got {other:?}"),
        }
    }

    #[test]
    fn skill_prefers_role_then_lane_then_default() {
        let mut wo = sample();
        assert_eq!(
            workorder_to_taskspec(&wo).unwrap().skill,
            "backend-dev",
            "role wins"
        );
        wo.role = None;
        assert_eq!(
            workorder_to_taskspec(&wo).unwrap().skill,
            "lane_d_frontdoor",
            "then owner_lane"
        );
        wo.owner_lane = None;
        assert_eq!(
            workorder_to_taskspec(&wo).unwrap().skill,
            DEFAULT_SKILL,
            "then the default skill"
        );
    }

    #[test]
    fn covers_all_statuses_and_priorities() {
        for status in [
            Status::Backlog,
            Status::Active,
            Status::Claimed,
            Status::Blocked,
            Status::Checkpointed,
            Status::Review,
            Status::Done,
        ] {
            for priority in [Priority::P0, Priority::P1, Priority::P2, Priority::P3] {
                let mut wo = sample();
                wo.status = status;
                wo.priority = priority;
                let back = taskspec_to_workorder(&workorder_to_taskspec(&wo).unwrap()).unwrap();
                assert_eq!(
                    wo, back,
                    "status {status:?} / priority {priority:?} round-trip"
                );
            }
        }
    }

    #[test]
    fn missing_envelope_errors() {
        let wo = sample();
        let mut spec = workorder_to_taskspec(&wo).unwrap();
        spec.metadata = serde_json::Value::Null;
        assert!(matches!(
            taskspec_to_workorder(&spec),
            Err(AdapterError::MissingWorkOrder)
        ));
    }

    #[test]
    fn decode_error_on_malformed_envelope() {
        let wo = sample();
        let mut spec = workorder_to_taskspec(&wo).unwrap();
        // present envelope, but not a valid WorkOrder (id is a number, not a string)
        spec.metadata = serde_json::json!({ "workorder": { "id": 123 } });
        assert!(matches!(
            taskspec_to_workorder(&spec),
            Err(AdapterError::Decode(_))
        ));
    }

    #[test]
    fn all_none_optionals_round_trip() {
        let mut wo = sample();
        wo.correlation_id = None;
        wo.owner_lane = None;
        wo.role = None;
        wo.verification_command = None;
        wo.rollback_plan = None;
        wo.intent_lock = None;
        wo.path_scope.clear();
        wo.acceptance_criteria.clear();
        wo.dependencies.clear();
        wo.blocked_by.clear();
        let back = taskspec_to_workorder(&workorder_to_taskspec(&wo).unwrap()).unwrap();
        assert_eq!(wo, back, "the all-None / empty side must also round-trip");
    }
}
