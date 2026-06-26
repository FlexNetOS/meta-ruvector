# rvagent-core

Core types for the rvAgent agent framework — typed agent state, configuration, model resolution, and the agent execution graph.

## Overview

`rvagent-core` is the foundational crate of the rvAgent family in the meta-ruvector workspace. Every other rvAgent crate (`-tools`, `-backends`, `-middleware`, `-subagents`, `-mcp`, `-acp`, `-a2a`, `-cli`) depends on it for shared message, state, config, and model abstractions. It defines the `ChatModel` trait and provider/model resolution, a typed agent state with `Arc`-based cheap cloning, copy-on-write forking, resource-budget enforcement, and an RVF (ruvector format) bridge for witnessed, content-addressed agent containers.

## Key API

- `RvAgentConfig`, `SecurityPolicy`, `ResourceBudget`, `BackendConfig` — agent configuration (`config`).
- `AgentState`, `FileData`, `TodoItem`, `TodoStatus`, `SkillMetadata` — typed agent state (`state`).
- `CowStateBackend` — copy-on-write state backend for efficient forking.
- `AgentGraph`, `AgentNode`, `GraphConfig`, `ToolExecutor` — agent execution graph / state machine (`graph`).
- `Message`, `AiMessage`, `HumanMessage`, `SystemMessage`, `ToolMessage`, `ToolCall` — message types.
- `ChatModel`, `StreamingChatModel`, `ModelConfig`, `Provider`, `StreamChunk`, `StreamUsage` — model abstractions.
- `BudgetEnforcer`, `BudgetError`, `BudgetUtilization` — resource budget enforcement.
- `RvfManifest`, `RvfBridgeConfig`, `MountTable`, `GovernanceMode`, `RvfWitnessHeader`, `PolicyCheck` — RVF bridge (`rvf_bridge`).
- `AgiContainerBuilder`, `ParsedContainer`, `SegmentType`, `SkillDefinition`, `ToolDefinition` — AGI container building (`agi_container`).
- `SessionCrypto`, `EncryptionKey`, `generate_key`, `derive_key` — session encryption (`session_crypto`).
- `SystemPromptBuilder`, `BASE_AGENT_PROMPT` — system prompt construction.
- `RvAgentError`, `Result` — error taxonomy.

## License

Licensed under either MIT OR Apache-2.0.
