# rvagent-middleware

Composable request/response middleware for the rvAgent pipeline — todolist, filesystem, subagents, summarization, memory, skills, prompt caching, HITL, witness, and tool sanitizer.

## Overview

`rvagent-middleware` implements the `Middleware` trait and `MiddlewarePipeline` that compose agent behaviour around each model call, following the DeepAgents architecture (ADR-095, ADR-103). Each middleware hooks into `before_agent`, `modify_request`, `wrap_model_call`, and tool injection, and the pipeline chains them so the outermost wraps the innermost. `build_default_pipeline` assembles the standard ordering, with optional SONA adaptive learning, HNSW semantic retrieval, and Unicode-security stages.

## Key API

- `Middleware` — core async middleware trait with default no-op hooks.
- `MiddlewarePipeline` — ordered pipeline executor (`run`, `run_wrap_model_call`, `collect_tools`, `run_before_agent`).
- `build_default_pipeline()`, `PipelineConfig` — default pipeline assembly and configuration.
- `ModelRequest`, `ModelResponse`, `ModelHandler`, `AsyncModelHandler` — model-call types and handler traits.
- `Message`, `Role`, `ToolCall`, `AgentState`, `AgentStateUpdate`, `TodoItem`, `Usage`, `ToolDefinition`, `Tool` — shared pipeline types.
- Middleware modules: `todolist`, `filesystem`, `subagents`, `summarization`, `memory`, `skills`, `prompt_caching`, `patch_tool_calls`, `hitl`, `witness`, `tool_sanitizer`, `retry`, `mcp_bridge`, `rvf_manifest`, `sona`, `hnsw`.
- `UnicodeSecurityMiddleware`, `UnicodeSecurityChecker`, `UnicodeSecurityConfig`, `UnicodeIssue` — Unicode-security stage.

## Features

- `default` — no optional features.
- `sona` — enable SONA adaptive-learning middleware (pulls in `ruvector-sona`).
- `hnsw` — enable HNSW semantic-retrieval middleware.

## License

Licensed under either MIT OR Apache-2.0.
