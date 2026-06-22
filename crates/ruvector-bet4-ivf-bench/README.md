# ruvector-bet4-ivf-bench

Frozen reproducibility benchmark for **BET 4** (SepRAG, ruvnet/RuVector #534):
does **lower-bound-ordered branch-and-bound IVF probing** beat a tuned plain
`IvfFlat` `nprobe` on unfiltered ANN over real 128-d embeddings, at matched
recall@10?

This closes the BET 4 caveat left open by [ADR-201](../../docs/adr): the
region-pruning IVF kernel was previously only run against ACORN (BET 2), never
head-to-head against its natural incumbent — plain IVF `nprobe`. The
branch-and-bound kernel is rebuilt self-contained here over the same
`ruvector-rairs` k-means substrate as the incumbent. Frozen pre-registration
gate: `docs/plans/bet4-ivf-pruning/PRE-REGISTRATION.md`.

This crate is `publish = false` — it is a benchmark harness, not a library
release.

## Modules and exports

- `kernel` — `BnBIvf`, the LB-ordered branch-and-bound IVF prober.
- `pq` — `PqIvf` (product-quantized IVF) and `AdcCost`.
- `data` — embedding corpus loading / generation.
- `oracle` — ground-truth top-k for recall measurement.
- `pca` — dimensionality reduction support.

Built on [`ruvector-rairs`](../ruvector-rairs) for the k-means / IVF substrate.

## License

MIT
