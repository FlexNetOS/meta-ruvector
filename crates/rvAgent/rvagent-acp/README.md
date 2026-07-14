# rvagent-acp

Agent Communication Protocol (ACP) server for rvAgent — an axum HTTP server with API-key auth, rate limiting, body-size limits, and TLS enforcement.

## Overview

`rvagent-acp` exposes an rvAgent over HTTP using the Agent Communication Protocol, per ADR-099 and ADR-103 C6. It wires the rvAgent stack (`core`, `backends`, `middleware`, `tools`, `subagents`) behind an axum router that manages sessions and dispatches prompt requests to the agent, returning structured responses. The server hardens the endpoint with layered tower middleware: API-key authentication, request rate limiting, maximum-body-size enforcement, and optional TLS-required gating. The crate ships an `rvagent-acp` binary.

## Key API

- `AcpServer`, `AcpConfig`, `AppState` — server, configuration, and shared state (`server`).
- `AcpAgent`, `Session` — agent and session handling (`agent`).
- `require_api_key`, `rate_limiter`, `request_size_limit`, `require_tls_middleware` — tower middleware layers (`auth`).
- `ApiKeyState`, `RateLimiterState`, `MaxBodySize`, `RequireTls` — middleware state/config types.
- `PromptRequest`, `PromptResponse`, `ResponseMessage`, `ContentBlock`, `SessionInfo`, `CreateSessionRequest`, `HealthResponse`, `ErrorResponse` — protocol types (`types`).

## License

Licensed under either MIT OR Apache-2.0.
