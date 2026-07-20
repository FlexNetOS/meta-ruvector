# ruvector-proof-gate

**Tamper-evident vector writes** — a Merkle / hash-chain write-ahead log that gives every write to a
vector store cryptographic evidence of *what* was stored, *when*, and *by whom*. Part of the
[ruvector](https://github.com/ruvnet/ruvector) ecosystem.

> Every major vector DB (Qdrant, Milvus, Weaviate, LanceDB, FAISS) accepts writes with **zero**
> integrity evidence. This crate closes that gap — the defense against silent memory poisoning
> (the *MemoryGraft* attack, arxiv 2512.16962).

## What it gives you

Admit a `WritePayload`, get a `WriteReceipt` you can store alongside the index entry and verify
offline. The gate's **chain root is a cryptographic commitment to the entire ordered write log**: any
mutation, insertion, deletion, or reorder of writes produces a different root.

## Gates

All three gates implement the `WriteGate` trait (`admit`, `verify_receipt`,
`chain_root`, `len`, `is_empty`, `variant`).

| gate | per-write cost | property |
|---|---|---|
| `NullGate` | ~0 | baseline (no integrity) — throughput ceiling |
| `HashChainGate` | O(1), ~700 ns | sequential SHA-256 chain |
| `MerkleGate` | O(log n) | Merkle Mountain Range |

`HashChainGate` additionally provides an inherent `verify_integrity()` method
that re-derives the whole chain from genesis (see below).

## Usage

```rust
use ruvector_proof_gate::{HashChainGate, WriteGate, WritePayload};

let mut gate = HashChainGate::new();

// WritePayload::new takes (id: u64, vector: Vec<f32>).
let payload = WritePayload::new(0, vec![0.1, 0.2, 0.3]);
let receipt = gate.admit(&payload).unwrap();

// store `receipt` with the index entry; later:
assert!(gate.verify_receipt(&receipt));   // ~6 ns, available on every gate
let root = gate.chain_root();              // publish/anchor this commitment
```

Optional payload metadata is set with builder methods on `WritePayload`:
`with_metadata(Vec<u8>)`, `with_agent([u8; 16])`, `with_timestamp(u64)`.

`HashChainGate` (only) offers a full re-derivation check:

```rust
use ruvector_proof_gate::{HashChainGate, WriteGate, WritePayload};

let mut gate = HashChainGate::new();
gate.admit(&WritePayload::new(0, vec![0.1, 0.2, 0.3])).unwrap();
assert!(gate.verify_integrity());          // re-derives the chain from genesis
```

## Performance

Measured (DIM=128): **HashChainGate.admit ≈ 700 ns/write (~1.4 M/s)**, `verify_receipt ≈ 6 ns`
(157 M/s). The integrity tax is ~675 ns/write — negligible next to the HNSW insertion a real write
already performs.

## Guarantees (tested)

- Any mutation / insertion / deletion / reorder changes the root.
- Forged commitments, out-of-range receipts, and foreign-chain receipts are rejected (no panic).
- `HashChainGate::verify_integrity()` re-derives every commitment from genesis + stored payload
  hashes — it catches a tamper that mutates a commitment, a payload hash, the order, or desyncs lengths.

## Features

- `serde` — derive serialization for receipts (off by default).

## License

MIT OR Apache-2.0 © Ruvector Team. See ADR-194 for design notes.
