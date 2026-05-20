#!/usr/bin/env python3
"""Attach a 12-step tour to slice-10b-layered.json and write slice-10b-with-tour.json."""
import json
import sys


IN_PATH = '/home/drdave/repos/RuVector/.understand-anything/tmp/slice-10b-layered.json'
OUT_PATH = '/home/drdave/repos/RuVector/.understand-anything/tmp/slice-10b-with-tour.json'

LAYER_PR = 'layer:slice10b-prime-radiant'
LAYER_NS = 'layer:slice10b-nervous-system'
LAYER_NSW = 'layer:slice10b-nervous-system-wasm'
LAYER_SONA = 'layer:slice10b-sona'


TOUR = [
    {
        "step": 1,
        "title": "Cognition cluster: workspace overview",
        "description": "Slice 10b spans four cooperating crates that together form RuVector's cognition stack: Prime Radiant (sheaf-Laplacian coherence engine), the Nervous System primitives, its WASM bindings, and Sona (continual-learning trainer). Start at the per-crate READMEs to anchor the vocabulary used throughout the rest of the tour.",
        "focusNodeIds": [
            "file:crates/prime-radiant/README.md",
            "document:crates/sona/README.md",
            "document:crates/ruvector-nervous-system-wasm/README.md",
            "crate:ruvector-sona",
            "crate:ruvector-nervous-system-wasm",
        ],
        "focusLayerIds": [LAYER_PR, LAYER_NS, LAYER_NSW, LAYER_SONA],
    },
    {
        "step": 2,
        "title": "Prime Radiant: sheaf Laplacian engine",
        "description": "The cohomology module is the mathematical heart of Prime Radiant — it builds sheaves over the substrate graph, computes restriction maps, and drives the Laplacian/diffusion solver that measures attentional coherence. The neural.rs adapter and SheafCoherenceValidator wire that math into runtime attention.",
        "focusNodeIds": [
            "file:crates/prime-radiant/src/cohomology/mod.rs",
            "file:crates/prime-radiant/src/cohomology/sheaf.rs",
            "file:crates/prime-radiant/src/cohomology/laplacian.rs",
            "file:crates/prime-radiant/src/cohomology/diffusion.rs",
            "file:crates/prime-radiant/src/cohomology/cocycle.rs",
            "file:crates/prime-radiant/src/cohomology/neural.rs",
            "struct:SheafCoherenceValidator",
        ],
        "focusLayerIds": [LAYER_PR],
    },
    {
        "step": 3,
        "title": "Prime Radiant: distributed Raft replication",
        "description": "To keep coherence state consistent across a cluster, Prime Radiant wraps its CoherenceStateMachine in a Raft replication layer. The adapter translates CoherenceCommand log entries into deterministic state transitions, while distributed/state.rs holds the replicated graph.",
        "focusNodeIds": [
            "file:crates/prime-radiant/src/distributed/mod.rs",
            "file:crates/prime-radiant/src/distributed/state.rs",
            "file:crates/prime-radiant/src/distributed/adapter.rs",
            "file:crates/prime-radiant/src/distributed/config.rs",
            "struct:DistributedCoherence",
            "struct:CoherenceStateMachine",
            "struct:crates/prime-radiant/src/distributed/adapter.rs:RaftAdapter",
            "enum:crates/prime-radiant/src/distributed/adapter.rs:CoherenceCommand",
        ],
        "focusLayerIds": [LAYER_PR],
    },
    {
        "step": 4,
        "title": "Prime Radiant: GPU kernels and dispatch",
        "description": "When the sheaf graph gets large, coherence is offloaded to the GPU through a wgpu pipeline. device.rs picks an adapter, buffer.rs/engine.rs manage VRAM, dispatch.rs schedules workgroups, and kernels.rs holds the compute shaders that implement the Laplacian on-device.",
        "focusNodeIds": [
            "file:crates/prime-radiant/src/gpu/mod.rs",
            "file:crates/prime-radiant/src/gpu/device.rs",
            "file:crates/prime-radiant/src/gpu/buffer.rs",
            "file:crates/prime-radiant/src/gpu/engine.rs",
            "file:crates/prime-radiant/src/gpu/dispatch.rs",
            "file:crates/prime-radiant/src/gpu/pipeline.rs",
            "file:crates/prime-radiant/src/gpu/kernels.rs",
        ],
        "focusLayerIds": [LAYER_PR],
    },
    {
        "step": 5,
        "title": "Prime Radiant: governance, witnesses, and lineage",
        "description": "Every coherence decision is auditable. The governance module defines PolicyBundles that gate execution, while WitnessRecord and LineageRecord (plus the ruvllm_integration witness log) provide cryptographic provenance for each generation step, persisted through a repository abstraction.",
        "focusNodeIds": [
            "file:crates/prime-radiant/src/governance/mod.rs",
            "file:crates/prime-radiant/src/governance/policy.rs",
            "file:crates/prime-radiant/src/governance/witness.rs",
            "file:crates/prime-radiant/src/governance/lineage.rs",
            "file:crates/prime-radiant/src/governance/repository.rs",
            "file:crates/prime-radiant/src/ruvllm_integration/witness.rs",
            "file:crates/prime-radiant/src/ruvllm_integration/witness_log.rs",
            "struct:GenerationWitness",
        ],
        "focusLayerIds": [LAYER_PR],
    },
    {
        "step": 6,
        "title": "Nervous System: neural primitives",
        "description": "The Nervous System crate provides biologically-inspired building blocks that downstream crates compose. Winner-take-all competition, dendritic compartments with coincidence/plateau detection, a sharded eventbus, hyperdimensional computing (HDC) vectors, and Hopfield attractor memory together form the substrate for cognition.",
        "focusNodeIds": [
            "file:crates/ruvector-nervous-system/src/compete/wta.rs",
            "file:crates/ruvector-nervous-system/src/dendrite/tree.rs",
            "file:crates/ruvector-nervous-system/src/dendrite/coincidence.rs",
            "file:crates/ruvector-nervous-system/src/eventbus/mod.rs",
            "file:crates/ruvector-nervous-system/src/hdc/vector.rs",
            "file:crates/ruvector-nervous-system/src/hdc/ops.rs",
            "file:crates/ruvector-nervous-system/src/hopfield/network.rs",
        ],
        "focusLayerIds": [LAYER_NS],
    },
    {
        "step": 7,
        "title": "Nervous System: integration & collection versioning",
        "description": "These primitives are stitched into RuVector's vector store through integration tests that exercise collection versioning, throughput, and end-to-end retrieval. The integration test files are the load-bearing examples of how NervousVectorIndex-style code composes WTA, HDC, and Hopfield against a real collection.",
        "focusNodeIds": [
            "file:crates/ruvector-nervous-system/tests/integration.rs",
            "file:crates/ruvector-nervous-system/tests/integration/nervous_integration_test.rs",
            "file:crates/ruvector-nervous-system/tests/throughput.rs",
            "file:crates/ruvector-nervous-system/tests/retrieval_quality.rs",
            "function:crates/ruvector-nervous-system/tests/integration/nervous_integration_test.rs:test_collection_versioning_continual_learning",
        ],
        "focusLayerIds": [LAYER_NS],
    },
    {
        "step": 8,
        "title": "Nervous System: plasticity (BTSP, EWC, replay)",
        "description": "Plasticity rules let the system learn without catastrophic forgetting. BTSP supplies one-shot behavioral-time-scale potentiation, EWC adds Fisher-weighted consolidation, and a replay buffer interleaves rehearsals. The ewc_tests file is the canonical reference for how Fisher information, replay, and reward-modulated consolidation interact.",
        "focusNodeIds": [
            "file:crates/ruvector-nervous-system/tests/ewc_tests.rs",
            "file:crates/ruvector-nervous-system/tests/memory_bounds.rs",
            "function:crates/ruvector-nervous-system/tests/ewc_tests.rs:test_fisher_information_accuracy",
            "function:crates/ruvector-nervous-system/tests/ewc_tests.rs:test_forgetting_reduction",
            "function:crates/ruvector-nervous-system/tests/ewc_tests.rs:test_replay_buffer_management",
            "function:crates/ruvector-nervous-system/tests/memory_bounds.rs:btsp_one_shot_no_memory_leak",
            "function:crates/ruvector-nervous-system/tests/throughput.rs:btsp_consolidation_replay",
        ],
        "focusLayerIds": [LAYER_NS],
    },
    {
        "step": 9,
        "title": "Nervous System: routing & global workspace",
        "description": "Cognitive routing decides which content reaches conscious processing. A Miller's-Law-bounded GlobalWorkspace arbitrates salience-based competition, an oscillatory router gates information by phase, and workspace_integration ties these into the registry that holds active cognitive content.",
        "focusNodeIds": [
            "file:crates/ruvector-nervous-system/tests/workspace_integration.rs",
            "function:crates/ruvector-nervous-system/tests/workspace_integration.rs:test_complete_workspace_workflow",
            "function:crates/ruvector-nervous-system/tests/workspace_integration.rs:test_millers_law_capacity",
            "function:crates/ruvector-nervous-system/tests/workspace_integration.rs:test_salience_based_competition",
            "function:crates/ruvector-nervous-system/tests/workspace_integration.rs:test_workspace_registry_integration",
            "function:crates/ruvector-nervous-system/tests/integration.rs:test_cognitive_routing_workspace",
            "function:crates/ruvector-nervous-system/tests/memory_bounds.rs:oscillatory_router_memory",
        ],
        "focusLayerIds": [LAYER_NS],
    },
    {
        "step": 10,
        "title": "Sona: continual learning core (EWC + LoRA + reasoning bank)",
        "description": "Sona is the trainer that adapts language-model weights on the fly. lora.rs supplies low-rank micro-adapters for fast per-task updates, ewc.rs protects prior knowledge with Fisher penalties, and reasoning_bank.rs stores reusable reasoning trajectories that the engine pulls from during fine-tuning.",
        "focusNodeIds": [
            "file:crates/sona/src/lib.rs",
            "file:crates/sona/src/engine.rs",
            "file:crates/sona/src/lora.rs",
            "file:crates/sona/src/ewc.rs",
            "file:crates/sona/src/reasoning_bank.rs",
            "file:crates/sona/src/trajectory.rs",
            "file:crates/sona/src/types.rs",
        ],
        "focusLayerIds": [LAYER_SONA],
    },
    {
        "step": 11,
        "title": "Sona: training pipelines (instant, background, federated)",
        "description": "Two learning loops run side-by-side: an instant loop applies micro-LoRAs within a single turn, while a background loop consolidates knowledge into the base adapter. A coordinator schedules between them, the training pipeline + factory build trainers, and federated.rs aggregates updates across peers.",
        "focusNodeIds": [
            "file:crates/sona/src/loops/coordinator.rs",
            "file:crates/sona/src/loops/instant.rs",
            "file:crates/sona/src/loops/background.rs",
            "file:crates/sona/src/training/pipeline.rs",
            "file:crates/sona/src/training/factory.rs",
            "file:crates/sona/src/training/federated.rs",
            "file:crates/sona/src/training/metrics.rs",
        ],
        "focusLayerIds": [LAYER_SONA],
    },
    {
        "step": 12,
        "title": "Nervous System WASM bindings",
        "description": "The WASM crate exposes the cognition primitives to JavaScript so the browser can drive BTSP plasticity, HDC encoding, WTA competition, and the GlobalWorkspace directly. lib.rs is the wasm-bindgen surface; the per-module files are thin idiomatic wrappers over the Rust core covered earlier.",
        "focusNodeIds": [
            "file:crates/ruvector-nervous-system-wasm/src/lib.rs",
            "file:crates/ruvector-nervous-system-wasm/src/btsp.rs",
            "file:crates/ruvector-nervous-system-wasm/src/hdc.rs",
            "file:crates/ruvector-nervous-system-wasm/src/wta.rs",
            "file:crates/ruvector-nervous-system-wasm/src/workspace.rs",
            "struct:crates/ruvector-nervous-system-wasm/src/workspace.rs:GlobalWorkspace",
            "file:crates/ruvector-nervous-system-wasm/tests/web.rs",
        ],
        "focusLayerIds": [LAYER_NSW],
    },
]


def main():
    with open(IN_PATH) as f:
        data = json.load(f)
    node_ids = {n['id'] for n in data['nodes']}
    layer_ids = {l['id'] for l in data['layers']}

    # Validate every reference
    anomalies = []
    for step in TOUR:
        for nid in step['focusNodeIds']:
            if nid not in node_ids:
                anomalies.append(f"step {step['step']}: missing node {nid}")
        for lid in step['focusLayerIds']:
            if lid not in layer_ids:
                anomalies.append(f"step {step['step']}: missing layer {lid}")

    data['tour'] = TOUR
    with open(OUT_PATH, 'w') as f:
        json.dump(data, f, indent=2)

    print(f'Wrote {OUT_PATH}')
    print(f'steps: {len(TOUR)}')
    print(f'anomalies: {len(anomalies)}')
    for a in anomalies:
        print('  -', a)


if __name__ == '__main__':
    try:
        main()
    except Exception as e:
        print(f'ERROR: {e}', file=sys.stderr)
        sys.exit(1)
