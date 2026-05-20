#!/usr/bin/env python3
"""Extract Rust structure for slice-10e batches and emit graph JSON."""
import json
import os
import re
import sys
from pathlib import Path

PROJECT_ROOT = Path("/home/drdave/repos/RuVector")

# Regex patterns for Rust source extraction
RE_FN = re.compile(r'^\s*(?:pub(?:\([^)]*\))?\s+)?(?:async\s+)?(?:const\s+)?(?:unsafe\s+)?fn\s+([a-zA-Z_][a-zA-Z0-9_]*)', re.MULTILINE)
RE_STRUCT = re.compile(r'^\s*(?:pub(?:\([^)]*\))?\s+)?struct\s+([A-Z][a-zA-Z0-9_]*)', re.MULTILINE)
RE_ENUM = re.compile(r'^\s*(?:pub(?:\([^)]*\))?\s+)?enum\s+([A-Z][a-zA-Z0-9_]*)', re.MULTILINE)
RE_TRAIT = re.compile(r'^\s*(?:pub(?:\([^)]*\))?\s+)?(?:unsafe\s+)?trait\s+([A-Z][a-zA-Z0-9_]*)', re.MULTILINE)
RE_IMPL = re.compile(r'^\s*impl(?:<[^>]+>)?\s+(?:([A-Z][a-zA-Z0-9_:]*(?:<[^>]+>)?)\s+for\s+)?([A-Z][a-zA-Z0-9_:]*)', re.MULTILINE)
RE_MOD = re.compile(r'^\s*(?:pub(?:\([^)]*\))?\s+)?mod\s+([a-zA-Z_][a-zA-Z0-9_]*)', re.MULTILINE)
RE_USE = re.compile(r'^\s*use\s+(crate::|super::|self::|ruvector[_a-zA-Z]*::)([a-zA-Z0-9_:{}*,\s]+);', re.MULTILINE)
RE_USE_INTERNAL = re.compile(r'^\s*use\s+(crate|super|self|ruvector[_a-zA-Z]*)::', re.MULTILINE)

# For markdown - get title
RE_MD_TITLE = re.compile(r'^#\s+(.+)$', re.MULTILINE)


def line_of(text, pos):
    return text.count('\n', 0, pos) + 1


def crate_name_for(path):
    """Extract crate name from path like crates/ruvector-foo/src/..."""
    parts = path.split('/')
    if len(parts) >= 2 and parts[0] == 'crates':
        return parts[1]
    return None


def analyze_rust(rel_path, content):
    """Return dict of structural info from Rust source."""
    info = {
        'functions': [],
        'structs': [],
        'enums': [],
        'traits': [],
        'impls': [],
        'mods': [],
        'uses_internal': set(),
        'line_count': content.count('\n') + 1,
    }
    for m in RE_FN.finditer(content):
        info['functions'].append((m.group(1), line_of(content, m.start())))
    for m in RE_STRUCT.finditer(content):
        info['structs'].append((m.group(1), line_of(content, m.start())))
    for m in RE_ENUM.finditer(content):
        info['enums'].append((m.group(1), line_of(content, m.start())))
    for m in RE_TRAIT.finditer(content):
        info['traits'].append((m.group(1), line_of(content, m.start())))
    for m in RE_IMPL.finditer(content):
        info['impls'].append((m.group(1), m.group(2), line_of(content, m.start())))
    for m in RE_MOD.finditer(content):
        # Skip 'mod tests' / mod test inline
        name = m.group(1)
        if name not in ('tests', 'test'):
            info['mods'].append((name, line_of(content, m.start())))
    for m in RE_USE.finditer(content):
        prefix = m.group(1).rstrip(':')
        info['uses_internal'].add(prefix)
    return info


def summarize_rust_file(rel_path, info):
    base = os.path.basename(rel_path).replace('.rs', '')
    parts = []
    if info['structs']:
        parts.append(f"{len(info['structs'])} struct(s)")
    if info['enums']:
        parts.append(f"{len(info['enums'])} enum(s)")
    if info['traits']:
        parts.append(f"{len(info['traits'])} trait(s)")
    if info['functions']:
        parts.append(f"{len(info['functions'])} fn(s)")
    if not parts:
        return f"Rust module `{base}` with no public top-level items."
    return f"Rust module `{base}` defining " + ", ".join(parts) + "."


