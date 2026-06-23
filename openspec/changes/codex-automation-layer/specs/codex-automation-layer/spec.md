## ADDED Requirements

### Requirement: Codex automation graph generation
The Codex environment generator SHALL compile `.claude` source material into a deterministic repo-local automation graph under `.codex` that summarizes automation intent without requiring consumers to load every mirrored source file.

#### Scenario: Generate graph from source truth
- **GIVEN** a repository with tracked `.claude` agents, commands, hooks, skills, settings, and Rusty IDD adapter material
- **WHEN** `codex-env mirror` runs
- **THEN** `.codex/automation-graph.json` records source counts, hook events, command route counts, skill counts, MCP declarations, Rusty IDD integration, and generated agent-team membership

### Requirement: Agent teams derive from configured roles
The Codex environment generator SHALL derive non-empty agent teams from configured Codex-native agent roles rather than emitting empty or schema-ambiguous teams.

#### Scenario: Source agents produce team members
- **GIVEN** `.claude/agents` contains source agent definitions that generate Codex agent TOML files
- **WHEN** `codex-env mirror` runs
- **THEN** `.codex/agent-teams.json` contains required teams with non-empty `agents` and `members` arrays that reference configured agents

### Requirement: Repo-local prompt ownership
The Codex environment generator SHALL keep generated prompt commands repo-local unless an explicit future source-truth requirement says otherwise.

#### Scenario: Install prompts does not write user global prompts
- **GIVEN** generated prompt files exist under `.codex/prompts`
- **WHEN** `codex-env install-prompts` runs with a separate `--codex-home`
- **THEN** prompt files remain in the repository `.codex/prompts` tree and are not copied into `<codex-home>/prompts`

### Requirement: Runtime hooks avoid stale absolute workspace paths
Generated Codex runtime hook scripts SHALL avoid stale absolute `/workspaces/ruvector` paths while preserving raw `.claude` files as evidence in `.codex/mirror`.

#### Scenario: Hook script normalization
- **GIVEN** a `.claude/hooks/*.sh` file contains `/workspaces/ruvector`
- **WHEN** `codex-env mirror` generates `.codex/hooks/*.sh`
- **THEN** the generated runtime hook script resolves the repository root dynamically and contains no `/workspaces/ruvector` literal
- **AND** the raw mirror file under `.codex/mirror/.claude/hooks` preserves the original source bytes
