# neural-trader-replay

Witnessable replay segments, RVF serialization stubs, and audit receipt logging for the RuVector Neural Trader (ADR-084).

## Overview

This crate defines the selective, bounded replay memory for the neural-trader stack. It models
sealed replay segments — compact market-event windows tagged with an embedding snapshot, realized
labels, coherence statistics, and lineage — and the trait surface for storing and retrieving them.
Writes are admitted only when a `CoherenceDecision` from `neural-trader-coherence` allows them, and
witness receipts are logged for auditability. It builds on `neural-trader-core` (`MarketEvent`) and
`neural-trader-coherence`, and is consumed by `neural-trader-wasm`.

## Key API

- `ReplaySegment` — a sealed segment: events, optional embedding, labels, `CoherenceStats`, `SegmentLineage`, witness hash.
- `SegmentKind` — segment classification (`HighUncertainty`, `LargeImpact`, `RegimeTransition`, `StructuralAnomaly`, `RareQueuePattern`, `HeadDisagreement`, `Routine`).
- `CoherenceStats`, `SegmentLineage` — coherence snapshot and origin metadata captured at write time.
- `MemoryStore` — trait: `retrieve(&MemoryQuery)` and `maybe_write(seg, &CoherenceDecision)` (gated admission).
- `MemoryQuery` — symbol, embedding, optional regime, and result limit.
- `ReservoirStore` — bounded reservoir store with O(1) front eviction (`VecDeque`-backed).
- `InMemoryReceiptLog` — in-memory `WitnessLogger` implementation for testing and research.

## License

MIT OR Apache-2.0

## Disclaimer

Research and experimental software. Not financial advice and not a recommendation to trade.
