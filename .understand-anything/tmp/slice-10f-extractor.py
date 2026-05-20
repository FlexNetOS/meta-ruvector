#!/usr/bin/env python3
"""Slice 10f batch extractor: build per-batch graph JSON from file lists."""
import json
import os
import re
import sys
from pathlib import Path

PROJECT_ROOT = Path("/home/drdave/repos/RuVector")
TMP = PROJECT_ROOT / ".understand-anything/tmp"
OUT = PROJECT_ROOT / ".understand-anything/intermediate"

# Rust patterns
RUST_FN = re.compile(r'^\s*(?:pub(?:\([^)]+\))?\s+)?(?:async\s+)?(?:unsafe\s+)?(?:const\s+)?fn\s+([a-zA-Z_][a-zA-Z0-9_]*)', re.M)
RUST_STRUCT = re.compile(r'^\s*(?:pub(?:\([^)]+\))?\s+)?struct\s+([A-Z][a-zA-Z0-9_]*)', re.M)
RUST_ENUM = re.compile(r'^\s*(?:pub(?:\([^)]+\))?\s+)?enum\s+([A-Z][a-zA-Z0-9_]*)', re.M)
RUST_TRAIT = re.compile(r'^\s*(?:pub(?:\([^)]+\))?\s+)?(?:unsafe\s+)?trait\s+([A-Z][a-zA-Z0-9_]*)', re.M)
RUST_MOD = re.compile(r'^\s*(?:pub(?:\([^)]+\))?\s+)?mod\s+([a-z_][a-zA-Z0-9_]*)\s*;', re.M)
RUST_USE = re.compile(r'^\s*use\s+(?:crate::|super::|self::)([a-zA-Z0-9_:]+)', re.M)
RUST_IMPL = re.compile(r'^\s*impl(?:\s*<[^>]+>)?\s+(?:([A-Z][a-zA-Z0-9_:<>,\s]*?)\s+for\s+)?([A-Z][a-zA-Z0-9_<>,\s]*)', re.M)

# Markdown
MD_H = re.compile(r'^#+\s+(.+)$', re.M)

# TOML
TOML_NAME = re.compile(r'^\s*name\s*=\s*"([^"]+)"', re.M)
TOML_DEP_SECTION = re.compile(r'^\[(dependencies|dev-dependencies|build-dependencies)\]', re.M)


def get_line(content: str, name: str, pattern: str) -> int:
    """Find line number of first match."""
    for i, line in enumerate(content.split('\n'), 1):
        if re.search(pattern.replace('NAME', re.escape(name)), line):
            return i
    return 1


def crate_from_path(p: str) -> str:
    parts = p.split('/')
    if parts[0] == 'crates' and len(parts) > 1:
        return parts[1]
    return ''


def process_rust(path: str, content: str):
    nodes = []
    edges = []
    file_id = f"file:{path}"
    nodes.append({
        "id": file_id, "type": "file", "name": os.path.basename(path),
        "filePath": path,
        "summary": f"Rust source file in {crate_from_path(path)} crate"
    })

    # Functions
    for m in RUST_FN.finditer(content):
        name = m.group(1)
        if name in ('main',) or len(name) > 2:
            ln = content[:m.start()].count('\n') + 1
            nid = f"function:{path}:{name}"
            nodes.append({
                "id": nid, "type": "function", "name": name,
                "filePath": path, "lineNumber": ln,
                "summary": f"Function `{name}` in {os.path.basename(path)}"
            })
            edges.append({"source": file_id, "target": nid, "type": "contains"})

    # Structs
    for m in RUST_STRUCT.finditer(content):
        name = m.group(1)
        ln = content[:m.start()].count('\n') + 1
        nid = f"struct:{path}:{name}"
        nodes.append({
            "id": nid, "type": "struct", "name": name,
            "filePath": path, "lineNumber": ln,
            "summary": f"Struct `{name}` defined in {os.path.basename(path)}"
        })
        edges.append({"source": file_id, "target": nid, "type": "contains"})

    # Enums
    for m in RUST_ENUM.finditer(content):
        name = m.group(1)
        ln = content[:m.start()].count('\n') + 1
        nid = f"enum:{path}:{name}"
        nodes.append({
            "id": nid, "type": "enum", "name": name,
            "filePath": path, "lineNumber": ln,
            "summary": f"Enum `{name}` defined in {os.path.basename(path)}"
        })
        edges.append({"source": file_id, "target": nid, "type": "contains"})

    # Traits
    for m in RUST_TRAIT.finditer(content):
        name = m.group(1)
        ln = content[:m.start()].count('\n') + 1
        nid = f"trait:{path}:{name}"
        nodes.append({
            "id": nid, "type": "trait", "name": name,
            "filePath": path, "lineNumber": ln,
            "summary": f"Trait `{name}` defined in {os.path.basename(path)}"
        })
        edges.append({"source": file_id, "target": nid, "type": "contains"})

    # Modules (declarations)
    for m in RUST_MOD.finditer(content):
        name = m.group(1)
        ln = content[:m.start()].count('\n') + 1
        nid = f"module:{path}:{name}"
        nodes.append({
            "id": nid, "type": "module", "name": name,
            "filePath": path, "lineNumber": ln,
            "summary": f"Submodule `{name}` declared in {os.path.basename(path)}"
        })
        edges.append({"source": file_id, "target": nid, "type": "contains"})

    # impl blocks - implements edges
    for m in RUST_IMPL.finditer(content):
        trait_name = m.group(1)
        target_name = m.group(2).strip().split('<')[0].split(' ')[0]
        if trait_name:
            tn = trait_name.strip().split('<')[0].split(' ')[0]
            edges.append({
                "source": f"struct:{path}:{target_name}",
                "target": f"trait:{path}:{tn}",
                "type": "implements"
            })

    # Internal use imports
    seen_imports = set()
    for m in RUST_USE.finditer(content):
        imp = m.group(1).split('::')[0]
        if imp and imp not in seen_imports:
            seen_imports.add(imp)
            edges.append({
                "source": file_id,
                "target": f"module:{imp}",
                "type": "imports"
            })

    return nodes, edges


