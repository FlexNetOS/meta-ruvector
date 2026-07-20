//! Bounded ATAS orchestration and report assembly.

use crate::{
    baseline::{mean_forecast, nrmse, persistence_forecast, targets},
    dataset::{DatasetContract, SeriesDataset},
    ensemble::EnsembleConfig,
    esn::EsnModel,
    schema::{
        Calibration, FailureNode, MetricRow, ResearchLabel, ResourceBurn, StudioReport, Timeline,
        TimelineStep,
    },
};

#[derive(Clone, Debug)]
pub struct SimulationBudget {
    pub max_members: usize,
    pub max_steps: usize,
    pub max_state_updates: u64,
}

impl Default for SimulationBudget {
    fn default() -> Self {
        Self {
            max_members: 32,
            max_steps: 100_000,
            max_state_updates: 10_000_000,
        }
    }
}

#[derive(Clone, Debug)]
pub struct StudioConfig {
    pub ensemble: EnsembleConfig,
    pub budget: SimulationBudget,
    pub nominal_coverage: f64,
    pub spike_window: usize,
    pub spike_ratio: f64,
    pub spike_floor: f64,
    pub divergence_bound: f64,
}

#[derive(Clone, Debug)]
pub enum AtasError {
    BudgetExceeded {
        what: String,
        limit: u64,
        requested: u64,
    },
    Invalid(String),
}

impl From<String> for AtasError {
    fn from(value: String) -> Self {
        Self::Invalid(value)
    }
}

pub fn run_studio(
    config: &StudioConfig,
    dataset: &SeriesDataset,
    contract: &DatasetContract,
) -> Result<StudioReport, AtasError> {
    contract.validate()?;
    if dataset.values.len() != contract.series_len {
        return Err(AtasError::Invalid(
            "dataset length does not match contract".to_owned(),
        ));
    }
    let members = config.ensemble.members;
    if members == 0 {
        return Err(AtasError::Invalid(
            "an ensemble requires at least one member".to_owned(),
        ));
    }
    let steps = contract.test_range.1 - contract.test_range.0;
    let state_updates =
        members as u64 * (contract.train_range.1 as u64 + contract.test_range.1 as u64);
    if members > config.budget.max_members {
        return Err(AtasError::BudgetExceeded {
            what: "members".to_owned(),
            limit: config.budget.max_members as u64,
            requested: members as u64,
        });
    }
    if steps > config.budget.max_steps {
        return Err(AtasError::BudgetExceeded {
            what: "steps".to_owned(),
            limit: config.budget.max_steps as u64,
            requested: steps as u64,
        });
    }
    if state_updates > config.budget.max_state_updates {
        return Err(AtasError::BudgetExceeded {
            what: "state_updates".to_owned(),
            limit: config.budget.max_state_updates,
            requested: state_updates,
        });
    }

    let mut members_predictions = Vec::with_capacity(members);
    let mut calibration_predictions = Vec::with_capacity(members);
    let mut timelines = Vec::with_capacity(members);
    for member in 0..members {
        let mut member_config = config.ensemble.base.clone();
        member_config.seed = member_config.seed.wrapping_add(member as u64);
        let mut model = EsnModel::new(member_config);
        model.fit(&dataset.values, contract)?;
        let (predictions, state_diffs) =
            model.predict_with_diffs(&dataset.values, contract.test_range)?;
        let calibration = model.predict_range(&dataset.values, contract.calib_range)?;
        let timeline_steps = predictions
            .iter()
            .copied()
            .zip(state_diffs.iter().copied())
            .enumerate()
            .map(|(offset, (forecast, state_diff))| TimelineStep {
                step: contract.test_range.0 + offset,
                forecast,
                state_diff,
            })
            .collect();
        timelines.push(Timeline {
            member,
            label: ResearchLabel::research_only(),
            steps: timeline_steps,
            complexity_spikes: complexity_spikes(&dataset.values, contract.test_range, config),
            failure_nodes: failure_nodes(
                &dataset.values,
                contract.test_range,
                &state_diffs,
                config,
            ),
        });
        members_predictions.push(predictions);
        calibration_predictions.push(calibration);
    }

    let ensemble_mean = mean_tracks(&members_predictions);
    let calibration_mean = mean_tracks(&calibration_predictions);
    let observed = targets(&dataset.values, contract.test_range);
    let calibration_targets = targets(&dataset.values, contract.calib_range);
    let calibration = conformal_calibration(
        config.nominal_coverage,
        &calibration_mean,
        &calibration_targets,
        &ensemble_mean,
        &observed,
    );
    let persistence = persistence_forecast(&dataset.values, contract.test_range);
    let train_mean = mean_forecast(&dataset.values, contract.train_range, contract.test_range);
    let first_member = members_predictions.first().cloned().unwrap_or_default();
    let baselines = vec![
        MetricRow {
            name: "persistence".to_owned(),
            nrmse: nrmse(&persistence, &observed),
        },
        MetricRow {
            name: "train_mean".to_owned(),
            nrmse: nrmse(&train_mean, &observed),
        },
    ];
    let ablations = vec![
        MetricRow {
            name: "esn_ensemble".to_owned(),
            nrmse: nrmse(&ensemble_mean, &observed),
        },
        MetricRow {
            name: "readout_only".to_owned(),
            nrmse: nrmse(&first_member, &observed),
        },
        baselines[0].clone(),
        baselines[1].clone(),
    ];
    let flops_per_update = (config.ensemble.base.reservoir_size.max(1) * 2) as u64;
    Ok(StudioReport {
        schema: "atas.future-timeline.v0".to_owned(),
        label: ResearchLabel::research_only(),
        timelines,
        baselines,
        ablations,
        calibration,
        resource_burn: ResourceBurn {
            state_updates,
            members_run: members,
            flops_per_update,
            flop_estimate: state_updates * flops_per_update,
            // A wall clock would make reproducibility claims brittle. The harness reports
            // deterministic operation counts as the authoritative resource accounting.
            wall_time_ms: 0.0,
        },
    })
}

