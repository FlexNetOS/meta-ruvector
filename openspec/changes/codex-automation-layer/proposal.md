# codex-automation-layer

## Why

The tracked `.claude` surface contains the project automation source material, but the generated `.codex` surface is still too close to a token-heavy file mirror. That makes Codex pay for skill/prompt overload, lets MCP and hook compatibility rot hide in generated files, and leaves team orchestration ambiguous. The Rust-owned `crates/codex-env` harness must compile `.claude` into deterministic, lower-token automation metadata that the project crates can consume.

## What Changes

- Treat `.codex` as the Rust automation extraction frontier, not a vendor harness or a global prompt installer.
- Generate a deterministic `.codex/automation-graph.json` summarizing source assets, teams, hooks, command routes, skills, MCP, and Rusty IDD integration.
- Generate agent teams from actual available Codex agent roles and expose both `agents` and `members` for compatibility with consumers that count team members.
- Normalize generated Codex hook scripts away from stale absolute `/workspaces/ruvector` paths while preserving the raw byte-for-byte `.claude` mirror for evidence.
- Enforce the ownership path and repo-local prompt behavior with tests and doctor validation.

## Capabilities

### New Capabilities
- `codex-automation-layer`: Rust-native extraction and validation of `.claude` automation semantics into `.codex` and crate-owned metadata.

### Modified Capabilities
- None.

## Impact

- `crates/codex-env/src/**`
- `crates/codex-env/tests/**`
- generated `.codex/**` and `.agents/skills/codex-*`
- Rusty IDD change tracking under `openspec/changes/codex-automation-layer`
