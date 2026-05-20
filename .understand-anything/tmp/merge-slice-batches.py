#!/usr/bin/env python3
"""Merge per-batch graph files into a single slice graph.

Usage:
    python merge-slice-batches.py <slice> <output_path>

Reads:
    .understand-anything/tmp/slice-<slice>-batch-*-graph.json

Writes:
    <output_path> with deduped nodes/edges and slice metadata.
"""
import glob
import json
import sys
from pathlib import Path

if len(sys.argv) != 3:
    print("usage: merge-slice-batches.py <slice> <output_path>")
    sys.exit(1)

slice_id = sys.argv[1]
out_path = Path(sys.argv[2])

batch_files = sorted(glob.glob(f".understand-anything/tmp/slice-{slice_id}-batch-*-graph.json"))
if not batch_files:
    print(f"No batches found for slice {slice_id}")
    sys.exit(1)

print(f"Merging {len(batch_files)} batches for slice {slice_id}")

nodes_by_id = {}
edges_seen = set()
edges = []
node_collisions = 0
batch_stats = []

for bf in batch_files:
    with open(bf) as f:
        d = json.load(f)
    nb = d.get("nodes", [])
    eb = d.get("edges", [])
    batch_stats.append((Path(bf).name, len(nb), len(eb)))
    for n in nb:
        nid = n.get("id")
        if not nid:
            continue
        if nid in nodes_by_id:
            node_collisions += 1
            existing = nodes_by_id[nid]
            for k, v in n.items():
                if k not in existing or not existing.get(k):
                    existing[k] = v
        else:
            nodes_by_id[nid] = n
    for e in eb:
        src = e.get("source")
        tgt = e.get("target")
        typ = e.get("type", "uses")
        if not src or not tgt:
            continue
        key = (src, tgt, typ)
        if key in edges_seen:
            continue
        edges_seen.add(key)
        edges.append(e)

graph = {
    "version": "1.0.0",
    "project": {"name": "RuVector", "slice": slice_id, "scope": f"crates/{'rvf' if slice_id == '6a' else 'rvAgent'}"},
    "nodes": list(nodes_by_id.values()),
    "edges": edges,
}

out_path.parent.mkdir(parents=True, exist_ok=True)
with open(out_path, "w") as f:
    json.dump(graph, f, indent=2)

print(f"\nResult: {len(graph['nodes'])} nodes, {len(graph['edges'])} edges, {node_collisions} node-id collisions merged")
print(f"Written: {out_path}")
print("\nBatch breakdown:")
for name, n, e in batch_stats:
    print(f"  {name}: {n} nodes, {e} edges")
