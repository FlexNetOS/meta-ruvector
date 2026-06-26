# neural-trader-strategies

Venue-agnostic strategy and risk-gate runtime for the RuVector Neural Trader (ADR-153).

## Overview

This crate hosts the strategy runtime for the neural-trader stack. Strategies consume canonical
`neural_trader_core::MarketEvent` values and hold no venue-specific state, so the same strategy
runs in paper replay and against a live venue (e.g. Kalshi) that normalizes to `MarketEvent`.
Every strategy emits an `Intent`, which must pass through the mandatory `RiskGate` (position cap,
daily-loss kill, concentration, min-edge, live-trade env flag) and an optional coherence bridge to
`neural-trader-coherence` before any order reaches the exchange.

## Key API

- `Strategy` — trait: `name()` and `on_event(&mut self, &MarketEvent) -> Option<Intent>`.
- `Intent`, `Action`, `Side` — canonical, venue-agnostic trade request and its enums.
- `RiskGate`, `RiskConfig`, `RiskDecision`, `RejectReason` — mandatory pre-trade risk wrapper around every `Intent`.
- `PortfolioState`, `Position` — portfolio inputs the risk gate evaluates against.
- `ExpectedValueKelly` / `ExpectedValueKellyConfig` — fractional-Kelly sizing from externally supplied priors.
- `AttentionScalper` / `AttentionScalperConfig` — order-book imbalance scalper (geometric decay or `ruvector-attention` SDPA).
- `CoherenceArb` / `CoherenceArbConfig` — cross-market divergence arbitrage over `(reference, mirror)` pairs.
- `CoherenceChecker` / `CoherenceOutcome` — bridge to the `neural-trader-coherence` gate (re-exports `CoherenceGate`, `ThresholdGate`, `GateConfig`, `GateContext`, `RegimeLabel`).

## License

MIT OR Apache-2.0

## Disclaimer

Research and experimental software. Not financial advice and not a recommendation to trade.
