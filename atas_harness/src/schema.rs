//! Public report schema for bounded, non-authoritative research timelines.

use crate::rng::Fnv1a;

#[derive(Clone, Debug)]
pub struct ResearchLabel {
    pub authority: String,
    pub non_authoritative: bool,
    pub statement: String,
}

impl ResearchLabel {
    pub fn research_only() -> Self {
        Self {
            authority: "research-only".to_owned(),
            non_authoritative: true,
            statement:
                "Research-only forecasts must never gate completion or route production work."
                    .to_owned(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct TimelineStep {
    pub step: usize,
    pub forecast: f64,
    pub state_diff: f64,
}

#[derive(Clone, Debug)]
pub struct FailureNode {
    pub step: usize,
    pub reason: String,
}

#[derive(Clone, Debug)]
pub struct Timeline {
    pub member: usize,
    pub label: ResearchLabel,
    pub steps: Vec<TimelineStep>,
    pub complexity_spikes: Vec<usize>,
    pub failure_nodes: Vec<FailureNode>,
}

#[derive(Clone, Debug)]
pub struct MetricRow {
    pub name: String,
    pub nrmse: f64,
}

#[derive(Clone, Debug)]
pub struct Calibration {
    pub nominal_coverage: f64,
    pub conformal_width: f64,
    pub empirical_coverage: f64,
    pub calib_points: usize,
    pub test_points: usize,
}

#[derive(Clone, Debug)]
pub struct ResourceBurn {
    pub state_updates: u64,
    pub members_run: usize,
    pub flops_per_update: u64,
    pub flop_estimate: u64,
    pub wall_time_ms: f64,
}

#[derive(Clone, Debug)]
pub struct StudioReport {
    pub schema: String,
    pub label: ResearchLabel,
    pub timelines: Vec<Timeline>,
    pub baselines: Vec<MetricRow>,
    pub ablations: Vec<MetricRow>,
    pub calibration: Calibration,
    pub resource_burn: ResourceBurn,
}

impl StudioReport {
    pub fn forecast_fingerprint(&self) -> u64 {
        let mut hash = Fnv1a::new();
        for timeline in &self.timelines {
            hash.write_u64(timeline.member as u64);
            for step in &timeline.steps {
                hash.write_u64(step.step as u64);
                hash.write_f64(step.forecast);
                hash.write_f64(step.state_diff);
            }
        }
        hash.finish()
    }

    pub fn to_json(&self) -> String {
        let timelines = self
            .timelines
            .iter()
            .map(|timeline| {
                format!(
                    "{{\"member\":{},\"label\":{},\"steps\":{},\"complexity_spikes\":{},\"failure_nodes\":[{}]}}",
                    timeline.member,
                    label_json(&timeline.label),
                    timeline.steps.len(),
                    number_list(&timeline.complexity_spikes),
                    timeline
                        .failure_nodes
                        .iter()
                        .map(|node| format!("{{\"step\":{},\"reason\":{}}}", node.step, crate::json::string(&node.reason)))
                        .collect::<Vec<_>>()
                        .join(","),
                )
            })
            .collect::<Vec<_>>()
            .join(",");
        let ablations = self
            .ablations
            .iter()
            .map(|row| {
                format!(
                    "{{\"name\":{},\"nrmse\":{}}}",
                    crate::json::string(&row.name),
                    row.nrmse
                )
            })
            .collect::<Vec<_>>()
            .join(",");
        format!(
            "{{\"schema\":{},\"label\":{},\"timelines\":[{}],\"ablations\":[{}]}}",
            crate::json::string(&self.schema),
            label_json(&self.label),
            timelines,
            ablations
        )
    }
}

fn label_json(label: &ResearchLabel) -> String {
    format!(
        "{{\"authority\":{},\"non_authoritative\":{},\"statement\":{}}}",
        crate::json::string(&label.authority),
        label.non_authoritative,
        crate::json::string(&label.statement)
    )
}

fn number_list(values: &[usize]) -> String {
    format!(
        "[{}]",
        values
            .iter()
            .map(usize::to_string)
            .collect::<Vec<_>>()
            .join(",")
    )
}
