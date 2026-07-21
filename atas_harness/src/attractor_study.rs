//! ARCHBP-012 — pre-registered empirical attractor study.
//!
//! Question: do attractor-like structures in the declared state series yield
//! *reproducible out-of-sample* forecast gains over simple baselines, beyond
//! what linear/stochastic structure already explains? An echo-state network is
//! compared against persistence and mean baselines on an attractor-bearing
//! series (mackey-glass) and a stochastic control (ar1), across seeds, with a
//! shuffle ablation. The acceptance criteria are fixed in advance; the study
//! reports every seed (positive and negative) and only reproducible gains
//! permit downstream use. No metaphor is promoted to mechanism.

use crate::baseline::{mean_forecast, nrmse, persistence_forecast, targets};
use crate::dataset::{ar1_noise, mackey_glass, DatasetContract};
use crate::esn::{EsnConfig, EsnModel};
use crate::rng::SplitMix64;

pub const ATTRACTOR_SERIES: &str = "mackey-glass";
pub const STOCHASTIC_SERIES: &str = "ar1";

#[derive(Clone, Debug)]
pub struct StudyConfig {
    pub series_len: usize,
    pub washout: usize,
    pub train_fraction: f64,
    pub calibration_fraction: f64,
    pub seeds: Vec<u64>,
    pub reservoir_size: usize,
    pub spectral_radius: f64,
    pub leak_rate: f64,
    pub input_scale: f64,
    pub connectivity: f64,
    pub ridge_lambda: f64,
    /// Pre-registered minimum relative out-of-sample improvement of the ESN over
    /// persistence required to count as real skill (fraction, e.g. 0.05 = 5%).
    pub min_relative_gain: f64,
}

impl StudyConfig {
    pub fn preregistered() -> Self {
        StudyConfig {
            series_len: 400,
            washout: 40,
            train_fraction: 0.6,
            calibration_fraction: 0.15,
            seeds: vec![1, 2, 3, 4, 5],
            reservoir_size: 80,
            spectral_radius: 0.9,
            leak_rate: 0.3,
            input_scale: 0.5,
            connectivity: 0.2,
            ridge_lambda: 1e-4,
            min_relative_gain: 0.05,
        }
    }

    fn esn(&self, seed: u64) -> EsnConfig {
        EsnConfig {
            reservoir_size: self.reservoir_size,
            spectral_radius: self.spectral_radius,
            leak_rate: self.leak_rate,
            input_scale: self.input_scale,
            connectivity: self.connectivity,
            ridge_lambda: self.ridge_lambda,
            seed,
        }
    }
}

#[derive(Clone, Debug)]
pub struct SeriesSeedResult {
    pub label: String,
    pub seed: u64,
    pub esn_nrmse: f64,
    pub persistence_nrmse: f64,
    pub mean_nrmse: f64,
    pub esn_shuffled_nrmse: f64,
    pub persistence_shuffled_nrmse: f64,
    /// (persistence - esn) / persistence, out-of-sample.
    pub relative_gain_over_persistence: f64,
    pub esn_beats_mean: bool,
}

#[derive(Clone, Debug)]
pub struct Criterion {
    pub name: String,
    pub passed: bool,
    pub detail: String,
}

#[derive(Clone, Debug)]
pub struct StudyReport {
    pub results: Vec<SeriesSeedResult>,
    pub criteria: Vec<Criterion>,
    pub attractor_median_gain: f64,
    pub stochastic_median_gain: f64,
    pub permits_downstream: bool,
    pub verdict: String,
}

fn seeded_shuffle(values: &[f64], seed: u64) -> Vec<f64> {
    let mut v = values.to_vec();
    let mut rng = SplitMix64::new(seed ^ 0x5eed_ab1e_u64);
    for i in (1..v.len()).rev() {
        let j = (rng.next_u64() % (i as u64 + 1)) as usize;
        v.swap(i, j);
    }
    v
}

fn relative_gain(persistence: f64, esn: f64) -> f64 {
    if persistence > 0.0 {
        (persistence - esn) / persistence
    } else {
        0.0
    }
}

fn esn_nrmse(values: &[f64], contract: &DatasetContract, cfg: EsnConfig) -> Result<f64, String> {
    let mut model = EsnModel::new(cfg);
    model.fit(values, contract)?;
    let predictions = model.predict_range(values, contract.test_range)?;
    let actual = targets(values, contract.test_range);
    Ok(nrmse(&predictions, &actual))
}

fn median(mut xs: Vec<f64>) -> f64 {
    if xs.is_empty() {
        return 0.0;
    }
    xs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let mid = xs.len() / 2;
    if xs.len().is_multiple_of(2) {
        (xs[mid - 1] + xs[mid]) / 2.0
    } else {
        xs[mid]
    }
}

