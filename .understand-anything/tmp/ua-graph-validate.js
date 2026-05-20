#!/usr/bin/env node
'use strict';

const fs = require('fs');

const VALID_NODE_TYPES = new Set([
  'file', 'function', 'class', 'module', 'concept', 'config', 'document',
  'service', 'table', 'endpoint', 'pipeline', 'schema', 'resource',
  'domain', 'flow', 'step'
]);

const VALID_EDGE_TYPES = new Set([
  'imports', 'exports', 'contains', 'inherits', 'implements', 'calls',
  'subscribes', 'publishes', 'middleware', 'reads_from', 'writes_to',
  'transforms', 'validates', 'depends_on', 'tested_by', 'configures',
  'related', 'similar_to', 'deploys', 'serves', 'migrates', 'documents',
  'provisions', 'routes', 'defines_schema', 'triggers', 'contains_flow',
  'flow_step', 'cross_domain'
]);

const VALID_COMPLEXITY = new Set(['simple', 'moderate', 'complex']);
const VALID_DIRECTIONS = new Set(['forward', 'backward', 'bidirectional']);

const FILE_LEVEL_TYPES = new Set([
  'file', 'config', 'document', 'service', 'pipeline', 'table',
  'schema', 'resource', 'endpoint'
]);

function isString(v) { return typeof v === 'string'; }
function isNonEmptyString(v) { return typeof v === 'string' && v.length > 0; }

