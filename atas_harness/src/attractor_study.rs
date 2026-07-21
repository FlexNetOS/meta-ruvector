//! ARCHBP-009x — RED STUB. Contract surface only; the pre-registered attractor
//! study is unimplemented so the empirical gate fails closed before the real
//! study lands.

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
    pub min_relative_gain: f64,
}

impl StudyConfig {
    pub fn preregistered() -> Self {
        StudyConfig {
            series_len: 0,
            washout: 0,
            train_fraction: 0.0,
            calibration_fraction: 0.0,
            seeds: Vec::new(),
            reservoir_size: 0,
            spectral_radius: 0.0,
            leak_rate: 0.0,
            input_scale: 0.0,
            connectivity: 0.0,
            ridge_lambda: 0.0,
            min_relative_gain: 0.0,
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

pub fn run_attractor_study(_config: &StudyConfig) -> Result<StudyReport, String> {
    // RED: not implemented — return an optimistic, unproven verdict.
    Ok(StudyReport {
        results: Vec::new(),
        criteria: Vec::new(),
        attractor_median_gain: 0.0,
        stochastic_median_gain: 0.0,
        permits_downstream: true,
        verdict: "unimplemented".to_owned(),
    })
}
