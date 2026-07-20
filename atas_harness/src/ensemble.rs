//! Deterministic ensemble aggregation for independently seeded reservoirs.

use crate::{
    dataset::DatasetContract,
    esn::{EsnConfig, EsnModel},
};

#[derive(Clone, Debug)]
pub struct EnsembleConfig {
    pub base: EsnConfig,
    pub members: usize,
}

#[derive(Clone, Debug)]
pub struct EnsembleResult {
    pub member_preds: Vec<Vec<f64>>,
    pub mean: Vec<f64>,
    pub variance: Vec<f64>,
}

pub fn run_ensemble(
    config: &EnsembleConfig,
    values: &[f64],
    contract: &DatasetContract,
    range: (usize, usize),
) -> Result<EnsembleResult, String> {
    if config.members == 0 {
        return Err("an ensemble requires at least one member".to_owned());
    }
    contract.validate()?;
    let mut member_preds = Vec::with_capacity(config.members);
    for member in 0..config.members {
        let mut member_config = config.base.clone();
        member_config.seed = member_config.seed.wrapping_add(member as u64);
        let mut model = EsnModel::new(member_config);
        model.fit(values, contract)?;
        member_preds.push(model.predict_range(values, range)?);
    }
    let count = member_preds[0].len();
    let mut mean = vec![0.0; count];
    let mut variance = vec![0.0; count];
    for index in 0..count {
        mean[index] = member_preds
            .iter()
            .map(|prediction| prediction[index])
            .sum::<f64>()
            / config.members as f64;
        variance[index] = member_preds
            .iter()
            .map(|prediction| (prediction[index] - mean[index]).powi(2))
            .sum::<f64>()
            / config.members as f64;
    }
    Ok(EnsembleResult {
        member_preds,
        mean,
        variance,
    })
}
