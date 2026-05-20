#!/usr/bin/env node
// Layer assignment for slice 10f (WASM satellites + misc cluster)
const fs = require('fs');

const inputPath = process.argv[2];
const outputPath = process.argv[3];

if (!inputPath || !outputPath) {
  console.error('Usage: node ua-arch-slice10f.js <input.json> <output.json>');
  process.exit(1);
}

const data = JSON.parse(fs.readFileSync(inputPath, 'utf8'));

// Layer definitions: ordered list of {id, name, description, crates[]}
const layerDefs = [
  {
    id: 'layer:slice10f-wasm-cores',
    name: 'WASM Cores',
    description: 'Foundational WebAssembly crates that expose RuVector core primitives (HNSW, CNN, DAG, graph, math, learning) to browser hosts.',
    crates: [
      'micro-hnsw-wasm',
      'ruvector-wasm',
      'ruvector-cnn-wasm',
      'ruvector-dag-wasm',
      'ruvector-graph-wasm',
      'ruvector-math-wasm',
      'ruvector-learning-wasm',
    ],
  },
  {
    id: 'layer:slice10f-wasm-attention-mincut',
    name: 'WASM Attention & Mincut Family',
    description: 'WASM crates implementing attention mechanisms, mincut partitioning, gated transformers, and hyperbolic HNSW for advanced retrieval.',
    crates: [
      'ruvector-attention-wasm',
      'ruvector-attention-unified-wasm',
      'ruvector-mincut-wasm',
      'ruvector-mincut-gated-transformer-wasm',
      'ruvector-hyperbolic-hnsw-wasm',
    ],
  },
  {
    id: 'layer:slice10f-node-bindings',
    name: 'Node Bindings',
    description: 'Node.js / CLI bindings that wrap RuVector subsystems (attention, GNN, graph, DiskANN, mincut) for server-side JavaScript consumers.',
    crates: [
      'ruvector-attention-node',
      'ruvector-attention-cli',
      'ruvector-gnn-node',
      'ruvector-graph-node',
      'ruvector-diskann-node',
      'ruvector-mincut-node',
      'ruvector-mincut-brain-node',
      'ruvector-node',
    ],
  },
  {
    id: 'layer:slice10f-specialized-wasm',
    name: 'Specialized WASM',
    description: 'Domain-specialized WASM and companion native crates: GNN, RaBitQ quantization, domain expansion, economy/exotic encoders, and sparsifiers.',
    crates: [
      'ruvector-gnn-wasm',
      'ruvector-rabitq-wasm',
      'ruvector-rabitq',
      'ruvector-domain-expansion-wasm',
      'ruvector-domain-expansion',
      'ruvector-economy-wasm',
      'ruvector-exotic-wasm',
      'ruvector-sparsifier',
      'ruvector-sparsifier-wasm',
    ],
  },
  {
    id: 'layer:slice10f-routing',
    name: 'Routing',
    description: 'Router stack — core routing engine plus CLI, FFI, and WASM surfaces for embedding RuVector routing in heterogeneous hosts.',
    crates: [
      'ruvector-router-cli',
      'ruvector-router-core',
      'ruvector-router-ffi',
      'ruvector-router-wasm',
    ],
  },
  {
    id: 'layer:slice10f-search-infra',
    name: 'Search Infrastructure',
    description: 'Native search and supporting infrastructure: DiskANN, hyperbolic HNSW, benchmarking, server, profiling, metrics, collections, CRV, dither, and filter utilities.',
    crates: [
      'ruvector-diskann',
      'ruvector-hyperbolic-hnsw',
      'ruvector-bench',
      'ruvector-server',
      'ruvector-profiler',
      'ruvector-metrics',
      'ruvector-collections',
      'ruvector-crv',
      'ruvector-dither',
      'ruvector-filter',
    ],
  },
  {
    id: 'layer:slice10f-thermal-profiling',
    name: 'Thermal & Profiling',
    description: 'Thermal-aware runtime crates and profiling utilities (ruos-thermal, thermorust, profiling) for hardware-conscious scheduling.',
    crates: [
      'ruos-thermal',
      'thermorust',
      'profiling',
    ],
  },
];

// Build crate -> layerId map and crate -> path prefix map
const crateToLayer = new Map();
for (const layer of layerDefs) {
  for (const crate of layer.crates) {
    crateToLayer.set(crate, layer.id);
  }
}

// Helper: extract crate name from a filePath like "crates/<crate>/..." or fallback
function crateFromPath(filePath) {
  if (!filePath) return null;
  const m = filePath.match(/^crates\/([^/]+)/);
  if (m) return m[1];
  return null;
}

// Helper: extract crate from a node id when filePath unavailable
function crateFromId(id) {
  if (!id) return null;
  // patterns: "crate:crates/<name>", "file:crates/<name>/...", "module:crates/<name>/..."
  const m = id.match(/crates\/([^/]+)/);
  if (m) return m[1];
  return null;
}

// Initialize layer member maps
const layerMembers = new Map();
for (const layer of layerDefs) {
  layerMembers.set(layer.id, []);
}

const unassigned = [];
const crateMisses = new Set();

for (const node of data.nodes) {
  const crate = crateFromPath(node.filePath) || crateFromId(node.id);
  let layerId = crate ? crateToLayer.get(crate) : null;
  if (!layerId) {
    // Fallback for misc / non-crates paths
    unassigned.push({ id: node.id, filePath: node.filePath, crate });
    if (crate) crateMisses.add(crate);
    continue;
  }
  layerMembers.get(layerId).push(node.id);
}

// Build output layers array
const layers = layerDefs.map(l => ({
  id: l.id,
  name: l.name,
  description: l.description,
  memberNodeIds: layerMembers.get(l.id),
}));

// Compute stats
const totalNodes = data.nodes.length;
const assignedCount = layers.reduce((s, l) => s + l.memberNodeIds.length, 0);

const output = Object.assign({}, data, { layers });

fs.writeFileSync(outputPath, JSON.stringify(output, null, 2));

console.error(`Total nodes: ${totalNodes}`);
console.error(`Assigned: ${assignedCount}`);
console.error(`Unassigned: ${unassigned.length}`);
if (crateMisses.size) {
  console.error(`Crates not mapped to any layer: ${[...crateMisses].join(', ')}`);
}
if (unassigned.length && unassigned.length <= 25) {
  console.error('Unassigned node samples:');
  for (const u of unassigned.slice(0, 25)) {
    console.error(`  ${u.id} | ${u.filePath} | crate=${u.crate}`);
  }
}
for (const l of layers) {
  console.error(`  ${l.id}: ${l.memberNodeIds.length}`);
}
