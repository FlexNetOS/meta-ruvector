# neural-trader-coherence

MinCut coherence gate, CUSUM drift detection, and proof-gated mutation for the RuVector Neural Trader (ADR-084).

## Overview

This crate provides the coherence gate that every memory write, model update, retrieval,
and actuation must pass through before proceeding in the neural-trader pipeline. It evaluates
the current market-graph state — a regime-dependent MinCut floor, a rolling CUSUM drift score,
embedding drift, and boundary stability — and returns a per-operation allow/deny decision.
It also defines the proof-gated mutation primitives (verified tokens and witness receipts)
used by downstream crates such as `neural-trader-replay` and `neural-trader-wasm`.

## Key API

- `CoherenceGate` — trait: `evaluate(&self, ctx: &GateContext) -> anyhow::Result<CoherenceDecision>`.
- `ThresholdGate` — default threshold-based gate built from a `GateConfig`.
- `GateConfig` — per-regime MinCut floors, CUSUM threshold, boundary-stability windows, max drift (has `Default`).
- `GateContext` — input state: MinCut value, partition hash, CUSUM/drift scores, `RegimeLabel`, boundary stability count.
- `CoherenceDecision` — per-operation flags (`allow_retrieve` / `allow_write` / `allow_learn` / `allow_act`) plus `reasons`; helpers `all_allowed()` and `fully_blocked()`.
- `RegimeLabel` — `Calm` / `Normal` / `Volatile`.
- `VerifiedToken`, `WitnessReceipt` — proof-gated mutation token and audit receipt.
- `WitnessLogger` — trait for appending witness receipts.

## License

MIT OR Apache-2.0

## Disclaimer

Research and experimental software. Not financial advice and not a recommendation to trade.
