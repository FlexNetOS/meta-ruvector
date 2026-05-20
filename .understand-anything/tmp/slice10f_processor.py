#!/usr/bin/env python3
"""Process slice-10f batches 1-7: extract Rust structural info and build graph files."""
import json
import os
import re
import sys
from pathlib import Path

PROJECT_ROOT = Path("/home/drdave/repos/RuVector")
TMP_DIR = PROJECT_ROOT / ".understand-anything/tmp"
OUT_DIR = PROJECT_ROOT / ".understand-anything/intermediate"
OUT_DIR.mkdir(parents=True, exist_ok=True)

# Regex patterns for Rust
RE_FN = re.compile(r'^\s*(?:pub(?:\([^)]*\))?\s+)?(?:async\s+)?(?:unsafe\s+)?(?:const\s+)?fn\s+([a-zA-Z_][a-zA-Z0-9_]*)\s*[<(]', re.MULTILINE)
RE_STRUCT = re.compile(r'^\s*(?:pub(?:\([^)]*\))?\s+)?struct\s+([A-Z][a-zA-Z0-9_]*)', re.MULTILINE)
RE_ENUM = re.compile(r'^\s*(?:pub(?:\([^)]*\))?\s+)?enum\s+([A-Z][a-zA-Z0-9_]*)', re.MULTILINE)
RE_TRAIT = re.compile(r'^\s*(?:pub(?:\([^)]*\))?\s+)?(?:unsafe\s+)?trait\s+([A-Z][a-zA-Z0-9_]*)', re.MULTILINE)
RE_MOD = re.compile(r'^\s*(?:pub(?:\([^)]*\))?\s+)?mod\s+([a-zA-Z_][a-zA-Z0-9_]*)\s*;', re.MULTILINE)
RE_IMPL = re.compile(r'^\s*impl(?:\s*<[^>]*>)?\s+(?:([A-Z][a-zA-Z0-9_:<>, ]*?)\s+for\s+)?([A-Z][a-zA-Z0-9_:<>, ]*?)\s*(?:<|where|\{)', re.MULTILINE)
RE_USE = re.compile(r'^\s*use\s+(?:crate::|super::|self::)?([a-zA-Z_][a-zA-Z0-9_:]*)', re.MULTILINE)


def line_of(text, idx):
    return text.count('\n', 0, idx) + 1


def extract_rust(path):
    """Extract fns/structs/enums/traits/mods/impls from a Rust file."""
    try:
        text = path.read_text(encoding='utf-8', errors='replace')
    except Exception as e:
        return None
    lines = text.count('\n') + 1
    items = {
        'functions': [],
        'structs': [],
        'enums': [],
        'traits': [],
        'mods': [],
        'impls': [],
        'uses': [],
        'lines': lines,
    }
    for m in RE_FN.finditer(text):
        name = m.group(1)
        if name in ('main',) or not name.startswith('_'):
            items['functions'].append((name, line_of(text, m.start())))
    for m in RE_STRUCT.finditer(text):
        items['structs'].append((m.group(1), line_of(text, m.start())))
    for m in RE_ENUM.finditer(text):
        items['enums'].append((m.group(1), line_of(text, m.start())))
    for m in RE_TRAIT.finditer(text):
        items['traits'].append((m.group(1), line_of(text, m.start())))
    for m in RE_MOD.finditer(text):
        items['mods'].append((m.group(1), line_of(text, m.start())))
    for m in RE_IMPL.finditer(text):
        trait_name = m.group(1)
        struct_name = m.group(2).split('<')[0].strip()
        items['impls'].append((trait_name, struct_name, line_of(text, m.start())))
    for m in RE_USE.finditer(text):
        items['uses'].append(m.group(1))
    return items


def parse_cargo(path):
    """Extract crate name and dependencies from Cargo.toml."""
    try:
        text = path.read_text(encoding='utf-8', errors='replace')
    except Exception:
        return None
    name = None
    m = re.search(r'^\s*name\s*=\s*"([^"]+)"', text, re.MULTILINE)
    if m:
        name = m.group(1)
    # Local path dependencies
    deps = re.findall(r'^\s*([a-zA-Z0-9_-]+)\s*=\s*\{[^}]*path\s*=', text, re.MULTILINE)
    return {'name': name, 'lines': text.count('\n') + 1, 'deps': deps}


