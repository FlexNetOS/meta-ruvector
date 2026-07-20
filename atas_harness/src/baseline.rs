//! Naive baselines used to keep ESN reports grounded.

pub fn targets(values: &[f64], range: (usize, usize)) -> Vec<f64> {
    (range.0..range.1).map(|index| values[index + 1]).collect()
}

pub fn persistence_forecast(values: &[f64], range: (usize, usize)) -> Vec<f64> {
    values[range.0..range.1].to_vec()
}

pub fn mean_forecast(
    values: &[f64],
    train_range: (usize, usize),
    forecast_range: (usize, usize),
) -> Vec<f64> {
    let training = &values[train_range.0..train_range.1];
    let mean = training.iter().copied().sum::<f64>() / training.len() as f64;
    vec![mean; forecast_range.1 - forecast_range.0]
}

pub fn nrmse(predictions: &[f64], actual: &[f64]) -> f64 {
    let pairs = predictions
        .iter()
        .copied()
        .zip(actual.iter().copied())
        .filter(|(prediction, observed)| prediction.is_finite() && observed.is_finite())
        .collect::<Vec<_>>();
    if pairs.is_empty() {
        return 0.0;
    }
    let mse = pairs
        .iter()
        .map(|(prediction, observed)| (prediction - observed).powi(2))
        .sum::<f64>()
        / pairs.len() as f64;
    let (mut minimum, mut maximum) = (f64::INFINITY, f64::NEG_INFINITY);
    for (_, observed) in &pairs {
        minimum = minimum.min(*observed);
        maximum = maximum.max(*observed);
    }
    let scale = (maximum - minimum).abs().max(1e-12);
    mse.sqrt() / scale
}
