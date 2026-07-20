# ADR-262 — prompt_hub intake via REST-bridge (not MCP)

**Status:** Accepted · **Date:** 2026-06-17 · **Components:** `ui/ruvocal`, `prompt_hub/prompthub-server`
**Owner decision:** "Avoid MCP if/when possible. it is a token pit | is REST-Bridge the best alternative?" → **yes.**

## Context
Pass 2 (ADR-260, single-app unification) calls for **prompt_hub intake** in RuVocal — the `SwarmBundle → handoff.task.v1` flow plus prompt list/search/render. Two integration options:

1. **MCP** — wrap prompt_hub as an MCP server; the LLM calls it as a tool.
2. **REST-bridge** — RuVocal's SvelteKit server (`+server.ts`) proxies `prompthub-server`'s axum REST API directly, server-to-server.

`prompthub-server` is already a clean axum REST API on `127.0.0.1:8077` (verified routes: `/api/v1/prompts`, `/api/v1/prompts/{id}`, `/api/v1/prompts/{id}/render`, `/api/v1/prompts/search`, `/api/v1/swarm/bundle`, `/health`). Uniform envelope: `{ success, data?, error? }`.

## Decision
**Use the REST-bridge. Do not expose prompt_hub over MCP.**

A typed server-side client (`src/lib/server/prompthub/client.ts`) calls `prompthub-server` over HTTP. Thin SvelteKit proxy endpoints expose it to the UI:

| RuVocal endpoint | → prompthub-server |
|---|---|
| `GET /api/prompthub/prompts?q=&domain=&page=&per_page=` | `/api/v1/prompts` or `/api/v1/prompts/search` |
| `GET /api/prompthub/prompts/[id]` | `/api/v1/prompts/{id}` |
| `POST /api/prompthub/prompts/[id]/render` | `/api/v1/prompts/{id}/render` |
| `GET /api/prompthub/swarm/bundle` | `/api/v1/swarm/bundle` |
| `GET /api/prompthub/health` | `/health` |

Config: `PROMPTHUB_URL` (default `http://127.0.0.1:8077`; `8080` is taken by `sqld` on dev hosts). Runner: `scripts/run-prompthub-server.sh`.

## Rationale
- **MCP is a token pit for deterministic data ops.** An MCP tool round-trips through the model every call: schema in context, call tokens out, result tokens back in, model re-reasons. prompt rendering / bundle generation are **deterministic** — no reasoning needed — so the LLM mediation is pure waste.
- **REST-bridge keeps the model out of the loop.** Browser → RuVocal server → prompthub-server, all plain HTTP. The model only ever sees a *rendered prompt* when one is deliberately injected — never protocol chatter. **Zero LLM tokens** for fetch/render/bundle.
- **Matches the single-app endgame (ADR-260).** Both are Rust+Node in one workspace. A direct HTTP proxy is the natural seam now; when repos merge into one app, the proxy collapses into an in-process call. MCP would have to be torn back out.

## When MCP *would* still be right
If a future need requires the **model to autonomously decide** to fetch/select a prompt mid-reasoning (agentic tool-use), an MCP facade can be added *on top of* the same client — the REST-bridge does not preclude it. Default stays REST.

## Consequences
- RuVocal gains prompt_hub data access with no token cost and no new protocol surface.
- `prompthub-server` must be running (or `PROMPTHUB_URL` repointed); the health endpoint + client degrade gracefully (502/503, never a crash).
- The SwarmBundle → handoff.task.v1 wiring builds on `/api/prompthub/swarm/bundle` (next increment).

## References
- ADR-260 (single-app unification architecture)
- `scripts/run-ruvector-mcp.sh` (the MCP seam — correct for the *engine*, where the model DOES reason over results; contrast with this REST seam for *data*)
