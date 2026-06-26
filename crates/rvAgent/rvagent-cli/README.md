# rvagent-cli

Terminal coding agent for rvAgent — the `rvagent` binary with an interactive TUI, session management, MCP tools, and A2A protocol commands.

## Overview

`rvagent-cli` is the command-line entry point that ties the rvAgent framework together into a usable coding agent. It depends on `core`, `backends`, `middleware`, `tools`, `subagents`, and `a2a`, and provides an interactive `ratatui`/`crossterm` TUI, a non-interactive single-prompt mode, persistent session management, MCP tool integration, and Agent-to-Agent operations. The default model is `anthropic:claude-sonnet-4-20250514`, and the API key is loaded from `.env` / `.env.local` (e.g. `ANTHROPIC_API_KEY`).

## Key API

This crate produces the `rvagent` binary (`src/main.rs`); it is not consumed as a library. Subcommands:

- `chat` — start an interactive agent session (TUI).
- `run <prompt>` — run a single prompt and exit.
- `session <action>` — list and manage sessions.
- `a2a <...>` — Agent-to-Agent protocol operations (serve, discover, send-task).

Top-level flags: `--model`, `--directory/-d`, `--resume <id>`, `--prompt/-p`.

Internal modules: `app` (agent loop), `tui` / `display` (terminal UI), `session` (session persistence), `mcp` (MCP integration), `a2a` (A2A subcommand).

## License

Licensed under either MIT OR Apache-2.0.
