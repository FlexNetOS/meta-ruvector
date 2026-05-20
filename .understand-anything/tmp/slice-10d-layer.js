#!/usr/bin/env node
'use strict';

const fs = require('fs');

const inputPath = process.argv[2];
const outputPath = process.argv[3];

if (!inputPath || !outputPath) {
  console.error('Usage: slice-10d-layer.js <input> <output>');
  process.exit(1);
}

const data = JSON.parse(fs.readFileSync(inputPath, 'utf8'));

// Crate -> layer mapping
const crateToLayer = {
  // LLM extras
  'ruvllm-cli': 'slice10d-llm-extras',
  'ruvllm-wasm': 'slice10d-llm-extras',
  'ruvllm_retrieval_diffusion': 'slice10d-llm-extras',
  'ruvllm_sparse_attention': 'slice10d-llm-extras',
  // Decompiler
  'ruvector-decompiler': 'slice10d-decompiler',
  'ruvector-decompiler-wasm': 'slice10d-decompiler',
  // Solver
  'ruvector-solver': 'slice10d-solver',
  'ruvector-solver-node': 'slice10d-solver',
  'ruvector-solver-wasm': 'slice10d-solver',
  // Temporal Tensor
  'ruvector-temporal-tensor': 'slice10d-temporal-tensor',
  'ruvector-temporal-tensor-wasm': 'slice10d-temporal-tensor',
  // Graph Transformer
  'ruvector-graph-transformer': 'slice10d-graph-transformer',
  'ruvector-graph-transformer-node': 'slice10d-graph-transformer',
  'ruvector-graph-transformer-wasm': 'slice10d-graph-transformer',
};

const layerDefs = {
  'slice10d-llm-extras': {
    id: 'layer:slice10d-llm-extras',
    name: 'LLM Extras',
    description: 'CLI, WASM bindings, retrieval-diffusion, and sparse-attention extensions to the ruvllm runtime.',
    nodeIds: [],
  },
  'slice10d-decompiler': {
    id: 'layer:slice10d-decompiler',
    name: 'Decompiler',
    description: 'Binary/IR decompilation engine with its WASM-targeted companion crate.',
    nodeIds: [],
  },
  'slice10d-solver': {
    id: 'layer:slice10d-solver',
    name: 'Solver',
    description: 'Constraint/optimization solver core plus Node.js and WASM target bindings.',
    nodeIds: [],
  },
  'slice10d-temporal-tensor': {
    id: 'layer:slice10d-temporal-tensor',
    name: 'Temporal Tensor',
    description: 'Time-aware tensor primitives and the matching WASM build.',
    nodeIds: [],
  },
  'slice10d-graph-transformer': {
    id: 'layer:slice10d-graph-transformer',
    name: 'Graph Transformer',
    description: 'Graph-attention transformer engine exposed natively, via Node, and via WASM.',
    nodeIds: [],
  },
};

let unassigned = 0;
const unassignedSamples = [];

for (const node of data.nodes) {
  const fp = node.filePath || '';
  const parts = fp.split('/');
  let crate = null;
  if (parts.length >= 1) {
    // Top-level prefix is the crate (slice has no 'crates/' prefix in filePath)
    crate = parts[0];
    if (crate === 'crates' && parts.length >= 2) crate = parts[1];
  }
  const layerKey = crateToLayer[crate];
  if (!layerKey) {
    unassigned++;
    if (unassignedSamples.length < 10) unassignedSamples.push({ id: node.id, filePath: fp });
    continue;
  }
  layerDefs[layerKey].nodeIds.push(node.id);
}

const layers = Object.values(layerDefs);

// Attach to data
data.layers = layers;

fs.writeFileSync(outputPath, JSON.stringify(data, null, 2));

const totalAssigned = layers.reduce((s, l) => s + l.nodeIds.length, 0);
console.log(`Layers: ${layers.length}`);
for (const l of layers) {
  console.log(`  ${l.id}: ${l.nodeIds.length} nodes`);
}
console.log(`Total assigned: ${totalAssigned}`);
console.log(`Total nodes: ${data.nodes.length}`);
console.log(`Unassigned: ${unassigned}`);
if (unassigned > 0) {
  console.log('Sample unassigned:', JSON.stringify(unassignedSamples, null, 2));
}
