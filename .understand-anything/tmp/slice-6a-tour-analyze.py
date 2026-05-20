#!/usr/bin/env python3
"""Analyze slice 6a graph and build a pedagogical tour.

Layers have no explicit membership edges — we infer membership from filePath
prefixes (the project is organised one directory per crate). For each layer we
pick the most structurally important nodes (top-level lib.rs, main.rs, Cargo.toml,
plus topic-specific module files) and emit a 15-step tour.
"""
import json
import sys
from collections import defaultdict


# Layer id -> list of path prefix predicates (relative to repo root).
# Order matters within a layer; first match wins for assignment.
LAYER_PATHS = {
    "layer:rvf-workspace": ["crates/rvf/Cargo.toml", "crates/rvf/README"],
    "layer:rvf-docs": ["crates/rvf/docs/", "crates/rvf/adr/", "crates/rvf/ADR", "crates/rvf/SECURITY"],
    "layer:rvf-types": ["crates/rvf/rvf-types/"],
    "layer:rvf-wire": ["crates/rvf/rvf-wire/"],
    "layer:rvf-manifest": ["crates/rvf/rvf-manifest/"],
    "layer:rvf-index": ["crates/rvf/rvf-index/"],
    "layer:rvf-quant": ["crates/rvf/rvf-quant/"],
    "layer:rvf-crypto": ["crates/rvf/rvf-crypto/"],
    "layer:rvf-ebpf": ["crates/rvf/rvf-ebpf/"],
    "layer:rvf-kernel": ["crates/rvf/rvf-kernel/"],
    "layer:rvf-launch": ["crates/rvf/rvf-launch/"],
    "layer:rvf-runtime": ["crates/rvf/rvf-runtime/"],
    "layer:rvf-server": ["crates/rvf/rvf-server/"],
    "layer:rvf-cli": ["crates/rvf/rvf-cli/"],
    "layer:rvf-federation": ["crates/rvf/rvf-federation/"],
    "layer:rvf-import": ["crates/rvf/rvf-import/"],
    "layer:rvf-wasm": ["crates/rvf/rvf-wasm/"],
    "layer:rvf-solver": ["crates/rvf/rvf-solver/"],
    "layer:rvf-node": ["crates/rvf/rvf-node/"],
    "layer:rvf-adapters": ["crates/rvf/rvf-adapters/"],
    "layer:rvf-integration": ["crates/rvf/tests/"],
    "layer:rvf-benches": ["crates/rvf/benches/"],
}


def assign_layer(file_path: str) -> str | None:
    """Return the best-matching layer id for a node's filePath, or None.

    We pick the layer whose first matching prefix is the *longest* (most specific),
    so e.g. crates/rvf/rvf-types/... wins over crates/rvf/...
    """
    if not file_path:
        return None
    fp = file_path
    best = None
    best_len = -1
    for lid, prefixes in LAYER_PATHS.items():
        for p in prefixes:
            if fp.startswith(p) or p in fp:
                if len(p) > best_len:
                    best_len = len(p)
                    best = lid
                break
    return best


