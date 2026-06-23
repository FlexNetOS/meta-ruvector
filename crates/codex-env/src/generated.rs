use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde_json::{json, Map};
use walkdir::WalkDir;

use super::agent_roles::CodexAgentRole;
use super::{
    escape_toml_string, first_heading, is_executable, is_ignored_path, normalize_generated_bytes,
    normalize_generated_text, strip_leading_frontmatter, yaml_scalar, PlannedFile,
};

#[derive(Debug, Clone, Copy)]
struct CodexAgentTeam {
    name: &'static str,
    description: &'static str,
    strategy: &'static str,
    parallel: bool,
    consolidation_owner: &'static str,
    preferred_agents: &'static [&'static str],
    selection_keywords: &'static [&'static str],
    min_agents: usize,
    max_agents: usize,
}

#[derive(Debug, Clone)]
struct CodexAgentTeamPlan {
    name: &'static str,
    description: &'static str,
    strategy: &'static str,
    parallel: bool,
    consolidation_owner: &'static str,
    agents: Vec<String>,
}

fn codex_agent_teams() -> &'static [CodexAgentTeam] {
    &[
        CodexAgentTeam {
            name: "core",
            description: "Plan, research, implement, test, and review broad repo work.",
            strategy: "parallel-evidence-then-parent-implementation",
            parallel: true,
            consolidation_owner: "parent",
            preferred_agents: &[
                "claude-core-planner",
                "claude-core-researcher",
                "claude-core-coder",
                "claude-core-tester",
                "claude-core-reviewer",
            ],
            selection_keywords: &[
                "core",
                "planner",
                "researcher",
                "coder",
                "tester",
                "reviewer",
            ],
            min_agents: 3,
            max_agents: 6,
        },
        CodexAgentTeam {
            name: "review",
            description:
                "Find correctness, production, security, and regression risks before shipping.",
            strategy: "parallel-review-then-parent-remediation",
            parallel: true,
            consolidation_owner: "parent",
            preferred_agents: &[
                "reviewer",
                "claude-core-reviewer",
                "claude-testing-production-validator",
                "claude-v3-security-auditor",
            ],
            selection_keywords: &[
                "review",
                "reviewer",
                "validator",
                "test",
                "security",
                "audit",
            ],
            min_agents: 3,
            max_agents: 6,
        },
        CodexAgentTeam {
            name: "rust",
            description: "Trace Rust code paths, implement Rust changes, test, and optimize.",
            strategy: "parallel-rust-research-then-parent-patch",
            parallel: true,
            consolidation_owner: "parent",
            preferred_agents: &[
                "explorer",
                "claude-core-coder",
                "claude-core-tester",
                "claude-v3-performance-engineer",
            ],
            selection_keywords: &[
                "rust",
                "coder",
                "tester",
                "performance",
                "optimizer",
                "backend",
            ],
            min_agents: 3,
            max_agents: 6,
        },
        CodexAgentTeam {
            name: "security",
            description:
                "Review architecture, audit implementation, inspect PII, and harden defenses.",
            strategy: "parallel-security-analysis-then-parent-remediation",
            parallel: true,
            consolidation_owner: "parent",
            preferred_agents: &[
                "claude-v3-security-architect",
                "claude-v3-security-auditor",
                "claude-v3-pii-detector",
                "claude-v3-aidefence-guardian",
            ],
            selection_keywords: &[
                "security", "auditor", "pii", "defence", "defense", "guardian",
            ],
            min_agents: 2,
            max_agents: 6,
        },
        CodexAgentTeam {
            name: "github",
            description:
                "Prepare PRs, review GitHub feedback, and keep repository automation aligned.",
            strategy: "parallel-github-coordination-then-parent-publish",
            parallel: true,
            consolidation_owner: "parent",
            preferred_agents: &[
                "claude-github-pr-manager",
                "claude-github-code-review-swarm",
                "claude-github-workflow-automation",
            ],
            selection_keywords: &["github", "pr", "release", "workflow", "issue"],
            min_agents: 2,
            max_agents: 5,
        },
        CodexAgentTeam {
            name: "swarm",
            description:
                "Coordinate larger multi-agent efforts with hierarchical and hive-mind controllers.",
            strategy: "parallel-swarm-coordination-then-parent-decision",
            parallel: true,
            consolidation_owner: "parent",
            preferred_agents: &[
                "claude-swarm-hierarchical-coordinator",
                "claude-hive-mind-queen-coordinator",
                "claude-v3-v3-queen-coordinator",
            ],
            selection_keywords: &["swarm", "hive", "queen", "coordinator", "planner"],
            min_agents: 2,
            max_agents: 5,
        },
    ]
}

fn available_agent_names(agent_roles: &[CodexAgentRole]) -> BTreeSet<String> {
    let mut names = ["explorer", "reviewer", "docs_researcher"]
        .into_iter()
        .map(ToOwned::to_owned)
        .collect::<BTreeSet<_>>();
    names.extend(agent_roles.iter().map(|role| role.role_name.clone()));
    names
}

fn fallback_agent_team_members(available: &BTreeSet<String>) -> Vec<String> {
    ["explorer", "reviewer", "docs_researcher"]
        .into_iter()
        .filter(|agent| available.contains(*agent))
        .map(ToOwned::to_owned)
        .collect()
}

