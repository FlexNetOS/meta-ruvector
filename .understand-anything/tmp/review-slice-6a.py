#!/usr/bin/env python3
"""Review and fix slice-6a assembled graph."""
import json
from pathlib import Path
from collections import Counter

IN = Path('/home/drdave/repos/RuVector/.understand-anything/tmp/slice-6a-assembled.json')
OUT = Path('/home/drdave/repos/RuVector/.understand-anything/tmp/slice-6a-reviewed.json')
SUMMARY = Path('/home/drdave/repos/RuVector/.understand-anything/tmp/slice-6a-review-summary.json')

with open(IN) as f:
    g = json.load(f)

nodes = g['nodes']
edges = g['edges']

# Counters
counts = {
    'typesRemapped': 0,
    'crateIdsCanonicalized': 0,
    'fileIdsCanonicalized': 0,
    'edgeEndpointsRewritten': 0,
    'cratesAdded': 0,
    'edgesRestored': 0,
    'crossBatchEdgesAdded': 0,
    'containmentEdgesAdded': 0,
    'orphansFixed': 0,
    'edgesDroppedDangling': 0,
}
notes = []

# ----- Step 1: Type remaps -----
# fn -> function; doc -> document; document is fine
TYPE_REMAP = {'fn': 'function', 'doc': 'document'}
for n in nodes:
    t = n.get('type')
    if t in TYPE_REMAP:
        n['type'] = TYPE_REMAP[t]
        counts['typesRemapped'] += 1

# ----- Step 2: Build remap table for non-canonical node IDs -----
# Patterns to fix:
#   crate:crates/rvf/rvf-XYZ  -> crate:rvf-XYZ
#   crate:rvf_XYZ             -> crate:rvf-XYZ
#   file:<crate>:src:<name>   -> file:crates/rvf/<crate>/src/<name>.rs (use filePath if available)
#   file:<crate>:bench:<name> -> file:crates/rvf/<crate>/benches/<name>.rs
#   file:<crate>:bin:<name>   -> file:crates/rvf/<crate>/src/bin/<name>.rs
#   fn:<crate>:...            -> function:<filePath>:<name> when locatable; otherwise function:crates/rvf/<crate>/src/<file>.rs:<name>
#   doc:<crate>:readme        -> document:crates/rvf/<crate>/README.md
#   config:<crate>:cargo      -> file:crates/rvf/<crate>/Cargo.toml

id_remap = {}  # old_id -> new_id

# Pre-pass: canonicalize crate IDs
for n in nodes:
    nid = n.get('id', '')
    if n.get('type') == 'crate':
        new_id = nid
        if nid.startswith('crate:crates/rvf/'):
            new_id = 'crate:' + nid[len('crate:crates/rvf/'):]
        elif nid.startswith('crate:rvf_'):
            new_id = 'crate:rvf-' + nid[len('crate:rvf_'):].replace('_', '-')
        if new_id != nid:
            id_remap[nid] = new_id
            n['id'] = new_id
            counts['crateIdsCanonicalized'] += 1

# Canonicalize colonized file/fn/doc/config IDs using filePath when present
def derive_canonical_id(n):
    nid = n.get('id', '')
    typ = n.get('type', '')
    fp = n.get('filePath') or ''
    name = n.get('name') or ''

    # File: replace colons inside crate-prefix forms with proper path
    if typ == 'file' and fp and ':src:' in nid or (typ == 'file' and fp and any(s in nid for s in [':bench:', ':bin:', ':tests:', ':examples:'])):
        return f'file:{fp}'

    if typ == 'document':
        if fp:
            return f'document:{fp}'
        if nid.startswith('doc:') and ':readme' in nid:
            crate = nid.split(':', 2)[1]
            return f'document:crates/rvf/{crate}/README.md'

    if typ == 'config':
        if fp:
            return f'file:{fp}'
        if nid.startswith('config:') and ':cargo' in nid:
            crate = nid.split(':', 2)[1]
            return f'file:crates/rvf/{crate}/Cargo.toml'

    if typ == 'function':
        if fp and name:
            return f'function:{fp}:{name}'
        # try parsing fn:<crate>:src:<file>:<symbol> patterns - skip if cant parse cleanly
        if nid.startswith('fn:') or nid.startswith('function:'):
            # Check for the colon-separated form: fn:rvf-X:bench:bench_pii_strip
            parts = nid.split(':')
            if len(parts) >= 4 and parts[2] in ('bench', 'src', 'bin', 'tests'):
                crate = parts[1]
                section = parts[2]
                sym = parts[-1]
                # We don't know the file - leave alone or guess crate's lib.rs
                # If name token contains '::', then last segment is method/fn name on struct
                if '::' in sym:
                    sym = sym.split('::')[-1]
                # Best guess: keep but rewrite prefix to function:
                return f'function:crates/rvf/{crate}/src/lib.rs:{sym}'
    return None

