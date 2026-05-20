#!/usr/bin/env node
// Validation script for slice-6a-with-tour.json
// Output: /home/drdave/repos/RuVector/.understand-anything/tmp/slice-6a-graph-review.json

const fs = require('fs');

const inputPath = process.argv[2];
const outputPath = process.argv[3];

if (!inputPath || !outputPath) {
  console.error('Usage: validate-slice-6a.js <input> <output>');
  process.exit(1);
}

let g;
try {
  g = JSON.parse(fs.readFileSync(inputPath, 'utf8'));
} catch (e) {
  console.error('Cannot read/parse input:', e.message);
  process.exit(1);
}

const issues = [];
const recommendations = [];

const nodes = g.nodes || [];
const edges = g.edges || [];
const layers = g.layers || [];
const tour = g.tour || {};

// --- Check 1: Unique node IDs ---
const idCount = new Map();
const indicesById = new Map();
nodes.forEach((n, i) => {
  if (!n || typeof n.id !== 'string' || n.id.length === 0) {
    issues.push(`Node at index ${i} has missing or invalid 'id'`);
    return;
  }
  idCount.set(n.id, (idCount.get(n.id) || 0) + 1);
  if (!indicesById.has(n.id)) indicesById.set(n.id, []);
  indicesById.get(n.id).push(i);
});
const duplicateIds = [];
for (const [id, c] of idCount) {
  if (c > 1) {
    duplicateIds.push(id);
    issues.push(`Duplicate node id '${id}' appears ${c} times at indices: ${indicesById.get(id).join(', ')}`);
  }
}

const nodeIdSet = new Set(idCount.keys());

// --- Check 2: Edge referential integrity ---
let danglingEdges = 0;
edges.forEach((e, i) => {
  if (!e || typeof e.source !== 'string' || typeof e.target !== 'string') {
    issues.push(`Edge at index ${i} missing string source/target`);
    danglingEdges++;
    return;
  }
  if (!nodeIdSet.has(e.source)) {
    issues.push(`Edge at index ${i} references non-existent source '${e.source}' (type=${e.type}, target=${e.target})`);
    danglingEdges++;
  }
  if (!nodeIdSet.has(e.target)) {
    issues.push(`Edge at index ${i} references non-existent target '${e.target}' (type=${e.type}, source=${e.source})`);
    danglingEdges++;
  }
});

// --- Check 3: Layer coverage — every node in exactly one layer ---
const nodeToLayers = new Map();
const layerIds = new Set();
layers.forEach((layer, li) => {
  if (!layer || !layer.id) {
    issues.push(`Layer at index ${li} missing 'id'`);
    return;
  }
  if (layerIds.has(layer.id)) {
    issues.push(`Duplicate layer id '${layer.id}'`);
  }
  layerIds.add(layer.id);
  if (!layer.description || typeof layer.description !== 'string' || layer.description.trim().length === 0) {
    issues.push(`Layer '${layer.id}' missing description`);
  }
  const memberIds = layer.memberNodeIds || layer.nodeIds || [];
  if (!Array.isArray(memberIds) || memberIds.length === 0) {
    issues.push(`Layer '${layer.id}' has empty memberNodeIds`);
  }
  memberIds.forEach((nid) => {
    if (!nodeIdSet.has(nid)) {
      issues.push(`Layer '${layer.id}' references non-existent node '${nid}'`);
    }
    if (!nodeToLayers.has(nid)) nodeToLayers.set(nid, []);
    nodeToLayers.get(nid).push(layer.id);
  });
});

// Detect nodes missing from any layer
const nodesMissingFromLayers = [];
const nodesInMultipleLayers = [];
for (const nid of nodeIdSet) {
  const inLayers = nodeToLayers.get(nid) || [];
  if (inLayers.length === 0) {
    nodesMissingFromLayers.push(nid);
  } else if (inLayers.length > 1) {
    nodesInMultipleLayers.push({ id: nid, layers: inLayers });
  }
}
if (nodesMissingFromLayers.length > 0) {
  // Limit detail to first 10
  const sample = nodesMissingFromLayers.slice(0, 10).join(', ');
  issues.push(`${nodesMissingFromLayers.length} node(s) not assigned to any layer. First: ${sample}`);
}
if (nodesInMultipleLayers.length > 0) {
  const sample = nodesInMultipleLayers.slice(0, 10).map(x => `${x.id}=>[${x.layers.join(',')}]`).join('; ');
  issues.push(`${nodesInMultipleLayers.length} node(s) appear in multiple layers. First: ${sample}`);
}

