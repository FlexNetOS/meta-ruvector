# ADR-260 — Single-App Unification Architecture (RuVector as the heart)

**Status:** Proposed (Phase 1 plan) · **Date:** 2026-06-17
**Builds on:** `RUVECTOR-RUNBOOK.md` (314-crate code-walk, theses T1–T15), `RUVECTOR-META-MAPPING-S1.md` (S1 adoption map), S2 LOCKED decisions, `docs/RUVECTOR-COMPLETENESS-AUDIT.md` (Phase 0).

## Context

The meta workspace is **one app, not a fleet of apps.** RuVector is the heart; the
meta multi-peer repos are built under it and will **eventually be combined into a
single app codebase.** Phase 0 (the completeness audit) confirmed RuVector is a sound
foundation (84% production; 9 partial crates, isolated and named). This ADR defines
the architecture the single app converges to, and the order of convergence.

**Owner-confirmed spine:** RuVector substrate **core** + RuVocal **shell** + rvAgent
**runtime**. First unification pass = **meta-ruvector + meta/ruflo**. Next priority =
a **proper RuVector UI wired to prompt_hub** (the front door).

## Decision — the layered single-app architecture

```
┌──────────────────────────────────────────────────────────────────┐
│  SHELL / FRONT DOOR     RuVocal UI (SvelteKit)  ◄── prompt_hub intake │  ← user-facing
├──────────────────────────────────────────────────────────────────┤
│  AGENT RUNTIME          rvAgent (Rust) + ruv-swarm   ∥  ruflo (TS, legacy→ported) │
├──────────────────────────────────────────────────────────────────┤
│  INTEGRATION SEAMS      .rvf format (T4)  ·  MCP (T11)  ·  wasm-bridge-through-rvf-kernel │
├──────────────────────────────────────────────────────────────────┤
│  SUBSTRATE / ENGINE     ruvector-core/graph/math/mincut/coherence · rvf-* · index zoo │  ← the heart
├──────────────────────────────────────────────────────────────────┤
│  STATE / CONTINUITY     .handoff ledger (hf) + RVF witness  ·  ruvector-postgres (opt) │
└──────────────────────────────────────────────────────────────────┘
```

- **Substrate (the heart):** the `.rvf` format (`rvf-types` = #1 dep hub) bridges native (napi) and wasm; `ruvector-core/graph/math/mincut/coherence` + the index zoo are the engine. **Production-grade per Phase 0.**
- **Runtime:** `rvAgent` (Rust) is the canonical agent runtime; **ruflo (TS) is legacy/compat** and gets ported in (rust-port harness) where still needed. (Per ADR direction: every TS has a Rust-native replacement.)
- **Shell:** RuVocal UI is the single app's face; **prompt_hub** is the intake/front-door that feeds it (S2 LOCKED). RVF document store is the UI's persistence (already wired + verified).
- **Seams:** the **only** ways components attach — the `.rvf` format, **MCP** servers (universal control seam), and the **wasm-bridge-through-rvf-kernel** pattern for plugins. New code attaches via a seam before it merges.
- **State:** `.handoff` ledger (Git > ledger > tasks) is source-of-truth; RVF witness for tamper-evidence.

## Laws (govern every unification step)

1. **Adopt-then-extend** — RuVector is the foundation; we extend it, never fork-and-diverge.
2. **No-downgrade** — a feature that exists keeps working; ports preserve behavior (differential-verified). Partial impls get *hardened*, never silently dropped.
3. **Dependency direction is inward** — shell → runtime → seams → substrate. The substrate never depends on the shell.
4. **One seam, then merge** — a repo integrates via a seam (`.rvf`/MCP/wasm-bridge) and is proven working *before* its code is folded into the unified workspace.

## Convergence roadmap

**Pass 1 — meta-ruvector + ruflo (in scope now)**
- Harden the 2 load-bearing partials first: **`ruvllm`** (LLM engine) and **`ruvector-gnn`** — they anchor the runtime/UI.
- Port the still-needed ruflo (TS) capabilities into `rvAgent`/RuVector via the rust-port harness; treat ruflo TS as legacy/compat during the transition (parallel front-ends over one substrate, T3).
- Keep `build:all` green and the RuVocal UI live throughout.

**Pass 2 — front-end priority: proper RuVector UI + prompt_hub**
- Wire RuVocal to the *real* substrate end-to-end: RVF store (done) → rvAgent runtime → MCP seams → models.
- Integrate **prompt_hub** as the intake/front-door (the `SwarmBundle → handoff.task.v1` dispatch over MCP — the missing outbound wiring noted in the runbook S2 NEXT).
- Resolve the UI config gaps surfaced in bring-up (e.g. `COOKIE_NAME`) by adopting the chart-driven config as the canonical source.

**Pass 3+ — broaden (later):** map remaining meta repos (envctl secrets, weave messaging, handoff ledger, …) to roles and fold them in via seams (separate ADRs per repo).

## Consequences
- A clear target: every repo knows its layer and its seam.
- The partial-impl risk is contained to a named, prioritized list (Phase 0) — hardened in dependency order, not discovered late.
- TS→Rust convergence is incremental and differential-verified (no big-bang rewrite).

## Open decisions (for the owner)
- `prime-radiant` (2nd GPU convergence runtime) and `ruvector-postgres` (pgvector backend): **in the single app, or optional modules?** (Both standalone today.)
- Unified-workspace mechanics: one Cargo workspace + one npm workspace, or keep the meta-CLI multi-repo orchestration until the end?

## References
- `RUVECTOR-RUNBOOK.md`, `RUVECTOR-META-MAPPING-S1.md`, S2 decisions
- `docs/RUVECTOR-COMPLETENESS-AUDIT.md` (Phase 0 evidence)
