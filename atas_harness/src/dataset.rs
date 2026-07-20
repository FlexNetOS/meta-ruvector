//! Deterministic time-series fixtures and leakage-proof split contracts.

use crate::rng::SplitMix64;

#[derive(Clone, Debug)]
pub struct SeriesDataset {
    pub name: String,
    pub seed: u64,
    pub values: Vec<f64>,
}

impl SeriesDataset {
    pub fn from_values(name: impl Into<String>, seed: u64, values: Vec<f64>) -> Self {
        Self {
            name: name.into(),
            seed,
            values,
        }
    }
}

/// A strictly chronological split. Range ends are exclusive input indices.
#[derive(Clone, Debug)]
pub struct DatasetContract {
    pub series_len: usize,
    pub washout: usize,
    pub train_range: (usize, usize),
    pub calib_range: (usize, usize),
    pub test_range: (usize, usize),
}

impl DatasetContract {
    pub fn chronological(
        series_len: usize,
        washout: usize,
        train_fraction: f64,
        calibration_fraction: f64,
    ) -> Result<Self, String> {
        if series_len < 4 || washout >= series_len.saturating_sub(2) {
            return Err("series is too short for washout and one-step targets".to_owned());
        }
        if !(0.0 < train_fraction && train_fraction < 1.0)
            || !(0.0 < calibration_fraction && calibration_fraction < 1.0)
            || train_fraction + calibration_fraction >= 1.0
        {
            return Err("split fractions must be positive and leave a test partition".to_owned());
        }

        // The final usable input must have a future target at index `series_len - 1`.
        let usable_end = series_len - 1;
        let usable = usable_end - washout;
        let train_len = ((usable as f64) * train_fraction).floor() as usize;
        let calibration_len = ((usable as f64) * calibration_fraction).floor() as usize;
        let train_end = washout + train_len.max(1);
        let calibration_end = (train_end + calibration_len.max(1)).min(usable_end);
        let contract = Self {
            series_len,
            washout,
            train_range: (washout, train_end),
            calib_range: (train_end, calibration_end),
            test_range: (calibration_end, usable_end),
        };
        contract.validate()?;
        Ok(contract)
    }

    pub fn validate(&self) -> Result<(), String> {
        let (train_start, train_end) = self.train_range;
        let (calibration_start, calibration_end) = self.calib_range;
        let (test_start, test_end) = self.test_range;
        if self.series_len < 3 || self.washout >= self.series_len {
            return Err("invalid series length or washout".to_owned());
        }
        if train_start < self.washout
            || train_start >= train_end
            || calibration_start >= calibration_end
            || test_start >= test_end
        {
            return Err("every split must be non-empty and start after washout".to_owned());
        }
        if train_end > calibration_start || calibration_end > test_start {
            return Err("dataset splits overlap or are not chronological".to_owned());
        }
        if test_end > self.series_len - 1 {
            return Err("test inputs must leave one future target in the series".to_owned());
        }
        Ok(())
    }
}

/// Deterministic, delay-coupled nonlinear series used as an ESN fixture.
pub fn mackey_glass(len: usize, seed: u64) -> SeriesDataset {
    let mut rng = SplitMix64::new(seed);
    let mut values = Vec::with_capacity(len);
    for _ in 0..len.min(18) {
        values.push(1.1 + rng.next_uniform(-0.04, 0.04));
    }
    while values.len() < len {
        let i = values.len();
        let previous = values[i - 1];
        let delayed = values[i - 18];
        let drift = 0.18 * delayed / (1.0 + delayed.powi(10)) - 0.1 * previous;
        values.push(previous + 0.1 * drift + rng.next_uniform(-0.001, 0.001));
    }
    SeriesDataset::from_values("mackey-glass", seed, values)
}

/// Deterministic AR(1) noise with a finite innovation stream.
pub fn ar1_noise(len: usize, phi: f64, sigma: f64, seed: u64) -> SeriesDataset {
    let mut rng = SplitMix64::new(seed);
    let mut values = Vec::with_capacity(len);
    let mut current = 0.0;
    for _ in 0..len {
        current = phi * current + sigma * rng.next_gauss();
        values.push(current);
    }
    SeriesDataset::from_values("ar1", seed, values)
}
