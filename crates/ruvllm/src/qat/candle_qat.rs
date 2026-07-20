//! Candle-autodiff QAT engine (ADR-261).
//!
//! Keystone: [`FakeQuant`], a candle `CustomOp1` that bridges the Vec-based
//! [`DifferentiableQuantizer`] (straight-through estimator) into candle's
//! tensor autograd, so quantization-aware training can run with real gradients.
//!
//! Forward: quantize→dequantize the weights (the graph sees quantized values).
//! Backward: STE gradient from the wrapped quantizer (`dL/dw`).
//!
//! This module is only compiled with the `candle` feature.

use std::sync::Arc;

use candle_core::{CpuStorage, CustomOp1, Device, Layout, Result as CandleResult, Shape, Tensor, Var};
use candle_nn::{AdamW, Optimizer};

use crate::qat::differentiable_quant::DifferentiableQuantizer;

/// Fake-quantization op for candle autograd.
///
/// Wraps a [`DifferentiableQuantizer`]; `apply_op1` threads gradients back to the
/// latent weight `Var` via the straight-through estimator.
pub struct FakeQuant {
    quantizer: Arc<dyn DifferentiableQuantizer>,
}

impl FakeQuant {
    pub fn new(quantizer: Arc<dyn DifferentiableQuantizer>) -> Self {
        Self { quantizer }
    }
}

impl CustomOp1 for FakeQuant {
    fn name(&self) -> &'static str {
        "fake-quant"
    }

    fn cpu_fwd(&self, storage: &CpuStorage, layout: &Layout) -> CandleResult<(CpuStorage, Shape)> {
        // QAT weights are contiguous (they are `Var`s); read the contiguous window.
        let data = storage.as_slice::<f32>()?;
        let start = layout.start_offset();
        let n = layout.shape().elem_count();
        let w = &data[start..start + n];
        // forward() returns (quantized_indices, dequantized_values); the graph uses
        // the dequantized values so downstream ops see quantization error.
        let (_q_int, dequant) = self.quantizer.forward(w);
        Ok((CpuStorage::F32(dequant), layout.shape().clone()))
    }

    fn bwd(
        &self,
        arg: &Tensor,
        res: &Tensor,
        grad_res: &Tensor,
    ) -> CandleResult<Option<Tensor>> {
        // STE: dL/dw from the quantizer, given latent weights (arg), the quantized
        // output (res) and the upstream gradient (grad_res).
        let w = arg.flatten_all()?.to_vec1::<f32>()?;
        let q = res.flatten_all()?.to_vec1::<f32>()?;
        let g = grad_res.flatten_all()?.to_vec1::<f32>()?;
        let grad_w = self.quantizer.backward(&w, &q, &g);
        let grad = Tensor::from_vec(grad_w, arg.shape().clone(), arg.device())?;
        Ok(Some(grad))
    }
}

/// Apply fake quantization to `w` (autograd-aware).
pub fn fake_quantize(w: &Tensor, quantizer: Arc<dyn DifferentiableQuantizer>) -> CandleResult<Tensor> {
    w.apply_op1(FakeQuant::new(quantizer))
}

/// A fake-quantized MLP for quantization-aware training.
///
/// Each linear weight is a trainable latent `Var`; the forward pass fake-quantizes
/// it via [`FakeQuant`] (STE), so AdamW trains the latent weights while the forward
/// sees quantization error — the essence of QAT.
pub struct QuantizedMlp {
    weights: Vec<Var>,
    quantizer: Arc<dyn DifferentiableQuantizer>,
}

impl QuantizedMlp {
    /// `dims` = `[in, hidden.., out]`; one weight matrix `(out, in)` per adjacent pair.
    pub fn new(
        dims: &[usize],
        quantizer: Arc<dyn DifferentiableQuantizer>,
        device: &Device,
    ) -> CandleResult<Self> {
        let mut weights = Vec::new();
        for pair in dims.windows(2) {
            let (din, dout) = (pair[0], pair[1]);
            weights.push(Var::randn(0f32, 0.1f32, (dout, din), device)?);
        }
        Ok(Self { weights, quantizer })
    }

