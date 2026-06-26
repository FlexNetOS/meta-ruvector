# rvagent-mcp

Model Context Protocol (MCP) integration for rvAgent — protocol types, tool/resource registries, transports, server, and client.

## Overview

`rvagent-mcp` is a complete MCP implementation for the rvAgent framework. It exposes rvAgent tools and resources to MCP clients (server side) and connects to external MCP servers (client side), speaking JSON-RPC 2.0 over stdio, in-memory, or SSE transports. It includes a thread-safe tool registry with handler dispatch, resource providers (static/file/template), topology strategies for multi-agent routing, and a skills-format bridge for Claude Code and Codex. The crate ships both a library and an `rvagent-mcp` binary.

## Key API

- `McpServer`, `McpServerConfig` — MCP server routing requests to tools/resources.
- `McpClient` — client for connecting to external MCP servers.
- `McpToolRegistry`, `McpToolDefinition`, `McpToolHandler` — tool registry and dispatch.
- `ResourceProvider`, `ResourceRegistry` — resource providers and registry.
- `Transport`, `StdioTransport`, `MemoryTransport`, `SseTransport`, `SseConfig`, `TransportConfig`, `TransportType` — transport abstraction.
- `JsonRpcRequest`, `JsonRpcResponse`, `JsonRpcError`, `McpMethod`, `McpTool`, `McpResource`, `McpResourceTemplate`, `McpPrompt`, `Content`, `ServerCapabilities` — protocol types.
- `ToolGroup`, `ToolFilter` — tool grouping/filtering.
- `TopologyRouter`, `TopologyConfig`, `TopologyNode`, `TopologyType`, `NodeRole`, `NodeStatus`, `ConsensusType` — multi-agent topology routing.
- `McpError`, `Result` — error taxonomy.

## License

Licensed under either MIT OR Apache-2.0.