def main(in_path: str, out_path: str) -> int:
    with open(in_path) as f:
        data = json.load(f)

    nodes = data["nodes"]
    edges = data["edges"]
    layers = data["layers"]

    nodes_by_id = {n["id"]: n for n in nodes}

    # Map each node to a layer by filePath.
    node_layer = {}
    by_layer = defaultdict(list)
    for n in nodes:
        lid = assign_layer(n.get("filePath", "") or "")
        if lid:
            node_layer[n["id"]] = lid
            by_layer[lid].append(n)

    # Fan-in / fan-out across import/use/call/implement edges (skip 'contains').
    fan_in = defaultdict(int)
    fan_out = defaultdict(int)
    structural_types = {"imports", "uses", "calls", "implements"}
    for e in edges:
        if e.get("type") not in structural_types:
            continue
        s, t = e.get("source"), e.get("target")
        if s in nodes_by_id and t in nodes_by_id:
            fan_in[t] += 1
            fan_out[s] += 1

    # Helpers
    def node_score(nid: str) -> int:
        return 2 * fan_in.get(nid, 0) + fan_out.get(nid, 0)

    def pick(layer_id, max_n=8, prefer_substrings=None, prefer_types=None,
             exclude_substrings=None, must_substrings=None):
        prefer_substrings = [s.lower() for s in (prefer_substrings or [])]
        prefer_types = set(prefer_types or [])
        exclude_substrings = [s.lower() for s in (exclude_substrings or [])]
        must_substrings = [s.lower() for s in (must_substrings or [])]
        items = by_layer.get(layer_id, [])
        scored = []
        for n in items:
            name = (n.get("name") or "").lower()
            fp = (n.get("filePath") or "").lower()
            ntype = n.get("type", "")
            if any(ex in fp or ex in name for ex in exclude_substrings):
                continue
            if must_substrings and not any(m in fp or m in name for m in must_substrings):
                continue
            # Prefer rank
            pref_hit = 0
            for i, p in enumerate(prefer_substrings):
                if p in fp or p in name:
                    pref_hit = len(prefer_substrings) - i
                    break
            type_bonus = 2 if ntype in prefer_types else 0
            score = node_score(n["id"])
            depth = fp.count("/") if fp else 99
            scored.append((pref_hit, type_bonus, score, -depth, n["id"]))
        scored.sort(reverse=True)
        return [nid for *_, nid in scored[:max_n]]

    # Discover ADR nodes (by file path or name)
    def find_by_substr(*needles):
        needles = [n.lower() for n in needles]
        out = []
        for n in nodes:
            fp = (n.get("filePath") or "").lower()
            nm = (n.get("name") or "").lower()
            if any(x in fp or x in nm for x in needles):
                out.append(n["id"])
        return out

    # --- Build tour ---
    tour = []

    def add(step, title, desc, focus_node_ids, focus_layer_ids):
        seen = set()
        cleaned = []
        for nid in focus_node_ids:
            if nid in nodes_by_id and nid not in seen:
                cleaned.append(nid)
                seen.add(nid)
        tour.append({
            "step": step,
            "title": title,
            "description": desc,
            "focusNodeIds": cleaned,
            "focusLayerIds": focus_layer_ids,
        })

    # Step 1 — Workspace overview
    ws = []
    if "crate:rvf" in nodes_by_id:
        ws.append("crate:rvf")
    ws += pick("layer:rvf-workspace", max_n=4, prefer_substrings=["readme", "cargo.toml"],
               prefer_types={"file", "crate", "config", "doc"})
    ws += pick("layer:rvf-docs", max_n=3, prefer_substrings=["readme", "overview", "architecture"],
               prefer_types={"doc", "document", "file"})
    add(
        1,
        "What is rvf — workspace overview",
        "Start at the top: the rvf workspace Cargo manifest and README declare the ~26 sub-crates that make up the RuVector Format family. This step orients you to the federation of types, wire, manifest, index, runtime, server, CLI, adapters, and tooling crates the rest of the tour visits.",
        ws[:6],
        ["layer:rvf-workspace", "layer:rvf-docs"],
    )

    # Step 2 — Types & schemas
    types_focus = pick(
        "layer:rvf-types", max_n=6,
        prefer_substrings=["lib.rs", "schema", "quality", "refcount", "container", "vector"],
        prefer_types={"file", "struct", "module"},
    )
    types_focus += find_by_substr("adr-031", "adr-033", "adr-036")
    add(
        2,
        "Types & schemas — the shared vocabulary",
        "rvf-types is the foundation every other crate depends on: shared structs, schemas, quality metrics, and ADR-anchored primitives (ADR-031 reference counting, ADR-033 quality bands, ADR-036 AGI container). Read this before anything else — almost every later step refers back to these types.",
        types_focus[:8],
        ["layer:rvf-types"],
    )

    # Step 3 — Wire protocol
    wire_focus = pick(
        "layer:rvf-wire", max_n=8,
        prefer_substrings=["lib.rs", "segment", "codec", "varint", "hash"],
        prefer_types={"file", "struct", "module"},
    )
    add(
        3,
        "Wire protocol — segments, varints, hash dispatch",
        "rvf-wire defines how rvf-types are serialized on the wire: segment codecs, varint encoding, and hash-based dispatch. Both the runtime and the manifest layer speak this protocol, so understanding the segment layout here unlocks the storage and network surfaces.",
        wire_focus,
        ["layer:rvf-wire"],
    )

    # Step 4 — Storage format / manifest
    manifest_focus = pick(
        "layer:rvf-manifest", max_n=8,
        prefer_substrings=["lib.rs", "level0", "level1", "reader", "writer", "manifest"],
        prefer_types={"file", "struct", "module"},
    )
    add(
        4,
        "Storage format — Level-0 / Level-1 manifests",
        "rvf-manifest is the on-disk format: Level-0 readers/writers materialize raw segments while Level-1 layers structure and indexing on top. Combined with the wire codecs from Step 3, this is how rvf files persist and are re-opened by the runtime.",
        manifest_focus,
        ["layer:rvf-manifest"],
    )

    # Step 5 — Quantization
    quant_focus = pick(
        "layer:rvf-quant", max_n=8,
        prefer_substrings=["lib.rs", "scalar", "product", "binary", "sketch"],
        prefer_types={"file", "struct", "module"},
    )
    add(
        5,
        "Quantization — scalar, product, binary, sketches",
        "rvf-quant compresses high-dimensional vectors before they hit the index: scalar, product, and binary quantization plus probabilistic sketches. These primitives feed both the index and the runtime's memory budget, so they precede the index step.",
        quant_focus,
        ["layer:rvf-quant"],
    )

    # Step 6 — Index
    index_focus = pick(
        "layer:rvf-index", max_n=8,
        prefer_substrings=["lib.rs", "hnsw", "progressive", "search", "graph"],
        prefer_types={"file", "struct", "module"},
    )
    add(
        6,
        "Index — progressive HNSW",
        "rvf-index is RuVector's progressive HNSW implementation: graph construction, traversal, and persistence. It consumes quantized vectors from Step 5 and is the engine that powers similarity search at the runtime layer.",
        index_focus,
        ["layer:rvf-index"],
    )

    # Step 7 — Cryptography
    crypto_focus = pick(
        "layer:rvf-crypto", max_n=8,
        prefer_substrings=["lib.rs", "shake", "ed25519", "witness", "lineage", "attest"],
        prefer_types={"file", "struct", "module", "trait"},
    )
    add(
        7,
        "Cryptography — SHAKE-256, Ed25519, witness chains",
        "rvf-crypto provides the integrity and provenance layer: SHAKE-256 hashing, Ed25519 signatures, and the witness/lineage chains that bind manifests to their authors. The runtime weaves these primitives through every mutation to produce verifiable history.",
        crypto_focus,
        ["layer:rvf-crypto"],
    )

    # Step 8 — Runtime core
    runtime_focus = pick(
        "layer:rvf-runtime", max_n=8,
        prefer_substrings=["lib.rs", "store", "rvfstore", "cow", "ffi", "qr", "seed"],
        prefer_types={"file", "struct", "module"},
    )
    add(
        8,
        "Runtime core — RvfStore, CoW, FFI, QR seeds",
        "rvf-runtime is the glue: RvfStore composes types, wire, manifest, quant, index, and crypto into a coherent store with copy-on-write semantics, a stable FFI surface, and QR-seeded sessions per ADR-033/036. Everything above this point is a transport for what this layer exposes.",
        runtime_focus,
        ["layer:rvf-runtime"],
    )

    # Step 9 — HTTP server
    server_focus = pick(
        "layer:rvf-server", max_n=8,
        prefer_substrings=["main.rs", "lib.rs", "routes", "axum", "handler", "server"],
        prefer_types={"file", "struct", "module", "function", "fn"},
    )
    add(
        9,
        "HTTP server — Axum surface",
        "rvf-server wraps the runtime in an Axum HTTP API. Routes mirror RvfStore operations so a remote caller can drive the same primitives the CLI uses locally — this is the first networked face of rvf.",
        server_focus,
        ["layer:rvf-server"],
    )

    # Step 10 — CLI
    cli_focus = pick(
        "layer:rvf-cli", max_n=8,
        prefer_substrings=["main.rs", "lib.rs", "cmd", "command", "subcommand", "clap"],
        prefer_types={"file", "struct", "module", "function", "fn"},
    )
    add(
        10,
        "CLI — clap subcommands",
        "rvf-cli is the operator-facing entry point: clap subcommands map onto runtime calls for ingest, search, verify, and admin tasks. Reading the subcommand modules reveals the daily-driver shape of the system.",
        cli_focus,
        ["layer:rvf-cli"],
    )

    # Step 11 — Federation
    fed_focus = pick(
        "layer:rvf-federation", max_n=8,
        prefer_substrings=["lib.rs", "aggregat", "fedavg", "fedprox", "privacy", "noise", "byzantine", "pii"],
        prefer_types={"file", "struct", "module"},
    )
    fed_focus += find_by_substr("adr-057")
    add(
        11,
        "Federation — ADR-057 aggregation & differential privacy",
        "rvf-federation implements ADR-057: federated aggregation across nodes with differential-privacy noise mechanisms and Byzantine tolerance. It builds on the runtime and crypto layers to combine partial results without leaking per-node data.",
        fed_focus[:8],
        ["layer:rvf-federation"],
    )

    # Step 12 — WASM microkernel
    wasm_focus = pick(
        "layer:rvf-wasm", max_n=8,
        prefer_substrings=["lib.rs", "cdylib", "export", "wasm", "ffi"],
        prefer_types={"file", "struct", "module", "function", "fn"},
    )
    add(
        12,
        "WASM microkernel — cdylib exports",
        "rvf-wasm compiles a minimal runtime subset as a cdylib for browser and host embedding. The exported FFI surface lets external embedders drive rvf without linking the full Rust runtime — useful for sandboxed and edge deployments.",
        wasm_focus,
        ["layer:rvf-wasm"],
    )

    # Step 13 — eBPF + microkernel + launcher
    ebpf_focus = pick("layer:rvf-ebpf", max_n=3,
                      prefer_substrings=["lib.rs", "compile", "program"],
                      prefer_types={"file", "module"})
    kernel_focus = pick("layer:rvf-kernel", max_n=3,
                        prefer_substrings=["lib.rs", "pipeline", "cpio"],
                        prefer_types={"file", "module", "struct"})
    launch_focus = pick("layer:rvf-launch", max_n=3,
                        prefer_substrings=["lib.rs", "qemu", "qmp", "launcher"],
                        prefer_types={"file", "module", "struct"})
    combined = (ebpf_focus + kernel_focus + launch_focus)[:8]
    add(
        13,
        "eBPF + microkernel + launcher — the sandboxed pipeline",
        "Three cooperating layers sandbox rvf workloads: rvf-ebpf compiles eBPF programs, rvf-kernel packs them into a CPIO microkernel pipeline, and rvf-launch boots a QEMU microVM and drives it over QMP. Together they form rvf's hardened execution path.",
        combined,
        ["layer:rvf-ebpf", "layer:rvf-kernel", "layer:rvf-launch"],
    )

    # Step 14 — Adapters
    adapter_focus = pick(
        "layer:rvf-adapters", max_n=8,
        prefer_substrings=["agentdb", "agentic", "claude-flow", "ospipe", "rvlite", "sona", "lib.rs"],
        prefer_types={"file", "module", "struct"},
    )
    add(
        14,
        "Adapters — agentdb, agentic-flow, claude-flow, ospipe, rvlite, sona",
        "The adapter crates bridge rvf to neighbouring systems: AgentDB persistence, agentic-flow / claude-flow agent runtimes, ospipe IPC, rvlite embedded variant, and sona. Each adapter is a thin translation layer — read them to see how rvf plugs into a larger toolchain.",
        adapter_focus,
        ["layer:rvf-adapters"],
    )

    # Step 15 — Integration tests (capstone)
    integ_focus = pick(
        "layer:rvf-integration", max_n=8,
        prefer_substrings=["integration", "end_to_end", "e2e", "round", "tests"],
        prefer_types={"file", "function", "fn", "module"},
    )
    add(
        15,
        "Integration tests — the capstone",
        "crates/rvf/tests exercises the whole stack end-to-end: ingest, manifest persistence, indexed search, signed witnesses, federation, and adapter round-trips. These tests are the executable specification — when you finish the tour, run them to see every prior step interacting in one process.",
        integ_focus,
        ["layer:rvf-integration"],
    )

    # Emit extended output
    out = dict(data)
    out["tour"] = tour
    with open(out_path, "w") as f:
        json.dump(out, f, indent=2)

    # Diagnostics
    diag = {
        "steps": len(tour),
        "stepSummary": [
            {
                "step": s["step"],
                "title": s["title"],
                "focusCount": len(s["focusNodeIds"]),
                "layers": s["focusLayerIds"],
            }
            for s in tour
        ],
        "emptySteps": [s["step"] for s in tour if not s["focusNodeIds"]],
        "layerNodeCounts": {lid: len(by_layer.get(lid, [])) for lid in [l["id"] for l in layers]},
        "unassignedNodes": sum(1 for n in nodes if n["id"] not in node_layer),
    }
    print(json.dumps(diag, indent=2))
    return 0


if __name__ == "__main__":
    if len(sys.argv) != 3:
        print("usage: slice-6a-tour-analyze.py <in.json> <out.json>", file=sys.stderr)
        sys.exit(1)
    sys.exit(main(sys.argv[1], sys.argv[2]))
