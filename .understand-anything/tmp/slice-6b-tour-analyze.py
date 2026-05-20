#!/usr/bin/env python3
"""Analyze slice 6b graph and build a pedagogical tour for crates/rvAgent.

Layers have no explicit membership edges — we infer membership from filePath
prefixes (the rvAgent crate is organised one directory per sub-crate). For each
layer we pick the most structurally important nodes (lib.rs, Cargo.toml, plus
topic-specific module files) and emit an 11-step tour.
"""
import json
import sys
from collections import defaultdict


# Layer id -> list of path prefix predicates (relative to repo root).
LAYER_PATHS = {
    "layer:workspace-meta": [
        "crates/rvAgent/Cargo.toml",
        "crates/rvAgent/README",
        "crates/rvAgent/A7_OPTIMIZATION_REPORT",
        "crates/rvAgent/.ruv/",
    ],
    "layer:rvagent-core": ["crates/rvAgent/rvagent-core/"],
    "layer:rvagent-backends": ["crates/rvAgent/rvagent-backends/"],
    "layer:rvagent-tools": ["crates/rvAgent/rvagent-tools/"],
    "layer:rvagent-mcp": ["crates/rvAgent/rvagent-mcp/"],
    "layer:rvagent-middleware": ["crates/rvAgent/rvagent-middleware/"],
    "layer:rvagent-a2a": ["crates/rvAgent/rvagent-a2a/"],
    "layer:rvagent-acp": ["crates/rvAgent/rvagent-acp/"],
    "layer:rvagent-subagents": ["crates/rvAgent/rvagent-subagents/"],
    "layer:rvagent-cli": ["crates/rvAgent/rvagent-cli/"],
    "layer:rvagent-wasm": ["crates/rvAgent/rvagent-wasm/"],
}


def assign_layer(file_path: str) -> str | None:
    if not file_path:
        return None
    best = None
    best_len = -1
    for lid, prefixes in LAYER_PATHS.items():
        for p in prefixes:
            if file_path.startswith(p) or p in file_path:
                if len(p) > best_len:
                    best_len = len(p)
                    best = lid
                break
    return best