fn mean_tracks(tracks: &[Vec<f64>]) -> Vec<f64> {
    if tracks.is_empty() {
        return Vec::new();
    }
    (0..tracks[0].len())
        .map(|index| tracks.iter().map(|track| track[index]).sum::<f64>() / tracks.len() as f64)
        .collect()
}

fn conformal_calibration(
    nominal_coverage: f64,
    predictions: &[f64],
    observed: &[f64],
    test_predictions: &[f64],
    test_observed: &[f64],
) -> Calibration {
    let mut residuals = predictions
        .iter()
        .zip(observed)
        .filter_map(|(prediction, target)| {
            (prediction.is_finite() && target.is_finite()).then_some((prediction - target).abs())
        })
        .collect::<Vec<_>>();
    residuals.sort_by(f64::total_cmp);
    let coverage = nominal_coverage.clamp(0.0, 1.0);
    let index = (((residuals.len() + 1) as f64 * coverage).ceil() as usize)
        .saturating_sub(1)
        .min(residuals.len().saturating_sub(1));
    // The small conservative factor makes finite-sample transfer explicit rather than
    // accidentally treating the calibration point estimate as an authority claim.
    let width = residuals.get(index).copied().unwrap_or(0.0).max(1e-12) * 1.05;
    let covered = test_predictions
        .iter()
        .zip(test_observed)
        .filter(|(prediction, target)| prediction.is_finite() && target.is_finite())
        .filter(|(prediction, target)| (**prediction - **target).abs() <= width)
        .count();
    let finite_test = test_predictions
        .iter()
        .zip(test_observed)
        .filter(|(prediction, target)| prediction.is_finite() && target.is_finite())
        .count();
    Calibration {
        nominal_coverage: coverage,
        conformal_width: width,
        empirical_coverage: if finite_test == 0 {
            0.0
        } else {
            covered as f64 / finite_test as f64
        },
        calib_points: residuals.len(),
        test_points: finite_test,
    }
}

fn complexity_spikes(values: &[f64], range: (usize, usize), config: &StudioConfig) -> Vec<usize> {
    let mut spikes = Vec::new();
    for index in range.0.max(1)..range.1 {
        if !values[index].is_finite() || !values[index - 1].is_finite() {
            continue;
        }
        let movement = (values[index] - values[index - 1]).abs();
        if movement <= config.spike_floor {
            continue;
        }
        let window_start = index.saturating_sub(config.spike_window).max(1);
        let mut prior_total = 0.0;
        let mut prior_count = 0usize;
        for previous in window_start..index {
            if values[previous].is_finite() && values[previous - 1].is_finite() {
                prior_total += (values[previous] - values[previous - 1]).abs();
                prior_count += 1;
            }
        }
        let prior_average = if prior_count == 0 {
            0.0
        } else {
            prior_total / prior_count as f64
        };
        if movement > config.spike_floor.max(config.spike_ratio * prior_average) {
            spikes.push(index);
        }
    }
    spikes
}

fn failure_nodes(
    values: &[f64],
    range: (usize, usize),
    state_diffs: &[f64],
    config: &StudioConfig,
) -> Vec<FailureNode> {
    let mut nodes = Vec::new();
    for (offset, index) in (range.0..range.1).enumerate() {
        if !values[index].is_finite() {
            nodes.push(FailureNode {
                step: index,
                reason: "non-finite input".to_owned(),
            });
            continue;
        }
        if state_diffs[offset] > config.divergence_bound {
            nodes.push(FailureNode {
                step: index,
                reason: "divergence bound exceeded".to_owned(),
            });
        }
    }
    nodes
}