fn codex_agent_team_plan(agent_roles: &[CodexAgentRole]) -> Vec<CodexAgentTeamPlan> {
    let available = available_agent_names(agent_roles);
    let fallback = fallback_agent_team_members(&available);
    codex_agent_teams()
        .iter()
        .map(|team| {
            let mut agents = team
                .preferred_agents
                .iter()
                .filter(|agent| available.contains(**agent))
                .map(|agent| (*agent).to_owned())
                .collect::<Vec<_>>();

            for role in agent_roles {
                if agents.len() >= team.max_agents {
                    break;
                }
                let haystack =
                    format!("{} {}", role.role_name, role.description).to_ascii_lowercase();
                if team
                    .selection_keywords
                    .iter()
                    .any(|keyword| haystack.contains(keyword))
                    && !agents.contains(&role.role_name)
                    && available.contains(&role.role_name)
                {
                    agents.push(role.role_name.clone());
                }
            }

            if agents.len() < team.min_agents {
                for agent in &fallback {
                    if agents.len() >= team.min_agents {
                        break;
                    }
                    if !agents.contains(agent) {
                        agents.push(agent.clone());
                    }
                }
            }
            if agents.is_empty() {
                agents = fallback.clone();
            }
            CodexAgentTeamPlan {
                name: team.name,
                description: team.description,
                strategy: team.strategy,
                parallel: team.parallel,
                consolidation_owner: team.consolidation_owner,
                agents,
            }
        })
        .collect()
}

fn codex_agent_team_markdown(agent_roles: &[CodexAgentRole]) -> String {
    codex_agent_team_plan(agent_roles)
        .iter()
        .map(|team| format!("- {}: {}", team.name, team.agents.join(", ")))
        .collect::<Vec<_>>()
        .join("\n")
}

pub(super) fn codex_agent_teams_json(
    codex_dir: &Path,
    agent_roles: &[CodexAgentRole],
) -> Result<PlannedFile> {
    let teams = codex_agent_team_plan(agent_roles)
        .iter()
        .map(|team| {
            json!({
                "name": team.name,
                "description": team.description,
                "strategy": team.strategy,
                "parallel": team.parallel,
                "consolidationOwner": team.consolidation_owner,
                "agents": team.agents,
                "members": team.agents,
                "source": "codex-env automation extraction",
            })
        })
        .collect::<Vec<_>>();
    let output = json!({
        "schemaVersion": 1,
        "generatedBy": "codex-env",
        "teams": teams,
    });
    let mut text = serde_json::to_string_pretty(&output)?;
    text.push('\n');
    Ok(PlannedFile {
        path: codex_dir.join("agent-teams.json"),
        bytes: text.into_bytes(),
        executable: false,
    })
}

pub(super) fn codex_automation_graph_json(
    codex_dir: &Path,
    claude_dir: &Path,
    claude_files: &[PathBuf],
    agent_roles: &[CodexAgentRole],
) -> Result<PlannedFile> {
    let mut source_counts = BTreeMap::new();
    for file in claude_files {
        *source_counts
            .entry(automation_source_kind(file).to_owned())
            .or_insert(0usize) += 1;
    }

    let settings_path = claude_dir.join("settings.json");
    let settings: serde_json::Value = serde_json::from_slice(
        &fs::read(&settings_path)
            .with_context(|| format!("failed to read {}", settings_path.display()))?,
    )?;
    let mut hook_events = settings
        .get("hooks")
        .and_then(serde_json::Value::as_object)
        .map(|hooks| hooks.keys().cloned().collect::<Vec<_>>())
        .unwrap_or_default();
    hook_events.sort();
    let mut env_keys = settings
        .get("env")
        .and_then(serde_json::Value::as_object)
        .map(|env| env.keys().cloned().collect::<Vec<_>>())
        .unwrap_or_default();
    env_keys.sort();

    let teams = codex_agent_team_plan(agent_roles)
        .into_iter()
        .map(|team| {
            json!({
                "name": team.name,
                "description": team.description,
                "strategy": team.strategy,
                "parallel": team.parallel,
                "consolidationOwner": team.consolidation_owner,
                "members": team.agents,
            })
        })
        .collect::<Vec<_>>();

    let team_member_references = teams
        .iter()
        .filter_map(|team| team.get("members").and_then(serde_json::Value::as_array))
        .map(Vec::len)
        .sum::<usize>();
    let source_agent_count = *source_counts.get("agent").unwrap_or(&0);
    let team_coverage_percent = if source_agent_count == 0 {
        100.0
    } else {
        (team_member_references as f64 / source_agent_count as f64) * 100.0
    };

    let output = json!({
        "schemaVersion": 1,
        "generatedBy": "codex-env",
        "sourceRoot": ".claude",
        "rawMirrorRoot": ".codex/mirror/.claude",
        "runtimeRoot": ".codex",
        "ownership": {
            "sourceMaterial": ".claude",
            "rawEvidence": ".codex/mirror/.claude",
            "rustExtractor": "crates/codex-env",
            "runtimeTarget": "crate-owned Codex automation",
            "notTarget": "vendor harness"
        },
        "sourceCounts": source_counts,
        "tokenLoadControls": {
            "primaryIndex": ".codex/automation-graph.json",
            "rawMirrorPolicy": "evidence-only; do not load bulk mirrored source unless needed",
            "skillActivationPolicy": "on-demand by task, not all skills by default",
            "teamCoveragePercent": team_coverage_percent
        },
        "agents": {
            "configuredCount": agent_roles.len(),
            "rolesPath": ".codex/agents",
            "teamsPath": ".codex/agent-teams.json",
            "teams": teams
        },
        "commands": {
            "sourceRoot": ".claude/commands",
            "promptRoot": ".codex/prompts",
            "routeCount": source_counts.get("command").copied().unwrap_or(0)
        },
        "skills": {
            "sourceRoot": ".claude/skills",
            "generatedRoot": ".agents/skills",
            "activation": "on-demand",
            "sourceCount": source_counts.get("skill").copied().unwrap_or(0)
        },
        "hooks": {
            "source": ".claude/settings.json",
            "runtime": ".codex/hooks.json",
            "scriptRoot": ".codex/hooks",
            "events": hook_events,
            "absoluteWorkspacePathPolicy": "generated runtime hooks must resolve repo root dynamically"
        },
        "mcp": {
            "source": ".claude/settings.json and .codex/config.toml",
            "runtime": ".codex/config.toml",
            "policy": "declare only runtime MCP servers needed by Codex; do not treat MCP sprawl as success"
        },
        "rustyIdd": {
            "sessionStartHook": true,
            "adapterSource": ".claude/rusty-idd-adapter.md",
            "adapterRuntime": ".codex/rusty-idd-adapter.md"
        },
        "environment": {
            "envKeys": env_keys
        }
    });
    let mut text = serde_json::to_string_pretty(&output)?;
    text.push('\n');
    Ok(PlannedFile {
        path: codex_dir.join("automation-graph.json"),
        bytes: text.into_bytes(),
        executable: false,
    })
}