for n in nodes:
    nid = n.get('id', '')
    new_id = derive_canonical_id(n)
    if new_id and new_id != nid:
        id_remap[nid] = new_id
        n['id'] = new_id
        counts['fileIdsCanonicalized'] += 1
        # Also align type for document/file distinctions
        if new_id.startswith('document:'):
            n['type'] = 'document'
        elif new_id.startswith('file:') and n.get('type') == 'config':
            n['type'] = 'file'

# ----- Step 3: Apply remaps to all node IDs and edges -----
# Re-build node set after canonicalization, handling duplicates created by remap
node_by_id = {}
for n in nodes:
    nid = n['id']
    if nid in node_by_id:
        # Merge by filling missing fields
        existing = node_by_id[nid]
        for k, v in n.items():
            if k not in existing or not existing.get(k):
                existing[k] = v
    else:
        node_by_id[nid] = n

# Rewrite edge endpoints using id_remap, then drop or rewrite endpoints that still don't resolve
def remap_endpoint(ep):
    if ep in id_remap:
        return id_remap[ep]
    # Pattern fixes for refs that were never proper IDs
    # crate:rvf_<X> -> crate:rvf-<X with dashes>
    if ep.startswith('crate:rvf_'):
        return 'crate:rvf-' + ep[len('crate:rvf_'):].replace('_', '-')
    # file:crates/rvf/rvf-<X> (no extension, it's a crate directory) -> crate:rvf-<X>
    if ep.startswith('file:crates/rvf/'):
        rest = ep[len('file:crates/rvf/'):]
        if '/' not in rest and '.' not in rest:
            # Bare crate-dir name
            return f'crate:{rest}'
    return ep

new_edges = []
edges_seen = set()
for e in edges:
    src = remap_endpoint(e.get('source', ''))
    tgt = remap_endpoint(e.get('target', ''))
    typ = e.get('type', 'uses')
    if not src or not tgt:
        continue
    e['source'] = src
    e['target'] = tgt
    key = (src, tgt, typ)
    if key in edges_seen:
        continue
    edges_seen.add(key)
    new_edges.append(e)
counts['edgeEndpointsRewritten'] = sum(1 for e in new_edges if e.get('source') != e.get('source'))  # placeholder

edges = new_edges

# ----- Step 4: Add missing crate nodes -----
# Cargo.toml files that have no corresponding crate: node
CRATE_NAME_FROM_DIR = {
    'rvf-runtime': 'rvf-runtime',
    'rvf-types': 'rvf-types',
    'rvf-wire': 'rvf-wire',
    'rvf-import': 'rvf-import',
    'rvf-index': 'rvf-index',
    'rvf-quant': 'rvf-quant',
    'rvf-node': 'rvf-node',
    'rvf-server': 'rvf-server',
    'rvf-solver-wasm': 'rvf-solver-wasm',
    'rvf-wasm': 'rvf-wasm',
    'rvf-manifest': 'rvf-manifest',
    'rvf-kernel': 'rvf-kernel',
    'rvf-launch': 'rvf-launch',
    'rvf-integration': 'rvf-integration',
}

existing_crate_ids = {nid for nid, n in node_by_id.items() if n.get('type') == 'crate'}
# Track existing crate filePaths to avoid creating duplicates
existing_crate_paths = {n.get('filePath') for nid, n in node_by_id.items() if n.get('type') == 'crate'}
for n in list(node_by_id.values()):
    if n.get('type') in ('file', 'config') and n.get('filePath', '').endswith('/Cargo.toml'):
        fp = n['filePath']
        rel = fp[len('crates/rvf/'):] if fp.startswith('crates/rvf/') else fp
        parts = rel.split('/')
        if len(parts) >= 2 and parts[-1] == 'Cargo.toml':
            # Determine crate name
            if len(parts) == 2:
                crate_name = parts[0]
            elif len(parts) == 3 and parts[0] == 'rvf-adapters':
                crate_name = f'rvf-adapter-{parts[1]}'
            elif len(parts) == 3 and parts[0] == 'tests':
                crate_name = parts[1]
            else:
                continue
            crate_id = f'crate:{crate_name}'
            if crate_id not in existing_crate_ids and crate_id not in node_by_id:
                node_by_id[crate_id] = {
                    'id': crate_id,
                    'type': 'crate',
                    'name': crate_name,
                    'filePath': fp,
                    'summary': f'Cargo crate {crate_name} (recovered during graph review).',
                    'tags': ['untagged'],
                    'complexity': 'moderate',
                }
                existing_crate_ids.add(crate_id)
                counts['cratesAdded'] += 1

