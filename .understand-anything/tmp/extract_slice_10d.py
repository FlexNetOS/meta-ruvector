#!/usr/bin/env python3
"""Extract Rust structural data for RuVector slice 10d batches 1-5."""
import json
import re
import sys
from pathlib import Path

PROJECT_ROOT = Path("/home/drdave/repos/RuVector")
TMP = PROJECT_ROOT / ".understand-anything/tmp"

# Regexes for Rust constructs
RE_FN = re.compile(r'^\s*(?:pub(?:\([^)]*\))?\s+)?(?:async\s+)?(?:unsafe\s+)?(?:extern\s+"[^"]*"\s+)?fn\s+([a-zA-Z_][a-zA-Z0-9_]*)', re.MULTILINE)
RE_STRUCT = re.compile(r'^\s*(?:pub(?:\([^)]*\))?\s+)?struct\s+([A-Z][a-zA-Z0-9_]*)', re.MULTILINE)
RE_ENUM = re.compile(r'^\s*(?:pub(?:\([^)]*\))?\s+)?enum\s+([A-Z][a-zA-Z0-9_]*)', re.MULTILINE)
RE_TRAIT = re.compile(r'^\s*(?:pub(?:\([^)]*\))?\s+)?trait\s+([A-Z][a-zA-Z0-9_]*)', re.MULTILINE)
RE_MOD = re.compile(r'^\s*(?:pub(?:\([^)]*\))?\s+)?mod\s+([a-zA-Z_][a-zA-Z0-9_]*)\s*[;{]', re.MULTILINE)
RE_IMPL = re.compile(r'^\s*impl(?:\s*<[^>]+>)?\s+(?:([A-Z][a-zA-Z0-9_:<>, \']*?)\s+for\s+)?([A-Z][a-zA-Z0-9_:<>, \']*)', re.MULTILINE)
RE_USE = re.compile(r'^\s*use\s+(?:crate::|super::|self::)([a-zA-Z_][a-zA-Z0-9_:]*)', re.MULTILINE)


def line_of(text: str, pos: int) -> int:
    return text.count('\n', 0, pos) + 1


def crate_of(path: str) -> str:
    # crates/<crate-name>/...
    m = re.match(r'crates/([^/]+)/', path)
    return m.group(1) if m else "unknown"


def file_summary(path: str, code: str) -> str:
    """Generate a short summary for a Rust source file."""
    fname = Path(path).name
    crate = crate_of(path)
    n_fn = len(RE_FN.findall(code))
    n_st = len(RE_STRUCT.findall(code))
    n_en = len(RE_ENUM.findall(code))
    n_tr = len(RE_TRAIT.findall(code))

    if fname == "Cargo.toml":
        return f"Cargo manifest for crate {crate}: dependencies, features, and build config."
    if fname == "lib.rs":
        return f"Library root for {crate}: defines public module surface and re-exports ({n_st} structs, {n_en} enums, {n_tr} traits)."
    if fname == "main.rs":
        return f"Binary entry point for {crate}: CLI bootstrap and command dispatch."
    if fname == "mod.rs":
        parent = Path(path).parent.name
        return f"Module root for {crate}/{parent}: aggregates submodules ({n_fn} fns, {n_st} structs)."
    if fname == "build.rs":
        return f"Build script for {crate}: compile-time codegen or platform detection."
    if fname.endswith(".rs"):
        stem = Path(path).stem
        return f"{crate}::{stem} — {n_fn} fns, {n_st} structs, {n_en} enums, {n_tr} traits."
    if fname.endswith(".md"):
        return f"{crate} documentation: {fname}."
    return f"{crate}/{fname}"


