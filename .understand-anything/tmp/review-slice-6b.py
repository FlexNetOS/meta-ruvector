#!/usr/bin/env python3
"""Review and fix slice-6b assembled graph for RuVector rvAgent crate."""
import json
from pathlib import Path
from collections import Counter

IN = Path('/home/drdave/repos/RuVector/.understand-anything/tmp/slice-6b-assembled.json')
OUT = Path('/home/drdave/repos/RuVector/.understand-anything/tmp/slice-6b-reviewed.json')
SUMMARY = Path('/home/drdave/repos/RuVector/.understand-anything/tmp/slice-6b-review-summary.json')

with open(IN) as f:
    g = json.load(f)

nodes = g['nodes']
edges = g['edges']

counts = {
    'typesRemapped': 0,
    'fnIdsCanonicalized': 0,
    'docIdsCanonicalized': 0,
    'modIdsCanonicalized': 0,
    'traitIdsCanonicalized': 0,
    'cratesAdded': 0,
    'containmentEdgesAdded': 0,
    'workspaceRootAdded': 0,
    'traitTargetsRemapped': 0,
    'edgeEndpointsRewritten': 0,
    'crossBatchImpliesAdded': 0,
    'edgesDroppedDangling': 0,
    'orphansRemaining': 0,
}
notes = []

# Sub-crates we expect under crates/rvAgent/
SUB_CRATES = [
    'rvagent-a2a',
    'rvagent-acp',
    'rvagent-backends',
    'rvagent-cli',
    'rvagent-core',
    'rvagent-mcp',
    'rvagent-middleware',
    'rvagent-subagents',
    'rvagent-tools',
    'rvagent-wasm',
]

# Sub-crates by Rust module name (underscored)
CRATE_BY_MODULE = {sc.replace('-', '_'): sc for sc in SUB_CRATES}

# ----- Step 1: Type remaps -----
# fn -> function, doc -> document, mod -> module
TYPE_REMAP = {'fn': 'function', 'doc': 'document', 'mod': 'module'}
for n in nodes:
    t = n.get('type')
    if t in TYPE_REMAP:
        n['type'] = TYPE_REMAP[t]
        counts['typesRemapped'] += 1

# ----- Step 2: Canonicalize node IDs -----
id_remap = {}  # old_id -> new_id


def canonicalize(n):
    """Return a canonical ID for the node, or None if already canonical."""
    nid = n.get('id', '')
    typ = n.get('type', '')
    fp = n.get('filePath') or ''
    name = n.get('name') or ''

    # Function IDs: function:<filePath>:<name>
    if typ == 'function' and fp and name:
        canonical = f'function:{fp}:{name}'
        if nid != canonical:
            return canonical

    # Document IDs: document:<filePath>
    if typ == 'document' and fp:
        canonical = f'document:{fp}'
        if nid != canonical:
            return canonical

    # Module IDs: module:<filePath>:<name>
    if typ == 'module' and fp and name:
        canonical = f'module:{fp}:{name}'
        if nid != canonical:
            return canonical

    # Trait IDs: trait:<filePath>:<name>
    if typ == 'trait' and fp and name:
        canonical = f'trait:{fp}:{name}'
        if nid != canonical:
            return canonical

    # Struct IDs: struct:<filePath>:<name>
    if typ == 'struct' and fp and name:
        canonical = f'struct:{fp}:{name}'
        if nid != canonical:
            return canonical

    # Enum IDs: enum:<filePath>:<name>
    if typ == 'enum' and fp and name:
        canonical = f'enum:{fp}:{name}'
        if nid != canonical:
            return canonical

    # File IDs: file:<filePath>
    if typ == 'file' and fp:
        canonical = f'file:{fp}'
        if nid != canonical:
            return canonical

    # Config -> file:<filePath>
    if typ == 'config' and fp:
        canonical = f'file:{fp}'
        if nid != canonical:
            return canonical
    return None


for n in nodes:
    nid = n.get('id', '')
    typ_before = n.get('type', '')
    new_id = canonicalize(n)
    if new_id and new_id != nid:
        id_remap[nid] = new_id
        n['id'] = new_id
        if typ_before == 'function':
            counts['fnIdsCanonicalized'] += 1
        elif typ_before == 'document':
            counts['docIdsCanonicalized'] += 1
        elif typ_before == 'module':
            counts['modIdsCanonicalized'] += 1
        elif typ_before == 'trait':
            counts['traitIdsCanonicalized'] += 1
        # config -> file: align type
        if new_id.startswith('file:') and typ_before == 'config':
            n['type'] = 'file'

# ----- Step 3: Deduplicate nodes after remap -----
node_by_id = {}
for n in nodes:
    nid = n['id']
    if nid in node_by_id:
        existing = node_by_id[nid]
        # Merge — fill missing fields
        for k, v in n.items():
            if k not in existing or not existing.get(k):
                existing[k] = v
    else:
        node_by_id[nid] = n

