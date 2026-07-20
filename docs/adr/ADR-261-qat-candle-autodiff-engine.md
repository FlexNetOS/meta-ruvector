# ADR-261 — QAT Engine on candle autodiff

**Status:** In progress (build) · **Date:** 2026-06-17 · **Crate:** `ruvllm`
**Context:** `qat/training_loop.rs` `QatTrainer` is a scaffold — losses + calibration exist, but there is **no model, no real forward, no backprop** (`forward_quantized` returns dummy zeros; `grad_norm = 0.0`). Owner decision: build the **full QAT engine on candle autodiff**.

## Decision
Build a real quantization-aware-training engine in `ruvllm` (feature `candle`) that:
1. owns a candle model whose trainable weights are `candle_nn::Var` in a `VarMap`,
2. applies **fake quantization** to weights in the forward pass via a candle `CustomOp1` that wraps the existing `DifferentiableQuantizer` (quantize→dequantize forward, **STE** backward),
3. computes loss, runs candle autograd `loss.backward()`, and steps an optimizer (`candle_nn::AdamW`),
4. reports real `grad_norm` from the gradients.

## Verified primitives (all present)
- candle-core `CustomOp1` (`cpu_fwd(storage, layout)`, `bwd(arg, res, grad_res)`) + `Tensor::apply_op1` (autograd-aware).
- candle-nn `VarMap`, `Var`, `Optimizer` trait, `AdamW`.
- ruvllm `qat::DifferentiableQuantizer` (`forward(&[f32])->(Vec<i8>,Vec<f32>)`, `backward(w,q,grad)->Vec<f32>`, `dequantize`) — the fake-quant + STE math.
- ruvllm `qat` losses (distillation, task, reasoning) + `CalibrationEngine` (scales).

## Keystone: `FakeQuant` (candle CustomOp1)
- `cpu_fwd`: read f32 weights → `quantizer.forward()` (quantize then dequantize) → return dequantized f32 (same shape). This makes the forward see quantized values.
- `bwd`: straight-through estimator — `quantizer.backward(w, q, grad_res)` (pass-through grad, clamped outside the representable range). Returns the grad wrt the original weight.
- Invoked via `weight_var.as_tensor().apply_op1(FakeQuant{quantizer})` so candle threads gradients to the `Var`.

## Build steps (each committed + verified)
1. **[this PR] `qat/candle_qat.rs`: `FakeQuant` CustomOp1 + unit test** (fwd quantizes, bwd STE grad flows). ← keystone
2. Trainable quantized model: `VarMap` weights + per-layer `FakeQuant` in forward (start with a small MLP/linear stack keyed by the quantizer layer names).
3. Wire into `QatTrainer`: add a model + `VarMap` + `AdamW`; thread through `run → train_epoch → train_step → forward_quantized` (real logits).
4. Real backward: `loss.backward()`, `optimizer.step()`, compute `grad_norm` from `VarMap` grads (replace the `0.0`/`32000` placeholders; vocab from model config).
5. Differential test: loss decreases over steps on a toy task; fake-quant matches the quantizer's PTQ output at step 0.

## No-downgrade / honesty
- The existing scaffold (losses, calibration, STE math) is reused, not replaced.
- `forward_quantized` becomes a **real** forward; until step 3 lands it stays the documented stub (not silently "done").
- Non-`candle` builds keep the scaffold (feature-gated); the engine is `#[cfg(feature = "candle")]`.

## References
- ADR-260 (single-app architecture; ruvllm = advanced engine, kept alongside shimmy)
- `docs/RUVECTOR-COMPLETENESS-AUDIT.md`
