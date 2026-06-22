# RuvLLM

[![Crates.io](https://img.shields.io/crates/v/ruvllm.svg)](https://crates.io/crates/ruvllm)
[![docs.rs](https://docs.rs/ruvllm/badge.svg)](https://docs.rs/ruvllm)
[![npm](https://img.shields.io/npm/v/@ruvector/ruvllm.svg)](https://www.npmjs.com/package/@ruvector/ruvllm)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)

**The local LLM inference engine that learns from every request -- Metal, CUDA, WebGPU, no cloud APIs.**

```bash
cargo add ruvllm
```

RuvLLM loads GGUF models and runs them on your hardware with full acceleration -- Apple Silicon, NVIDIA GPUs, WebAssembly, whatever you have. Unlike other local inference tools, it gets smarter over time: SONA (Self-Optimizing Neural Architecture) watches how you use it and adapts automatically, so responses improve without manual tuning. It's part of [RuVector](https://github.com/ruvnet/ruvector), the self-learning vector database with graph intelligence.

| | RuvLLM | OpenAI API | llama.cpp | Ollama | vLLM |
|---|---|---|---|---|---|
| **Cost** | Free after hardware | Per-token billing | Free | Free | Free |
| **Privacy** | Data stays on your machine | Sent to third party | Local | Local | Local |
| **Self-learning** | SONA adapts automatically | Static | Static | Static | Static |
| **Per-request tuning** | MicroLoRA in <1 ms | Not available | Not available | Not available | Not available |
| **Hardware support** | Metal, CUDA, ANE, WebGPU, CPU | N/A | Metal, CUDA, CPU | Metal, CUDA, CPU | CUDA only |
| **WASM / Browser** | Yes (5.5 KB runtime) | Via network call | Not available | Not available | Not available |
| **Vector DB integration** | Built-in (RuVector) | Separate service | Not available | Not available | Not available |
| **Speculative decoding** | Yes | N/A | Yes | No | Yes |
| **Continuous batching** | Yes | N/A | No | No | Yes |
| **Production serving** | Continuous batch scheduler | N/A | Server mode | Server mode | Native |

## Key Features

| Feature | What It Does | Why It Matters |
|---------|-------------|----------------|
| **SONA three-tier learning** | Adapts to your queries at three speeds: instant (<1 ms), background (~100 ms), deep (minutes) | Responses improve automatically without manual retraining |
| **Metal + CUDA + ANE** | Hardware-accelerated inference across Apple Silicon, NVIDIA GPUs, and Apple Neural Engine | Get the most out of whatever hardware you have |
| **TurboQuant KV-Cache** | 2-4 bit asymmetric per-channel quantization with H2O/PyramidKV eviction | 6-8x memory reduction, <0.5% quality loss |
| **Flash Attention 2** | Memory-efficient attention with O(N) complexity and online softmax | Longer contexts with less memory |
| **GGUF memory mapping** | Memory-mapped model loading with quantization (Q4K, Q8, FP16) | Load large models fast, use 4-8x less RAM |
| **Speculative decoding** | Draft model generates candidates, target model verifies in parallel | 2-3x faster text generation |
| **Continuous batching** | Dynamic batch scheduling for concurrent requests | 2-3x throughput improvement for serving |
| **MicroLoRA** | Per-request fine-tuning with rank 1-2 adapters | Personalize responses in <1 ms without full retraining |
| **HuggingFace Hub** | Download and upload models directly | One-line model access, easy sharing |
| **Task-specific adapters** | 5 pre-trained LoRA adapters (coder, researcher, security, architect, reviewer) | Instant specialization with hot-swap |

> Part of the [RuVector](https://github.com/ruvnet/ruvector) ecosystem -- the self-learning vector database with graph intelligence, local AI, and PostgreSQL built in.

## Quick Start

```rust
// ruvllm has no `prelude` module — import the types directly from the crate root.
// `CandleBackend` requires the `candle` feature.
use std::path::Path;
use ruvllm::{CandleBackend, DeviceType, GenerateParams, ModelConfig};

let mut backend = CandleBackend::with_device(DeviceType::Metal)?;
backend.load_gguf(Path::new("models/qwen2.5-7b-q4_k.gguf"), &ModelConfig::default())?;

let response = backend.generate("Explain quantum computing in simple terms.",
    GenerateParams {
        max_tokens: 256,
        temperature: 0.7,
        top_p: 0.9,
        ..Default::default()
    }
)?;

println!("{}", response);
```

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
# Recommended for Apple Silicon Mac
ruvllm = { version = "2.1", features = ["inference-metal", "coreml", "parallel"] }

# For NVIDIA GPUs
ruvllm = { version = "2.1", features = ["inference-cuda", "parallel"] }

# Minimal (CPU only)
ruvllm = { version = "2.1" }
```

Or install the npm package:

```bash
npm install @ruvector/ruvllm
```

## What's New in v2.6

| Feature | Description | Benefit |
|---------|-------------|---------|
| **Sparse Attention Kernel** | Subquadratic O(N log N) attention via local window + log-stride + landmarks | 29× fewer edge comparisons at seq=8192 vs dense |
| **Hailo-10H Edge Inference** | GQA/MQA support fits Mistral-7B KV cache in 2.1 GB | Runs on Raspberry Pi 5 + AI HAT+ cluster |
| **KV Cache Incremental Decode** | `decode_step()` — O(log T) per token instead of O(T log T) | Sustained generation on memory-constrained edge nodes |
| **Zero Runtime Deps** | `ruvllm_sparse_attention` has no runtime dependencies | Minimal binary footprint for embedded / WASM targets |

See [`ruvllm_sparse_attention`](../ruvllm_sparse_attention/README.md) for the full kernel documentation (ADR-183 – ADR-190).

## What's New in v2.5

| Feature | Description | Benefit |
|---------|-------------|---------|
| **TurboQuant** | 2-4 bit asymmetric per-channel KV-cache quantization | 6-8x memory reduction, <0.5% perplexity loss |
| **TurboQuant Embedding Store** | Quantized vector storage with asymmetric inner product search | 10-30x memory savings for embeddings |
| **H2O / PyramidKV Eviction** | Intelligent cache eviction based on attention scores | Keep most important tokens in long-context |
| **Optimized Inner Product** | Compute distances directly on quantized data | 2-4x faster search, skip decompression |

### Previous: v2.3

| Feature | Description | Benefit |
|---------|-------------|---------|
| **RuvLTRA-Medium 3B** | Purpose-built 3B model for Claude Flow | 42 layers, 256K context, speculative decode |
| **HuggingFace Hub** | Full Hub integration (download/upload) | Easy model sharing and distribution |
| **Task-Specific LoRA** | 5 pre-trained adapters for agent types | Optimized for coder/researcher/security/architect/reviewer |
| **Adapter Merging** | TIES, DARE, SLERP, Task Arithmetic | Combine adapters for multi-task models |
| **Hot-Swap Adapters** | Zero-downtime adapter switching | Runtime task specialization |
| **Claude Dataset** | 2,700+ Claude-style training examples | Optimized for Claude Flow integration |
| **HNSW Routing** | 150x faster semantic pattern matching | <25µs pattern retrieval |
| **Evaluation Harness** | Real model evaluation with SWE-Bench | 5 ablation modes, quality metrics |
| **HNSW Auto-Dimension** | Automatic embedding dimension detection | No manual config needed |
| **mistral-rs Backend** | Production-scale serving with PagedAttention | 5-10x concurrent users, X-LoRA, ISQ |

### Previous v2.0-2.2 Features

| Feature | Description | Benefit |
|---------|-------------|---------|
| **Apple Neural Engine** | Core ML backend with ANE routing | 38 TOPS, 3-4x power efficiency |
| **Hybrid GPU+ANE Pipeline** | Intelligent operation routing | Best of both accelerators |
| **Multi-threaded GEMM** | Rayon parallelization | 4-12x speedup on M4 Pro |
| **Flash Attention 2** | Auto block sizing, online softmax | O(N) memory, +10% throughput |
| **Quantized Inference** | INT8/INT4/Q4_K/Q8_K kernels | 4-8x memory reduction |
| **Metal GPU Shaders** | simdgroup_matrix operations | 3x speedup on Apple Silicon |
| **GGUF Support** | Memory-mapped model loading | Fast loading, reduced RAM |
| **Continuous Batching** | Dynamic batch scheduling | 2-3x throughput improvement |
| **Speculative Decoding** | Draft model acceleration | 2-3x faster generation |
| **Gemma-2 & Phi-3** | New model architectures | Extended model support |

## Backends

| Backend | Best For | Acceleration |
|---------|----------|-------------|
| **Candle** | Single user, edge, WASM | Metal, CUDA, CPU |
| **Core ML** | Apple Silicon efficiency | Apple Neural Engine (38 TOPS) |
| **Hybrid Pipeline** | Maximum throughput on Mac | GPU for attention, ANE for MLP |
| **Hailo-10H** | Pi 5 + AI HAT+ cluster | Sparse O(N log N) attention, GQA, KV cache decode |

> **Planned:** a `mistral-rs` backend (PagedAttention, X-LoRA, ISQ) for production serving is scaffolded in `backends::mistral` but is gated behind an unpublished `mistralrs` dependency. See [mistral-rs Backend (Planned)](#mistral-rs-backend-planned) below.

### Feature Flags

| Feature | Description |
|---------|-------------|
| `candle` | Enable Candle backend (HuggingFace) |
| `metal` | Apple Silicon GPU acceleration via Candle |
| `metal-compute` | Native Metal compute shaders (M4 Pro optimized) |
| `cuda` | NVIDIA GPU acceleration |
| `coreml` | Apple Neural Engine via Core ML |
| `hybrid-ane` | GPU+ANE hybrid pipeline (recommended for Mac) |
| `inference-metal` | Full Metal inference stack |
| `inference-metal-native` | Metal + native shaders (best M4 Pro perf) |
| `inference-cuda` | Full CUDA inference stack |
| `parallel` | Multi-threaded GEMM/GEMV with Rayon |
| `accelerate` | Apple Accelerate BLAS (~2x GEMV speedup) |
| `gguf-mmap` | Memory-mapped GGUF loading |
| `async-runtime` | Tokio async support |
| `wasm` | WebAssembly support |
| `quantize` | Quantization helpers and TurboQuant (default-on) |
| `hub-download` | HuggingFace Hub auto-download (default-on) |

> The `mistral-rs` feature flags (`mistral-rs`, `mistral-rs-metal`, `mistral-rs-cuda`) are commented out in `Cargo.toml` and **not yet available** — they depend on the unpublished `mistralrs` crate. They will be enabled once `mistralrs` ships on crates.io.

## Architecture

```
+----------------------------------+
|         Application              |
+----------------------------------+
               |
+----------------------------------+
|        RuvLLM Backend            |
|  +----------------------------+  |
|  |   Hybrid Pipeline Router   |  |
|  |  ┌─────────┐ ┌──────────┐  |  |
|  |  │  Metal  │ │   ANE    │  |  |
|  |  │   GPU   │ │ Core ML  │  |  |
|  |  └────┬────┘ └────┬─────┘  |  |
|  |       │    ↕      │        |  |
|  |  Attention    MLP/FFN      |  |
|  |  RoPE         Activations  |  |
|  |  Softmax      LayerNorm    |  |
|  +----------------------------+  |
|               |                  |
|  +----------------------------+  |
|  |     SONA Learning          |  |
|  |  - Instant (<1ms)          |  |
|  |  - Background (~100ms)     |  |
|  |  - Deep (minutes)          |  |
|  +----------------------------+  |
|               |                  |
|  +----------------------------+  |
|  |     NEON/SIMD Kernels      |  |
|  |  - Flash Attention 2       |  |
|  |  - Paged KV Cache          |  |
|  |  - Quantized MatMul        |  |
|  +----------------------------+  |
+----------------------------------+
```

## Supported Models

| Model Family | Sizes | Quantization | Backend |
|--------------|-------|--------------|---------|
| **RuvLTRA-Small** | 0.5B | Q4K, Q5K, Q8, FP16 | Candle/Metal/ANE |
| **RuvLTRA-Medium** | 3B | Q4K, Q5K, Q8, FP16 | Candle/Metal |
| Qwen 2.5 | 0.5B-72B | Q4K, Q8, FP16 | Candle/Metal |
| Llama 3.x | 8B-70B | Q4K, Q8, FP16 | Candle/Metal |
| Mistral | 7B-22B | Q4K, Q8, FP16 | Candle/Metal |
| Phi-3 | 3.8B-14B | Q4K, Q8, FP16 | Candle/Metal |
| Gemma-2 | 2B-27B | Q4K, Q8, FP16 | Candle/Metal |

### RuvLTRA Models (Claude Flow Optimized)

| Model | Parameters | Hidden | Layers | Context | Features |
|-------|------------|--------|--------|---------|----------|
| RuvLTRA-Small | 494M | 896 | 24 | 32K | GQA 7:1, SONA hooks |
| RuvLTRA-Medium | 3.0B | 2560 | 42 | 256K | Flash Attention 2, Speculative Decode |

<details>
<summary>📊 Performance Benchmarks (M4 Pro 14-core)</summary>

### Inference Benchmarks

| Model | Quant | Prefill (tok/s) | Decode (tok/s) | Memory |
|-------|-------|-----------------|----------------|--------|
| Qwen2.5-7B | Q4K | 2,800 | 95 | 4.2 GB |
| Qwen2.5-7B | Q8 | 2,100 | 72 | 7.8 GB |
| Llama3-8B | Q4K | 2,600 | 88 | 4.8 GB |
| Mistral-7B | Q4K | 2,500 | 85 | 4.1 GB |
| Phi-3-3.8B | Q4K | 3,500 | 135 | 2.3 GB |
| Gemma2-9B | Q4K | 2,200 | 75 | 5.2 GB |

### ANE vs GPU Performance (M4 Pro)

| Dimension | ANE | GPU | Winner |
|-----------|-----|-----|--------|
| < 512 | +30-50% | - | ANE |
| 512-1024 | +10-30% | - | ANE |
| 1024-1536 | ~Similar | ~Similar | Either |
| 1536-2048 | - | +10-20% | GPU |
| > 2048 | - | +30-50% | GPU |

### Kernel Benchmarks

| Kernel | Single-thread | Multi-thread (10-core) |
|--------|---------------|------------------------|
| GEMM 4096x4096 | 1.2 GFLOPS | 12.7 GFLOPS |
| GEMV 4096x4096 | 0.8 GFLOPS | 6.4 GFLOPS |
| Flash Attention (seq=2048) | 850μs | 320μs |
| RMS Norm (4096) | 2.1μs | 0.8μs |
| RoPE (4096, 128) | 4.3μs | 1.6μs |

</details>

<details>
<summary>🍎 Apple Neural Engine (ANE) Integration</summary>

RuvLLM includes ANE support via Core ML. `CoreMLBackend` is exported from
`backends`; `AneStrategy` and `HybridPipeline` require the `hybrid-ane` feature.

```rust
// CoreMLBackend::new() takes no arguments; compute units are configured separately.
use ruvllm::backends::{CoreMLBackend, ComputeUnits};

let backend = CoreMLBackend::new()?
    .with_compute_units(ComputeUnits::CpuAndNeuralEngine);
```

```rust
// Hybrid GPU+ANE pipeline (requires the `hybrid-ane` feature).
// `AneStrategy` and `HybridPipeline` are re-exported from `ruvllm::backends`.
use ruvllm::backends::{AneStrategy, HybridPipeline, HybridPipelineConfig};

let pipeline = HybridPipeline::new(HybridPipelineConfig {
    ane_strategy: AneStrategy::Adaptive,
    metal_for_attention: true,  // Attention on Metal GPU, MLP routed to ANE
    ..Default::default()
})?;
```

### ANE Routing Recommendations

| Operation | Recommended | Reason |
|-----------|-------------|--------|
| Attention | GPU | Better for variable sequence lengths |
| Flash Attention | GPU | GPU memory bandwidth advantage |
| MLP/FFN | ANE | Optimal for fixed-size matmuls |
| GELU/SiLU | ANE | Dedicated activation units |
| LayerNorm/RMSNorm | ANE | Good for small dimensions |
| Embedding | GPU | Sparse operations |

</details>

## MicroLoRA Real-Time Adaptation

RuvLLM supports per-request fine-tuning using MicroLoRA:

```rust
use ruvllm::lora::{MicroLoRA, MicroLoraConfig, AdaptFeedback};

// Create MicroLoRA adapter
let config = MicroLoraConfig::for_hidden_dim(4096);
let lora = MicroLoRA::new(config);

// Adapt on user feedback
let feedback = AdaptFeedback::from_quality(0.9);
lora.adapt(&input_embedding, feedback)?;

// Apply learned updates
lora.apply_updates(0.01); // learning rate

// Inspect adapter counters
println!("Adaptations: {}", lora.adaptation_count());
println!("Params: {}, Memory: {} bytes", lora.param_count(), lora.memory_bytes());
```

## SONA Three-Tier Learning

Continuous improvement with three learning loops:

```rust
use ruvllm::{SonaLlm, SonaLlmConfig, TrainingSample};

let config = SonaLlmConfig {
    instant_lr: 0.01,
    background_interval_ms: 100,
    ..Default::default()
};

let sona = SonaLlm::new(config);

// 1. Instant Loop (<1ms): Per-request MicroLoRA
let result = sona.instant_adapt("user query", "model response", 0.85);
println!("Instant adapt: {}μs", result.latency_us);

// 2. Background Loop (~100ms): Pattern consolidation
if let Some(bg) = sona.maybe_background() {
    if bg.applied {
        println!("Consolidated {} samples", bg.samples_used);
    }
}

// 3. Deep Loop (minutes): Full optimization over a batch of samples
if sona.should_trigger_deep() {
    let dataset: Vec<TrainingSample> = collect_training_samples();
    let result = sona.deep_optimize(&dataset);
    println!("Deep optimization: {:.1}s", result.latency_us as f64 / 1_000_000.0);
}

// Check learning stats (LearningLoopStats)
let stats = sona.stats();
println!("{:?}", stats);
```

## Two-Tier KV Cache

Memory-efficient caching with automatic tiering:

```rust
use ruvllm::{TwoTierKvCache, KvCacheConfig, Precision};

let config = KvCacheConfig {
    tail_length: 256,              // Recent tokens in FP16
    tail_precision: Precision::FP16,
    store_precision: Precision::Q4,  // Older tokens in Q4
    max_tokens: 8192,
    num_kv_heads: 8,
    head_dim: 128,
    migration_batch: 64,           // Tokens migrated tail -> store per batch
};

let cache = TwoTierKvCache::new(config);
cache.append(&keys, &values)?;

// Automatic migration from tail to quantized store
let stats = cache.stats();
println!("Tail: {} tokens, Store: {} tokens", stats.tail_tokens, stats.store_tokens);
println!("Compression ratio: {:.2}x", stats.compression_ratio);
println!("Tail bytes: {}, Store bytes: {}", stats.tail_bytes, stats.store_bytes);
```

## TurboQuant KV-Cache Compression

Aggressive quantization for long-context inference:

```rust
use ruvllm::quantize::turbo_quant::{
    TurboQuantCompressor, TurboQuantConfig, TurboQuantBits,
    TurboQuantCacheTier, TurboQuantEmbeddingStore,
};

// Compress KV-cache entries at 3-bit (10.7x compression)
let config = TurboQuantConfig {
    bits: TurboQuantBits::Bits3_5,
    enable_qjl_residual: true, // QJL residual correction for better inner products
    ..Default::default()
};
let compressor = TurboQuantCompressor::new(config)?;

// Compress a batch of KV vectors
let keys: Vec<&[f32]> = kv_pairs.iter().map(|p| p.key.as_slice()).collect();
let compressed = compressor.compress_batch(&keys)?;
println!("Compression: {:.1}x", compressed.compression_ratio());

// Asymmetric inner product — no decompression needed
let scores = compressor.inner_product_batch_optimized(
    &query_vector, &compressed
)?;

// TurboQuant KV-Cache Tier with eviction
let mut cache = TurboQuantCacheTier::new(config)?;
cache.push(&keys_f32, &values_f32, position)?;
let stats = cache.stats();
println!("Compressed: {} bytes, Pairs: {}", stats.compressed_bytes, stats.num_pairs);

// Quantized embedding store with search
let mut store = TurboQuantEmbeddingStore::new(dim, config)?;
store.build_from_batch(&embeddings, &ids)?;
let results = store.search(&query, top_k)?; // Returns (id, score) pairs
```

| Bits | Compression | Perplexity Loss | Best For |
|------|-------------|-----------------|----------|
| 2-bit | 32x | ~2% | Edge devices, maximum compression |
| 3-bit | 10.7x | <1% | Balanced — recommended default |
| 4-bit | 8x | <0.5% | High quality, long-context |
| 8-bit | 4x | ~0% | Baseline quantization |

## Continuous Batching

High-throughput serving with dynamic batching:

```rust
use ruvllm::{
    ContinuousBatchScheduler, SchedulerConfig, KvCachePoolConfig,
    RequestQueue, InferenceRequest, PreemptionMode, GenerateParams,
};

let mut scheduler = ContinuousBatchScheduler::new(
    SchedulerConfig {
        max_batch_size: 32,
        max_tokens_per_batch: 4096,
        preemption_mode: PreemptionMode::Recompute,
        ..Default::default()
    },
    KvCachePoolConfig::default(),
);

// Queue requests (prompt token ids + generation params)
let mut queue = RequestQueue::new();
queue.add(InferenceRequest::new(tokens, GenerateParams::default()));

// Schedule the next batch of work
let batch = scheduler.schedule(&mut queue);
println!("Batch: {} requests, {} tokens", batch.requests.len(), batch.total_tokens);

// Inspect scheduler stats (SchedulerStats)
let stats = scheduler.stats();
println!("Batches scheduled: {}", stats.batches_scheduled);
println!("KV cache utilization: {:.1}%", stats.kv_cache_utilization * 100.0);
```

## Speculative Decoding

Accelerate generation with draft models:

```rust
use ruvllm::speculative::{SpeculativeDecoder, SpeculativeConfig};
use ruvllm::GenerateParams;

let config = SpeculativeConfig {
    lookahead: 4,              // Tokens to speculate ahead per step
    acceptance_threshold: 0.8, // Min probability for acceptance
    ..Default::default()
};

// new(main_model, draft_model, config) -> SpeculativeDecoder
let decoder = SpeculativeDecoder::new(main_model, draft_model, config);

// generate(prompt, params) -> Result<String>
let text = decoder.generate(prompt, GenerateParams {
    max_tokens: 256,
    ..Default::default()
})?;
println!("{}", text);

// Acceptance metrics live on the decoder (SpeculativeStats)
let stats = decoder.stats();
println!("Acceptance rate: {:.1}%", stats.acceptance_rate * 100.0);
println!("Speedup: {:.2}x", stats.speedup);
```

> Note: `SpeculativeConfig` is also re-exported at the crate root as
> `ruvllm::SpeculativeDecodingConfig` (aliased to avoid a name clash with the
> `optimization` module's own `SpeculativeConfig`).

## GGUF Model Loading

Efficient loading with memory mapping:

```rust
use std::path::Path;
use ruvllm::{GgufLoader, LoadConfig};

// LoadConfig derives Default; enable memory mapping for large models.
let config = LoadConfig {
    use_mmap: true,        // Memory-map for fast loading
    keep_quantized: true,  // Keep weights quantized (don't dequantize to F32)
    ..Default::default()
};

// new(path, config) reads the GGUF header up front.
let loader = GgufLoader::new(Path::new("model.gguf"), config)?;

// Inspect the extracted model configuration (GgufModelConfig).
let cfg = loader.model_config();
println!("Architecture: {:?}", cfg.architecture);
println!("Context length: {:?}", cfg.context_length);
println!("Layers: {:?}", cfg.layer_count);

// Load all weights (or use load_layer / load_tensor for partial loads).
let weights = loader.load_weights()?;
```

## mistral-rs Backend (Planned)

> **Status: not yet functional.** The config types below (`MistralBackend`,
> `MistralBackendConfig`, `XLoraConfig`, `IsqConfig`, `IsqMethod`) are
> re-exported from `ruvllm::backends` and the scaffolding exists in
> `backends::mistral_backend`, but the actual inference path depends on the
> **unpublished `mistralrs` crate**. The `mistral-rs` / `mistral-rs-metal` /
> `mistral-rs-cuda` feature flags are commented out in `Cargo.toml` and will be
> enabled once `mistralrs` ships on crates.io. Treat this as a design preview.

<details>
<summary>🚀 mistral-rs Backend (planned production serving)</summary>

The planned [mistral-rs](https://github.com/EricLBuehler/mistral.rs) integration
targets production-scale serving with advanced memory management.

### Planned Features

| Feature | Description | Benefit |
|---------|-------------|---------|
| **PagedAttention** | vLLM-style KV cache management | 5-10x concurrent users, 85-95% memory utilization |
| **X-LoRA** | Per-token adapter routing | <1ms routing overhead, multi-task inference |
| **ISQ** | In-Situ Quantization (AWQ, GPTQ, RTN) | Runtime quantization without re-export |

### Config Surface (types exist today; backend is a stub)

```rust
// These types are re-exported from ruvllm::backends.
use ruvllm::backends::{
    MistralBackend, MistralBackendConfig,
    XLoraConfig, IsqConfig, IsqMethod,
};

// MistralBackendConfig derives Default; fields are plain structs/Options.
let config = MistralBackendConfig {
    xlora: Some(XLoraConfig::default()),
    isq: Some(IsqConfig {
        method: IsqMethod::AWQ, // also: GPTQ, ...
        ..Default::default()
    }),
    max_batch_size: 32,
    max_seq_len: 4096,
    ..Default::default()
};

// MistralBackend::new() takes no arguments today (stub until `mistralrs` lands).
let _backend = MistralBackend::new()?;
```

### When to Use mistral-rs vs Candle (once available)

| Scenario | Recommended Backend | Reason |
|----------|---------------------|--------|
| Single user / Edge | Candle | Simpler, smaller binary |
| 10-100 concurrent users | mistral-rs | PagedAttention memory efficiency |
| Multi-task models | mistral-rs | X-LoRA per-token routing |
| Runtime quantization | mistral-rs | ISQ without model re-export |
| WASM / Browser | Candle | mistral-rs doesn't support WASM |

### Feature Flags (planned, commented out in `Cargo.toml`)

```toml
# Not yet available — depends on the unpublished `mistralrs` crate.
# ruvllm = { version = "2.1", features = ["mistral-rs"] }
# ruvllm = { version = "2.1", features = ["mistral-rs-metal"] }
# ruvllm = { version = "2.1", features = ["mistral-rs-cuda"] }
```

</details>

## Configuration

### Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `RUVLLM_CACHE_DIR` | Model cache directory | `~/.cache/ruvllm` |
| `RUVLLM_LOG_LEVEL` | Logging level | `info` |
| `RUVLLM_METAL_DEVICE` | Metal device index | `0` |
| `RUVLLM_ANE_ENABLED` | Enable ANE routing | `true` |
| `RUVLLM_SONA_ENABLED` | Enable SONA learning | `true` |

### Model Configuration

```rust
use ruvllm::{ModelConfig, Quantization};

let config = ModelConfig {
    max_sequence_length: 8192,
    use_flash_attention: true,
    quantization: Some(Quantization::Q4K),
    sliding_window: Some(4096),
    rope_theta: Some(1_000_000.0),
    ..Default::default()
};
```

## Benchmarks

Run benchmarks with:

```bash
# Attention benchmarks
cargo bench --bench attention_bench --features inference-metal

# ANE benchmarks (Mac only)
cargo bench --bench ane_bench --features coreml

# LoRA benchmarks
cargo bench --bench lora_bench

# End-to-end inference
cargo bench --bench e2e_bench --features inference-metal

# Metal shader benchmarks
cargo bench --bench metal_bench --features metal-compute

# Serving benchmarks
cargo bench --bench serving_bench --features inference-metal
```

## HuggingFace Hub Integration (v2.3)

Download and upload models to HuggingFace Hub:

```rust
use std::path::Path;
use ruvllm::hub::{ModelDownloader, ModelUploader, RuvLtraRegistry};

// Look up a known RuvLTRA model from the registry (get returns Option<&ModelInfo>).
let registry = RuvLtraRegistry::new();
let model_info = registry.get("ruvltra-small").expect("model in registry");

// Download it. ModelDownloader::new() takes no args;
// download(&ModelInfo, Option<&Path>) -> Result<PathBuf>.
let downloader = ModelDownloader::new();
let model_path = downloader.download(model_info, Some(Path::new("./models")))?;
println!("Downloaded to: {}", model_path.display());

// Upload to Hub. ModelUploader::new(token);
// upload(path, repo_id, Option<ModelMetadata>) -> Result<String> (the repo URL).
let uploader = ModelUploader::new("hf_your_token");
let url = uploader.upload(
    "./my-model.gguf",
    "username/my-ruvltra-model",
    None, // or Some(ModelMetadata { .. })
)?;
println!("Uploaded to: {}", url);
```

<details>
<summary>🎯 Task-Specific LoRA Adapters (v2.3)</summary>

Pre-trained adapters optimized for Claude Flow agent types:

```rust
use ruvllm::lora::{RuvLtraAdapters, AdapterMerger, MergeConfig, LoraConfig, HotSwapManager};

// Create adapter for specific task -> Result<MicroLoRA>
let adapters = RuvLtraAdapters::new();
let coder = adapters.create_lora("coder", 768)?;       // code generation
let security = adapters.create_lora("security", 768)?; // vulnerability detection

// Available adapter presets:
// - coder, researcher, security, architect, reviewer

// Merge adapters for multi-task models.
// MergeConfig::weighted takes a HashMap<String, f32>; merge takes (name, adapter) pairs
// and an output LoraConfig (built via LoraConfig::builder).
let merger = AdapterMerger::new(MergeConfig::weighted(weights));
let output_config = LoraConfig::builder("multi_task").rank(16).build();
let multi_task = merger.merge(
    &[("coder".to_string(), coder), ("security".to_string(), security)],
    &output_config,
    768,
)?;

// Hot-swap adapters at runtime (set_active / prepare_standby take MicroLoRA by value)
let mut manager = HotSwapManager::new();
manager.set_active(adapters.create_lora("coder", 768)?);
manager.prepare_standby(adapters.create_lora("security", 768)?);
manager.swap()?; // Zero-downtime switch
```

### Adapter Merging Strategies

| Strategy | Description | Use Case |
|----------|-------------|----------|
| **Average** | Equal-weight averaging | Simple multi-task |
| **WeightedSum** | User-defined weights | Task importance weighting |
| **SLERP** | Spherical interpolation | Smooth transitions |
| **TIES** | Trim, Elect, Merge | Robust multi-adapter |
| **DARE** | Drop And REscale | Sparse merging |
| **TaskArithmetic** | Add/subtract vectors | Task composition |

</details>

<details>
<summary>🧪 Evaluation Harness (v2.3)</summary>

RuvLLM includes an evaluation harness for loading models and SWE-Bench tasks.
Build the harness with `RealEvaluationHarness::with_config` (which loads the
model when a path is set), then inspect with `is_model_loaded()`:

```rust
use ruvllm::evaluation::{RealEvaluationHarness, EvalConfig, RealInferenceConfig};

// EvalConfig and RealInferenceConfig both derive Default.
let harness = RealEvaluationHarness::with_config(
    EvalConfig::default(),
    RealInferenceConfig {
        model_path: "./models/tinyllama-1.1b-chat-v1.0.Q4_K_M.gguf".to_string(),
        enable_hnsw: true,
        enable_sona: true,
        ..Default::default()
    },
)?;

assert!(harness.is_model_loaded());
```

> **Planned:** a one-call `evaluate(...)` and `run_ablation_study(...)` scoring
> API (returning success/quality per `AblationMode`) is on the roadmap. The
> `AblationMode` enum (`Baseline`, `RetrievalOnly`, `AdaptersOnly`,
> `RetrievalPlusAdapters`, `Full`) already exists; the driving loop is exposed
> today via the `run_eval` example rather than a single harness method.

### Ablation Modes

| Mode | Description | Use Case |
|------|-------------|----------|
| **Baseline** | No enhancements | Control baseline |
| **RetrievalOnly** | HNSW pattern retrieval | Measure retrieval impact |
| **AdaptersOnly** | LoRA adapters | Measure adaptation impact |
| **RetrievalPlusAdapters** | HNSW + LoRA | Combined without SONA |
| **Full** | All systems (SONA + HNSW + LoRA) | Production mode |

### SWE-Bench Task Loader

```rust
use ruvllm::evaluation::swe_bench::{SweBenchLoader, SweBenchConfig};

// SweBenchConfig has `lite()` / `test()` presets; new(config) builds the loader.
let loader = SweBenchLoader::new(SweBenchConfig::lite());

// Load tasks from a JSONL file (or use load_from_file / load_from_cache_or_url).
let tasks = loader.load_from_jsonl("./data/swe-bench-lite.jsonl")?;

for task in &tasks {
    println!("Instance: {}", task.instance_id);
    println!("Problem: {}", task.problem_statement);
}
```

### CLI Evaluation

```bash
# Run evaluation with default settings
cargo run --example run_eval --features async-runtime -- \
    --model ./models/tinyllama-1.1b-chat-v1.0.Q4_K_M.gguf

# Run SWE-Bench subset
cargo run --example run_eval --features async-runtime -- \
    --model ./models/model.gguf \
    --swe-bench-path ./data/swe-bench \
    --subset lite \
    --max-tasks 100

# Output report
cargo run --example run_eval --features async-runtime -- \
    --model ./models/model.gguf \
    --output ./reports/eval-report.json
```

### HNSW Auto-Dimension Detection

The evaluation harness automatically detects model embedding dimensions:

```rust
// HNSW router automatically uses model's hidden_size
// TinyLlama 1.1B → 2048 dimensions
// Qwen2 0.5B → 896 dimensions
// RuvLTRA-Small → 896 dimensions
// RuvLTRA-Medium → 2560 dimensions

let harness = RealEvaluationHarness::with_config(
    EvalConfig::default(),
    RealInferenceConfig {
        enable_hnsw: true,
        hnsw_config: None, // Auto-detect from model
        ..Default::default()
    },
)?;
```

</details>

## Examples

See the `/examples` directory for:

- `download_test_model.rs` - Download and validate models
- `benchmark_model.rs` - Full inference benchmarking
- `run_eval.rs` - Run evaluation harness with SWE-Bench
- Basic inference
- Streaming generation
- MicroLoRA adaptation
- Multi-turn chat
- Speculative decoding
- Continuous batching
- ANE hybrid inference

## Error Handling

```rust
use ruvllm::error::{Result, RuvLLMError};

match backend.generate(prompt, params) {
    Ok(response) => println!("{}", response),
    Err(RuvLLMError::Model(e)) => eprintln!("Model error: {}", e),
    Err(RuvLLMError::OutOfMemory(e)) => eprintln!("OOM: {}", e),
    Err(RuvLLMError::Generation(e)) => eprintln!("Generation failed: {}", e),
    Err(RuvLLMError::CoreML(e)) => eprintln!("Core ML / ANE error: {}", e),
    Err(RuvLLMError::Gguf(e)) => eprintln!("GGUF loading error: {}", e),
    Err(e) => eprintln!("Error: {}", e),
}
```

## Sparse Attention — Edge / Hailo-10H

`ruvllm_sparse_attention` is the companion crate that provides the subquadratic attention kernel used for edge inference on the cognitum Pi 5 cluster. It implements ADR-183 through ADR-190 and ships as a standalone zero-runtime-dep library.

```toml
[dependencies]
ruvllm_sparse_attention = "2.2"
```

```rust
use ruvllm_sparse_attention::{
    SubquadraticSparseAttention, SparseAttentionConfig, KvCache, Tensor3, AttentionBackend,
};

// GQA prefill — Mistral-7B (32 Q heads, 8 KV heads)
let attn = SubquadraticSparseAttention::new(SparseAttentionConfig::default()).unwrap();
let q = Tensor3::zeros(512, 32, 128);
let k = Tensor3::zeros(512, 8, 128);  // 4× smaller KV cache
let v = Tensor3::zeros(512, 8, 128);
let out = attn.forward_auto(&q, &k, &v).unwrap();  // dispatches MHA or GQA automatically

// Incremental decode — O(log T) per token
let mut cache = KvCache::new(4096, 8, 128);
cache.append(&Tensor3::zeros(1, 8, 128), &Tensor3::zeros(1, 8, 128));
let out = attn.decode_step(&Tensor3::zeros(1, 32, 128), &cache).unwrap();
```

| seq | x86-64 | Pi 5 Cortex-A76 | vs dense |
|-----|--------|-----------------|---------|
| 512 | 13.1 ms | 85.8 ms | 2.2× |
| 1024 | 28.4 ms | 190.5 ms | 4.0× |
| 2048 | 60.1 ms | 401.0 ms | 7.7× |
| 4096 | 126.5 ms | 836.2 ms | 15.0× |

Validated: 17/17 tests on all 4 cognitum cluster nodes (cognitum-v0/v1/cluster-2/cluster-3). See the [kernel README](../ruvllm_sparse_attention/README.md) for full documentation.

## npm Package

RuvLLM is also available as an npm package with native bindings:

```bash
npm install @ruvector/ruvllm
```

```typescript
import { RuvLLM } from '@ruvector/ruvllm';

const llm = new RuvLLM();
const response = llm.query('Explain quantum computing');
console.log(response.text);
```

See [@ruvector/ruvllm on npm](https://www.npmjs.com/package/@ruvector/ruvllm) for full documentation.

## License

Apache-2.0 / MIT dual license.

## Contributing

Contributions welcome! Please see [CONTRIBUTING.md](../../CONTRIBUTING.md) for guidelines.

## Links

- [GitHub Repository](https://github.com/ruvnet/ruvector)
- [API Documentation](https://docs.rs/ruvllm)
- [npm Package](https://www.npmjs.com/package/@ruvector/ruvllm)
- [Issue Tracker](https://github.com/ruvnet/ruvector/issues)

---

Part of [RuVector](https://github.com/ruvnet/ruvector) -- the self-learning vector database.