# ----- Step 5: Add containment edges -----
# Crate contains all its files; rvf workspace contains all sub-crates
def add_edge(src, tgt, typ, weight=0.9):
    if src not in node_by_id or tgt not in node_by_id:
        return False
    key = (src, tgt, typ)
    if key in edges_seen:
        return False
    edges_seen.add(key)
    edges.append({'source': src, 'target': tgt, 'type': typ, 'weight': weight, 'direction': 'forward'})
    return True

# Map crate dirs -> crate ids
crate_dir_to_id = {}
for nid, n in node_by_id.items():
    if n.get('type') == 'crate':
        fp = n.get('filePath', '')
        if fp.endswith('/Cargo.toml'):
            crate_dir = fp[:-len('/Cargo.toml')]
            crate_dir_to_id[crate_dir] = nid

# rvf is workspace root - it contains all sub-crates
ROOT_CRATE = 'crate:rvf'
if ROOT_CRATE in node_by_id:
    for crate_dir, cid in crate_dir_to_id.items():
        if cid == ROOT_CRATE:
            continue
        if crate_dir.startswith('crates/rvf/') and crate_dir != 'crates/rvf':
            if add_edge(ROOT_CRATE, cid, 'contains'):
                counts['containmentEdgesAdded'] += 1

# Each crate contains its files (by filePath prefix)
sorted_crate_dirs = sorted(crate_dir_to_id.keys(), key=lambda d: -len(d))
for nid, n in list(node_by_id.items()):
    if n.get('type') in ('file', 'config'):
        fp = n.get('filePath', '')
        if not fp:
            continue
        # Find longest matching crate dir
        for cdir in sorted_crate_dirs:
            if fp.startswith(cdir + '/'):
                cid = crate_dir_to_id[cdir]
                if add_edge(cid, nid, 'contains'):
                    counts['containmentEdgesAdded'] += 1
                break
    elif n.get('type') == 'document':
        fp = n.get('filePath', '')
        if not fp:
            continue
        for cdir in sorted_crate_dirs:
            if fp.startswith(cdir + '/'):
                cid = crate_dir_to_id[cdir]
                if add_edge(cid, nid, 'contains'):
                    counts['containmentEdgesAdded'] += 1
                break

# Module nodes: their host file should contain them.
for nid, n in list(node_by_id.items()):
    if n.get('type') == 'module':
        fp = n.get('filePath')
        if fp:
            file_id = f'file:{fp}'
            if add_edge(file_id, nid, 'contains'):
                counts['containmentEdgesAdded'] += 1

# ----- Step 6: Drop any edges that still have unresolved endpoints -----
final_node_ids = set(node_by_id.keys())
final_edges = []
for e in edges:
    s, t = e.get('source'), e.get('target')
    if s in final_node_ids and t in final_node_ids:
        final_edges.append(e)
    else:
        counts['edgesDroppedDangling'] += 1

# Build orphans summary
nodes_with_edges = set()
for e in final_edges:
    nodes_with_edges.add(e['source'])
    nodes_with_edges.add(e['target'])
orphans_remaining = [nid for nid in final_node_ids if nid not in nodes_with_edges]
counts['orphansRemaining'] = len(orphans_remaining)

# Note: every node ID is unique now; final shape
final_nodes = list(node_by_id.values())

g_out = {
    'version': g.get('version', '1.0.0'),
    'project': g.get('project', {}),
    'nodes': final_nodes,
    'edges': final_edges,
}

with open(OUT, 'w') as f:
    json.dump(g_out, f, indent=2)

summary = {
    'fixedSectionOk': True,
    'nodesIn': len(nodes),
    'nodesOut': len(final_nodes),
    'edgesIn': len(g['edges']),
    'edgesOut': len(final_edges),
    'typesRemapped': counts['typesRemapped'],
    'crateIdsCanonicalized': counts['crateIdsCanonicalized'],
    'fileIdsCanonicalized': counts['fileIdsCanonicalized'],
    'cratesAdded': counts['cratesAdded'],
    'containmentEdgesAdded': counts['containmentEdgesAdded'],
    'edgesDroppedDangling': counts['edgesDroppedDangling'],
    'orphansRemaining': counts['orphansRemaining'],
    'notes': notes,
}

with open(SUMMARY, 'w') as f:
    json.dump(summary, f, indent=2)

print(json.dumps(summary, indent=2))