# ----- Step 4: Build trait-target remap to resolve dangling impls -----
# Dangling target patterns:
#   rvagent_core::models::ChatModel -> trait:rvagent-core/src/models.rs:ChatModel
#   rvagent_core::graph::ToolExecutor -> trait:crates/rvAgent/rvagent-core/src/graph.rs:ToolExecutor
#   rvagent_a2a::executor::TaskRunner -> trait:crates/rvAgent/rvagent-a2a/src/executor.rs:TaskRunner
#   Backend -> trait:Backend (already exists) or trait:rvagent-tools/src/lib.rs:Backend
#   SandboxBackend -> trait:SandboxBackend (already exists)
#   trait:rvagent-tools::Tool -> trait:rvagent-tools/src/lib.rs:Tool
#   trait:rvagent-tools::Backend -> trait:rvagent-tools/src/lib.rs:Backend
#   crate:rvagent-tools -> needs to be created
#   crate:rvagent-wasm -> needs to be created

# Build a lookup: bare trait name -> canonical trait id (prefer ones with fewer paths)
trait_by_name = {}
for nid, n in node_by_id.items():
    if n.get('type') == 'trait':
        nm = n.get('name')
        if nm and nm not in trait_by_name:
            trait_by_name[nm] = nid

# Build a lookup: rust path -> trait id
# e.g., rvagent_core::models::ChatModel -> trait:crates/rvAgent/rvagent-core/src/models.rs:ChatModel
trait_by_rust_path = {}
for nid, n in node_by_id.items():
    if n.get('type') == 'trait':
        fp = n.get('filePath', '')
        nm = n.get('name', '')
        if not fp or not nm:
            continue
        # Derive Rust module path
        if 'crates/rvAgent/' in fp:
            rel = fp.split('crates/rvAgent/', 1)[1]
        else:
            rel = fp
        parts = rel.split('/')
        if len(parts) < 2:
            continue
        sub_crate = parts[0]
        if sub_crate not in SUB_CRATES:
            continue
        mod_name = sub_crate.replace('-', '_')
        # path after src/
        if 'src' in parts:
            idx = parts.index('src')
            after = parts[idx + 1:]
            # e.g. ["models.rs"] or ["graph.rs"] or ["transport", "mod.rs"]
            mods = []
            for p in after:
                if p == 'lib.rs':
                    continue
                if p == 'mod.rs':
                    continue
                if p.endswith('.rs'):
                    mods.append(p[:-3])
                else:
                    mods.append(p)
            rust_path = '::'.join([mod_name] + mods + [nm])
            trait_by_rust_path[rust_path] = nid
            # Also accept the path without ".rs" segment in last
            alt = '::'.join([mod_name] + mods + [nm])
            trait_by_rust_path[alt] = nid

# Step 4b: Add missing crate nodes for rvagent-tools and rvagent-wasm
# (Cargo.toml files for these sub-crates exist but crate nodes were not produced.)
existing_crate_ids = {nid for nid, n in node_by_id.items() if n.get('type') == 'crate'}

# Discover all sub-crates that have Cargo.toml files in nodes
discovered_cargos = {}
for nid, n in node_by_id.items():
    fp = n.get('filePath', '')
    if fp.endswith('/Cargo.toml') and 'crates/rvAgent/' in fp:
        rel = fp.split('crates/rvAgent/', 1)[1]
        parts = rel.split('/')
        if len(parts) == 2:  # sub-crate Cargo.toml
            discovered_cargos[parts[0]] = fp

for sc in SUB_CRATES:
    crate_id = f'crate:{sc}'
    if crate_id in existing_crate_ids:
        continue
    cargo_path = discovered_cargos.get(sc) or f'crates/rvAgent/{sc}/Cargo.toml'
    node_by_id[crate_id] = {
        'id': crate_id,
        'type': 'crate',
        'name': sc,
        'filePath': cargo_path,
        'summary': f'Cargo crate {sc} (added during graph review).',
        'tags': ['untagged'],
        'complexity': 'moderate',
    }
    existing_crate_ids.add(crate_id)
    counts['cratesAdded'] += 1

# Step 4c: Add a workspace root node for rvAgent
ROOT_ID = 'crate:rvAgent'
if ROOT_ID not in node_by_id:
    node_by_id[ROOT_ID] = {
        'id': ROOT_ID,
        'type': 'crate',
        'name': 'rvAgent',
        'filePath': 'crates/rvAgent/Cargo.toml',
        'summary': 'rvAgent workspace root — meta-crate containing all rvagent-* sub-crates.',
        'tags': ['workspace'],
        'complexity': 'moderate',
    }
    counts['workspaceRootAdded'] = 1

# ----- Step 5: Edge endpoint remapping -----
edges_seen = set()


