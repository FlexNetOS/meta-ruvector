use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

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
    agents: &'static [&'static str],
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
            agents: &[
                "claude-core-planner",
                "claude-core-researcher",
                "claude-core-coder",
                "claude-core-tester",
                "claude-core-reviewer",
            ],
        },
        CodexAgentTeam {
            name: "review",
            description:
                "Find correctness, production, security, and regression risks before shipping.",
            strategy: "parallel-review-then-parent-remediation",
            parallel: true,
            consolidation_owner: "parent",
            agents: &[
                "reviewer",
                "claude-core-reviewer",
                "claude-testing-production-validator",
                "claude-v3-security-auditor",
            ],
        },
        CodexAgentTeam {
            name: "rust",
            description: "Trace Rust code paths, implement Rust changes, test, and optimize.",
            strategy: "parallel-rust-research-then-parent-patch",
            parallel: true,
            consolidation_owner: "parent",
            agents: &[
                "explorer",
                "claude-core-coder",
                "claude-core-tester",
                "claude-v3-performance-engineer",
            ],
        },
        CodexAgentTeam {
            name: "security",
            description:
                "Review architecture, audit implementation, inspect PII, and harden defenses.",
            strategy: "parallel-security-analysis-then-parent-remediation",
            parallel: true,
            consolidation_owner: "parent",
            agents: &[
                "claude-v3-security-architect",
                "claude-v3-security-auditor",
                "claude-v3-pii-detector",
                "claude-v3-aidefence-guardian",
            ],
        },
        CodexAgentTeam {
            name: "github",
            description:
                "Prepare PRs, review GitHub feedback, and keep repository automation aligned.",
            strategy: "parallel-github-coordination-then-parent-publish",
            parallel: true,
            consolidation_owner: "parent",
            agents: &[
                "claude-github-pr-manager",
                "claude-github-code-review-swarm",
                "claude-github-workflow-automation",
            ],
        },
        CodexAgentTeam {
            name: "swarm",
            description:
                "Coordinate larger multi-agent efforts with hierarchical and hive-mind controllers.",
            strategy: "parallel-swarm-coordination-then-parent-decision",
            parallel: true,
            consolidation_owner: "parent",
            agents: &[
                "claude-swarm-hierarchical-coordinator",
                "claude-hive-mind-queen-coordinator",
                "claude-v3-v3-queen-coordinator",
            ],
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
                .agents
                .iter()
                .filter(|agent| available.contains(**agent))
                .map(|agent| (*agent).to_owned())
                .collect::<Vec<_>>();
            if agents.len() < 2 {
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
        r#"# Codex Mirror Surface

This directory is generated from the tracked `.claude` surface by the Rust
`codex-env` harness.

## Refresh

```bash
cargo run -p codex-env -- install
cargo run -p codex-env -- run --dry-run "inspect the Codex surface"
cargo run -p codex-env -- mirror --check
cargo run -p codex-env -- install-prompts --check
cargo run -p codex-env -- doctor
```

## Mirrored Surfaces

- `.claude/**` -> `.codex/mirror/.claude/**` byte-for-byte
- `.claude/**` -> `.codex/mirror-symbols.json` deterministic file/symbol inventory
- `.claude/settings.json` -> `.codex/hooks.json` and shell environment defaults
- `.claude/hooks/` -> `.codex/hooks/`
- `.claude/skills/` -> `.agents/skills/`
- `.claude/commands/**/*.md` -> `.agents/skills/source-command-*`
- `.claude/commands/**/*.md` -> `.codex/prompts/*.md` for `/prompts:*`,
  including Claude namespace aliases such as `/prompts:sparc:code`
- Codex-native workflow upgrades -> `.agents/skills/codex-*` and
  `.codex/prompts/codex-*`

Use `--lua-policy <path>` when a repo-local transformation is needed. The Lua
script receives a `mirror` table with `repo_root` and `claude_dir`, and may
return `{ config_footer = "...", skill_prelude = "..." }`.

## Install Prompt Commands

Codex loads custom prompts from `$CODEX_HOME/prompts`, not directly from a
repository. Refresh the mirror and install the generated prompt commands with:

```bash
.codex/helpers/install-prompts.sh
```

That helper runs `cargo run -p codex-env -- install`, which mirrors `.claude`,
installs `$CODEX_HOME/prompts`, and runs doctor validation in one pass. Restart
Codex after installing. The Claude command mirrors then appear as Codex prompt
commands such as `/prompts:sparc-code`, `/prompts:sparc:code`, and
`/prompts:claude-flow-swarm`.

## Run Actual Work

Use the repo-owned runner when you want Codex to execute a bounded task from the
validated local environment and leave artifacts:

```bash
cargo run -p codex-env -- run "fix the next Codex parity gap"
```

Each run refreshes/validates the Codex surface, then invokes `codex exec --json`
with artifacts under `.codex/harness/runs/`: `prompt.md`, `events.jsonl`,
`stderr.log`, `last-message.md`, and `status.json`. Use `--dry-run` to materialize
the exact prompt and status without launching a nested Codex run.
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

Select the smallest effective team. Spawn the agents in parallel, wait for all results, then consolidate:
Use the configured custom agent TOMLs as the routing source: heavy agents run on `gpt-5.5`, lighter explorer/template agents run on `gpt-5.4-mini`, and each agent carries its own reasoning effort.

{team_markdown}

Give each subagent a bounded brief with concrete evidence to return. Do not let subagents modify the same file concurrently. After all results return, decide the implementation path, make the edits in the parent thread, verify, commit, push, and update the PR when publishing applies.
"#
            ),
        ),
        (
            "codex-auto-loop.md",
            "Run the full Codex autonomous implementation loop",
            "[GOAL]",
            String::from(r#"Run the Codex autonomous loop for this goal: $ARGUMENTS

1. Recall project memory and read the closest AGENTS.md instructions.
2. Inspect current git, branch, PR, and generated-surface state before trusting prior context.
3. Identify requirements and evidence that would prove completion.
4. For broad work, spawn a focused Codex subagent team in parallel and wait for results.
5. Implement the smallest complete upgrade that makes the requested end state more true.
6. Regenerate deterministic surfaces with codex-env when source or generator changes require it.
7. Run targeted tests plus mirror/install checks, then broader gates proportional to risk.
8. Commit, push, and open or update the PR. Store ICM memory for significant completed work.
9. Continue with the next gap unless the whole objective is proven complete.
"#),
        ),
        (
            "codex-gap-hunt.md",
            "Run a deep Codex parity gap hunt before upgrading",
            "[SURFACE=hooks|agents|skills|prompts|all] [GOAL]",
            String::from(r#"Run a deep current-state gap hunt for this Codex surface: $ARGUMENTS

Compare the actual repo state against Codex-native behavior, not Claude assumptions:

- commands and prompts: .claude/commands, .agents/skills/source-command-*, .codex/prompts, CODEX_HOME/prompts
- agents and teams: .claude/agents, .codex/agents, custom-agent schema, explicit subagent workflows
- hooks and helpers: .claude/settings.json, .codex/hooks.json, .codex/hooks, .codex/helpers, supported Codex hook events
- settings and MCP: .codex/config.toml, active MCP servers, features, model and sandbox defaults
- auto loop: AGENTS.md, ICM recall/store, verification gates, commit/push/PR workflow

Return missed items ranked by user impact. Implement only upgrades that move Codex closer to the requested final state, then verify with authoritative command output.
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

Use Codex subagents explicitly. Pick the smallest effective team, spawn agents in parallel, wait for all results, then consolidate in the parent thread.
Use the configured custom agent TOMLs as the model-routing source: heavy agents run on `gpt-5.5`, lighter explorer/template agents run on `gpt-5.4-mini`, and each agent carries its own reasoning effort.

Recommended teams:
{team_markdown}

Give each subagent a bounded brief and a required evidence format. Keep write ownership in the parent thread unless a subagent has an isolated file scope.
"#
            ),
        ),
        (
            "codex-auto-loop",
            "Use when the user wants autonomous end-to-end Codex execution with memory recall, gap analysis, implementation, verification, commit, push, and PR updates.",
            String::from(r#"# Codex Auto Loop

Run this loop until the requested end state is true or a real blocker is proven:

1. Recall ICM memory and inspect the current repo/branch/PR state.
2. Derive concrete requirements and completion evidence.
3. Spawn focused Codex subagents for broad or uncertain work.
4. Implement upgrades in the parent thread using repo patterns.
5. Regenerate deterministic Codex surfaces with codex-env when needed.
6. Run targeted gates, mirror checks, install checks, and risk-appropriate broader gates.
7. Commit, push, update or open the PR, and store ICM memory for significant work.
8. Continue to the next gap while the active objective remains incomplete.
"#),
        ),
        (
            "codex-gap-hunt",
            "Use when auditing Codex parity gaps across hooks, helpers, prompts, skills, custom agents, subagents, settings, MCP, and auto-loop workflows.",
            String::from(r#"# Codex Gap Hunt

Audit from current evidence, not memory. Compare source and generated surfaces:

- .claude/commands -> .agents/skills/source-command-* and .codex/prompts -> CODEX_HOME/prompts
- .claude/agents -> .codex/agents custom-agent TOML schema and explicit subagent workflows
- .claude/settings.json -> .codex/config.toml and .codex/hooks.json using supported Codex hook events
- .claude/hooks and helpers -> .codex/hooks and .codex/helpers
- AGENTS.md, ICM, verification, commit/push/PR workflow

Rank gaps by user impact, then implement upgrades only. Verify with commands that prove the touched surface works.
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
