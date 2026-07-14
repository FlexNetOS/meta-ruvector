# photonlayer-ruvector

PhotonLayer experiment memory on RuVector: mask/frame embeddings, similarity recall, pass/fail boundary, coherence, and RVF receipts (ADR-260 Phase 2).

## Overview

`photonlayer-ruvector` is the experiment-memory and verification substrate for PhotonLayer
optical simulations (ADR-260 §5, §11–§15). It uses RuVector as a dedicated experiment-memory
layer — not a generic data store — building L2-normalised embeddings from mask phase-histograms
and detector-frame spectra, recalling the nearest prior experiments by cosine similarity, and
analysing what separates passing from failing runs. It sits alongside `photonlayer-core` and is
consumed by `photonlayer-cli` within the meta-ruvector workspace, with spectral coherence backed
by `ruvector-coherence`.

## Key API

- `experiment_embedding`, `mask_embedding` — build 32-dim experiment / mask embeddings from masks and frames.
- `ExperimentMemory`, `ExperimentRecord`, `NearestHit`, `MaskSearchHit` — in-memory store with nearest-experiment recall.
- `explain_boundary`, `BoundaryReport` — Fiedler spectral partitioning to find the `OpticalConfig` variable that best separates pass/fail outcomes.
- `mask_family_coherence`, `FamilyCoherence` — spectral-gap coherence of a mask-family similarity graph (gates demo promotion).
- `ReceiptStore` — JSON persistence and binding-digest verification of RVF-style experiment receipts.
- `version()` — crate version string.

## License

MIT