def process_toml(path: str, content: str):
    nodes = []
    edges = []
    file_id = f"config:{path}"
    name_match = TOML_NAME.search(content)
    crate_name = name_match.group(1) if name_match else os.path.basename(os.path.dirname(path))
    nodes.append({
        "id": file_id, "type": "config", "name": os.path.basename(path),
        "filePath": path,
        "summary": f"Cargo manifest for `{crate_name}` defining dependencies and metadata"
    })
    return nodes, edges


def process_md(path: str, content: str):
    nodes = []
    edges = []
    file_id = f"document:{path}"
    headings = MD_H.findall(content)[:3]
    head_summary = '; '.join(h.strip() for h in headings) if headings else 'Markdown documentation'
    nodes.append({
        "id": file_id, "type": "document", "name": os.path.basename(path),
        "filePath": path,
        "summary": f"Documentation: {head_summary[:200]}"
    })
    return nodes, edges


def process_file(path: str):
    full = PROJECT_ROOT / path
    try:
        content = full.read_text(encoding='utf-8', errors='replace')
    except Exception as e:
        return [{"id": f"file:{path}", "type": "file", "name": os.path.basename(path),
                 "filePath": path, "summary": f"Unreadable file: {e}"}], []

    if path.endswith('.rs'):
        return process_rust(path, content)
    if path.endswith('.toml'):
        return process_toml(path, content)
    if path.endswith('.md'):
        return process_md(path, content)
    # default
    return [{
        "id": f"file:{path}", "type": "file", "name": os.path.basename(path),
        "filePath": path,
        "summary": f"Configuration or data file: {os.path.basename(path)}"
    }], []


def main():
    summary = []
    for batch_n in range(8, 15):
        batch_file = TMP / f"slice-10f-batch-{batch_n:02d}.json"
        out_file = OUT / f"slice-10f-batch-{batch_n:02d}-graph.json"
        files = json.loads(batch_file.read_text())

        all_nodes = []
        all_edges = []
        seen_ids = set()

        for fp in files:
            nodes, edges = process_file(fp)
            for n in nodes:
                if n['id'] not in seen_ids:
                    seen_ids.add(n['id'])
                    all_nodes.append(n)
            all_edges.extend(edges)

        # Dedupe edges
        seen_edges = set()
        deduped = []
        for e in all_edges:
            key = (e['source'], e['target'], e['type'])
            if key not in seen_edges and e['source'] != e['target']:
                seen_edges.add(key)
                deduped.append(e)

        graph = {
            "version": "1.0.0",
            "project": {"name": "RuVector", "slice": "10f", "batch": batch_n},
            "nodes": all_nodes,
            "edges": deduped
        }
        out_file.write_text(json.dumps(graph, indent=2))
        summary.append((batch_n, len(files), len(all_nodes), len(deduped)))

    for b, fc, nc, ec in summary:
        print(f"batch-{b:02d}: files={fc} nodes={nc} edges={ec}")


if __name__ == '__main__':
    main()
