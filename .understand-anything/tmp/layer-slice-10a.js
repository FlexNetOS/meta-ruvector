#!/usr/bin/env node
// Layer assigner for slice 10a (ruvix crate)
const fs = require('fs');
const path = require('path');

const INPUT = process.argv[2];
const OUTPUT = process.argv[3];

if (!INPUT || !OUTPUT) {
  console.error('Usage: layer-slice-10a.js <input.json> <output.json>');
  process.exit(1);
}

const graph = JSON.parse(fs.readFileSync(INPUT, 'utf8'));

// Layer definitions, with deterministic ordering and matching strategy.
// For each node we compute a relative path beneath "crates/ruvix/" and match
// it against a sub-crate name (under crates/ruvix/crates/<name>/) OR against
// a top-level segment (aarch64-boot, benches, examples/..., qemu-swarm, tests).
// Crate-typed nodes are matched against their name property as a fallback.

const SUBCRATE_LAYERS = {
  aarch64:   { id: 'layer:slice10a-aarch64',     name: 'ARM64 HAL',          description: 'ARM64-specific boot, exception vectors, MMU, and system registers for the ruvix kernel.' },
  bcm2711:   { id: 'layer:slice10a-bcm2711',     name: 'BCM2711 Drivers',    description: 'Raspberry Pi 4 (BCM2711) SoC peripheral drivers including GPIO, mailbox, and timers.' },
  boot:      { id: 'layer:slice10a-boot',        name: 'Boot Pipeline',      description: 'Boot loader stages, early initialisation, attestation, and witness log for the ruvix kernel bring-up.' },
  cap:       { id: 'layer:slice10a-cap',         name: 'Capability System',  description: 'Capability tokens, grants, and rights management underpinning ruvix\'s capability-based security model.' },
  cli:       { id: 'layer:slice10a-cli',         name: 'Host CLI',           description: 'Host-side `ruvix` command-line binary for driving the microhypervisor from a developer workstation.' },
  dma:       { id: 'layer:slice10a-dma',         name: 'DMA',                description: 'DMA controller abstractions and descriptor management for high-throughput device transfers.' },
  drivers:   { id: 'layer:slice10a-drivers',     name: 'Generic Drivers',    description: 'Cross-platform device drivers such as the GIC interrupt controller and PL011 UART.' },
  dtb:       { id: 'layer:slice10a-dtb',         name: 'Device Tree',        description: 'Flattened Device Tree (DTB) parser used to discover hardware topology at boot.' },
  fs:        { id: 'layer:slice10a-fs',          name: 'Filesystem',         description: 'BlockDevice trait plus FAT32 and RamFs filesystem implementations for kernel storage.' },
  hal:       { id: 'layer:slice10a-hal',         name: 'HAL',                description: 'Architecture-agnostic hardware abstraction layer traits and shared utilities.' },
  net:       { id: 'layer:slice10a-net',         name: 'Networking',         description: 'Embedded network stack covering ARP, IPv4, UDP, ICMP, and the NetworkStack facade.' },
  nucleus:   { id: 'layer:slice10a-nucleus',     name: 'Kernel Nucleus',     description: 'Kernel nucleus aggregating syscalls, dispatch, and core kernel primitives.' },
  physmem:   { id: 'layer:slice10a-physmem',     name: 'Physical Memory',    description: 'Physical memory management with a buddy allocator and frame accounting.' },
  proof:     { id: 'layer:slice10a-proof',       name: 'Proof / Attestation', description: 'Proof engine, attestation tiers, and tier router supporting proof-gated mutation.' },
  queue:     { id: 'layer:slice10a-queue',       name: 'Queue',              description: 'In-kernel message queue primitive used for inter-task and inter-component communication.' },
  region:    { id: 'layer:slice10a-region',      name: 'Memory Region',      description: 'Virtual memory region manager handling mappings, permissions, and address space layout.' },
  'rpi-boot':{ id: 'layer:slice10a-rpi-boot',    name: 'Pi Boot',            description: 'Raspberry Pi specific boot configuration, firmware glue, and platform initialisation.' },
  sched:     { id: 'layer:slice10a-sched',       name: 'Scheduler',          description: 'Task scheduler and task-control structures driving ruvix\'s execution model.' },
  shell:     { id: 'layer:slice10a-shell',       name: 'Shell',              description: 'Interactive shell command parser and executor for in-kernel introspection.' },
  smp:       { id: 'layer:slice10a-smp',         name: 'SMP',                description: 'Symmetric multiprocessing support including per-CPU state and inter-processor interrupts.' },
  types:     { id: 'layer:slice10a-types',       name: 'Typed Handles',      description: 'Typed handle and ID newtypes shared across ruvix sub-crates for type-safe interfaces.' },
  vecgraph:  { id: 'layer:slice10a-vecgraph',    name: 'VecGraph',           description: 'Vector/graph store with HNSW indexing and SIMD distance kernels for in-kernel cognition.' },
};

