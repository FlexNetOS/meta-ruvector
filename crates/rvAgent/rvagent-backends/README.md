# rvagent-backends

Backend implementations for rvAgent — filesystem, shell, composite, in-memory state, persistent store, and sandbox protocols, plus model clients.

## Overview

`rvagent-backends` provides the concrete `Backend` and `SandboxBackend` implementations that rvAgent tools execute against, following ADR-094 (Backend Protocol & Trait System) and ADR-103. Backends range from an ephemeral in-memory store to security-hardened local disk access and shell execution, with a composite backend that routes by path prefix. Security hardening includes path-traversal protection, environment sanitization, Unicode security detection, and literal grep mode. The crate also bundles chat-model clients for Anthropic (Claude) and Google Gemini.

## Key API

- `Backend`, `SandboxBackend` — backend traits (`protocol`).
- `StateBackend` — ephemeral in-memory file store.
- `FilesystemBackend` — local disk with security hardening.
- `LocalShellBackend`, `LocalShellConfig`, `CommandAllowlist` — filesystem + shell execution.
- `CompositeBackend`, `BackendRef` — path-prefix routing to sub-backends.
- `StoreBackend` — persistent key-value storage.
- `BaseSandbox`, `LocalSandbox`, `SandboxConfig`, `SandboxError` — sandbox protocols.
- `AnthropicClient`, `GeminiClient` — chat-model clients.
- `EditResult`, `ExecuteResponse`, `FileInfo`, `GrepMatch`, `WriteResult`, `FileOperationError`, `MountedToolInfo` — operation types.

## License

Licensed under either MIT OR Apache-2.0.
