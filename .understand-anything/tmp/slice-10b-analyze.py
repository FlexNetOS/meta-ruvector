#!/usr/bin/env python3
"""Analyze slice-10b-layered.json to surface key nodes for tour building."""
import json
import sys
from collections import defaultdict


def main():
    in_path = sys.argv[1]
    out_path = sys.argv[2]
    with open(in_path) as f:
        data = json.load(f)
    nodes = data['nodes']
    edges = data['edges']
    layers = data['layers']

    node_by_id = {n['id']: n for n in nodes}
    fan_in = defaultdict(int)
    fan_out = defaultdict(int)
    for e in edges:
        src = e.get('source') or e.get('from')
        tgt = e.get('target') or e.get('to')
        if src is None or tgt is None:
            continue
        fan_out[src] += 1
        fan_in[tgt] += 1

    # Layer membership map
    layer_members = {l['id']: set(l.get('memberNodeIds', [])) for l in layers}

    # Helper to search nodes by name keywords / path patterns
    def find(keyword_groups, layer_id=None, prefer_types=None, limit=20):
        """Return list of node IDs whose name/path contains any keyword AND in layer (if given)."""
        results = []
        keywords = [k.lower() for k in keyword_groups]
        for n in nodes:
            nid = n['id']
            if layer_id and nid not in layer_members.get(layer_id, set()):
                continue
            name = (n.get('name') or '').lower()
            path = (n.get('filePath') or '').lower()
            blob = name + ' ' + path + ' ' + nid.lower()
            if any(k in blob for k in keywords):
                score = fan_in.get(nid, 0) + fan_out.get(nid, 0)
                # prefer file/struct/crate types
                t = n.get('type', '')
                if prefer_types and t in prefer_types:
                    score += 100
                # prefer crate root files (lib.rs/mod.rs)
                if name in ('lib.rs', 'mod.rs'):
                    score += 50
                results.append((score, nid, n))
        results.sort(key=lambda x: -x[0])
        return results[:limit]

    # For each tour bucket compute a candidate list
    buckets = {}

    # 1. Workspace overview (Cargo.toml roots, README, top-level crates)
    overview = []
    for n in nodes:
        nid = n['id']
        name = (n.get('name') or '').lower()
        path = (n.get('filePath') or '').lower()
        if 'cargo.toml' in name and path.count('/') <= 2:
            overview.append((fan_in.get(nid, 0) + fan_out.get(nid, 0), nid, n))
        elif 'readme' in name.lower() and path.count('/') <= 2:
            overview.append((100, nid, n))
    overview.sort(key=lambda x: -x[0])
    buckets['overview'] = overview[:20]

    # 2. PR sheaf Laplacian engine
    buckets['pr_sheaf'] = find(
        ['attentioncoherence', 'coherence_engine', 'coherenceengine', 'sheaf', 'laplacian', 'cohomology', 'cochain'],
        layer_id='layer:slice10b-prime-radiant',
        prefer_types=['file', 'struct'],
    )

    # 3. PR distributed Raft
    buckets['pr_raft'] = find(
        ['raft', 'distributed', 'state_machine', 'statemachine', 'replication', 'consensus', 'leader_election'],
        layer_id='layer:slice10b-prime-radiant',
    )

    # 4. PR GPU kernels
    buckets['pr_gpu'] = find(
        ['device', 'buffermanager', 'dispatcher', 'gpu', 'kernel', 'wgpu', 'compute_shader', 'shader'],
        layer_id='layer:slice10b-prime-radiant',
    )

    # 5. PR governance
    buckets['pr_governance'] = find(
        ['policybundle', 'witness', 'lineage', 'policy', 'governance', 'repository', 'repo'],
        layer_id='layer:slice10b-prime-radiant',
    )

    # 6. NS neural primitives
    buckets['ns_neural'] = find(
        ['wta', 'winner_take_all', 'dendrite', 'eventbus', 'event_bus', 'hdc', 'hopfield', 'hyperdimensional'],
        layer_id='layer:slice10b-nervous-system',
    )

    # 7. NS integration
    buckets['ns_integration'] = find(
        ['nervousvectorindex', 'nervous_vector_index', 'predictivewriter', 'predictive_writer', 'collectionversioning', 'collection_versioning'],
        layer_id='layer:slice10b-nervous-system',
    )

    # 8. NS plasticity
    buckets['ns_plasticity'] = find(
        ['btsp', 'eprop', 'e_prop', 'ewc', 'plasticity', 'spiking', 'consolidation'],
        layer_id='layer:slice10b-nervous-system',
    )

    # 9. NS routing / workspace
    buckets['ns_routing'] = find(
        ['circadian', 'oscillatory', 'phase', 'globalworkspace', 'global_workspace', 'workspace', 'router', 'gating'],
        layer_id='layer:slice10b-nervous-system',
    )

    # 10. Sona continual learning core
    buckets['sona_continual'] = find(
        ['ewc', 'lora', 'adapter', 'reasoning_bank', 'reasoningbank', 'continual', 'replay'],
        layer_id='layer:slice10b-sona',
    )

    # 11. Sona training pipelines
    buckets['sona_training'] = find(
        ['instant', 'background', 'coordinator', 'training_factory', 'trainingfactory', 'federated', 'pipeline', 'trainer'],
        layer_id='layer:slice10b-sona',
    )

    # 12. WASM bindings
    buckets['wasm'] = find(
        ['btsp', 'hdc', 'wta', 'workspace', 'wasm', 'bindgen'],
        layer_id='layer:slice10b-nervous-system-wasm',
    )

    # Compact each bucket to (id, name, type, score)
    out_buckets = {}
    for key, items in buckets.items():
        out_buckets[key] = [
            {
                'id': nid,
                'name': n.get('name'),
                'type': n.get('type'),
                'filePath': n.get('filePath'),
                'summary': (n.get('summary') or '')[:300],
                'score': score,
                'fanIn': fan_in.get(nid, 0),
                'fanOut': fan_out.get(nid, 0),
            }
            for (score, nid, n) in items
        ]

    # Top fan-in nodes overall (filtered to file/struct/crate types — non-leaf)
    important_types = {'file', 'struct', 'crate', 'module'}
    top_fan_in = []
    for nid, count in fan_in.items():
        n = node_by_id.get(nid)
        if not n:
            continue
        if n.get('type') not in important_types:
            continue
        top_fan_in.append((count, nid, n))
    top_fan_in.sort(key=lambda x: -x[0])
    out_fan_in = [
        {'id': nid, 'name': n.get('name'), 'type': n.get('type'), 'filePath': n.get('filePath'), 'fanIn': cnt}
        for (cnt, nid, n) in top_fan_in[:40]
    ]

    # Layer summary
    out_layers = []
    for l in layers:
        out_layers.append({
            'id': l['id'],
            'name': l['name'],
            'description': l.get('description', ''),
            'memberCount': len(l.get('memberNodeIds', [])),
        })

    output = {
        'totalNodes': len(nodes),
        'totalEdges': len(edges),
        'layers': out_layers,
        'buckets': out_buckets,
        'topFanIn': out_fan_in,
    }
    with open(out_path, 'w') as f:
        json.dump(output, f, indent=2)
    print(f'Wrote {out_path}')


if __name__ == '__main__':
    try:
        main()
    except Exception as e:
        print(f'ERROR: {e}', file=sys.stderr)
        sys.exit(1)