const TOP_LEVEL_LAYERS = {
  'aarch64-boot':           { id: 'layer:slice10a-aarch64-boot', name: 'AArch64 Boot Crate', description: 'Standalone aarch64-boot crate providing the lowest-level ARM64 boot stub for ruvix images.' },
  'qemu-swarm':             { id: 'layer:slice10a-qemu-testbed', name: 'QEMU Testbed',       description: 'QEMU multi-node swarm harness used to exercise ruvix in a virtual cluster.' },
  'examples/cognitive_demo':{ id: 'layer:slice10a-cognitive-demo', name: 'Cognitive Demo',   description: 'End-to-end cognitive pipeline example showcasing ruvix syscalls and the VecGraph store.' },
  'examples/rvf-demos':     { id: 'layer:slice10a-rvf-demos',    name: 'RVF Demos',          description: 'Additional reference-virtual-firmware demo programs exercising the ruvix runtime.' },
  'tests':                  { id: 'layer:slice10a-integration-tests', name: 'Integration Tests', description: 'Integration tests that validate ruvix subsystems against the public crate surface.' },
  'benches':                { id: 'layer:slice10a-benchmarks',   name: 'Benchmarks',         description: 'Criterion benchmarks and benchmark harness binaries comparing ruvix against Linux baselines.' },
  'workspace-meta':         { id: 'layer:slice10a-workspace-meta', name: 'Workspace Meta',   description: 'Top-level ruvix crate manifest, README, and aggregate workspace metadata.' },
};

function classify(node) {
  // For crate nodes, prefer matching by name first; their filePath usually
  // points at the sub-crate's Cargo.toml or the root crate folder.
  const filePath = node.filePath || '';
  const name = node.name || '';

  // Strip leading 'crates/ruvix/' to get a relative path within the slice.
  const PREFIX = 'crates/ruvix/';
  let rel = filePath;
  if (filePath === 'crates/ruvix') {
    return TOP_LEVEL_LAYERS['workspace-meta'].id;
  }
  if (rel.startsWith(PREFIX)) rel = rel.slice(PREFIX.length);
  else if (filePath && !filePath.startsWith('crates/ruvix')) {
    // Fallback: not under crates/ruvix at all — treat as workspace meta.
    return TOP_LEVEL_LAYERS['workspace-meta'].id;
  }

  // Sub-crate under crates/ruvix/crates/<name>/...
  const subMatch = rel.match(/^crates\/([^\/]+)(?:\/|$)/);
  if (subMatch) {
    const sub = subMatch[1];
    if (SUBCRATE_LAYERS[sub]) return SUBCRATE_LAYERS[sub].id;
  }

  // examples/<name>/...
  const exMatch = rel.match(/^examples\/([^\/]+)(?:\/|$)/);
  if (exMatch) {
    const ex = exMatch[1];
    const key = `examples/${ex}`;
    if (TOP_LEVEL_LAYERS[key]) return TOP_LEVEL_LAYERS[key].id;
    // Fallback for an example we haven't named explicitly.
    return TOP_LEVEL_LAYERS['examples/rvf-demos'].id;
  }

  // Top-level directories
  const topSeg = rel.split('/')[0];
  if (topSeg === 'aarch64-boot') return TOP_LEVEL_LAYERS['aarch64-boot'].id;
  if (topSeg === 'qemu-swarm')   return TOP_LEVEL_LAYERS['qemu-swarm'].id;
  if (topSeg === 'tests')        return TOP_LEVEL_LAYERS['tests'].id;
  if (topSeg === 'benches')      return TOP_LEVEL_LAYERS['benches'].id;

  // Crate-typed nodes: try matching by name.
  if (node.type === 'crate') {
    // names look like 'ruvix-<sub>'
    const nm = name.replace(/^ruvix-/, '');
    if (SUBCRATE_LAYERS[nm]) return SUBCRATE_LAYERS[nm].id;
    if (nm === 'aarch64-boot') return TOP_LEVEL_LAYERS['aarch64-boot'].id;
    if (nm === 'qemu-swarm')   return TOP_LEVEL_LAYERS['qemu-swarm'].id;
    if (name === 'ruvix')      return TOP_LEVEL_LAYERS['workspace-meta'].id;
  }

  // Top-level files (Cargo.toml, README.md at slice root) → workspace meta
  if (!rel.includes('/')) {
    return TOP_LEVEL_LAYERS['workspace-meta'].id;
  }

  // Unknown → workspace meta as a safety catch-all
  return TOP_LEVEL_LAYERS['workspace-meta'].id;
}

