#!/usr/bin/env python3
"""Slice 6b graph validator."""
import json
import sys
from collections import Counter, defaultdict


def main():
    in_path = sys.argv[1]
    out_path = sys.argv[2]

    with open(in_path) as f:
        g = json.load(f)

    nodes = g.get("nodes", [])
    edges = g.get("edges", [])
    layers = g.get("layers", [])
    tour = g.get("tour", [])

    issues = []
    recommendations = []

    # --- Uniqueness ---
    id_counts = Counter(n.get("id") for n in nodes)
    duplicates = {nid: c for nid, c in id_counts.items() if c > 1}
    if duplicates:
        for nid, c in list(duplicates.items())[:10]:
            issues.append(f"Duplicate node id '{nid}' appears {c} times")
        if len(duplicates) > 10:
            issues.append(f"...and {len(duplicates) - 10} more duplicate IDs")

    node_ids = set(id_counts.keys())

    # --- Node schema basics ---
    missing_id = sum(1 for n in nodes if not n.get("id"))
    missing_type = sum(1 for n in nodes if not n.get("type"))
    missing_name = sum(1 for n in nodes if not n.get("name"))
    if missing_id:
        issues.append(f"{missing_id} nodes missing 'id'")
    if missing_type:
        issues.append(f"{missing_type} nodes missing 'type'")
    if missing_name:
        issues.append(f"{missing_name} nodes missing 'name'")

    # --- Edge referential integrity ---
    dangling_edges = []
    self_edges = 0
    for i, e in enumerate(edges):
        s, t = e.get("source"), e.get("target")
        if not s or not t or not e.get("type"):
            dangling_edges.append(f"edge[{i}] missing source/target/type: {e}")
            continue
        if s == t:
            self_edges += 1
        if s not in node_ids:
            dangling_edges.append(f"edge[{i}] source '{s}' -> '{t}' (source missing)")
        if t not in node_ids:
            dangling_edges.append(f"edge[{i}] target '{s}' -> '{t}' (target missing)")

    if dangling_edges:
        for d in dangling_edges[:10]:
            issues.append(d)
        if len(dangling_edges) > 10:
            issues.append(f"...and {len(dangling_edges) - 10} more dangling edges")

    if self_edges:
        recommendations.append(f"{self_edges} self-referencing edges (source == target)")

    # --- Layer assignments ---
    node_layer = defaultdict(list)
    empty_layers = []
    missing_desc = []
    for layer in layers:
        lid = layer.get("id", "<no-id>")
        if not layer.get("description"):
            missing_desc.append(lid)
        members = layer.get("memberNodeIds", []) or layer.get("nodeIds", [])
        if not members:
            empty_layers.append(lid)
        for nid in members:
            node_layer[nid].append(lid)

    if empty_layers:
        issues.append(f"Empty layers: {empty_layers}")
    if missing_desc:
        recommendations.append(f"Layers missing descriptions: {missing_desc}")

    # Find nodes appearing in multiple layers OR none
    multi_layer = {nid: lids for nid, lids in node_layer.items() if len(lids) > 1}
    if multi_layer:
        for nid, lids in list(multi_layer.items())[:5]:
            issues.append(f"Node '{nid}' assigned to multiple layers: {lids}")
        if len(multi_layer) > 5:
            issues.append(f"...and {len(multi_layer) - 5} more nodes in multiple layers")

    # Layer membership IDs that don't exist as nodes
    missing_layer_refs = []
    for lid, members in [(l.get("id"), l.get("memberNodeIds", []) or l.get("nodeIds", [])) for l in layers]:
        for nid in members:
            if nid not in node_ids:
                missing_layer_refs.append((lid, nid))
    if missing_layer_refs:
        for lid, nid in missing_layer_refs[:10]:
            issues.append(f"Layer '{lid}' references missing node '{nid}'")
        if len(missing_layer_refs) > 10:
            issues.append(f"...and {len(missing_layer_refs) - 10} more missing layer refs")

    # Nodes not in any layer
    unlayered = [n["id"] for n in nodes if n.get("id") not in node_layer]
    if unlayered:
        for nid in unlayered[:10]:
            issues.append(f"Node '{nid}' not assigned to any layer")
        if len(unlayered) > 10:
            issues.append(f"...and {len(unlayered) - 10} more unlayered nodes")

    # --- Tour validation ---
    tour_issues = []
    if not tour:
        issues.append("Tour is empty")
    else:
        seen_steps = set()
        for i, ts in enumerate(tour):
            step = ts.get("step")
            if step in seen_steps:
                tour_issues.append(f"tour[{i}] duplicate step number {step}")
            seen_steps.add(step)
            fnodes = ts.get("focusNodeIds", [])
            if not fnodes:
                tour_issues.append(f"tour[{i}] (step {step}) has empty focusNodeIds")
            for nid in fnodes:
                if nid not in node_ids:
                    tour_issues.append(f"tour[{i}] (step {step}) focusNodeId '{nid}' not in nodes")
    if tour_issues:
        for ti in tour_issues[:10]:
            issues.append(ti)
        if len(tour_issues) > 10:
            issues.append(f"...and {len(tour_issues) - 10} more tour issues")

    # --- Orphan nodes (warning only) ---
    edge_nodes = set()
    for e in edges:
        if e.get("source"): edge_nodes.add(e["source"])
        if e.get("target"): edge_nodes.add(e["target"])
    orphan_count = sum(1 for n in nodes if n.get("id") not in edge_nodes)

    # --- Stats ---
    stats = {
        "nodes": len(nodes),
        "edges": len(edges),
        "layers": len(layers),
        "tourSteps": len(tour),
        "orphanNodes": orphan_count,
        "danglingEdges": len(dangling_edges),
        "duplicateIds": len(duplicates),
        "unlayeredNodes": len(unlayered),
        "multiLayerNodes": len(multi_layer),
        "selfEdges": self_edges,
    }

    if orphan_count:
        recommendations.append(f"{orphan_count} orphan nodes (no edges)")

    result = {
        "scriptCompleted": True,
        "issues": issues,
        "recommendations": recommendations,
        "stats": stats,
    }

    with open(out_path, "w") as f:
        json.dump(result, f, indent=2)

    print(f"Validation complete: {len(issues)} issues, {len(recommendations)} recommendations")
    return 0


if __name__ == "__main__":
    sys.exit(main())