fn automation_source_kind(relative_source: &Path) -> &'static str {
    let path = relative_source.to_string_lossy();
    if path == ".claude/settings.json" || path.ends_with("/settings.json") {
        "setting"
    } else if path.starts_with(".claude/agents/") {
        "agent"
    } else if path.starts_with(".claude/commands/") {
        "command"
    } else if path.starts_with(".claude/hooks/") {
        "hook"
    } else if path.starts_with(".claude/helpers/") {
        "helper"
    } else if path.starts_with(".claude/skills/") {
        "skill"
    } else if path.starts_with(".claude/intelligence/") {
        "intelligence"
    } else {
        "other"
    }
}

pub(super) fn read_claude_env(claude_dir: &Path) -> Result<BTreeMap<String, String>> {
    let settings_path = claude_dir.join("settings.json");
    let settings: serde_json::Value = serde_json::from_slice(
        &fs::read(&settings_path)
            .with_context(|| format!("failed to read {}", settings_path.display()))?,
    )
    .with_context(|| format!("failed to parse {}", settings_path.display()))?;

    let mut env = BTreeMap::new();
    if let Some(object) = settings.get("env").and_then(serde_json::Value::as_object) {
        for (key, value) in object {
            if let Some(value) = value.as_str() {
                env.insert(key.clone(), value.to_owned());
            }
        }
    }
    Ok(env)
}

pub(super) fn codex_config(
    env: &BTreeMap<String, String>,
    agent_roles: &[CodexAgentRole],
    footer: Option<&str>,
) -> String {
    let mut toml = String::from(
        r#"#:schema https://developers.openai.com/codex/config-schema.json

# Generated by `cargo run -p codex-env -- mirror`.
approval_policy = "on-request"
approvals_reviewer = "auto_review"
sandbox_mode = "workspace-write"
web_search = "live"
model = "gpt-5.5"
model_reasoning_effort = "high"
model_catalog_json = "model-catalog.json"
model_context_window = 4000000

[mcp_servers.github]
command = "npx"
args = ["-y", "@modelcontextprotocol/server-github"]

[mcp_servers.context7]
command = "npx"
args = ["-y", "@upstash/context7-mcp@latest"]

[mcp_servers.exa]
url = "https://mcp.exa.ai/mcp"

[mcp_servers.memory]
command = "npx"
args = ["-y", "@modelcontextprotocol/server-memory"]

[mcp_servers.playwright]
command = "npx"
args = ["-y", "@playwright/mcp@latest", "--extension"]

[mcp_servers.sequential-thinking]
command = "npx"
args = ["-y", "@modelcontextprotocol/server-sequential-thinking"]

[mcp_servers.claude-flow]
command = "npx"
args = ["@claude-flow/cli@latest", "mcp", "start"]

[mcp_servers.claude-flow.env]
CLAUDE_FLOW_MODE = "v3"
CLAUDE_FLOW_HOOKS_ENABLED = "true"
CLAUDE_FLOW_TOPOLOGY = "hierarchical-mesh"
CLAUDE_FLOW_MAX_AGENTS = "15"
CLAUDE_FLOW_MEMORY_BACKEND = "hybrid"

[features]
multi_agent = true
goals = true

[skills]
include_instructions = true

[agents]
max_threads = 15
max_depth = 3

[agents.explorer]
description = "Read-only codebase explorer for gathering evidence before changes are proposed."
config_file = "agents/explorer.toml"

[agents.reviewer]
description = "PR reviewer focused on correctness, security, and missing tests."
config_file = "agents/reviewer.toml"

[agents.docs_researcher]
description = "Documentation specialist that verifies APIs, framework behavior, and release-note claims against primary documentation."
config_file = "agents/docs-researcher.toml"
"#,
    );

    for role in agent_roles {
        toml.push_str(&format!(
            "\n[agents.{}]\ndescription = \"{}\"\nconfig_file = \"{}\"\n",
            role.role_name,
            escape_toml_string(&role.description),
            escape_toml_string(&role.config_file.display().to_string())
        ));
    }

    toml.push_str(
        r#"
[shell_environment_policy]
inherit = "core"

[shell_environment_policy.set]
"#,
    );

    for (key, value) in env {
        toml.push_str(&format!("{key} = \"{}\"\n", escape_toml_string(value)));
    }

    if let Some(footer) = footer {
        toml.push('\n');
        toml.push_str(footer.trim());
        toml.push('\n');
    }

    toml
}