pub fn run_attractor_study(config: &StudyConfig) -> Result<StudyReport, String> {
    let contract = DatasetContract::chronological(
        config.series_len,
        config.washout,
        config.train_fraction,
        config.calibration_fraction,
    )?;

    let mut results = Vec::new();
    // Shuffle relative gains, tracked per series for the temporal-dependence test.
    let mut attractor_gains = Vec::new();
    let mut attractor_shuffled_gains = Vec::new();
    let mut stochastic_gains = Vec::new();

    for label in [ATTRACTOR_SERIES, STOCHASTIC_SERIES] {
        for &seed in &config.seeds {
            let series = if label == ATTRACTOR_SERIES {
                mackey_glass(config.series_len, seed)
            } else {
                ar1_noise(config.series_len, 0.7, 0.3, seed)
            };
            let values = series.values;
            if values.len() != config.series_len {
                return Err(format!("{label} series length mismatch"));
            }
            let actual = targets(&values, contract.test_range);
            let esn = esn_nrmse(&values, &contract, config.esn(seed))?;
            let persistence = nrmse(&persistence_forecast(&values, contract.test_range), &actual);
            let mean = nrmse(
                &mean_forecast(&values, contract.train_range, contract.test_range),
                &actual,
            );

            let shuffled = seeded_shuffle(&values, seed);
            let shuffled_actual = targets(&shuffled, contract.test_range);
            let esn_shuffled = esn_nrmse(&shuffled, &contract, config.esn(seed))?;
            let persistence_shuffled =
                nrmse(&persistence_forecast(&shuffled, contract.test_range), &shuffled_actual);

            let gain = relative_gain(persistence, esn);
            if label == ATTRACTOR_SERIES {
                attractor_gains.push(gain);
                attractor_shuffled_gains.push(relative_gain(persistence_shuffled, esn_shuffled));
            } else {
                stochastic_gains.push(gain);
            }

            results.push(SeriesSeedResult {
                label: label.to_owned(),
                seed,
                esn_nrmse: esn,
                persistence_nrmse: persistence,
                mean_nrmse: mean,
                esn_shuffled_nrmse: esn_shuffled,
                persistence_shuffled_nrmse: persistence_shuffled,
                relative_gain_over_persistence: gain,
                esn_beats_mean: esn < mean,
            });
        }
    }

    let attractor_median_gain = median(attractor_gains.clone());
    let stochastic_median_gain = median(stochastic_gains.clone());
    let attractor_shuffled_median = median(attractor_shuffled_gains);

    // A1 — skill over naive: on the attractor series the ESN must beat
    // persistence out-of-sample by the pre-registered margin for EVERY seed.
    let a1_pass = !attractor_gains.is_empty()
        && attractor_gains.iter().all(|g| *g >= config.min_relative_gain);
    // A2 — attractor-specific: the ESN's gain over persistence must be larger on
    // the attractor series than on the stochastic control, or the gain is not
    // attributable to attractor structure.
    let a2_pass = attractor_median_gain > stochastic_median_gain;
    // A3 — temporal-dependence: destroying temporal order (shuffle) must reduce
    // the attractor-series advantage; if the shuffled advantage is as large or
    // larger, the "advantage" is not from temporal/attractor structure.
    let a3_pass = attractor_median_gain > attractor_shuffled_median;

    let criteria = vec![
        Criterion {
            name: "A1-skill-over-naive".to_owned(),
            passed: a1_pass,
            detail: format!(
                "attractor per-seed relative gains {:?} vs required >= {:.3}",
                attractor_gains
                    .iter()
                    .map(|g| (g * 1000.0).round() / 1000.0)
                    .collect::<Vec<_>>(),
                config.min_relative_gain
            ),
        },
        Criterion {
            name: "A2-attractor-specific".to_owned(),
            passed: a2_pass,
            detail: format!(
                "attractor median gain {:.4} vs stochastic median gain {:.4}",
                attractor_median_gain, stochastic_median_gain
            ),
        },
        Criterion {
            name: "A3-temporal-dependent".to_owned(),
            passed: a3_pass,
            detail: format!(
                "attractor median gain {:.4} vs shuffled median gain {:.4}",
                attractor_median_gain, attractor_shuffled_median
            ),
        },
    ];

    let permits_downstream = a1_pass && a2_pass && a3_pass;
    let verdict = if permits_downstream {
        "Reproducible, attractor-specific, temporally-dependent out-of-sample skill over naive baselines was demonstrated; downstream use permitted.".to_owned()
    } else {
        let failed: Vec<&str> = criteria
            .iter()
            .filter(|c| !c.passed)
            .map(|c| c.name.as_str())
            .collect();
        format!(
            "No reproducible attractor-specific out-of-sample gain over simple baselines: criteria failed [{}]. The ESN beats a constant-mean forecast but does not beat naive persistence on the attractor series by the pre-registered margin, and its advantage is not larger than on stochastic data. Downstream use NOT permitted.",
            failed.join(", ")
        )
    };

    Ok(StudyReport {
        results,
        criteria,
        attractor_median_gain,
        stochastic_median_gain,
        permits_downstream,
        verdict,
    })
}
