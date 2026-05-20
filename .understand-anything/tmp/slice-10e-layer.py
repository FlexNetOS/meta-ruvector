#!/usr/bin/env python3
"""Assign every node in slice-10e to exactly one layer based on crate path prefix."""
import json
import sys
from collections import defaultdict

IN = "/home/drdave/repos/RuVector/.understand-anything/tmp/slice-10e-assembled.json"
OUT = "/home/drdave/repos/RuVector/.understand-anything/tmp/slice-10e-layered.json"

# Map each crate name to a layer id (kebab-case after layer:slice10e-)
CRATE_TO_LAYER = {
    # Delta — CRDT/operational deltas
    "ruvector-delta-consensus": "delta",
    "ruvector-delta-core": "delta",
    "ruvector-delta-graph": "delta",
    "ruvector-delta-index": "delta",
    "ruvector-delta-wasm": "delta",
    # Replication & Raft
    "ruvector-replication": "replication-raft",
    "ruvector-raft": "replication-raft",
    "ruvector-snapshot": "replication-raft",
    "ruvector-coherence": "replication-raft",
    # Lake & RAIRS
    "ruvector-rulake": "lake-rairs",
    "ruvector-rairs": "lake-rairs",
    # Tiny Dancer routing
    "ruvector-tiny-dancer-core": "tiny-dancer",
    "ruvector-tiny-dancer-node": "tiny-dancer",
    "ruvector-tiny-dancer-wasm": "tiny-dancer",
    # Verified & Acorn — verified compute
    "ruvector-verified": "verified-acorn",
    "ruvector-verified-wasm": "verified-acorn",
    "ruvector-acorn": "verified-acorn",
    "ruvector-acorn-wasm": "verified-acorn",
    # Kalshi prediction market
    "ruvector-kalshi": "kalshi",
}

LAYER_META = {
    "delta": {
        "name": "Delta (CRDT)",
        "description": "Operational delta and CRDT primitives (consensus clocks, core deltas, graph/index deltas, WASM bridge) used to encode incremental state changes across the cluster.",
    },
    "replication-raft": {
        "name": "Replication & Raft",
        "description": "Raft consensus, replication transport, snapshot management, and cross-replica coherence keeping cluster state durable and consistent.",
    },
    "lake-rairs": {
        "name": "Lake & RAIRS",
        "description": "Long-term storage substrate: RuLake data lake and RAIRS retrieval/indexing service backing analytical and archival reads.",
    },
    "tiny-dancer": {
        "name": "Tiny Dancer Routing",
        "description": "Tiny Dancer routing fabric (core algorithms, node runtime, WASM client) directing requests and shard traffic across the cluster.",
    },
    "verified-acorn": {
        "name": "Verified & Acorn",
        "description": "Verified-compute layer combining Verified attestation primitives and Acorn proof/execution crates (native and WASM) for trustworthy computation.",
    },
    "kalshi": {
        "name": "Kalshi Integration",
        "description": "Kalshi prediction-market integration crate exposing market data and event signals to the cluster.",
    },
}


def crate_of(file_path: str) -> str | None:
    """Extract the crate directory name from a path like 'crates/<crate>/...'."""
    if not file_path:
        return None
    parts = file_path.split("/")
    if len(parts) >= 2 and parts[0] == "crates":
        return parts[1]
    return None


def main() -> int:
    with open(IN) as f:
        data = json.load(f)

    nodes = data["nodes"]
    layer_to_nodes: dict[str, list[str]] = defaultdict(list)
    unassigned: list[dict] = []
    crate_miss: dict[str, int] = defaultdict(int)

    for node in nodes:
        fp = node.get("filePath", "")
        # Special-case crate-typed nodes whose filePath is the crate dir itself
        if node.get("type") == "crate":
            crate_name = node.get("name") or fp.split("/")[-1]
        else:
            crate_name = crate_of(fp)

        layer_key = CRATE_TO_LAYER.get(crate_name) if crate_name else None
        if layer_key is None:
            unassigned.append(node)
            crate_miss[str(crate_name)] += 1
            continue
        layer_to_nodes[layer_key].append(node["id"])

    # Build the layers array in a sensible order
    order = ["delta", "replication-raft", "lake-rairs", "tiny-dancer", "verified-acorn", "kalshi"]
    layers = []
    for key in order:
        if key not in layer_to_nodes:
            continue
        meta = LAYER_META[key]
        layers.append({
            "id": f"layer:slice10e-{key}",
            "name": meta["name"],
            "description": meta["description"],
            "nodeIds": layer_to_nodes[key],
        })

    data["layers"] = layers

    with open(OUT, "w") as f:
        json.dump(data, f, indent=2)

    total_assigned = sum(len(l["nodeIds"]) for l in layers)
    print(f"Total nodes: {len(nodes)}")
    print(f"Assigned: {total_assigned}")
    print(f"Unassigned: {len(unassigned)}")
    print(f"Layer count: {len(layers)}")
    for l in layers:
        print(f"  {l['id']}: {len(l['nodeIds'])} nodes")
    if unassigned:
        print("Unassigned crate buckets:")
        for k, v in sorted(crate_miss.items(), key=lambda kv: -kv[1]):
            print(f"  {k}: {v}")
        # show a couple of sample unassigned nodes
        for n in unassigned[:5]:
            print(f"  sample: {n.get('id')} | filePath={n.get('filePath')} | type={n.get('type')}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
