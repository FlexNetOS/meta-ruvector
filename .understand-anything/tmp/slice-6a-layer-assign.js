#!/usr/bin/env node
/**
 * Slice 6a layer assignment for the rvf crate family.
 * Reads the assembled graph, assigns every node to exactly one layer,
 * and writes the graph back out with an added `layers` array.
 */
'use strict';
const fs = require('fs');

const INPUT = process.argv[2] || '/home/drdave/repos/RuVector/.understand-anything/tmp/slice-6a-assembled.json';
const OUTPUT = process.argv[3] || '/home/drdave/repos/RuVector/.understand-anything/tmp/slice-6a-layered.json';

const data = JSON.parse(fs.readFileSync(INPUT, 'utf8'));
const nodes = data.nodes || [];

// ---------- Layer definitions ----------
const LAYERS = [
  { id: 'layer:rvf-types', name: 'Types & Schemas',
    description: 'Shared RVF data types, schemas, quality metrics, and cross-crate primitives consumed by every other rvf sub-crate.' },
  { id: 'layer:rvf-wire', name: 'Wire Protocol',
    description: 'On-the-wire codecs and segment encoding/decoding for the RVF binary format.' },
  { id: 'layer:rvf-manifest', name: 'Storage Format',
    description: 'Level-0 and Level-1 manifest readers/writers that materialize the on-disk RVF file layout.' },
  { id: 'layer:rvf-index', name: 'Index',
    description: 'Progressive HNSW vector index implementation, traversal, and persistence.' },
  { id: 'layer:rvf-quant', name: 'Quantization',
    description: 'Scalar, product, and binary quantization plus sketch structures used to compress vectors.' },
  { id: 'layer:rvf-crypto', name: 'Cryptography',
    description: 'SHAKE-256 hashing, Ed25519 signatures, and witness-chain primitives backing RVF integrity guarantees.' },
  { id: 'layer:rvf-ebpf', name: 'eBPF',
    description: 'eBPF program compiler and precompiled BPF blobs used by the kernel pipeline.' },
  { id: 'layer:rvf-kernel', name: 'Microkernel',
    description: 'Kernel pipeline, CPIO packaging, and microVM execution surface for sandboxed RVF workloads.' },
  { id: 'layer:rvf-launch', name: 'Launcher',
    description: 'Process launcher, QEMU integration, and QMP control for spinning up microVM-backed RVF runtimes.' },
  { id: 'layer:rvf-runtime', name: 'Runtime Core',
    description: 'Vector store, copy-on-write semantics, FFI surface, QR seeds, and ADR-033/036 runtime logic at the heart of RVF.' },
  { id: 'layer:rvf-server', name: 'HTTP Server',
    description: 'Axum-based HTTP server exposing RVF runtime operations over the network.' },
  { id: 'layer:rvf-cli', name: 'CLI',
    description: 'clap-based command-line interface and subcommands that drive RVF locally.' },
  { id: 'layer:rvf-federation', name: 'Federation',
    description: 'ADR-057 federated aggregation primitives and differential-privacy mechanisms across RVF nodes.' },
  { id: 'layer:rvf-import', name: 'Import',
    description: 'CSV, JSON, and NumPy parsers that ingest external data into the RVF format.' },
  { id: 'layer:rvf-wasm', name: 'WASM Microkernel',
    description: 'cdylib WASM microkernel build with FFI exports for browser/host embedding.' },
  { id: 'layer:rvf-solver', name: 'Solver',
    description: 'WASM puzzle/reasoning solver engine layered on top of the runtime.' },
  { id: 'layer:rvf-node', name: 'Node Binding',
    description: 'N-API native binding exposing RVF to Node.js applications.' },
  { id: 'layer:rvf-adapters', name: 'Adapters',
    description: 'Cross-system adapters (agentdb, agentic-flow, claude-flow, ospipe, rvlite, sona) bridging RVF to external runtimes.' },
  { id: 'layer:rvf-integration', name: 'Integration Tests',
    description: 'Cross-crate integration test suite under crates/rvf/tests exercising end-to-end RVF behavior.' },
  { id: 'layer:rvf-benches', name: 'Benchmarks',
    description: 'Criterion benches measuring wire, index, distance, quantization, manifest, runtime, and crypto performance.' },
  { id: 'layer:rvf-docs', name: 'Documentation',
    description: 'Architecture decision records and security audits documenting RVF design and guarantees.' },
  { id: 'layer:rvf-workspace', name: 'Workspace Root',
    description: 'Top-level rvf workspace manifest and README tying the sub-crate family together.' },
];
const LAYER_INDEX = Object.fromEntries(LAYERS.map(l => [l.id, []]));

