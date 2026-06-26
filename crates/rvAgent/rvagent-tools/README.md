# rvagent-tools

Built-in agent tools (`ls`, `read`, `write`, `edit`, `glob`, `grep`, `execute`, `todos`, `task`) with enum-based dispatch.

## Overview

`rvagent-tools` provides the concrete tools an rvAgent can invoke during a run. Tools operate against the `Backend` abstraction (filesystem, state, or sandbox) rather than touching the filesystem directly, so the same tool works across backends. Built-in tools are dispatched through the `BuiltinTool` enum to avoid vtable indirection on hot paths (ADR-103 A6), while custom tools are supported through the `AnyTool::Dynamic` trait-object variant. The crate also provides parallel tool execution (ADR-103 A2).

## Key API

- `Tool` — core async tool trait (`name`, `description`, `parameters_schema`, `invoke`, `ainvoke`).
- `BuiltinTool` / `AnyTool` — enum dispatch for built-in tools plus a dynamic trait-object variant.
- `LsTool`, `ReadFileTool`, `WriteFileTool`, `EditFileTool`, `GlobTool`, `GrepTool`, `ExecuteTool`, `WriteTodosTool`, `TaskTool` — the nine built-in tools.
- `Backend`, `BackendRef` — backend abstraction tools call into.
- `ToolRuntime` — runtime context (backend, store, stream writer, config) passed to invocations.
- `ToolResult`, `StateUpdate`, `ToolCall`, `ToolParam<T>` — invocation result and request types.
- `builtin_tools()`, `resolve_builtin()`, `resolve_tool()`, `execute_tools_parallel()` — registry and execution helpers.
- `format_content_with_line_numbers()`, `is_image_file()` — formatting helpers.

## License

Licensed under either MIT OR Apache-2.0.
