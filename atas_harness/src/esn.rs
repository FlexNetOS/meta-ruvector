//! Fixed-reservoir echo-state model with a ridge-trained linear readout.

use crate::{
    dataset::DatasetContract,
    linalg::ridge_solve,
    rng::{Fnv1a, SplitMix64},
};

#[derive(Clone, Debug)]
pub struct EsnConfig {
    pub reservoir_size: usize,
    pub spectral_radius: f64,
    pub leak_rate: f64,
    pub input_scale: f64,
    pub connectivity: f64,
    pub ridge_lambda: f64,
    pub seed: u64,
}

#[derive(Clone, Debug)]
pub struct Reservoir {
    weights: Vec<Vec<f64>>,
    input_weights: Vec<f64>,
    bias: Vec<f64>,
    leak_rate: f64,
}

impl Reservoir {
    pub fn fingerprint(&self) -> u64 {
        let mut hash = Fnv1a::new();
        hash.write_f64(self.leak_rate);
        for row in &self.weights {
            for weight in row {
                hash.write_f64(*weight);
            }
        }
        for weight in &self.input_weights {
            hash.write_f64(*weight);
        }
        for bias in &self.bias {
            hash.write_f64(*bias);
        }
        hash.finish()
    }

    fn advance(&self, state: &mut [f64], input: f64) -> f64 {
        let input = if input.is_finite() { input } else { 0.0 };
        let previous = state.to_vec();
        let mut movement = 0.0;
        for index in 0..state.len() {
            let recurrent = self.weights[index]
                .iter()
                .zip(&previous)
                .map(|(weight, value)| weight * value)
                .sum::<f64>();
            let activated =
                (recurrent + self.input_weights[index] * input + self.bias[index]).tanh();
            let next = (1.0 - self.leak_rate) * previous[index] + self.leak_rate * activated;
            movement += (next - previous[index]).abs();
            state[index] = next;
        }
        movement / state.len().max(1) as f64
    }
}

#[derive(Clone, Debug)]
pub struct Readout {
    pub weights: Vec<f64>,
}

#[derive(Clone, Debug)]
pub struct EsnModel {
    config: EsnConfig,
    pub reservoir: Reservoir,
    pub readout: Option<Readout>,
}

impl EsnModel {
    pub fn new(config: EsnConfig) -> Self {
        let size = config.reservoir_size.max(1);
        let mut rng = SplitMix64::new(config.seed);
        let radius_scale = config.spectral_radius / (size as f64).sqrt();
        let mut weights = vec![vec![0.0; size]; size];
        for row in &mut weights {
            for weight in row {
                if rng.next_f64() <= config.connectivity.clamp(0.0, 1.0) {
                    *weight = rng.next_uniform(-1.0, 1.0) * radius_scale;
                }
            }
        }
        let input_weights = (0..size)
            .map(|_| rng.next_uniform(-config.input_scale, config.input_scale))
            .collect();
        let bias = (0..size).map(|_| rng.next_uniform(-0.01, 0.01)).collect();
        Self {
            reservoir: Reservoir {
                weights,
                input_weights,
                bias,
                leak_rate: config.leak_rate.clamp(0.0, 1.0),
            },
            config,
            readout: None,
        }
    }

    pub fn fit(&mut self, values: &[f64], contract: &DatasetContract) -> Result<(), String> {
        contract.validate()?;
        if values.len() != contract.series_len {
            return Err("series length does not match dataset contract".to_owned());
        }
        let mut state = vec![0.0; self.reservoir.weights.len()];
        let mut rows = Vec::new();
        let mut targets = Vec::new();
        for index in 0..contract.train_range.1 {
            self.reservoir.advance(&mut state, values[index]);
            if index >= contract.train_range.0 {
                rows.push(features(&state, values[index]));
                targets.push(values[index + 1]);
            }
        }
        self.readout = Some(Readout {
            weights: ridge_solve(&rows, &targets, self.config.ridge_lambda)?,
        });
        Ok(())
    }

    pub fn predict_range(&self, values: &[f64], range: (usize, usize)) -> Result<Vec<f64>, String> {
        Ok(self.predict_with_diffs(values, range)?.0)
    }

    pub fn predict_with_diffs(
        &self,
        values: &[f64],
        range: (usize, usize),
    ) -> Result<(Vec<f64>, Vec<f64>), String> {
        let readout = self
            .readout
            .as_ref()
            .ok_or_else(|| "model is not fitted".to_owned())?;
        if range.1 > values.len().saturating_sub(1) || range.0 >= range.1 {
            return Err("prediction range has no future targets".to_owned());
        }
        let mut state = vec![0.0; self.reservoir.weights.len()];
        let mut predictions = Vec::with_capacity(range.1 - range.0);
        let mut diffs = Vec::with_capacity(range.1 - range.0);
        for index in 0..range.1 {
            let movement = self.reservoir.advance(&mut state, values[index]);
            if index >= range.0 {
                let feature = features(&state, values[index]);
                let prediction = feature
                    .iter()
                    .zip(&readout.weights)
                    .map(|(value, weight)| value * weight)
                    .sum();
                predictions.push(prediction);
                diffs.push(movement);
            }
        }
        Ok((predictions, diffs))
    }
}

fn features(state: &[f64], input: f64) -> Vec<f64> {
    let mean_state = state.iter().copied().sum::<f64>() / state.len().max(1) as f64;
    vec![1.0, if input.is_finite() { input } else { 0.0 }, mean_state]
}