function main() {
  const inputPath = process.argv[2];
  const outputPath = process.argv[3];

  if (!inputPath || !outputPath) {
    console.error('Usage: node ua-graph-validate.js <graph.json> <output.json>');
    process.exit(1);
  }

  let raw;
  try { raw = fs.readFileSync(inputPath, 'utf8'); }
  catch (e) { console.error('Read error:', e.message); process.exit(1); }

  let graph;
  try { graph = JSON.parse(raw); }
  catch (e) { console.error('JSON parse error:', e.message); process.exit(1); }

  const issues = [];
  const warnings = [];

  const nodes = Array.isArray(graph.nodes) ? graph.nodes : [];
  const edges = Array.isArray(graph.edges) ? graph.edges : [];
  const layers = Array.isArray(graph.layers) ? graph.layers : [];
  const tour = (graph.tour && Array.isArray(graph.tour.steps)) ? graph.tour.steps
              : (Array.isArray(graph.tourSteps) ? graph.tourSteps
              : (Array.isArray(graph.tour) ? graph.tour : []));

  // Build id -> indices map for duplicates
  const idIndices = new Map();
  nodes.forEach((n, i) => {
    if (n && isString(n.id)) {
      if (!idIndices.has(n.id)) idIndices.set(n.id, []);
      idIndices.get(n.id).push(i);
    }
  });

  // Check 5 - Duplicates
  for (const [id, indices] of idIndices.entries()) {
    if (indices.length > 1) {
      issues.push(`Duplicate node id '${id}' appears at indices: ${indices.join(', ')}`);
    }
  }

  const nodeIds = new Set(idIndices.keys());
  const nodeById = new Map();
  nodes.forEach(n => { if (n && isString(n.id)) nodeById.set(n.id, n); });

  // Detect domain graph
  const hasDomainNodes = nodes.some(n => n && (n.type === 'domain' || n.type === 'flow' || n.type === 'step'));

  // Check 1 - Node schema
  const nodeTypeCounts = {};
  nodes.forEach((n, i) => {
    if (!n || typeof n !== 'object') {
      issues.push(`Node at index ${i} is not an object`);
      return;
    }
    if (!isNonEmptyString(n.id)) issues.push(`Node at index ${i} missing/empty 'id'`);
    if (!isNonEmptyString(n.type)) {
      issues.push(`Node at index ${i} (${n.id || '?'}) missing/empty 'type'`);
    } else if (!VALID_NODE_TYPES.has(n.type)) {
      issues.push(`Node '${n.id}' has invalid type '${n.type}'`);
    } else {
      nodeTypeCounts[n.type] = (nodeTypeCounts[n.type] || 0) + 1;
    }
    if (!isNonEmptyString(n.name)) issues.push(`Node '${n.id}' missing/empty 'name'`);
    if (!isNonEmptyString(n.summary)) {
      issues.push(`Node '${n.id}' missing/empty 'summary'`);
    } else {
      // quality: summary not just the name
      if (n.name && n.summary.trim() === n.name.trim()) {
        warnings.push(`Node '${n.id}' summary equals its name`);
      }
    }
    if (!Array.isArray(n.tags) || n.tags.length < 1) {
      issues.push(`Node '${n.id}' missing 'tags' or empty tag array`);
    } else {
      n.tags.forEach((t, ti) => {
        if (!isString(t)) {
          issues.push(`Node '${n.id}' tag at ${ti} is not a string`);
        } else if (t !== t.toLowerCase() || /\s|_/.test(t)) {
          // lowercase and hyphenated (allow alphanumerics, dots and hyphens)
          // Soft check: warn instead of error since this is style
          warnings.push(`Node '${n.id}' tag '${t}' not lowercase-hyphenated`);
        }
      });
    }
    if (!isNonEmptyString(n.complexity)) {
      issues.push(`Node '${n.id}' missing 'complexity'`);
    } else if (!VALID_COMPLEXITY.has(n.complexity)) {
      issues.push(`Node '${n.id}' has invalid complexity '${n.complexity}'`);
    }
  });

  // Check 9 - type / id prefix consistency (warning)
  nodes.forEach(n => {
    if (!n || !isString(n.id) || !isString(n.type)) return;
    const colon = n.id.indexOf(':');
    if (colon <= 0) {
      warnings.push(`Node '${n.id}' id has no recognised prefix`);
      return;
    }
    const prefix = n.id.slice(0, colon);
    if (prefix !== n.type) {
      warnings.push(`Node '${n.id}' has prefix '${prefix}' but type '${n.type}'`);
    }
  });

  // Check 1/2 - Edge schema + referential integrity
  const edgeTypeCounts = {};
  const adjacency = new Map(); // nodeId -> count
  edges.forEach((e, i) => {
    if (!e || typeof e !== 'object') {
      issues.push(`Edge at index ${i} is not an object`);
      return;
    }
    if (!isNonEmptyString(e.source)) issues.push(`Edge at index ${i} missing 'source'`);
    if (!isNonEmptyString(e.target)) issues.push(`Edge at index ${i} missing 'target'`);
    if (!isNonEmptyString(e.type)) {
      issues.push(`Edge at index ${i} missing 'type'`);
    } else if (!VALID_EDGE_TYPES.has(e.type)) {
      issues.push(`Edge at index ${i} (${e.source}->${e.target}) has invalid type '${e.type}'`);
    } else {
      edgeTypeCounts[e.type] = (edgeTypeCounts[e.type] || 0) + 1;
    }
    if (!isNonEmptyString(e.direction)) {
      issues.push(`Edge at index ${i} missing 'direction'`);
    } else if (!VALID_DIRECTIONS.has(e.direction)) {
      issues.push(`Edge at index ${i} has invalid direction '${e.direction}'`);
    }
    if (typeof e.weight !== 'number' || isNaN(e.weight)) {
      issues.push(`Edge at index ${i} has missing/invalid 'weight'`);
    } else if (e.weight < 0 || e.weight > 1) {
      issues.push(`Edge at index ${i} weight ${e.weight} outside [0,1]`);
    }
    if (isNonEmptyString(e.source) && !nodeIds.has(e.source)) {
      issues.push(`Edge at index ${i} references non-existent source '${e.source}'`);
    }
    if (isNonEmptyString(e.target) && !nodeIds.has(e.target)) {
      issues.push(`Edge at index ${i} references non-existent target '${e.target}'`);
    }
    if (isNonEmptyString(e.source) && isNonEmptyString(e.target) && e.source === e.target) {
      warnings.push(`Edge at index ${i} is self-referencing on '${e.source}'`);
    }
    if (isNonEmptyString(e.source)) adjacency.set(e.source, (adjacency.get(e.source) || 0) + 1);
    if (isNonEmptyString(e.target)) adjacency.set(e.target, (adjacency.get(e.target) || 0) + 1);
  });

  // Count dangling edges (for stats)
  let danglingEdges = 0;
  edges.forEach(e => {
    if (!e) return;
    const sBad = isNonEmptyString(e.source) && !nodeIds.has(e.source);
    const tBad = isNonEmptyString(e.target) && !nodeIds.has(e.target);
    if (sBad || tBad) danglingEdges++;
  });

  // Check 3 - Completeness
  if (nodes.length === 0) issues.push('Graph has zero nodes');
  if (edges.length === 0) issues.push('Graph has zero edges');
  if (layers.length === 0) {
    if (hasDomainNodes) warnings.push('Graph has zero layers (domain graph)');
    else issues.push('Graph has zero layers');
  }
  if (tour.length === 0) {
    if (hasDomainNodes) warnings.push('Graph has zero tour steps (domain graph)');
    else issues.push('Graph has zero tour steps');
  }

  // Check 4 - Layer coverage
  const nodeLayerCount = new Map();
  layers.forEach((layer, li) => {
    if (!layer || typeof layer !== 'object') {
      issues.push(`Layer at index ${li} is not an object`);
      return;
    }
    const lid = layer.id || layer.name || `index ${li}`;
    const nids = Array.isArray(layer.nodeIds) ? layer.nodeIds : [];
    if (nids.length === 0) {
      issues.push(`Layer '${lid}' has empty nodeIds`);
    }
    if (!isNonEmptyString(layer.description) && !isNonEmptyString(layer.summary)) {
      warnings.push(`Layer '${lid}' missing description`);
    }
    nids.forEach(nid => {
      if (!nodeIds.has(nid)) {
        issues.push(`Layer '${lid}' references non-existent node '${nid}'`);
      } else {
        nodeLayerCount.set(nid, (nodeLayerCount.get(nid) || 0) + 1);
      }
    });
  });

  // file-level node coverage
  if (!(hasDomainNodes && layers.length === 0)) {
    nodes.forEach(n => {
      if (!n || !FILE_LEVEL_TYPES.has(n.type)) return;
      const c = nodeLayerCount.get(n.id) || 0;
      if (c === 0) {
        issues.push(`File-level node '${n.id}' (${n.type}) not assigned to any layer`);
      } else if (c > 1) {
        issues.push(`File-level node '${n.id}' assigned to ${c} layers`);
      }
    });
  }

  // Check 6 - Tour validation
  if (tour.length > 0) {
    if (tour.length < 5 || tour.length > 15) {
      warnings.push(`Tour has ${tour.length} steps (expected 5-15)`);
    }
    const seenOrder = new Set();
    tour.forEach((step, si) => {
      if (!step || typeof step !== 'object') {
        warnings.push(`Tour step at index ${si} is not an object`);
        return;
      }
      const ord = step.order;
      if (typeof ord !== 'number') {
        warnings.push(`Tour step at index ${si} missing numeric 'order'`);
      } else {
        if (seenOrder.has(ord)) warnings.push(`Tour has duplicate order value ${ord}`);
        seenOrder.add(ord);
      }
      const nids = Array.isArray(step.nodeIds) ? step.nodeIds
                : (Array.isArray(step.focusNodeIds) ? step.focusNodeIds : []);
      if (nids.length === 0) {
        warnings.push(`Tour step ${si} has no nodeIds/focusNodeIds`);
      }
      nids.forEach(nid => {
        if (!nodeIds.has(nid)) {
          issues.push(`Tour step ${si} references non-existent node '${nid}'`);
        }
      });
    });
    // sequential starting at 1
    const orders = Array.from(seenOrder).sort((a, b) => a - b);
    if (orders.length > 0) {
      if (orders[0] !== 1) warnings.push(`Tour orders do not start at 1 (start=${orders[0]})`);
      for (let k = 1; k < orders.length; k++) {
        if (orders[k] !== orders[k - 1] + 1) {
          warnings.push(`Tour orders not sequential at position ${k} (${orders[k - 1]} -> ${orders[k]})`);
          break;
        }
      }
    }
  }

  // Check 7 - Orphan nodes (warning)
  let orphanNodes = 0;
  nodes.forEach(n => {
    if (!n || !isString(n.id)) return;
    if (!adjacency.has(n.id)) orphanNodes++;
  });
  if (orphanNodes > 0) warnings.push(`${orphanNodes} orphan nodes with no edges`);

  const stats = {
    totalNodes: nodes.length,
    totalEdges: edges.length,
    totalLayers: layers.length,
    tourSteps: tour.length,
    orphanNodes,
    danglingEdges,
    duplicateIds: Array.from(idIndices.values()).filter(arr => arr.length > 1).length,
    nodeTypes: nodeTypeCounts,
    edgeTypes: edgeTypeCounts
  };

  const result = { scriptCompleted: true, issues, warnings, stats };
  fs.writeFileSync(outputPath, JSON.stringify(result, null, 2));
  process.exit(0);
}

main();
