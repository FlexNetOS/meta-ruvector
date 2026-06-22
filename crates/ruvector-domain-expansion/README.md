# ruvector-domain-expansion

[![Crates.io](https://img.shields.io/crates/v/ruvector-domain-expansion.svg)](https://crates.io/crates/ruvector-domain-expansion)
[![docs.rs](https://docs.rs/ruvector-domain-expansion/badge.svg)](https://docs.rs/ruvector-domain-expansion)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-1.77%2B-orange.svg)](https://www.rust-lang.org)

**Cross-domain transfer learning — train on one problem, get better at a different one automatically.**

```toml
ruvector-domain-expansion = "0.1"
```

Most AI systems learn one task at a time. `ruvector-domain-expansion` is a
cross-domain transfer-learning engine: knowledge (compact Beta priors) learned
in one domain can seed another — and it **proves** the transfer actually helped
before committing it. The crate ships **three built-in domains** (Rust program
synthesis, structured planning, tool orchestration) and is **extensible to any
problem space** via the `Domain` trait. Part of the
[RuVector](https://github.com/ruvnet/ruvector) ecosystem.

| | ruvector-domain-expansion | Traditional Fine-Tuning |
|---|---|---|
| **Learning scope** | Three built-in domains (Rust synthesis, planning, tool orchestration); add your own via the `Domain` trait | One task at a time |
| **Transfer** | Automatic: priors from Domain 1 seed Domain 2 | Manual: retrain from scratch per domain |
| **Verification** | Transfer only accepted if it helps target without hurting source | No verification — hope it works |
| **Strategy selection** | Thompson Sampling picks the best approach per context | Fixed strategy for all inputs |
| **Population search** | 8 policy variants evolve in parallel, best survives | Single model, single strategy |
| **Curiosity** | Explores under-visited areas automatically | Only learns from data you provide |

## Quick Start

```rust
use ruvector_domain_expansion::{
    DomainExpansionEngine, DomainId, ContextBucket, ArmId, Solution,
};

let mut engine = DomainExpansionEngine::new();

// Generate training tasks in a built-in domain
let domain = DomainId("rust_synthesis".into());
let tasks = engine.generate_tasks(&domain, 10, 0.5); // 10 tasks, medium difficulty

// Select strategy using Thompson Sampling
let bucket = ContextBucket { difficulty_tier: "medium".into(), category: "algorithm".into() };
let arm = engine.select_arm(&domain, &bucket).unwrap();

// A candidate solution (you produce these however you like)
let solution = Solution {
    task_id: tasks[0].id.clone(),
    content: "fn double(xs: &[i64]) -> Vec<i64> { xs.iter().map(|&x| x * 2).collect() }".into(),
    data: serde_json::Value::Null,
};

// Evaluate and learn
let eval = engine.evaluate_and_record(&domain, &tasks[0], &solution, bucket, arm);

// Transfer learned priors to another built-in domain
let target = DomainId("structured_planning".into());
engine.initiate_transfer(&domain, &target);
// The target domain now starts from transferred priors instead of uniform ones.
```

## Key Features

| Feature | What It Does | Why It Matters |
|---------|-------------|----------------|
| **Meta Thompson Sampling** | Picks the best strategy per context using uncertainty-aware selection | Explores when unsure, exploits when confident — no manual tuning |
| **Cross-Domain Transfer** | Extracts compact priors from one domain, seeds another | New domains learn faster by starting with knowledge from related domains |
| **Transfer Verification** | Accepts a transfer only if target improves without source regressing | Guarantees generalization — no silent regressions |
| **Population-Based Search** | Evolves 8 policy kernel variants in parallel | Finds optimal strategies faster than single-model training |
| **Curiosity-Driven Exploration** | UCB-style bonus for under-visited contexts | Automatically explores blind spots instead of getting stuck |
| **Pareto Front Tracking** | Tracks non-dominated kernels across accuracy, cost, and robustness | See the best tradeoffs, not just the single "best" model |
| **Plateau Detection** | Detects when learning stalls and recommends actions | Automatically switches strategies instead of wasting compute |
| **Counterexample Tracking** | Records failed solutions to inform future decisions | Learns from mistakes, not just successes |
| **Cost Curve & Scoreboard** | Tracks convergence speed per domain with acceleration metrics | Proves that transfer actually accelerated learning |
| **RVF Integration** | Package trained models as cognitive containers (optional `rvf` feature) | Ship a trained domain expansion engine as a single `.rvf` file |

## Domains

This crate ships **three built-in domains**, all implementing the `Domain`
trait. `DomainExpansionEngine::new()` registers exactly these three
(`engine.domain_ids()` returns `rust_synthesis`, `structured_planning`,
`tool_orchestration`).

### Built-In Domains

| Domain (`DomainId`) | What It Generates | What It Evaluates |
|---------------------|-------------------|-------------------|
| **Rust Synthesis** (`rust_synthesis`) | Rust function specs (transforms, filters, searches) | Correctness, efficiency, idiomatic style |
| **Structured Planning** (`structured_planning`) | Multi-step plans with dependencies and resources | Feasibility, completeness, dependency ordering |
| **Tool Orchestration** (`tool_orchestration`) | Tool coordination tasks (parallel, error handling) | Correct sequencing, parallelism, failure recovery |

### Adding Your Own Domains

The engine is extensible: implement the `Domain` trait and register it with
`engine.register_domain(Box::new(MyDomain::new()))`. The trait requires
`id`, `name`, `generate_tasks`, `evaluate`, `embed`, `embedding_dim`, and
`reference_solution`. Any domain that produces `DomainEmbedding` vectors in a
shared space can participate in transfer.

```rust
use ruvector_domain_expansion::{DomainExpansionEngine, Domain};

let mut engine = DomainExpansionEngine::new();   // 3 built-ins registered
// engine.register_domain(Box::new(MyCustomDomain::new()));
```

> **Note on the wider RuVector ecosystem.** Other RuVector crates and examples
> (e.g. `ruvector-gnn`, `sona`, `ruvector-graph-transformer`) provide
> complementary capabilities, but they are **not** registered domains in this
> engine — you would integrate them yourself behind the `Domain` trait. The
> tables below are *illustrative* of the kinds of domains you could build, not a
> list of built-ins.

### How Transfer Connects Domains

```
                    ┌──────────────┐
                    │   Domain     │
                    │  Expansion   │
                    │   Engine     │
                    └──────┬───────┘
                           │
            ┌──────────────┼──────────────┐
            │              │              │
     ┌──────▼──────┐ ┌────▼─────┐ ┌──────▼──────┐
     │    Rust     │ │ Planning │ │    Tool     │
     │  Synthesis  │ │          │ │ Orchestr.   │
     └──────┬──────┘ └────┬─────┘ └──────┬──────┘
            │              │              │
            └──────┬───────┘──────┬───────┘
                   │              │
            ┌──────▼──────┐ ┌────▼──────────┐
            │  Shared     │ │  Transfer     │
            │  Embedding  │ │  Verification │
            │  Space      │ │  (TransferVer)│
            └─────────────┘ └───────────────┘
```

Every domain produces `DomainEmbedding` vectors in the same space. When you
transfer between domains, the engine extracts compact priors (Beta posteriors
from Thompson Sampling), seeds them into the target domain, and verifies the
transfer helped (`TransferVerification`) — promoting it only if the target
improved without the source regressing.

## How Transfer Works

Using the built-in domains: train on `rust_synthesis`, transfer the learned
priors to `structured_planning`.

```
Domain 1 (rust_synthesis)          Domain 2 (structured_planning)
┌─────────────────────┐            ┌─────────────────────┐
│ Train on tasks       │            │ Start from scratch   │
│ Extract posteriors   │───prior──▶│ Seed with priors     │
│ Score: 0.85          │            │ Score after transfer │
│                      │            │   improves vs the    │
│                      │            │   no-transfer        │
│                      │            │   baseline           │
└─────────────────────┘            └─────────────────────┘
                                           │
                                    TransferVerification:
                                    ✓ Target improved
                                    ✓ Source didn't regress
                                    ✓ Acceleration > 1.0 (scoreboard)
                                    → Transfer PROMOTED
```

### Cross-Domain Transfer (illustrative)

These pairings are *examples of the kind of transfer the engine enables*. Only
the three built-in domains are registered out of the box; the rest would be
implemented via the `Domain` trait.

| Source Domain | Target Domain | What Transfers | Why It Works |
|--------------|---------------|----------------|--------------|
| Rust Synthesis | Structured Planning | Strategy priors (which arm works) per difficulty/category | Both reward correct, well-ordered, idiomatic structure |
| Structured Planning | Tool Orchestration | Dependency-ordering priors | Plans and tool pipelines share sequential, dependency-aware structure |
| Genomics (custom) | Molecular Design (custom) | Sparse feature embeddings | Both work with sparse biological feature vectors |
| Trading (custom) | Resource Allocation (custom) | Risk/reward tradeoff priors | Same math — allocate limited budget across uncertain options |

## Feature Flags

| Flag | Default | What It Enables |
|------|---------|-----------------|
| `rvf` | No | RVF cognitive container integration — serialize engines to `.rvf` format |

```toml
[dependencies]
ruvector-domain-expansion = { version = "0.1", features = ["rvf"] }
```

## API Overview

### Core Types

| Type | Description |
|------|-------------|
| `DomainExpansionEngine` | Main orchestrator — manages domains, transfer, population search |
| `Domain` (trait) | Implement to add custom domains — generate tasks, evaluate, embed |
| `DomainId` | Unique identifier for a domain |
| `Task` | A problem instance with difficulty, constraints, and spec |
| `Solution` | A candidate answer with content and structured data |
| `Evaluation` | Score (0.0–1.0) with correctness, efficiency, and elegance breakdown |

### Transfer & Strategy

| Type | Description |
|------|-------------|
| `MetaThompsonEngine` | Thompson Sampling with Beta priors across context buckets |
| `TransferPrior` | Compact posterior summary extracted from a trained domain |
| `TransferVerification` | Result of verifying a transfer — promotable only if both domains benefit |
| `PolicyKernel` | A strategy configuration with tunable knobs |
| `PopulationSearch` | Evolutionary search across policy kernel variants |

### Meta-Learning

| Type | Description |
|------|-------------|
| `MetaLearningEngine` | Regret tracking, plateau detection, Pareto front, curiosity bonuses |
| `CostCurve` | Convergence trajectory per domain |
| `AccelerationScoreboard` | Measures how much faster transfer makes learning |
| `ParetoFront` | Non-dominated set of kernels across accuracy/cost/robustness |

## Dependencies

This crate is intentionally lightweight. Its runtime dependencies are:

| Crate | Role |
|-------|------|
| `serde` / `serde_json` | (De)serialization of tasks, solutions, priors |
| `rand` | Sampling for Thompson Sampling / population search |
| `rvf-types`, `rvf-wire`, `rvf-crypto` | **Optional** (`rvf` feature) — package a trained engine as a `.rvf` container |

It does **not** depend on `ruvector-gnn`, `sona`, `ruvector-coherence`,
`ruvector-attn-mincut`, `ruvector-solver`, or `ruvector-graph-transformer`. Those
are separate RuVector crates you could integrate as custom domains, but the
transfer/verification logic here is self-contained.

## License

**MIT License** — see [LICENSE](../../LICENSE) for details.

---

Part of [RuVector](https://github.com/ruvnet/ruvector) — the self-learning vector database.
