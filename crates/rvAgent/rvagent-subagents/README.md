# rvagent-subagents

Subagent specification, compilation, orchestration, result validation, and CRDT-based result merging for rvAgent.

## Overview

`rvagent-subagents` lets an rvAgent delegate work to isolated child agents. A declarative `SubAgentSpec` is compiled into a runnable `CompiledSubAgent`, spawned (optionally in parallel) by `SubAgentOrchestrator`, and its `SubAgentResult` validated for security (ADR-103 C8). State passed to and from subagents is filtered through `EXCLUDED_STATE_KEYS` so parent-specific context (messages, todos, structured responses) never leaks. Concurrent subagent results are reconciled conflict-free using CRDTs with a `VectorClock` (ADR-097, ADR-103).

## Key API

- `SubAgentSpec` — declarative subagent definition (`new`, `general_purpose`).
- `CompiledSubAgent` — a spec compiled into a runnable graph + middleware pipeline.
- `SubAgentResult` — outcome of a subagent execution.
- `SubAgentOrchestrator`, `spawn_parallel`, `SpawnError` — orchestration / parallel spawning.
- `SubAgentResultValidator`, `ValidationConfig`, `ValidationError` — result security validation.
- `merge_subagent_results`, `CrdtState`, `VectorClock`, `MergeError` — CRDT result merging.
- `prepare_subagent_state`, `extract_result_message`, `merge_subagent_state`, `EXCLUDED_STATE_KEYS` — state isolation helpers.
- `GENERAL_PURPOSE_NAME`, `GENERAL_PURPOSE_DESCRIPTION`, `DEFAULT_SUBAGENT_PROMPT` — built-in subagent constants.

## License

Licensed under either MIT OR Apache-2.0.
