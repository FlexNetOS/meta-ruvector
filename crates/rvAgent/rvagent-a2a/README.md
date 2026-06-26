# rvagent-a2a

Agent2Agent (A2A) peer-to-peer protocol for rvAgent — the Google A2A spec plus ruvector identity, policy, budget, and trace-causality extensions (ADR-159).

## Overview

`rvagent-a2a` implements agent-to-agent interoperability over the Google A2A spec: JSON-RPC 2.0 over HTTP, `/.well-known/agent.json` discovery, `text/event-stream` streaming, and HMAC-signed push webhooks. On top of the baseline it adds the ruvector extensions from ADR-159: signed `AgentCard`s with content-addressed IDs (r2), per-task policy / pluggable peer selection / typed artifacts including zero-copy vector handoff (r2), and a global dispatch budget, `TaskContext` trace propagation, and a recursion/cycle guard (r3). It is a library — consumers mount `server::A2aServer` into their own axum binary, typically alongside `rvagent-acp`.

## Key API

- `server::A2aServer` — mountable axum A2A server.
- `client` — A2A client for peer discovery and task dispatch.
- `AgentCard`, `AgentCapabilities`, `AgentProvider`, `AgentSkill`, `AuthScheme` — agent discovery types.
- `Task`, `TaskSpec`, `TaskState`, `TaskStatus`, `TaskStatusUpdateEvent`, `TaskArtifactUpdateEvent` — task lifecycle types.
- `Message`, `Part`, `Role`, `Artifact`, `FileContent` — message and artifact types.
- Modules: `identity`, `policy`, `routing`, `budget`, `context`, `recursion_guard`, `artifact_types`, `executor`, `config`.
- `A2aError` — error taxonomy; `VERSION` — crate version constant.

## Features

- `ed25519-webhooks` (off by default) — additionally sign push webhooks with an Ed25519 keypair, emitted alongside the baseline HMAC signature under `X-A2A-Signature-Ed25519`.

## License

Licensed under either MIT OR Apache-2.0.