// --- Check 4: Tour validation ---
// Tour is an object keyed "0".."N-1" with step objects containing step/title/description/focusNodeIds/focusLayerIds
const tourKeys = Object.keys(tour).sort((a, b) => parseInt(a) - parseInt(b));
if (tourKeys.length === 0) {
  issues.push('Tour has zero steps');
} else {
  const seenOrder = new Set();
  tourKeys.forEach((k) => {
    const step = tour[k];
    if (!step || typeof step !== 'object') {
      issues.push(`Tour step key '${k}' is not an object`);
      return;
    }
    if (typeof step.step !== 'number') {
      issues.push(`Tour step '${k}' missing numeric 'step' field`);
    } else {
      if (seenOrder.has(step.step)) issues.push(`Tour has duplicate step order ${step.step}`);
      seenOrder.add(step.step);
    }
    if (!step.title || typeof step.title !== 'string') {
      issues.push(`Tour step '${k}' missing title`);
    }
    if (!step.description || typeof step.description !== 'string') {
      issues.push(`Tour step '${k}' missing description`);
    }
    const focus = step.focusNodeIds || [];
    if (!Array.isArray(focus) || focus.length === 0) {
      issues.push(`Tour step '${k}' has empty focusNodeIds`);
    } else {
      focus.forEach((nid) => {
        if (!nodeIdSet.has(nid)) {
          issues.push(`Tour step '${k}' references non-existent focus node '${nid}'`);
        }
      });
    }
    const focusLayers = step.focusLayerIds || [];
    if (Array.isArray(focusLayers)) {
      focusLayers.forEach((lid) => {
        if (!layerIds.has(lid)) {
          issues.push(`Tour step '${k}' references non-existent layer '${lid}'`);
        }
      });
    }
  });
}

// --- Recommendations / warnings ---
// Orphan nodes (no edges in or out)
const nodeEdgeCount = new Map();
edges.forEach((e) => {
  if (!e) return;
  nodeEdgeCount.set(e.source, (nodeEdgeCount.get(e.source) || 0) + 1);
  nodeEdgeCount.set(e.target, (nodeEdgeCount.get(e.target) || 0) + 1);
});
const orphanNodes = [];
for (const nid of nodeIdSet) {
  if (!nodeEdgeCount.has(nid)) orphanNodes.push(nid);
}
if (orphanNodes.length > 0) {
  recommendations.push(`${orphanNodes.length} orphan node(s) have no edges. Sample: ${orphanNodes.slice(0, 5).join(', ')}`);
}

// Node type / id prefix consistency (informational)
const prefixMismatches = [];
nodes.forEach((n) => {
  if (!n || !n.id || !n.type) return;
  const colon = n.id.indexOf(':');
  if (colon < 0) return;
  const prefix = n.id.slice(0, colon);
  if (prefix !== n.type) prefixMismatches.push(`${n.id} (type=${n.type})`);
});
if (prefixMismatches.length > 0) {
  recommendations.push(`${prefixMismatches.length} node(s) have id prefix that doesn't match 'type'. Sample: ${prefixMismatches.slice(0, 5).join(', ')}`);
}

// Self-loops
const selfLoops = edges.filter(e => e && e.source === e.target);
if (selfLoops.length > 0) {
  recommendations.push(`${selfLoops.length} self-referencing edge(s)`);
}

// Tour count
if (tourKeys.length < 5 || tourKeys.length > 15) {
  recommendations.push(`Tour has ${tourKeys.length} steps (outside typical 5-15 range)`);
}

// Compute stats
const nodeTypes = {};
const edgeTypes = {};
nodes.forEach(n => { if (n && n.type) nodeTypes[n.type] = (nodeTypes[n.type] || 0) + 1; });
edges.forEach(e => { if (e && e.type) edgeTypes[e.type] = (edgeTypes[e.type] || 0) + 1; });

const result = {
  approved: issues.length === 0,
  issues,
  recommendations,
  stats: {
    nodes: nodes.length,
    edges: edges.length,
    layers: layers.length,
    tourSteps: tourKeys.length,
    orphanNodes: orphanNodes.length,
    danglingEdges,
    duplicateIds: duplicateIds.length,
    nodesMissingFromLayers: nodesMissingFromLayers.length,
    nodesInMultipleLayers: nodesInMultipleLayers.length,
    nodeTypes,
    edgeTypes
  }
};

fs.writeFileSync(outputPath, JSON.stringify(result, null, 2));
console.log(`Validation complete. Approved: ${result.approved}. Issues: ${issues.length}. Recommendations: ${recommendations.length}.`);
console.log(`Wrote: ${outputPath}`);