def main(in_path: str, out_path: str) -> int:
    with open(in_path) as f:
        data = json.load(f)

    nodes = data["nodes"]
    edges = data["edges"]
    layers = data["layers"]

    nodes_by_id = {n["id"]: n for n in nodes}

    node_layer = {}
    by_layer = defaultdict(list)
    for n in nodes:
        lid = assign_layer(n.get("filePath", "") or "")
        if lid:
            node_layer[n["id"]] = lid
            by_layer[lid].append(n)

    fan_in = defaultdict(int)
    fan_out = defaultdict(int)
    structural_types = {"imports", "uses", "calls", "implements"}
    for e in edges:
        if e.get("type") not in structural_types:
            continue
        s, t = e.get("source"), e.get("target")
        if s in nodes_by_id and t in nodes_by_id:
            fan_in[t] += 1
            fan_out[s] += 1

    def node_score(nid: str) -> int:
        return 2 * fan_in.get(nid, 0) + fan_out.get(nid, 0)

    def pick(layer_id, max_n=8, prefer_substrings=None, prefer_types=None,
             exclude_substrings=None, must_substrings=None):
        prefer_substrings = [s.lower() for s in (prefer_substrings or [])]
        prefer_types = set(prefer_types or [])
        exclude_substrings = [s.lower() for s in (exclude_substrings or [])]
        must_substrings = [s.lower() for s in (must_substrings or [])]
        items = by_layer.get(layer_id, [])
        scored = []
        for n in items:
            name = (n.get("name") or "").lower()
            fp = (n.get("filePath") or "").lower()
            ntype = n.get("type", "")
            if any(ex in fp or ex in name for ex in exclude_substrings):
                continue
            if must_substrings and not any(m in fp or m in name for m in must_substrings):
                continue
            pref_hit = 0
            for i, p in enumerate(prefer_substrings):
                if p in fp or p in name:
                    pref_hit = len(prefer_substrings) - i
                    break
            type_bonus = 2 if ntype in prefer_types else 0
            score = node_score(n["id"])
            depth = fp.count("/") if fp else 99
            scored.append((pref_hit, type_bonus, score, -depth, n["id"]))
        scored.sort(reverse=True)
        return [nid for *_, nid in scored[:max_n]]

    def find_by_substr(*needles):
        needles = [n.lower() for n in needles]
        out = []
        for n in nodes:
            fp = (n.get("filePath") or "").lower()
            nm = (n.get("name") or "").lower()
            if any(x in fp or x in nm for x in needles):
                out.append(n["id"])
        return out

    tour = []

    def add(step, title, desc, focus_node_ids, focus_layer_ids):
        seen = set()
        cleaned = []
        for nid in focus_node_ids:
            if nid in nodes_by_id and nid not in seen:
                cleaned.append(nid)
                seen.add(nid)
        tour.append({
            "step": step,
            "title": title,
            "description": desc,
            "focusNodeIds": cleaned,
            "focusLayerIds": focus_layer_ids,
        })

    # Step 1 — Workspace overview
    ws = []
    for cid in ("crate:rvAgent", "crate:rvagent"):
        if cid in nodes_by_id:
            ws.append(cid)
    ws += pick(
        "layer:workspace-meta", max_n=6,
        prefer_substrings=["readme", "cargo.toml", "a7_optimization", "rvagent-queen", "rvagent-coder"],
        prefer_types={"file", "crate", "config", "doc", "document"},
    )
    add(
        1,
        "What is rvAgent — workspace overview",
        "rvAgent is RuVector's production Rust agent runtime, a workspace of 10 sub-crates that compose into a full agent stack: core types, model backends, tools, MCP, middleware, A2A, ACP, sub-agent orchestration, a CLI, and a WASM runtime. Start at the top-level Cargo manifest and README to see how those pieces are declared and the persona specs (.ruv/agents) that ship with it.",
        ws[:6],
        ["layer:workspace-meta"],
    )

    # Step 2 — Agent core foundations
    core_focus = pick(
        "layer:rvagent-core", max_n=8,
        prefer_substrings=[
            "lib.rs", "agent_state", "agentstate", "message", "config",
            "chat_model", "chatmodel", "rvf", "trait",
        ],
        prefer_types={"file", "struct", "trait", "enum", "module"},
    )
    add(
        2,
        "Agent core foundations — AgentState, Messages, ChatModel, RVF",
        "rvagent-core is the contract layer every other crate builds on: AgentState carries conversation history, RvAgentConfig binds defaults, Messages are the chat-model wire format, the ChatModel trait abstracts LLM providers, and the RVF bridge wires agent state into RuVector's verifiable format. Read this before anything else — Steps 3-11 all consume these primitives.",
        core_focus,
        ["layer:rvagent-core"],
    )

    # Step 3 — Backends
    backend_focus = pick(
        "layer:rvagent-backends", max_n=8,
        prefer_substrings=[
            "lib.rs", "anthropic", "gemini", "filesystem", "local_shell",
            "sandbox", "client", "fs",
        ],
        prefer_types={"file", "struct", "trait", "module"},
    )
    add(
        3,
        "Backends — Anthropic/Gemini clients and sandboxes",
        "rvagent-backends implements the ChatModel trait from Step 2 for live LLM providers (Anthropic, Gemini) and provides the filesystem + local-shell sandboxes that tools execute against. These are the I/O frontier: every model call and every side-effect leaves the agent through here.",
        backend_focus,
        ["layer:rvagent-backends"],
    )

    # Step 4 — Tools
    tool_focus = pick(
        "layer:rvagent-tools", max_n=8,
        prefer_substrings=[
            "lib.rs", "tool.rs", "builtin", "anytool", "registry",
            "read", "write", "edit", "shell", "search",
        ],
        prefer_types={"file", "struct", "trait", "enum", "module"},
    )
    add(
        4,
        "Tools — Tool trait, BuiltinTool dispatch, 9 implementors",
        "rvagent-tools defines the Tool trait and the BuiltinTool/AnyTool dispatch surface that the agent uses to call out to the world. Nine concrete tool implementors (read/write/edit files, shell, search, etc.) build on the backend sandboxes from Step 3 and produce structured results that flow back into AgentState.",
        tool_focus,
        ["layer:rvagent-tools"],
    )

    # Step 5 — MCP
    mcp_focus = pick(
        "layer:rvagent-mcp", max_n=8,
        prefer_substrings=[
            "lib.rs", "client", "server", "sse", "stdio", "axum",
            "middleware", "transport", "mcp",
        ],
        prefer_types={"file", "struct", "trait", "module", "function", "fn"},
    )
    add(
        5,
        "MCP — Model Context Protocol client and server",
        "rvagent-mcp is rvAgent's Model Context Protocol surface: an McpClient that talks to external MCP servers and a built-in axum-based MCP server with SSE and stdio transports, including middleware hooks. This is how rvAgent both consumes external tools and exposes its own tools to other agents.",
        mcp_focus,
        ["layer:rvagent-mcp"],
    )

    # Step 6 — Middleware
    mw_focus = pick(
        "layer:rvagent-middleware", max_n=8,
        prefer_substrings=[
            "lib.rs", "middleware", "hitl", "retry", "summariz",
            "witness", "guard", "limit", "log",
        ],
        prefer_types={"file", "struct", "trait", "enum", "module"},
    )
    add(
        6,
        "Middleware stack — 15 variants from HITL to witness chains",
        "rvagent-middleware wraps chat-model calls and tool invocations with 15 composable variants: human-in-the-loop approvals, retry/backoff, conversation summarization, witness-chain attestation, rate limits, logging, and more. Each middleware sees the AgentState from Step 2 and can short-circuit or transform requests before they reach a backend.",
        mw_focus,
        ["layer:rvagent-middleware"],
    )

    # Step 7 — A2A
    a2a_focus = pick(
        "layer:rvagent-a2a", max_n=8,
        prefer_substrings=[
            "lib.rs", "server", "routing", "push", "webhook", "sse",
            "budget_ledger", "agent2agent", "a2a",
        ],
        prefer_types={"file", "struct", "trait", "module", "function", "fn"},
    )
    add(
        7,
        "A2A — Agent-to-Agent protocol (ADR-159)",
        "rvagent-a2a implements ADR-159 Agent2Agent: an A2aServer with task routing, push webhooks, SSE streams, and a budget ledger that bounds spend across collaborating agents. It transports the same AgentState/Message types from Step 2 between independent rvAgent instances.",
        a2a_focus,
        ["layer:rvagent-a2a"],
    )

    # Step 8 — ACP
    acp_focus = pick(
        "layer:rvagent-acp", max_n=8,
        prefer_substrings=["lib.rs", "agent", "server", "acp", "session", "client"],
        prefer_types={"file", "struct", "trait", "module", "function", "fn"},
    )
    add(
        8,
        "ACP — Agent Connect Protocol",
        "rvagent-acp wraps an rvAgent in the Agent Connect Protocol: AcpAgent presents the canonical agent interface and AcpServer hosts it. ACP is the cross-runtime contract — it lets non-rvAgent clients drive an agent without speaking A2A or MCP directly.",
        acp_focus,
        ["layer:rvagent-acp"],
    )

    # Step 9 — Sub-agents + orchestration
    sub_focus = pick(
        "layer:rvagent-subagents", max_n=8,
        prefer_substrings=[
            "lib.rs", "builder", "orchestrator", "crdt", "merge",
            "subagent", "spawn", "compose",
        ],
        prefer_types={"file", "struct", "trait", "module", "function", "fn"},
    )
    add(
        9,
        "Sub-agents and orchestration — builder, CRDT merge, orchestrator",
        "rvagent-subagents is where one agent spawns and coordinates many: a fluent builder constructs child agents, an orchestrator schedules them, and CRDT merge logic reconciles their AgentStates back into the parent. This is the hierarchical-mesh topology from CLAUDE.md, expressed in code.",
        sub_focus,
        ["layer:rvagent-subagents"],
    )

    # Step 10 — CLI
    cli_focus = pick(
        "layer:rvagent-cli", max_n=8,
        prefer_substrings=[
            "main.rs", "lib.rs", "tui", "command", "subcommand",
            "clap", "rvagent",
        ],
        prefer_types={"file", "struct", "module", "function", "fn"},
    )
    add(
        10,
        "CLI — rvagent binary with TUI",
        "rvagent-cli is the operator-facing entry point: a clap-driven binary with a TUI for live interaction. The subcommands map onto the orchestrator, tools, and backends from prior steps, so reading the command modules is the fastest way to see how a real rvAgent session is wired end-to-end.",
        cli_focus,
        ["layer:rvagent-cli"],
    )

    # Step 11 — WASM agent runtime
    wasm_focus = pick(
        "layer:rvagent-wasm", max_n=8,
        prefer_substrings=[
            "lib.rs", "wasm_agent", "wasm_mcp", "gallery", "rvf",
            "builder", "export", "cdylib",
        ],
        prefer_types={"file", "struct", "trait", "module", "function", "fn"},
    )
    add(
        11,
        "WASM agent runtime — WasmAgent, WasmMcpServer, RVF builder",
        "rvagent-wasm compiles the agent runtime as a cdylib for sandboxed embedding: WasmAgent runs the core loop, WasmMcpServer exposes MCP from inside WASM, the gallery catalogs prebuilt agents, and the RVF builder serializes agent state into RuVector's verifiable format. This is the capstone — every prior layer reappears here, but constrained to a sandbox.",
        wasm_focus,
        ["layer:rvagent-wasm"],
    )

    out = dict(data)
    out["tour"] = tour
    with open(out_path, "w") as f:
        json.dump(out, f, indent=2)

    diag = {
        "steps": len(tour),
        "stepSummary": [
            {
                "step": s["step"],
                "title": s["title"],
                "focusCount": len(s["focusNodeIds"]),
                "layers": s["focusLayerIds"],
            }
            for s in tour
        ],
        "emptySteps": [s["step"] for s in tour if not s["focusNodeIds"]],
        "layerNodeCounts": {lid: len(by_layer.get(lid, [])) for lid in [l["id"] for l in layers]},
        "unassignedNodes": sum(1 for n in nodes if n["id"] not in node_layer),
    }
    print(json.dumps(diag, indent=2))
    return 0


if __name__ == "__main__":
    if len(sys.argv) != 3:
        print("usage: slice-6b-tour-analyze.py <in.json> <out.json>", file=sys.stderr)
        sys.exit(1)
    sys.exit(main(sys.argv[1], sys.argv[2]))