def remap_trait_target(ep):
    """Resolve dangling trait/struct/crate endpoint references."""
    # Direct hit in id_remap?
    if ep in id_remap:
        return id_remap[ep]
    # Already resolves to a node?
    if ep in node_by_id:
        return ep

    # Rust path pattern: <crate_mod>::<modpath>::<TraitName>
    if '::' in ep and not ep.startswith('trait:') and not ep.startswith('struct:'):
        # Try rust path lookup
        if ep in trait_by_rust_path:
            return trait_by_rust_path[ep]
        # Try bare trait name (last segment)
        last = ep.rsplit('::', 1)[-1]
        if last in trait_by_name:
            return trait_by_name[last]

    # trait:<crate_mod>::<TraitName> form, e.g., trait:rvagent-tools::Tool
    if ep.startswith('trait:') and '::' in ep:
        rest = ep[len('trait:'):]
        last = rest.rsplit('::', 1)[-1]
        if last in trait_by_name:
            return trait_by_name[last]
        # Try resolving via crate name + trait name -> "<crate>/src/lib.rs:<TraitName>"
        parts = rest.split('::')
        if len(parts) == 2:
            crate_seg = parts[0]
            tname = parts[1]
            # Replace underscores with dashes for crate dir
            crate_dir = crate_seg.replace('_', '-')
            candidate = f'trait:crates/rvAgent/{crate_dir}/src/lib.rs:{tname}'
            if candidate in node_by_id:
                return candidate

    # Bare trait name
    if ep in trait_by_name:
        return trait_by_name[ep]
    if ep in ('Backend', 'SandboxBackend', 'Tool'):
        # Direct trait name match
        if ep in trait_by_name:
            return trait_by_name[ep]

    return ep


def remap_endpoint(ep):
    if ep in id_remap:
        return id_remap[ep]
    return ep


new_edges = []
for e in edges:
    src = remap_endpoint(e.get('source', ''))
    tgt_raw = remap_endpoint(e.get('target', ''))
    typ = e.get('type', 'uses')

    # Special handling for implements edges - resolve trait targets
    if typ == 'implements' and tgt_raw not in node_by_id:
        new_tgt = remap_trait_target(tgt_raw)
        if new_tgt != tgt_raw and new_tgt in node_by_id:
            tgt_raw = new_tgt
            counts['traitTargetsRemapped'] += 1
    elif tgt_raw not in node_by_id:
        # Try generic trait/crate resolution
        new_tgt = remap_trait_target(tgt_raw)
        if new_tgt != tgt_raw and new_tgt in node_by_id:
            tgt_raw = new_tgt
            counts['edgeEndpointsRewritten'] += 1

    e['source'] = src
    e['target'] = tgt_raw
    key = (src, tgt_raw, typ)
    if key in edges_seen:
        continue
    edges_seen.add(key)
    new_edges.append(e)

edges = new_edges

# ----- Step 6: Add containment edges -----
def add_edge(src, tgt, typ, weight=0.9, direction='forward'):
    if src not in node_by_id or tgt not in node_by_id:
        return False
    key = (src, tgt, typ)
    if key in edges_seen:
        return False
    edges_seen.add(key)
    edges.append({'source': src, 'target': tgt, 'type': typ, 'weight': weight, 'direction': direction})
    return True


# Workspace root contains all sub-crate roots
for sc in SUB_CRATES:
    cid = f'crate:{sc}'
    if add_edge(ROOT_ID, cid, 'contains'):
        counts['containmentEdgesAdded'] += 1

# Each sub-crate contains files matching its directory prefix
sub_crate_dirs = {sc: f'crates/rvAgent/{sc}' for sc in SUB_CRATES}

for nid, n in list(node_by_id.items()):
    if n.get('type') in ('file', 'document'):
        fp = n.get('filePath', '')
        if not fp:
            continue
        for sc, sc_dir in sub_crate_dirs.items():
            if fp.startswith(sc_dir + '/'):
                cid = f'crate:{sc}'
                if add_edge(cid, nid, 'contains'):
                    counts['containmentEdgesAdded'] += 1
                break

# File contains its symbols (functions, structs, traits, enums, modules)
for nid, n in list(node_by_id.items()):
    if n.get('type') in ('function', 'struct', 'trait', 'enum', 'module', 'test'):
        fp = n.get('filePath')
        if not fp:
            continue
        file_id = f'file:{fp}'
        if file_id in node_by_id:
            if add_edge(file_id, nid, 'contains'):
                counts['containmentEdgesAdded'] += 1

# ----- Step 7: Drop any remaining dangling edges -----
final_node_ids = set(node_by_id.keys())
final_edges = []
for e in edges:
    s, t = e.get('source'), e.get('target')
    if s in final_node_ids and t in final_node_ids:
        final_edges.append(e)
    else:
        counts['edgesDroppedDangling'] += 1

# Orphan summary
nodes_with_edges = set()
for e in final_edges:
    nodes_with_edges.add(e['source'])
    nodes_with_edges.add(e['target'])
orphans = [nid for nid in final_node_ids if nid not in nodes_with_edges]
counts['orphansRemaining'] = len(orphans)

if orphans:
    notes.append(f'{len(orphans)} orphan nodes remain (no edges); top examples: {orphans[:5]}')

# Note about file-analyzer regex limitations
notes.append('All 10 batches used regex extraction (no tree-sitter); cross-batch trait impls resolved by Rust-path lookup.')

# Build output
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
    **counts,
    'notes': notes,
}

with open(SUMMARY, 'w') as f:
    json.dump(summary, f, indent=2)

print(json.dumps(summary, indent=2))