def process_batch(batch_idx, batch_file):
    with open(batch_file) as f:
        files = json.load(f)

    nodes = []
    edges = []
    seen_node_ids = set()

    def add_node(node):
        if node['id'] in seen_node_ids:
            return
        seen_node_ids.add(node['id'])
        nodes.append(node)

    def add_edge(source, target, etype):
        if source == target:
            return
        edges.append({'source': source, 'target': target, 'type': etype})

    crates_seen = set()

    for rel_path in files:
        abs_path = PROJECT_ROOT / rel_path
        if not abs_path.exists():
            continue

        crate = crate_name_for(rel_path)
        file_id = f"file:{rel_path}"
        basename = os.path.basename(rel_path)
        ext = os.path.splitext(rel_path)[1].lower()

        # Crate node
        if crate and crate not in crates_seen:
            crates_seen.add(crate)
            add_node({
                'id': f"crate:{crate}",
                'type': 'crate',
                'name': crate,
                'filePath': f"crates/{crate}",
                'summary': f"Rust crate `{crate}` — part of RuVector distributed/storage cluster."
            })

        # File-level summary
        if ext == '.toml':
            try:
                content = abs_path.read_text(errors='replace')
            except Exception:
                content = ''
            summary = f"Cargo manifest for crate `{crate or basename}`."
            if 'fuzz' in rel_path:
                summary = f"Cargo manifest for fuzz harness of crate `{crate}`."
            add_node({'id': file_id, 'type': 'file', 'name': basename, 'filePath': rel_path, 'summary': summary})
            if crate:
                add_edge(f"crate:{crate}", file_id, 'contains')
            continue

        if ext == '.md':
            try:
                content = abs_path.read_text(errors='replace')
            except Exception:
                content = ''
            m = RE_MD_TITLE.search(content)
            title = m.group(1).strip() if m else basename
            summary = f"Documentation: {title[:120]}"
            add_node({'id': file_id, 'type': 'file', 'name': basename, 'filePath': rel_path, 'summary': summary})
            if crate:
                add_edge(f"crate:{crate}", file_id, 'contains')
            continue

        if ext != '.rs':
            add_node({'id': file_id, 'type': 'file', 'name': basename, 'filePath': rel_path, 'summary': f"File `{basename}`."})
            if crate:
                add_edge(f"crate:{crate}", file_id, 'contains')
            continue

        # Rust file
        try:
            content = abs_path.read_text(errors='replace')
        except Exception:
            content = ''
        info = analyze_rust(rel_path, content)
        summary = summarize_rust_file(rel_path, info)
        # Determine if this is lib.rs / mod root
        is_lib = basename in ('lib.rs', 'main.rs')
        add_node({
            'id': file_id,
            'type': 'file',
            'name': basename,
            'filePath': rel_path,
            'summary': summary
        })
        if crate:
            add_edge(f"crate:{crate}", file_id, 'contains')

        # Emit sub-nodes for structs, enums, traits, fns
        for (name, line) in info['structs']:
            nid = f"struct:{rel_path}:{name}"
            add_node({
                'id': nid, 'type': 'struct', 'name': name,
                'filePath': rel_path, 'lineNumber': line,
                'summary': f"Struct `{name}` in {basename}."
            })
            add_edge(file_id, nid, 'contains')

        for (name, line) in info['enums']:
            nid = f"enum:{rel_path}:{name}"
            add_node({
                'id': nid, 'type': 'enum', 'name': name,
                'filePath': rel_path, 'lineNumber': line,
                'summary': f"Enum `{name}` in {basename}."
            })
            add_edge(file_id, nid, 'contains')

        for (name, line) in info['traits']:
            nid = f"trait:{rel_path}:{name}"
            add_node({
                'id': nid, 'type': 'trait', 'name': name,
                'filePath': rel_path, 'lineNumber': line,
                'summary': f"Trait `{name}` in {basename}."
            })
            add_edge(file_id, nid, 'contains')

        # Only top-level / pub functions worth emitting; skip very small ones
        # Heuristic: skip if function count > 30, only keep first 30 to avoid bloat
        fns_to_emit = info['functions'][:40]
        for (name, line) in fns_to_emit:
            # Skip common boilerplate names like new, default, fmt, drop unless few
            nid = f"function:{rel_path}:{name}:{line}"
            add_node({
                'id': nid, 'type': 'function', 'name': name,
                'filePath': rel_path, 'lineNumber': line,
                'summary': f"Function `{name}` in {basename}."
            })
            add_edge(file_id, nid, 'contains')

        # impl edges
        for (trait_name, type_name, line) in info['impls']:
            if trait_name:
                # impl Trait for Type → uses + implements edges (best-effort, by name)
                trait_id_candidate = f"trait:{rel_path}:{trait_name.split('<')[0]}"
                struct_id_candidate = f"struct:{rel_path}:{type_name.split('<')[0]}"
                if struct_id_candidate in seen_node_ids and trait_id_candidate in seen_node_ids:
                    add_edge(struct_id_candidate, trait_id_candidate, 'implements')

        # mod edges (module sub-files declared in this file). For lib.rs/mod.rs only.
        if is_lib or basename == 'mod.rs':
            for (modname, line) in info['mods']:
                # Resolve to sibling file: <dir>/<modname>.rs or <dir>/<modname>/mod.rs
                parent_dir = os.path.dirname(rel_path)
                cand1 = f"{parent_dir}/{modname}.rs" if parent_dir else f"{modname}.rs"
                cand2 = f"{parent_dir}/{modname}/mod.rs" if parent_dir else f"{modname}/mod.rs"
                for cand in (cand1, cand2):
                    cand_id = f"file:{cand}"
                    if cand_id in seen_node_ids or (PROJECT_ROOT / cand).exists():
                        add_edge(file_id, cand_id, 'contains')
                        break

        # imports edges - best-effort: 'use crate::foo::bar' → file:.../foo/bar.rs or foo.rs
        # We just record the prefix used; aggregate edges to crate-level for cross-crate.
        for use_prefix in info['uses_internal']:
            if use_prefix.startswith('ruvector') and use_prefix != crate:
                target_crate_id = f"crate:{use_prefix.replace('_', '-')}"
                # only add if seen
                if target_crate_id in seen_node_ids:
                    add_edge(file_id, target_crate_id, 'imports')
                # Also record imports even if crate not yet emitted in batch (best-effort)
                else:
                    add_edge(file_id, target_crate_id, 'imports')

    return nodes, edges