pub(super) fn codex_model_catalog_json() -> String {
    serde_json::to_string_pretty(&json!({
        "models": [
            {
                "slug": "gpt-5.5",
                "display_name": "GPT-5.5",
                "description": "Frontier model for complex coding, research, and real-world work.",
                "default_reasoning_level": "medium",
                "supported_reasoning_levels": [
                    {
                        "effort": "low",
                        "description": "Fast responses with lighter reasoning"
                    },
                    {
                        "effort": "medium",
                        "description": "Balances speed and reasoning depth for everyday tasks"
                    },
                    {
                        "effort": "high",
                        "description": "Greater reasoning depth for complex problems"
                    },
                    {
                        "effort": "xhigh",
                        "description": "Extra high reasoning depth for complex problems"
                    }
                ],
                "shell_type": "shell_command",
                "visibility": "list",
                "supported_in_api": true,
                "priority": 0,
                "additional_speed_tiers": ["fast"],
                "service_tiers": [
                    {
                        "id": "priority",
                        "name": "Fast",
                        "description": "1.5x speed, increased usage"
                    }
                ],
                "default_service_tier": null,
                "availability_nux": {
                    "message": "GPT-5.5 is configured for the local Codex harness."
                },
                "upgrade": null,
                "base_instructions": "",
                "supports_reasoning_summaries": true,
                "default_reasoning_summary": "none",
                "support_verbosity": true,
                "default_verbosity": "low",
                "apply_patch_tool_type": "freeform",
                "web_search_tool_type": "text_and_image",
                "truncation_policy": {
                    "mode": "tokens",
                    "limit": 10000
                },
                "supports_parallel_tool_calls": true,
                "supports_image_detail_original": true,
                "context_window": 4000000,
                "max_context_window": 4000000,
                "auto_compact_token_limit": null,
                "effective_context_window_percent": 95,
                "experimental_supported_tools": [],
                "input_modalities": ["text", "image"],
                "supports_search_tool": true
            },
            {
                "slug": "gpt-5.4-mini",
                "display_name": "GPT-5.4 mini",
                "description": "Fast model for focused subagent work.",
                "default_reasoning_level": "medium",
                "supported_reasoning_levels": [
                    {
                        "effort": "low",
                        "description": "Fast responses with lighter reasoning"
                    },
                    {
                        "effort": "medium",
                        "description": "Balances speed and reasoning depth for everyday tasks"
                    },
                    {
                        "effort": "high",
                        "description": "Greater reasoning depth for complex problems"
                    }
                ],
                "shell_type": "shell_command",
                "visibility": "list",
                "supported_in_api": true,
                "priority": 1,
                "additional_speed_tiers": [],
                "service_tiers": [],
                "default_service_tier": null,
                "availability_nux": null,
                "upgrade": null,
                "base_instructions": "",
                "supports_reasoning_summaries": true,
                "default_reasoning_summary": "none",
                "support_verbosity": true,
                "default_verbosity": "low",
                "apply_patch_tool_type": "freeform",
                "web_search_tool_type": "text_and_image",
                "truncation_policy": {
                    "mode": "tokens",
                    "limit": 10000
                },
                "supports_parallel_tool_calls": true,
                "supports_image_detail_original": true,
                "context_window": 1000000,
                "max_context_window": 1000000,
                "auto_compact_token_limit": null,
                "effective_context_window_percent": 95,
                "experimental_supported_tools": [],
                "input_modalities": ["text", "image"],
                "supports_search_tool": true
            }
        ]
    }))
    .expect("static Codex model catalog JSON should serialize")
}

pub(super) fn codex_agents_md() -> String {
    String::from(
        r#"# Codex Automation Extraction Surface

This directory is generated from tracked `.claude` source material by the Rust
`codex-env` harness. It is the Rust automation extraction frontier for this
repository, not a vendor harness and not a user-global prompt dump.

`.claude` remains source material. `.codex/mirror/.claude` is byte-for-byte
evidence/debug material. The crate-owned runtime target is the compact,
deterministic automation layer generated under `.codex`, especially
`.codex/automation-graph.json`, `.codex/agent-teams.json`,
`.codex/hooks.json`, `.codex/hooks/`, `.codex/prompts/`, and
`.codex/agents/`.

## Refresh

```bash
cargo run -p codex-env -- install
cargo run -p codex-env -- run --dry-run "inspect the Codex surface"
cargo run -p codex-env -- team-run --dry-run --team rust "inspect Rust parity gaps"
cargo run -p codex-env -- auto-loop --dry-run --team core "inspect autonomous loop wiring"
cargo run -p codex-env -- tdd-workflow --dry-run "trace Codex Rust tool ownership"
cargo run -p codex-env -- mirror --check
cargo run -p codex-env -- install-prompts --check
cargo run -p codex-env -- doctor
```

## Mirrored Surfaces

- `.claude/**` -> `.codex/mirror/.claude/**` byte-for-byte
- `.claude/**` -> `.codex/mirror-symbols.json` deterministic file/symbol evidence inventory
- `.claude/**` -> `.codex/automation-graph.json` compact crate-owned capability graph
- `.claude/settings.json` -> `.codex/hooks.json` and shell environment defaults
- `.claude/hooks/` -> normalized `.codex/hooks/` runtime scripts
- `.claude/skills/` -> `.agents/skills/`
- `.claude/commands/**/*.md` -> `.agents/skills/source-command-*`
- `.claude/commands/**/*.md` -> repo-local `.codex/prompts/*.md` for `/prompts:*`,
  including Claude namespace aliases such as `/prompts:sparc:code`
- Codex-native workflow upgrades -> `.agents/skills/codex-*` and
  `.codex/prompts/codex-*`

Use `--lua-policy <path>` when a repo-local transformation is needed. The Lua
script receives a `mirror` table with `repo_root` and `claude_dir`, and may
return `{ config_footer = "...", skill_prelude = "..." }`.

## Prompt Commands

Prompt commands generated for this repository stay in this repository's
`.codex/prompts`; do not copy meta-ruvector prompts into user-global
`~/.codex/prompts` or the meta root prompt set. Refresh and verify with:

```bash
.codex/helpers/install-prompts.sh
```

That helper runs `cargo run -p codex-env -- install`, which mirrors `.claude`
into repo-local Codex surfaces and runs doctor validation in one pass. Restart
Codex from this repository after refreshing. The Claude command mirrors then
appear as Codex prompt commands such as `/prompts:sparc-code`,
`/prompts:sparc:code`, and `/prompts:claude-flow-swarm`.

## Automation Ownership

Use `.codex/automation-graph.json` as the low-token routing and capability
index before loading bulk mirrored Markdown. Agent teams are generated from
actual configured Codex agent roles and expose both `agents` and `members` for
runtime consumers. Generated runtime hooks must resolve the repository root
dynamically; stale absolute paths such as `/workspaces/ruvector` are rejected by
doctor checks. Do not move this automation into a vendor harness.

## Run Actual Work

Use the repo-owned runner when you want Codex to execute a bounded task from the
validated local environment and leave artifacts:

```bash
cargo run -p codex-env -- run "fix the next Codex parity gap"
cargo run -p codex-env -- team-run --team rust "trace and fix the next Rust harness gap"
cargo run -p codex-env -- auto-loop --team core --max-iterations 3 "finish the Codex parity goal"
cargo run -p codex-env -- tdd-workflow "build, verify, and trace the Codex Rust tools"
```

Each run refreshes/validates the Codex surface, then invokes `codex exec --json`
with artifacts under `.codex/harness/runs/`: `prompt.md`, `events.jsonl`,
`stderr.log`, `last-message.md`, and `status.json`. Use `--dry-run` to materialize
the exact prompt and status without launching a nested Codex run. `team-run`
loads `.codex/agent-teams.json` plus the referenced `.codex/agents/*.toml`
profiles, starts every team member with its configured model and reasoning
effort in a read-only sandbox by default, then runs a parent consolidation
Codex pass that reads the member outputs, performs parent-owned edits, and
writes its own artifacts. Use `--member-sandbox workspace-write` only for a
deliberately isolated writable member scope.

`auto-loop` wraps `team-run` in bounded iterations, records
`auto-loop-status.json`, and stops early only when parent consolidation emits
`CODEX_AUTO_LOOP_STATUS: complete`. Otherwise it continues until
`--max-iterations` is reached.

`tdd-workflow` is the supervised red/green harness for the Rust-owned Codex
toolchain. It builds `codex-env`, then executes the built binary through
`mirror --check`, `install-prompts --check`, `doctor`, `inventory --check`,
and bounded dry-run `run`/`team-run`/`auto-loop` probes. Each status entry
records what the tool does, why it runs, where the behavior belongs, what Rust
extraction target owns it, and how Codex should supervise the background
terminal equivalent. Codex is the human-in-loop operator for this workflow:
start the background terminal equivalent, supervise status/artifacts, provide
follow-up guidance when the trace exposes a gap, end the worker session, and
extract durable behavior into the owning Rust crates rather than a vendor
harness. Non-dry-run workflow steps also write per-step stdout/stderr logs
under the workflow run directory so the supervising Codex session can inspect
what the background worker actually did before deciding the next extraction.
The workflow also writes `tdd-extraction-report.md` plus
`tdd-extraction-plan.json`; the JSON plan is the low-token machine-readable
crate-ownership handoff that turns the supervised trace into the next Rust
extraction action.
"#,
    )
}