    /// Forward: `x` is `(batch, in)` -> `(batch, out)`, ReLU between hidden layers,
    /// every weight fake-quantized.
    pub fn forward(&self, x: &Tensor) -> CandleResult<Tensor> {
        let mut h = x.clone();
        let n = self.weights.len();
        for (i, w) in self.weights.iter().enumerate() {
            let wq = fake_quantize(w.as_tensor(), self.quantizer.clone())?;
            h = h.matmul(&wq.t()?)?;
            if i + 1 < n {
                h = h.relu()?;
            }
        }
        Ok(h)
    }

    /// Trainable latent variables for the optimizer.
    pub fn vars(&self) -> Vec<Var> {
        self.weights.clone()
    }
}

/// Quantization-aware training on an MSE objective. Returns per-step loss.
///
/// The full engine: fake-quant forward → candle autograd backward through the STE
/// → AdamW update of the latent weights. A decreasing loss proves real learning.
pub fn train_qat_mse(
    model: &QuantizedMlp,
    x: &Tensor,
    y: &Tensor,
    steps: usize,
    lr: f64,
) -> CandleResult<Vec<f32>> {
    let mut opt = AdamW::new_lr(model.vars(), lr)?;
    let mut losses = Vec::with_capacity(steps);
    for _ in 0..steps {
        let pred = model.forward(x)?;
        let loss = pred.sub(y)?.sqr()?.mean_all()?;
        opt.backward_step(&loss)?;
        losses.push(loss.to_scalar::<f32>()?);
    }
    Ok(losses)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::qat::differentiable_quant::create_quantizer;
    use crate::qat::config::QatConfig;
    use candle_core::{Device, Tensor, Var};

    #[test]
    fn fake_quant_forward_quantizes_and_grad_flows() {
        let dev = Device::Cpu;
        let q: Arc<dyn DifferentiableQuantizer> = Arc::from(create_quantizer(&QatConfig::default()));

        // A latent-weight Var; fake-quant it, sum, backprop.
        let w = Var::from_vec(vec![0.10f32, -0.37, 0.92, -0.05], (4,), &dev).unwrap();
        let wt = w.as_tensor();
        let qw = fake_quantize(wt, q.clone()).unwrap();

        // forward produced finite, same-shape values (quantization applied).
        let out = qw.flatten_all().unwrap().to_vec1::<f32>().unwrap();
        assert_eq!(out.len(), 4);
        assert!(out.iter().all(|v| v.is_finite()));

        // backward: gradient reaches the latent weight (STE, not all-zero).
        let loss = qw.sqr().unwrap().sum_all().unwrap();
        let grads = loss.backward().unwrap();
        let gw = grads.get(&w).expect("grad for weight var");
        let gv = gw.flatten_all().unwrap().to_vec1::<f32>().unwrap();
        assert_eq!(gv.len(), 4);
        assert!(gv.iter().any(|v| v.abs() > 0.0), "STE gradient should flow");
    }

    #[test]
    fn qat_mse_training_loss_decreases() {
        let dev = Device::Cpu;
        // Calibrate the quantizer scale to the weight range (~N(0, 0.1)); the
        // default scale of 1.0 would snap every small weight to 0 (dead network).
        let mut quant = create_quantizer(&QatConfig::default());
        quant.set_scale(0.05);
        let q: Arc<dyn DifferentiableQuantizer> = Arc::from(quant);
        let model = QuantizedMlp::new(&[8, 16, 4], q, &dev).unwrap();
        let x = Tensor::randn(0f32, 1.0f32, (32, 8), &dev).unwrap();
        let y = Tensor::randn(0f32, 1.0f32, (32, 4), &dev).unwrap();

        let losses = train_qat_mse(&model, &x, &y, 100, 1e-2).unwrap();

        assert_eq!(losses.len(), 100);
        assert!(losses.iter().all(|l| l.is_finite()));
        // The QAT engine actually learns through the fake-quant STE.
        assert!(
            losses[losses.len() - 1] < losses[0],
            "QAT loss should decrease: {} -> {}",
            losses[0],
            losses[losses.len() - 1]
        );
    }
}
