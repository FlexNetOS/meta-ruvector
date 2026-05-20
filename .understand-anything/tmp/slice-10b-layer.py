#!/usr/bin/env python3
"""Assign nodes from slice 10b to the 4 cognition-cluster crate layers."""
import json
import sys
from pathlib import Path

INPUT = Path("/home/drdave/repos/RuVector/.understand-anything/tmp/slice-10b-assembled.json")
OUTPUT = Path("/home/drdave/repos/RuVector/.understand-anything/tmp/slice-10b-layered.json")

# Order matters: most specific (longer) prefix first so
# `ruvector-nervous-system-wasm` does not match `ruvector-nervous-system`.
LAYER_DEFS = [
    {
        "id": "layer:slice10b-prime-radiant",
        "name": "Prime Radiant",
        "description": "Sheaf-Laplacian coherence engine with GPU kernels, governance, and distributed Raft consensus.",
        "prefix": "crates/prime-radiant/",
    },
    {
        "id": "layer:slice10b-nervous-system-wasm",
        "name": "Nervous System WASM",
        "description": "WebAssembly bindings exposing the nervous-system neural primitives to browser and JS runtimes.",
        "prefix": "crates/ruvector-nervous-system-wasm/",
    },
    {
        "id": "layer:slice10b-nervous-system",
        "name": "Nervous System",
        "description": "Neural primitives including WTA competition, dendrites, event bus, HDC, and Hopfield networks.",
        "prefix": "crates/ruvector-nervous-system/",
    },
    {
        "id": "layer:slice10b-sona",
        "name": "Sona Continual Learning",
        "description": "Continual learning subsystem: EWC, LoRA, reasoning bank, training pipelines, and N-API exposure.",
        "prefix": "crates/sona/",
    },
]


def extract_path(node):
    """Return the best file path for a node, falling back to parsing the id."""
    fp = node.get("filePath")
    if fp:
        return fp
    nid = node.get("id", "")
    # IDs look like "<type>:<path>[:<symbol>...]". The path is the second
    # colon-delimited segment.
    if ":" in nid:
        # Drop the type prefix.
        rest = nid.split(":", 1)[1]
        # For sub-file symbols ("struct:path/to.rs:Name") we only want the
        # path component, but ":" is also valid inside Windows-style paths
        # (rare here). We keep everything up to the next ":".
        return rest.split(":", 1)[0]
    return ""


def main():
    data = json.loads(INPUT.read_text())
    nodes = data.get("nodes", [])

    members = {ld["id"]: [] for ld in LAYER_DEFS}
    unassigned = []

    for node in nodes:
        path = extract_path(node)
        assigned = None
        for ld in LAYER_DEFS:
            if path.startswith(ld["prefix"]):
                assigned = ld["id"]
                break
        if assigned:
            members[assigned].append(node["id"])
        else:
            unassigned.append({"id": node.get("id"), "path": path})

    layers = []
    for ld in LAYER_DEFS:
        layers.append(
            {
                "id": ld["id"],
                "name": ld["name"],
                "description": ld["description"],
                "memberNodeIds": members[ld["id"]],
            }
        )

    out = dict(data)
    out["layers"] = layers
    OUTPUT.write_text(json.dumps(out, indent=2))

    total_assigned = sum(len(m) for m in members.values())
    print(f"Total nodes: {len(nodes)}")
    print(f"Total assigned: {total_assigned}")
    print(f"Unassigned: {len(unassigned)}")
    for ld in LAYER_DEFS:
        print(f"  {ld['id']}: {len(members[ld['id']])}")
    if unassigned:
        print("First 10 unassigned:")
        for u in unassigned[:10]:
            print(f"  {u}")


if __name__ == "__main__":
    main()