def crate_root(fp):
    """Get crate directory path from file path."""
    parts = Path(fp).parts
    # e.g. crates/foo/src/lib.rs -> crates/foo
    if len(parts) >= 2 and parts[0] == 'crates':
        return f"{parts[0]}/{parts[1]}"
    return parts[0] if parts else ''


def file_summary_rust(fp, items):
    base = Path(fp).name
    crate = Path(fp).parts[1] if len(Path(fp).parts) > 1 else ''
    parts = []
    if items['structs']:
        parts.append(f"{len(items['structs'])} struct(s)")
    if items['enums']:
        parts.append(f"{len(items['enums'])} enum(s)")
    if items['traits']:
        parts.append(f"{len(items['traits'])} trait(s)")
    if items['functions']:
        parts.append(f"{len(items['functions'])} fn(s)")
    if items['mods']:
        parts.append(f"{len(items['mods'])} module decl(s)")
    detail = ', '.join(parts) if parts else 'misc Rust source'
    if base == 'lib.rs':
        return f"Library entry point for crate {crate}; declares {detail}."
    if base == 'main.rs':
        return f"Binary entry point for crate {crate}; contains {detail}."
    if base == 'mod.rs':
        return f"Module aggregator in {crate} ({detail})."
    if base == 'build.rs':
        return f"Cargo build script for {crate}."
    stem = Path(fp).stem
    return f"{stem} module in {crate} ({detail})."


