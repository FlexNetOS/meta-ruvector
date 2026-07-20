# mcp-gate

MCP (Model Context Protocol) server for the Anytime-Valid Coherence Gate.

`mcp-gate` exposes the coherence gate (backed by `cognitum-gate-tilezero`) over
the Model Context Protocol so AI agents and tool orchestrators can request
permission for actions, retrieve witness receipts, and replay past decisions for
audit. It ships both a library and a `mcp-gate` binary that speaks **JSON-RPC
2.0 over stdio**.

## Tools

The server exposes three tools:

| Tool | Purpose |
|------|---------|
| `permit_action` | Request permission for an action. Returns a `PermitToken` for permitted actions, escalation info for deferred actions, or denial details. |
| `get_receipt` | Retrieve a cryptographically signed witness receipt by sequence number for audit. |
| `replay_decision` | Deterministically replay a past decision for audit and verification (optionally verifying the hash-chain integrity). |

## Run the server

```bash
cargo run -p mcp-gate
# speaks JSON-RPC 2.0 on stdin/stdout; logs go to stderr
```

Thresholds are configurable via environment variables:
`MCP_GATE_TAU_DENY`, `MCP_GATE_TAU_PERMIT`, `MCP_GATE_MIN_CUT`,
`MCP_GATE_MAX_SHIFT`, `MCP_GATE_PERMIT_TTL_NS`.

## Library usage

```rust
use mcp_gate::McpGateServer;

#[tokio::main]
async fn main() {
    let server = McpGateServer::new();
    server.run_stdio().await.expect("Server failed");
}
```

## Protocol

JSON-RPC 2.0 over stdio. Example `tools/call` request:

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "tools/call",
  "params": {
    "name": "permit_action",
    "arguments": {
      "action_id": "cfg-push-7a3f",
      "action_type": "config_change",
      "target": {
        "device": "router-west-03",
        "path": "/network/interfaces/eth0"
      }
    }
  }
}
```

## Public API

- `McpGateServer`, `McpGateConfig`, `ServerCapabilities`, `ServerInfo`
- `McpGateTools`, `McpError`
- Re-exports from `cognitum-gate-tilezero`: `ActionContext`, `ActionMetadata`,
  `ActionTarget`, `EscalationInfo`, `GateDecision`, `GateThresholds`,
  `PermitToken`, `TileZero`, `WitnessReceipt`