// Build layer membership.
const layerMembership = new Map(); // layerId -> Set of node ids
const allLayerDefs = new Map();    // layerId -> def

for (const def of Object.values(SUBCRATE_LAYERS)) allLayerDefs.set(def.id, def);
for (const def of Object.values(TOP_LEVEL_LAYERS)) allLayerDefs.set(def.id, def);

let unassigned = 0;
for (const node of graph.nodes) {
  const layerId = classify(node);
  if (!layerId) { unassigned++; continue; }
  if (!layerMembership.has(layerId)) layerMembership.set(layerId, []);
  layerMembership.get(layerId).push(node.id);
}

// Preserve a stable order: subcrates first (alphabetical), then top-level.
const orderedLayerIds = [
  ...Object.keys(SUBCRATE_LAYERS).sort().map(k => SUBCRATE_LAYERS[k].id),
  TOP_LEVEL_LAYERS['aarch64-boot'].id,
  TOP_LEVEL_LAYERS['qemu-swarm'].id,
  TOP_LEVEL_LAYERS['examples/cognitive_demo'].id,
  TOP_LEVEL_LAYERS['examples/rvf-demos'].id,
  TOP_LEVEL_LAYERS['tests'].id,
  TOP_LEVEL_LAYERS['benches'].id,
  TOP_LEVEL_LAYERS['workspace-meta'].id,
];

const layers = [];
for (const layerId of orderedLayerIds) {
  const def = allLayerDefs.get(layerId);
  const members = layerMembership.get(layerId) || [];
  if (members.length === 0) continue; // drop empty layers
  layers.push({
    id: def.id,
    name: def.name,
    description: def.description,
    memberNodeIds: members,
  });
}

// Sanity: every node accounted for exactly once.
const totalAssigned = layers.reduce((s, l) => s + l.memberNodeIds.length, 0);

const out = Object.assign({}, graph, { layers });
fs.writeFileSync(OUTPUT, JSON.stringify(out, null, 2));

console.error(`Layers: ${layers.length}`);
console.error(`Total nodes: ${graph.nodes.length}`);
console.error(`Assigned:    ${totalAssigned}`);
console.error(`Unassigned:  ${unassigned}`);
for (const l of layers) console.error(`  ${l.id.padEnd(40)} ${String(l.memberNodeIds.length).padStart(5)} nodes`);

if (totalAssigned + unassigned !== graph.nodes.length) {
  console.error('ERROR: node accounting mismatch');
  process.exit(2);
}