pub(super) fn codex_native_workflow_prompts(
    codex_dir: &Path,
    agent_roles: &[CodexAgentRole],
) -> Vec<PlannedFile> {
    let team_markdown = codex_agent_team_markdown(agent_roles);
    [
        (
            "codex-agent-team.md",
            "Spawn a Codex-native subagent team from repo custom agents",
            "[TEAM=core|review|rust|security|github|swarm] [GOAL]",
            format!(
                r#"Use Codex-native subagents for this goal: $ARGUMENTS

Select the smallest effective team. Spawn the agents in parallel, wait for all results, then run parent consolidation:
Use the configured custom agent TOMLs as the routing source: heavy agents run on `gpt-5.5`, lighter explorer/template agents run on `gpt-5.4-mini`, and each agent carries its own reasoning effort.

{team_markdown}

Use the Rust harness when shell execution is appropriate:

```bash
cargo run -p codex-env -- team-run --team core "$ARGUMENTS"
```

The harness runs every team member with its configured model/reasoning effort in `read-only` mode by default, then launches a parent consolidation Codex pass. Give each subagent a bounded brief with concrete evidence to return. Do not let subagents modify files concurrently; use `--member-sandbox workspace-write` only for a deliberately isolated writable member scope. After all results return, the parent pass decides the implementation path, makes the edits, verifies, commits, pushes, and updates the PR when publishing applies.
"#
            ),
        ),
        (
            "codex-auto-loop.md",
            "Run the full Codex autonomous implementation loop",
            "[GOAL]",
            String::from(r#"Run the Codex autonomous loop for this goal: $ARGUMENTS

Use the Rust harness when shell execution is appropriate:

```bash
cargo run -p codex-env -- auto-loop --team core --max-iterations 3 "$ARGUMENTS"
```

The harness runs bounded team iterations, stores artifacts under
`.codex/harness/runs/`, and requires the parent consolidation pass to emit
`CODEX_AUTO_LOOP_STATUS: complete` before the loop stops early.
"#),
        ),
        (
            "codex-gap-hunt.md",
            "Run a deep Codex parity gap hunt before upgrading",
            "[SURFACE=hooks|agents|skills|prompts|all] [GOAL]",
            String::from(r#"Run a deep current-state gap hunt for this Codex surface: $ARGUMENTS

Compare the actual repo state against Codex-native behavior, not Claude assumptions. Start from `.codex/automation-graph.json` before loading bulk mirrored Markdown.

- commands and prompts: .claude/commands, .agents/skills/source-command-*, repo-local .codex/prompts
- agents and teams: .claude/agents, .codex/agents, custom-agent schema, explicit subagent workflows
- hooks and helpers: .claude/settings.json, .codex/hooks.json, .codex/hooks, .codex/helpers, supported Codex hook events
- settings and MCP: .codex/config.toml, active MCP servers, features, model and sandbox defaults
- auto loop: AGENTS.md, ICM recall/store, verification gates, commit/push/PR workflow

Return missed items ranked by user impact. Implement only upgrades that move Codex closer to the requested final state, then verify with authoritative command output.
"#),
        ),
        (
            "codex-tdd-workflow.md",
            "Build and execute the Rust-owned Codex TDD workflow gates",
            "[GOAL]",
            String::from(r#"Run the Codex Rust TDD workflow for this goal: $ARGUMENTS

Use the Rust harness when shell execution is appropriate:

```bash
cargo run -p codex-env -- tdd-workflow "$ARGUMENTS"
```

This builds `crates/codex-env`, then executes the built Codex Rust tools in
order: mirror check, repo-local prompt check, doctor, inventory check, and
bounded dry-run run/team-run/auto-loop probes. The workflow status records what
each tool does, why it runs, where the behavior belongs, and the Rust extraction
target. Treat Codex as the human-in-loop operator supervising a background
terminal: launch the workflow, watch status artifacts, give follow-up guidance
if the trace exposes a gap, end the worker session, then extract durable
automation into Rust-owned crates. Inspect each step's stdout/stderr log paths
and supervision events before deciding whether to proceed, guide, or stop the
worker. Then read `tdd-extraction-plan.json` first as the low-token
machine-readable next-action handoff, using `tdd-extraction-report.md` as the
human-readable evidence summary. Do not move this automation into a vendor
harness.
"#),
        ),
    ]
    .into_iter()
    .map(|(file, description, argument_hint, body)| {
        let mut prompt = String::new();
        prompt.push_str("---\n");
        prompt.push_str(&format!("description: {}\n", yaml_scalar(description)));
        prompt.push_str(&format!("argument-hint: {argument_hint}\n"));
        prompt.push_str("---\n\n");
        prompt.push_str(body.trim_start());
        if !prompt.ends_with('\n') {
            prompt.push('\n');
        }
        PlannedFile {
            path: codex_dir.join("prompts").join(file),
            bytes: normalize_generated_text(&prompt).into_bytes(),
            executable: false,
        }
    })
    .collect()
}

pub(super) fn codex_native_workflow_skills(
    skills_dir: &Path,
    agent_roles: &[CodexAgentRole],
) -> Vec<PlannedFile> {
    let team_markdown = codex_agent_team_markdown(agent_roles);
    [
        (
            "codex-agent-team",
            "Use when a task should spawn a Codex-native team of project custom agents for parallel research, implementation planning, review, security, GitHub, or swarm coordination.",
            format!(
                r#"# Codex Agent Team

Use Codex subagents explicitly. Pick the smallest effective team, spawn agents in parallel, wait for all results, then run parent consolidation.
Use the configured custom agent TOMLs as the model-routing source: heavy agents run on `gpt-5.5`, lighter explorer/template agents run on `gpt-5.4-mini`, and each agent carries its own reasoning effort.

Recommended teams:
{team_markdown}

When running from the shell, prefer the Rust harness:

```bash
cargo run -p codex-env -- team-run --team core "your goal"
```

The harness runs every team member with its configured model/reasoning effort in `read-only` mode by default, then launches a parent consolidation Codex pass. Give each subagent a bounded brief and a required evidence format. Keep write ownership in the parent pass; use `--member-sandbox workspace-write` only for a deliberately isolated writable member scope.
"#
            ),
        ),
        (
            "codex-auto-loop",
            "Use when the user wants autonomous end-to-end Codex execution with memory recall, gap analysis, implementation, verification, commit, push, and PR updates.",
            String::from(r#"# Codex Auto Loop

Run this loop until the requested end state is true or a real blocker is proven:

When running from the shell, prefer the Rust harness:

```bash
cargo run -p codex-env -- auto-loop --team core --max-iterations 3 "your goal"
```

The harness runs bounded team iterations, writes `auto-loop-status.json`, and
stops early only when parent consolidation emits
`CODEX_AUTO_LOOP_STATUS: complete`. Keep working while the marker is
`continue` or absent.
"#),
        ),
        (
            "codex-gap-hunt",
            "Use when auditing Codex parity gaps across hooks, helpers, prompts, skills, custom agents, subagents, settings, MCP, and auto-loop workflows.",
            String::from(r#"# Codex Gap Hunt

Audit from current evidence, not memory. Start from `.codex/automation-graph.json`, then compare source and generated surfaces only as needed:

- .claude/commands -> .agents/skills/source-command-* and repo-local .codex/prompts
- .claude/agents -> .codex/agents custom-agent TOML schema and explicit subagent workflows
- .claude/settings.json -> .codex/config.toml and .codex/hooks.json using supported Codex hook events
- .claude/hooks and helpers -> .codex/hooks and .codex/helpers
- AGENTS.md, ICM, verification, commit/push/PR workflow

Rank gaps by user impact, then implement upgrades only. Verify with commands that prove the touched surface works.
"#),
        ),
        (
            "codex-tdd-workflow",
            "Use when the task needs a supervised TDD workflow that builds codex-env, executes the Codex Rust tools, traces their purpose, and extracts durable behavior into Rust-owned crates.",
            String::from(r#"# Codex TDD Workflow

Use this when Codex needs to act as the human-in-loop operator for the
repo-owned automation layer.

When running from the shell, prefer the Rust harness:

```bash
cargo run -p codex-env -- tdd-workflow "your goal"
```

The harness builds `crates/codex-env`, then executes the built binary through
mirror, prompt, doctor, inventory, run, team-run, and auto-loop probes. Its
status file records what each tool does, why it runs, where the behavior
belongs, and the Rust extraction target. Supervise it like a background
terminal: inspect status artifacts, provide follow-up guidance if a probe
exposes a gap, terminate the worker session when the trace is complete, and
move durable automation into the correct Rust crate instead of a vendor harness.
Non-dry-run steps capture stdout/stderr logs and supervision events for
post-run extraction, then emit `tdd-extraction-plan.json` for machine-readable
next-action routing and `tdd-extraction-report.md` as the human-readable
summary.
"#),
        ),
    ]
    .into_iter()
    .map(|(slug, description, body)| {
        let mut skill = String::new();
        skill.push_str("---\n");
        skill.push_str(&format!("name: {slug}\n"));
        skill.push_str(&format!("description: {}\n", yaml_scalar(description)));
        skill.push_str("---\n\n");
        skill.push_str(body.trim_start());
        if !skill.ends_with('\n') {
            skill.push('\n');
        }
        PlannedFile {
            path: skills_dir.join(slug).join("SKILL.md"),
            bytes: normalize_generated_text(&skill).into_bytes(),
            executable: false,
        }
    })
    .collect()
}

pub(super) fn codex_agent_profiles(codex_dir: &Path) -> Vec<PlannedFile> {
    [
        (
            "explorer.toml",
            "explorer",
            "Read-only codebase explorer for gathering evidence before changes are proposed.",
            "gpt-5.4-mini",
            "medium",
            "Stay in exploration mode.\nTrace the real execution path, cite files and symbols, and avoid proposing fixes unless the parent agent asks for them.\nPrefer targeted search and file reads over broad scans.\n",
        ),
        (
            "reviewer.toml",
            "reviewer",
            "PR reviewer focused on correctness, security, and missing tests.",
            "gpt-5.5",
            "high",
            "Review like an owner.\nPrioritize correctness, security, behavioral regressions, and missing tests.\nLead with concrete findings and avoid style-only feedback unless it hides a real bug.\n",
        ),
        (
            "docs-researcher.toml",
            "docs-researcher",
            "Documentation specialist that verifies APIs, framework behavior, and release-note claims against primary documentation.",
            "gpt-5.4-mini",
            "medium",
            "Verify APIs, framework behavior, and release-note claims against primary documentation before changes land.\nCite the exact docs or file paths that support each claim.\nDo not invent undocumented behavior.\n",
        ),
    ]
    .into_iter()
    .map(
        |(file, name, description, model, effort, instructions)| PlannedFile {
        path: codex_dir.join("agents").join(file),
        bytes: format!(
            "name = \"{}\"\ndescription = \"{}\"\nmodel = \"{model}\"\nmodel_reasoning_effort = \"{effort}\"\nsandbox_mode = \"read-only\"\n\ndeveloper_instructions = \"\"\"\n{instructions}\"\"\"",
            escape_toml_string(name),
            escape_toml_string(description),
        )
        .into_bytes(),
        executable: false,
    },
    )
    .collect()
}

pub(super) fn codex_hooks_json(claude_dir: &Path) -> Result<String> {
    let settings_path = claude_dir.join("settings.json");
    let settings: serde_json::Value = serde_json::from_slice(
        &fs::read(&settings_path)
            .with_context(|| format!("failed to read {}", settings_path.display()))?,
    )?;

    let hooks = normalize_codex_hooks(settings.get("hooks"));
    let output = json!({
        "hooks": hooks,
    });
    Ok(format!("{}\n", serde_json::to_string_pretty(&output)?))
}

fn normalize_codex_hooks(source: Option<&serde_json::Value>) -> serde_json::Value {
    let Some(source) = source.and_then(serde_json::Value::as_object) else {
        return json!({});
    };

    let mut normalized = Map::new();
    for (source_event, groups) in source {
        let Some(codex_event) = codex_hook_event(source_event) else {
            continue;
        };
        let Some(groups) = groups.as_array() else {
            continue;
        };

        let target_groups = normalized
            .entry(codex_event.to_owned())
            .or_insert_with(|| json!([]))
            .as_array_mut()
            .expect("hook event value is initialized as array");

        for group in groups {
            let Some(group_object) = group.as_object() else {
                continue;
            };
            let Some(source_hooks) = group_object
                .get("hooks")
                .and_then(serde_json::Value::as_array)
            else {
                continue;
            };

            let hooks = source_hooks
                .iter()
                .filter_map(normalize_codex_hook_handler)
                .collect::<Vec<_>>();
            if hooks.is_empty() {
                continue;
            }

            let mut target_group = Map::new();
            if let Some(matcher) = group_object
                .get("matcher")
                .and_then(serde_json::Value::as_str)
            {
                target_group.insert("matcher".to_owned(), json!(matcher));
            }
            target_group.insert("hooks".to_owned(), json!(hooks));
            target_groups.push(serde_json::Value::Object(target_group));
        }
    }

    serde_json::Value::Object(normalized)
}

fn codex_hook_event(source_event: &str) -> Option<&'static str> {
    match source_event {
        "SessionStart" => Some("SessionStart"),
        "PreToolUse" => Some("PreToolUse"),
        "PermissionRequest" => Some("PermissionRequest"),
        "PostToolUse" => Some("PostToolUse"),
        "PreCompact" => Some("PreCompact"),
        "PostCompact" => Some("PostCompact"),
        "UserPromptSubmit" => Some("UserPromptSubmit"),
        "SubagentStart" => Some("SubagentStart"),
        "SubagentStop" => Some("SubagentStop"),
        "Stop" | "SessionEnd" => Some("Stop"),
        _ => None,
    }
}

fn normalize_codex_hook_handler(handler: &serde_json::Value) -> Option<serde_json::Value> {
    let handler = handler.as_object()?;
    if handler.get("async").and_then(serde_json::Value::as_bool) == Some(true) {
        return None;
    }
    if handler.get("type").and_then(serde_json::Value::as_str) != Some("command") {
        return None;
    }

    let command = handler.get("command").and_then(serde_json::Value::as_str)?;
    let mut normalized = Map::new();
    normalized.insert("type".to_owned(), json!("command"));
    normalized.insert("command".to_owned(), json!(codex_hook_command(command)));
    if let Some(timeout) = handler.get("timeout").and_then(serde_json::Value::as_u64) {
        normalized.insert(
            "timeout".to_owned(),
            json!(codex_hook_timeout_seconds(timeout)),
        );
    }
    if let Some(status) = handler
        .get("statusMessage")
        .and_then(serde_json::Value::as_str)
    {
        normalized.insert("statusMessage".to_owned(), json!(status));
    }
    Some(serde_json::Value::Object(normalized))
}

fn codex_hook_command(command: &str) -> String {
    let hook_handler = r#"node "${CLAUDE_PROJECT_DIR:-.}/.claude/helpers/hook-handler.cjs" "#;
    if let Some(args) = command.strip_prefix(hook_handler) {
        return format!(
            r#""$(git rev-parse --show-toplevel)/.codex/helpers/run-claude-hook.sh" hook-handler.cjs {}"#,
            args.trim()
        );
    }

    let auto_memory = r#"node "${CLAUDE_PROJECT_DIR:-.}/.claude/helpers/auto-memory-hook.mjs" "#;
    if let Some(args) = command.strip_prefix(auto_memory) {
        return format!(
            r#""$(git rev-parse --show-toplevel)/.codex/helpers/run-claude-hook.sh" auto-memory-hook.mjs {}"#,
            args.trim()
        );
    }

    command.replace(
        "${CLAUDE_PROJECT_DIR:-.}",
        "$(git rev-parse --show-toplevel)",
    )
}

fn codex_hook_timeout_seconds(timeout: u64) -> u64 {
    if timeout > 600 {
        timeout.div_ceil(1000)
    } else {
        timeout
    }
}

pub(super) fn copy_tree_plan(source: &Path, target: &Path) -> Result<Vec<PlannedFile>> {
    if !source.exists() {
        return Ok(Vec::new());
    }

    let mut files = Vec::new();
    for entry in WalkDir::new(source)
        .into_iter()
        .filter_entry(|entry| !is_ignored_path(entry.path()))
    {
        let entry = entry?;
        if !entry.file_type().is_file() {
            continue;
        }
        let relative = entry.path().strip_prefix(source)?;
        let bytes = normalize_generated_bytes(entry.path(), fs::read(entry.path())?);
        files.push(PlannedFile {
            path: target.join(relative),
            bytes,
            executable: is_executable(entry.path())?,
        });
    }
    files.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(files)
}

pub(super) fn codex_runtime_hook_plan(source: &Path, target: &Path) -> Result<Vec<PlannedFile>> {
    let mut files = copy_tree_plan(source, target)?;
    for file in &mut files {
        let Ok(text) = String::from_utf8(file.bytes.clone()) else {
            continue;
        };
        file.bytes = normalize_generated_text(&normalize_runtime_hook_paths(&text)).into_bytes();
    }
    Ok(files)
}

fn normalize_runtime_hook_paths(text: &str) -> String {
    if !text.contains("/workspaces/ruvector") {
        return text.to_owned();
    }

    let mut output = String::with_capacity(text.len() + 160);
    let mut inserted_repo_root = false;
    for line in text.lines() {
        output.push_str(line);
        output.push('\n');
        if !inserted_repo_root && line.trim_start().starts_with("set -e") {
            output.push_str(
                "repo_root=\"${CODEX_REPO_ROOT:-$(git rev-parse --show-toplevel 2>/dev/null || pwd)}\"\n",
            );
            inserted_repo_root = true;
        }
    }
    if !inserted_repo_root {
        output = format!(
            "repo_root=\"${{CODEX_REPO_ROOT:-$(git rev-parse --show-toplevel 2>/dev/null || pwd)}}\"\n{output}"
        );
    }
    output.replace("/workspaces/ruvector", "${repo_root}")
}

pub(super) fn command_skill_plan(
    commands_dir: &Path,
    skills_dir: &Path,
    prelude: Option<&str>,
) -> Result<Vec<PlannedFile>> {
    if !commands_dir.exists() {
        return Ok(Vec::new());
    }

    let mut files = Vec::new();
    for entry in WalkDir::new(commands_dir) {
        let entry = entry?;
        if !entry.file_type().is_file()
            || entry.path().extension().and_then(|s| s.to_str()) != Some("md")
        {
            continue;
        }

        let relative = entry.path().strip_prefix(commands_dir)?;
        let stem = relative.with_extension("");
        let name = format!("source-command-{}", super::slugify(&stem.to_string_lossy()));
        let command_name = claude_command_name(&stem);
        let source = fs::read_to_string(entry.path())?;
        let source_body = strip_leading_frontmatter(&source);
        let description = first_heading(&source)
            .unwrap_or_else(|| format!("Workflow command scaffold for {}.", stem.display()));
        let mut body = String::new();
        body.push_str("---\n");
        body.push_str(&format!("name: {name}\n"));
        body.push_str(&format!("description: {}\n", yaml_scalar(&description)));
        body.push_str("---\n\n");
        if let Some(prelude) = prelude {
            body.push_str(prelude.trim());
            body.push_str("\n\n");
        }
        body.push_str(&format!(
            "# /{}\n\nSource: `.claude/commands/{}`\n\n",
            command_name,
            relative.display()
        ));
        body.push_str(source_body.trim_start());
        if !body.ends_with('\n') {
            body.push('\n');
        }
        files.push(PlannedFile {
            path: skills_dir.join(name).join("SKILL.md"),
            bytes: normalize_generated_text(&body).into_bytes(),
            executable: false,
        });
    }
    files.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(files)
}

fn claude_command_name(stem: &Path) -> String {
    stem.components()
        .filter_map(|component| match component {
            std::path::Component::Normal(value) => {
                let segment = super::slugify(&value.to_string_lossy());
                (!segment.is_empty()).then_some(segment)
            }
            _ => None,
        })
        .collect::<Vec<_>>()
        .join(":")
}
