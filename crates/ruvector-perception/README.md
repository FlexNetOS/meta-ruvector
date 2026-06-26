# ruvector-perception

The layer under classification: physical delta → boundary → coherence → proof → action — a trusted-physical-memory engine that emits structured delta witnesses, not class labels.

## Overview

`ruvector-perception` builds the substrate underneath a classifier rather than a better classifier. Each reading becomes a delta against a rolling multi-modal baseline; zones form a coherence graph where a dynamic min-cut (reusing `ruvector-mincut`) isolates the boundary that moved; modality contradictions are first-class signals; and a proof gate turns novelty/coherence/contradiction into bounded authority (Ignore → Observe → Alert → Mutate) with an auditable SHA-256 evidence chain. It is a research-tier perception layer demonstrated on synthetic multi-modal deltas, not validated on real CSI hardware.

## Key API

- `DeltaEngine`, `EngineConfig` — observe readings and emit delta witnesses.
- `Reading`, `WorldState`, `Modality`, `Physics` — multi-modal input and baseline state.
- `detect_boundary`, `Boundary` — min-cut boundary detection.
- `DeltaWitness`, `ProofGate`, `Action`, `evidence_hash`, `novelty_level` — proof gating and the evidence chain.
- `Absence`, `SequenceMonitor` — detection of missing expected continuations.
- `CustodyLedger`, `CustodyRecord`, `CustodyError` — auditable custody trail.
- `RealityGraph`, `Query`, `GroundedAnswer` — grounded reality queries.
- `rank_hypotheses`, `Hypothesis`, `RankedHypothesis`, `DisagreementInput` — contradiction-weighted hypothesis ranking.
- `IdentityMemory`, `IdentityDrift`; `BoundaryPredictor`, `BoundaryForecast`; `TopologyManager`, `NodeRole`; `NervousSystemNode`; `CaptchaVerifier` / `RealityProof`; `FacilityGraph`, `FragilityReport`.
- `VERSION` — crate version constant.

## License

MIT