def analyze_file(rel_path: str):
    """Extract nodes + edges for one file."""
    abs_path = PROJECT_ROOT / rel_path
    nodes = []
    edges = []

    if not abs_path.exists():
        return nodes, edges

    crate = crate_of(rel_path)
    fname = Path(rel_path).name

    # Determine file node type
    if fname == "Cargo.toml":
        node_type = "crate"
        node_name = crate
    elif fname.endswith(".md"):
        node_type = "file"
        node_name = fname
    elif fname.endswith(".rs"):
        node_type = "file"
        node_name = fname
    else:
        node_type = "file"
        node_name = fname

    file_id = f"file:{rel_path}"

    try:
        code = abs_path.read_text(encoding="utf-8", errors="replace")
    except Exception:
        return nodes, edges

    summary = file_summary(rel_path, code)

    nodes.append({
        "id": file_id,
        "type": node_type,
        "name": node_name,
        "filePath": rel_path,
        "summary": summary,
    })

    if not fname.endswith(".rs"):
        return nodes, edges

    # Functions (only emit non-trivial / pub ones to keep graph sane)
    seen_fns = set()
    for m in RE_FN.finditer(code):
        name = m.group(1)
        if name in seen_fns:
            continue
        seen_fns.add(name)
        ln = line_of(code, m.start())
        # Heuristic: only emit if pub OR top-level / >= 4 lines context
        prefix = code[max(0, m.start()-20):m.start()]
        is_pub = "pub" in prefix
        # Emit pub fns and a few core private ones; skip closures-named-fn from impls won't matter
        if is_pub or name in ("main", "new", "build", "run", "execute"):
            fn_id = f"function:{rel_path}:{name}"
            nodes.append({
                "id": fn_id,
                "type": "function",
                "name": name,
                "filePath": rel_path,
                "lineNumber": ln,
                "summary": f"{crate}::{Path(rel_path).stem}::{name} — function.",
            })
            edges.append({
                "source": file_id,
                "target": fn_id,
                "type": "contains",
            })

    # Structs
    for m in RE_STRUCT.finditer(code):
        name = m.group(1)
        ln = line_of(code, m.start())
        sid = f"struct:{rel_path}:{name}"
        nodes.append({
            "id": sid,
            "type": "struct",
            "name": name,
            "filePath": rel_path,
            "lineNumber": ln,
            "summary": f"{crate}::{Path(rel_path).stem}::{name} — struct definition.",
        })
        edges.append({"source": file_id, "target": sid, "type": "contains"})

    # Enums
    for m in RE_ENUM.finditer(code):
        name = m.group(1)
        ln = line_of(code, m.start())
        eid = f"enum:{rel_path}:{name}"
        nodes.append({
            "id": eid,
            "type": "enum",
            "name": name,
            "filePath": rel_path,
            "lineNumber": ln,
            "summary": f"{crate}::{Path(rel_path).stem}::{name} — enum.",
        })
        edges.append({"source": file_id, "target": eid, "type": "contains"})

    # Traits
    for m in RE_TRAIT.finditer(code):
        name = m.group(1)
        ln = line_of(code, m.start())
        tid = f"trait:{rel_path}:{name}"
        nodes.append({
            "id": tid,
            "type": "trait",
            "name": name,
            "filePath": rel_path,
            "lineNumber": ln,
            "summary": f"{crate}::{Path(rel_path).stem}::{name} — trait.",
        })
        edges.append({"source": file_id, "target": tid, "type": "contains"})

    # Inline modules (mod foo;) -- create edges to sibling files when applicable
    for m in RE_MOD.finditer(code):
        name = m.group(1)
        # Skip `mod tests` etc. and inline blocks; we just emit an edge to a likely sibling file
        if name in ("tests", "test"):
            continue
        # Look for sibling file
        parent = Path(rel_path).parent
        candidates = [
            f"{parent}/{name}.rs",
            f"{parent}/{name}/mod.rs",
        ]
        for c in candidates:
            if (PROJECT_ROOT / c).exists():
                edges.append({
                    "source": file_id,
                    "target": f"file:{c}",
                    "type": "imports",
                })
                break

    # impl blocks for implements / uses edges
    for m in RE_IMPL.finditer(code):
        trait_name = m.group(1)
        type_name = m.group(2)
        if not type_name:
            continue
        type_name = type_name.strip().split('<')[0].split('::')[-1]
        type_id_candidate = f"struct:{rel_path}:{type_name}"
        # Check if we created that struct/enum node in this file
        if any(n["id"] == type_id_candidate for n in nodes):
            if trait_name:
                trait_simple = trait_name.strip().split('<')[0].split('::')[-1]
                # Best effort: look for trait in same file
                trait_id = f"trait:{rel_path}:{trait_simple}"
                if any(n["id"] == trait_id for n in nodes):
                    edges.append({
                        "source": type_id_candidate,
                        "target": trait_id,
                        "type": "implements",
                    })

    # crate-internal `use crate::foo::bar` => uses edge
    for m in RE_USE.finditer(code):
        target_mod = m.group(1).split('::')[0]
        # Try to resolve sibling path
        crate_src = f"crates/{crate}/src"
        for c in [f"{crate_src}/{target_mod}.rs", f"{crate_src}/{target_mod}/mod.rs"]:
            if (PROJECT_ROOT / c).exists() and c != rel_path:
                edges.append({
                    "source": file_id,
                    "target": f"file:{c}",
                    "type": "uses",
                })
                break

    return nodes, edges


def process_batch(batch_num: int):
    bfile = TMP / f"slice-10d-batch-{batch_num:02d}.json"
    files = json.loads(bfile.read_text())
    all_nodes = []
    all_edges = []
    seen_node_ids = set()
    seen_edges = set()

    for rel_path in files:
        nodes, edges = analyze_file(rel_path)
        for n in nodes:
            if n["id"] not in seen_node_ids:
                seen_node_ids.add(n["id"])
                all_nodes.append(n)
        for e in edges:
            key = (e["source"], e["target"], e["type"])
            if key not in seen_edges and e["source"] != e["target"]:
                seen_edges.add(key)
                all_edges.append(e)

    out = {
        "version": "1.0.0",
        "project": {"name": "RuVector", "slice": "10d", "batch": batch_num},
        "nodes": all_nodes,
        "edges": all_edges,
    }
    outpath = TMP / f"slice-10d-batch-{batch_num:02d}-graph.json"
    outpath.write_text(json.dumps(out, indent=2))
    return len(all_nodes), len(all_edges), len(files)


if __name__ == "__main__":
    for b in range(1, 6):
        n, e, fc = process_batch(b)
        print(f"batch {b}: files={fc} nodes={n} edges={e}")