// ---------- Routing helper ----------
function layerForPath(fp) {
  if (!fp) return null;
  // Strip 'crates/rvf/' prefix
  if (!fp.startsWith('crates/rvf/')) return null;
  const rest = fp.slice('crates/rvf/'.length);
  // Top-level workspace files
  if (rest === 'Cargo.toml' || rest === 'README.md') return 'layer:rvf-workspace';
  // Docs
  if (rest.startsWith('docs/')) return 'layer:rvf-docs';
  // Benches
  if (rest.startsWith('benches/') || rest === 'benches') return 'layer:rvf-benches';
  // Tests
  if (rest.startsWith('tests/')) return 'layer:rvf-integration';
  // Sub-crate routing
  const seg = rest.split('/')[0];
  switch (seg) {
    case 'rvf-types': return 'layer:rvf-types';
    case 'rvf-wire': return 'layer:rvf-wire';
    case 'rvf-manifest': return 'layer:rvf-manifest';
    case 'rvf-index': return 'layer:rvf-index';
    case 'rvf-quant': return 'layer:rvf-quant';
    case 'rvf-crypto': return 'layer:rvf-crypto';
    case 'rvf-ebpf': return 'layer:rvf-ebpf';
    case 'rvf-kernel': return 'layer:rvf-kernel';
    case 'rvf-launch': return 'layer:rvf-launch';
    case 'rvf-runtime': return 'layer:rvf-runtime';
    case 'rvf-server': return 'layer:rvf-server';
    case 'rvf-cli': return 'layer:rvf-cli';
    case 'rvf-federation': return 'layer:rvf-federation';
    case 'rvf-import': return 'layer:rvf-import';
    case 'rvf-wasm': return 'layer:rvf-wasm';
    case 'rvf-solver-wasm': return 'layer:rvf-solver';
    case 'rvf-node': return 'layer:rvf-node';
    case 'rvf-adapters': return 'layer:rvf-adapters';
  }
  return null;
}

// ---------- Assign every node ----------
const unassigned = [];
for (const n of nodes) {
  const layerId = layerForPath(n.filePath);
  if (layerId && LAYER_INDEX[layerId]) {
    LAYER_INDEX[layerId].push(n.id);
  } else {
    unassigned.push(n);
  }
}

// ---------- Emit, skipping empty layers ----------
const emittedLayers = [];
for (const l of LAYERS) {
  const memberNodeIds = LAYER_INDEX[l.id];
  if (memberNodeIds.length === 0) continue;
  emittedLayers.push({
    id: l.id,
    name: l.name,
    description: l.description,
    memberNodeIds,
  });
}

const out = { ...data, layers: emittedLayers };
fs.writeFileSync(OUTPUT, JSON.stringify(out, null, 2));

const totalAssigned = emittedLayers.reduce((s, l) => s + l.memberNodeIds.length, 0);
console.log(`layers emitted: ${emittedLayers.length}`);
console.log(`total nodes: ${nodes.length}`);
console.log(`assigned:    ${totalAssigned}`);
console.log(`unassigned:  ${unassigned.length}`);
if (unassigned.length) {
  console.log('--- unassigned sample (first 20) ---');
  for (const n of unassigned.slice(0, 20)) {
    console.log(' ', n.id, '|', n.type, '|', n.filePath);
  }
}
for (const l of emittedLayers) {
  console.log(`  ${l.id.padEnd(28)} -> ${l.memberNodeIds.length}`);
}