def process_batch(batch_num):
    batch_path = TMP_DIR / f"slice-10f-batch-{batch_num:02d}.json"
    files = json.loads(batch_path.read_text())

    nodes = []
    edges = []
    seen_node_ids = set()
    crate_info = {}  # crate_path -> {name, deps}

    def add_node(node):
        if node['id'] in seen_node_ids:
            return
        seen_node_ids.add(node['id'])
        nodes.append(node)

    def add_edge(src, tgt, etype):
        if src == tgt:
            return
        edges.append({'source': src, 'target': tgt, 'type': etype})

    # First pass: find Cargo.toml files to map crate names to paths
    for fp in files:
        if fp.endswith('Cargo.toml') and not fp.endswith('/.cargo/config.toml'):
            full = PROJECT_ROOT / fp
            if full.exists():
                info = parse_cargo(full)
                if info and info['name']:
                    crate_path = crate_root(fp)
                    crate_info[info['name']] = {'path': crate_path, 'deps': info['deps'], 'manifest': fp}

    # Crate-name -> id map for use lookups
    crate_name_to_id = {}
    for cname, ci in crate_info.items():
        cid = f"crate:{ci['path']}"
        crate_name_to_id[cname] = cid
        crate_name_to_id[cname.replace('-', '_')] = cid

    # Process each file
    for fp in files:
        full = PROJECT_ROOT / fp
        if not full.exists():
            continue
        base = Path(fp).name
        ext = Path(fp).suffix
        file_id = f"file:{fp}"

        if base == 'Cargo.toml' and '/.cargo/' not in fp:
            info = parse_cargo(full)
            if info:
                cname = info['name'] or Path(fp).parent.name
                cpath = crate_root(fp)
                crate_id = f"crate:{cpath}"
                add_node({
                    'id': crate_id,
                    'type': 'crate',
                    'name': cname,
                    'filePath': fp,
                    'summary': f"Cargo manifest for {cname} crate ({len(info['deps'])} local path dep(s))."
                })
                add_node({
                    'id': file_id,
                    'type': 'file',
                    'name': base,
                    'filePath': fp,
                    'summary': f"Cargo manifest declaring crate {cname}."
                })
                add_edge(crate_id, file_id, 'contains')
                for dep in info['deps']:
                    dep_id = crate_name_to_id.get(dep) or crate_name_to_id.get(dep.replace('-', '_'))
                    if dep_id:
                        add_edge(crate_id, dep_id, 'imports')
            continue

        if ext == '.toml':
            add_node({
                'id': file_id,
                'type': 'file',
                'name': base,
                'filePath': fp,
                'summary': f"TOML configuration ({base})."
            })
            continue

        if ext == '.md':
            stem = Path(fp).stem
            crate = Path(fp).parts[1] if len(Path(fp).parts) > 1 else ''
            add_node({
                'id': file_id,
                'type': 'file',
                'name': base,
                'filePath': fp,
                'summary': f"Documentation for {crate} ({stem})."
            })
            continue

        if ext == '.rs':
            items = extract_rust(full)
            if items is None:
                continue
            cpath = crate_root(fp)
            crate_id = f"crate:{cpath}"
            module_name = Path(fp).stem
            # Determine module-id (use file path)
            mod_id = f"module:{fp}"
            # File node
            add_node({
                'id': file_id,
                'type': 'file',
                'name': base,
                'filePath': fp,
                'summary': file_summary_rust(fp, items)
            })
            # Module node (lib/main create logical module = crate root)
            if base in ('lib.rs', 'main.rs'):
                add_node({
                    'id': mod_id,
                    'type': 'module',
                    'name': f"{cpath}::{module_name}",
                    'filePath': fp,
                    'summary': f"Crate root module for {cpath}."
                })
            else:
                add_node({
                    'id': mod_id,
                    'type': 'module',
                    'name': module_name,
                    'filePath': fp,
                    'summary': f"Module {module_name} in {cpath}."
                })
            add_edge(file_id, mod_id, 'contains')
            # Link to crate if present
            if cpath:
                # Crate node may be created elsewhere; emit a reference edge regardless
                add_edge(f"crate:{cpath}", mod_id, 'contains')

            # Structs
            for name, line in items['structs']:
                nid = f"struct:{fp}:{name}"
                add_node({
                    'id': nid, 'type': 'struct', 'name': name,
                    'filePath': fp, 'lineNumber': line,
                    'summary': f"Struct {name} defined in {Path(fp).name}."
                })
                add_edge(mod_id, nid, 'contains')
            # Enums
            for name, line in items['enums']:
                nid = f"enum:{fp}:{name}"
                add_node({
                    'id': nid, 'type': 'enum', 'name': name,
                    'filePath': fp, 'lineNumber': line,
                    'summary': f"Enum {name} defined in {Path(fp).name}."
                })
                add_edge(mod_id, nid, 'contains')
            # Traits
            for name, line in items['traits']:
                nid = f"trait:{fp}:{name}"
                add_node({
                    'id': nid, 'type': 'trait', 'name': name,
                    'filePath': fp, 'lineNumber': line,
                    'summary': f"Trait {name} defined in {Path(fp).name}."
                })
                add_edge(mod_id, nid, 'contains')
            # Public functions: emit only meaningful ones (limit per file to ~25)
            fn_seen = set()
            for name, line in items['functions'][:30]:
                if name in fn_seen:
                    continue
                fn_seen.add(name)
                nid = f"function:{fp}:{name}"
                add_node({
                    'id': nid, 'type': 'function', 'name': name,
                    'filePath': fp, 'lineNumber': line,
                    'summary': f"Function {name} in {Path(fp).name}."
                })
                add_edge(mod_id, nid, 'contains')
            # impl edges (trait implementations)
            for trait_name, struct_name, line in items['impls']:
                if trait_name:
                    # implements
                    src = f"struct:{fp}:{struct_name}"
                    tgt_id = f"trait:{fp}:{trait_name.split('::')[-1]}"
                    add_edge(src, tgt_id, 'implements')
            # Cross-crate uses (crate:: only matters for intra-crate; super crate refs in use)
            # Extract `use crate_name::...` for crate-level imports
            for use_path in items['uses']:
                root = use_path.split('::')[0]
                if root in ('std', 'core', 'alloc', 'crate', 'super', 'self'):
                    continue
                dep_id = crate_name_to_id.get(root) or crate_name_to_id.get(root.replace('-', '_'))
                if dep_id:
                    add_edge(file_id, dep_id, 'uses')
            continue

        # Default: unknown extension
        add_node({
            'id': file_id,
            'type': 'file',
            'name': base,
            'filePath': fp,
            'summary': f"Auxiliary file {base}."
        })

    # Build output
    output = {
        'version': '1.0.0',
        'project': {'name': 'RuVector', 'slice': '10f', 'batch': batch_num},
        'nodes': nodes,
        'edges': edges,
    }
    out_path = OUT_DIR / f"slice-10f-batch-{batch_num:02d}-graph.json"
    out_path.write_text(json.dumps(output, indent=2))
    return len(nodes), len(edges), out_path


if __name__ == '__main__':
    summary = []
    for n in range(1, 8):
        nodes, edges, path = process_batch(n)
        summary.append(f"batch-{n:02d}: nodes={nodes} edges={edges} -> {path.name}")
    print('\n'.join(summary))