def main():
    batch_files = [
        (1, "/home/drdave/repos/RuVector/.understand-anything/tmp/slice-10e-batch-01.json"),
        (2, "/home/drdave/repos/RuVector/.understand-anything/tmp/slice-10e-batch-02.json"),
        (3, "/home/drdave/repos/RuVector/.understand-anything/tmp/slice-10e-batch-03.json"),
        (4, "/home/drdave/repos/RuVector/.understand-anything/tmp/slice-10e-batch-04.json"),
        (5, "/home/drdave/repos/RuVector/.understand-anything/tmp/slice-10e-batch-05.json"),
    ]
    out_dir = Path("/home/drdave/repos/RuVector/.understand-anything/tmp")
    summary = []
    for batch_idx, batch_file in batch_files:
        nodes, edges = process_batch(batch_idx, batch_file)
        out = {
            "version": "1.0.0",
            "project": {"name": "RuVector", "slice": "10e", "batch": batch_idx},
            "nodes": nodes,
            "edges": edges
        }
        out_path = out_dir / f"slice-10e-batch-0{batch_idx}-graph.json"
        out_path.write_text(json.dumps(out, indent=2))
        # Count by type
        type_counts = {}
        for n in nodes:
            type_counts[n['type']] = type_counts.get(n['type'], 0) + 1
        edge_counts = {}
        for e in edges:
            edge_counts[e['type']] = edge_counts.get(e['type'], 0) + 1
        summary.append({
            'batch': batch_idx,
            'nodes': len(nodes),
            'edges': len(edges),
            'node_types': type_counts,
            'edge_types': edge_counts,
            'out_path': str(out_path),
        })
    print(json.dumps(summary, indent=2))


if __name__ == '__main__':
    main()
