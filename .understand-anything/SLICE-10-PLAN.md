# Slice 10 — "Other" Sub-Slice Plan

Total: 1505 files across 97 crates after slices 1-9 + 6a/6b/6c.

## Sub-slices

| ID | Theme | Crates | ~Files |
|---|---|---|---|
| **10a** | ruvix (single largest crate) | ruvix | 300 |
| **10b** | Cognition & symbol | prime-radiant, ruvector-nervous-system{,-wasm}, sona | ~250 |
| **10c** | Quantum + HW accel | ruQu, ruqu-{core,algorithms,exotic,wasm}, ruvector-hailo{,-cluster}, hailort-sys, ruvector-sparse-inference{,-wasm}, ruvector-fpga-transformer{,-wasm}, ruvector-mmwave | ~250 |
| **10d** | LLM/decompiler/solver | ruvllm-cli, ruvllm-wasm, ruvllm_retrieval_diffusion, ruvllm_sparse_attention, ruvector-decompiler{,-wasm}, ruvector-solver{,-node,-wasm}, ruvector-temporal-tensor{,-wasm}, ruvector-graph-transformer{,-node,-wasm} | ~200 |
| **10e** | Distributed/storage | ruvector-delta-{consensus,core,graph,index,wasm}, ruvector-replication, ruvector-raft, ruvector-snapshot, ruvector-coherence, ruvector-rulake, ruvector-rairs, ruvector-tiny-dancer-{core,node,wasm}, ruvector-verified{,-wasm}, ruvector-kalshi, ruvector-acorn{,-wasm} | ~250 |
| **10f** | WASM/node satellites + misc | all remaining *-wasm / *-node satellites + misc utility crates | ~250 |

## Execution policy

Per sub-slice:
1. Update `.understand-anything/.understandignore` to scope only the crates in this sub-slice
2. Project-scanner → fingerprints.slice-10X.json + file inventory
3. Split into ~22-file batches; dispatch file-analyzers in waves of 4-5 parallel
4. Merge batches → assemble-reviewer → architecture-analyzer → tour-builder → graph-reviewer
5. Cleanup pass (drop dangling, normalize types)
6. Save as `slice-10X-knowledge-graph.json`

After all 10a-10f done: merge-subdomain-graphs.py → master + meta.json update with `slicesPresent: ["1","2","3","4","5a","5b","6a","6b","6c","7","8","9","10a","10b","10c","10d","10e","10f"]`, `remaining: []`.
