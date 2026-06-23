use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::{anyhow, bail, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use walkdir::WalkDir;

use super::{
    install_codex_prompts, is_executable, mirror_codex_surface, strip_repo_prefix,
    validate_codex_home_settings, CodexHomeSettingsReport, MirrorOptions, PromptInstallOptions,
    REQUIRED_CODEX_CONTEXT_WINDOW, REQUIRED_CODEX_MODEL, REQUIRED_CODEX_MODEL_CATALOG,
};

const SUPPORTED_HOOK_EVENTS: &[&str] = &[
    "SessionStart",
    "PreToolUse",
    "PermissionRequest",
    "PostToolUse",
    "PreCompact",
    "PostCompact",
    "UserPromptSubmit",
    "SubagentStart",
    "SubagentStop",
    "Stop",
];

const REQUIRED_WORKFLOW_PROMPTS: &[&str] = &[
    "codex-agent-team.md",
    "codex-auto-loop.md",
    "codex-gap-hunt.md",
];

const REQUIRED_AGENT_TEAMS: &[&str] = &["core", "review", "rust", "security", "github", "swarm"];

const REQUIRED_MCP_SERVERS: &[&str] = &[
    "github",
    "context7",
    "exa",
    "memory",
    "playwright",
    "sequential-thinking",
    "claude-flow",
];

const REQUIRED_CLAUDE_FLOW_ENV: &[(&str, &str)] = &[
    ("CLAUDE_FLOW_MODE", "v3"),
    ("CLAUDE_FLOW_HOOKS_ENABLED", "true"),
    ("CLAUDE_FLOW_TOPOLOGY", "hierarchical-mesh"),
    ("CLAUDE_FLOW_MAX_AGENTS", "15"),
    ("CLAUDE_FLOW_MEMORY_BACKEND", "hybrid"),
];

type ConfiguredAgents = BTreeMap<String, PathBuf>;
type AgentCounts = (usize, BTreeMap<String, usize>, BTreeMap<String, usize>);

#[derive(Debug, Clone)]
struct ConfigValidationReport {
    model: String,
    reasoning_effort: String,
    model_catalog_json: String,
    approval_policy: String,
    approvals_reviewer: String,
    goals_enabled: bool,
    mcp_servers: Vec<String>,
    configured_agents: ConfiguredAgents,
}

#[derive(Debug, Clone)]
struct HookValidationReport {
    events: Vec<String>,
    handlers: usize,
    shim_handlers: usize,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AgentTeamsManifest {
    schema_version: u64,
    teams: Vec<AgentTeam>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AgentTeam {
    name: String,
    description: String,
    strategy: String,
    parallel: bool,
    consolidation_owner: String,
    agents: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct DoctorOptions {
    pub repo_root: PathBuf,
    pub lua_policy: Option<PathBuf>,
    pub codex_home: PathBuf,
}

#[derive(Debug, Clone, Serialize)]
pub struct DoctorReport {
    pub repo_root: PathBuf,
    pub codex_dir: PathBuf,
    pub codex_home: PathBuf,
    pub codex_home_settings: CodexHomeSettingsReport,
    pub config_model: String,
    pub config_reasoning_effort: String,
    pub config_model_catalog_json: String,
    pub config_approval_policy: String,
    pub config_approvals_reviewer: String,
    pub config_goals_enabled: bool,
    pub config_mcp_servers: Vec<String>,
    pub config_agent_entries: usize,
    pub agent_files: usize,
    pub agent_models: BTreeMap<String, usize>,
    pub agent_efforts: BTreeMap<String, usize>,
    pub agent_teams: usize,
    pub agent_team_members: usize,
    pub hook_events: Vec<String>,
    pub hook_handlers: usize,
    pub hook_shim_handlers: usize,
    pub prompt_files: usize,
    pub prompt_alias_files: usize,
    pub installed_prompt_files: usize,
    pub workflow_prompts: Vec<String>,
    pub git_ignored_generated_files: Vec<PathBuf>,
}

pub fn doctor_codex_surface(options: DoctorOptions) -> Result<DoctorReport> {
    let repo_root = options.repo_root.canonicalize().with_context(|| {
        format!(
            "failed to canonicalize repo root {}",
            options.repo_root.display()
        )
    })?;
    let codex_dir = repo_root.join(".codex");
    let codex_home = options.codex_home;

    let mirror_report = mirror_codex_surface(MirrorOptions {
        repo_root: repo_root.clone(),
        lua_policy: options.lua_policy,
        check: true,
    })?;
    install_codex_prompts(PromptInstallOptions {
        repo_root: repo_root.clone(),
        codex_home: codex_home.clone(),
        check: true,
    })?;

    let config_report = validate_config(&codex_dir)?;
    let config_agent_entries = config_report.configured_agents.len();
    let (agent_files, agent_models, agent_efforts) =
        validate_agents(&codex_dir, &config_report.configured_agents)?;
    let (agent_teams, agent_team_members) =
        validate_agent_teams(&codex_dir, &config_report.configured_agents)?;
    let hook_report = validate_hooks(&codex_dir)?;
    let (prompt_files, prompt_alias_files, installed_prompt_files, workflow_prompts) =
        validate_prompts(&repo_root, &codex_dir, &codex_home)?;
    let git_ignored_generated_files =
        validate_generated_files_visible_to_git(&repo_root, &mirror_report.generated)?;
    let codex_home_settings = validate_codex_home_settings(&codex_home)?;

    Ok(DoctorReport {
        repo_root,
        codex_dir,
        codex_home,
        codex_home_settings,
        config_model: config_report.model,
        config_reasoning_effort: config_report.reasoning_effort,
        config_model_catalog_json: config_report.model_catalog_json,
        config_approval_policy: config_report.approval_policy,
        config_approvals_reviewer: config_report.approvals_reviewer,
        config_goals_enabled: config_report.goals_enabled,
        config_mcp_servers: config_report.mcp_servers,
        config_agent_entries,
        agent_files,
        agent_models,
        agent_efforts,
        agent_teams,
        agent_team_members,
        hook_events: hook_report.events,
        hook_handlers: hook_report.handlers,
        hook_shim_handlers: hook_report.shim_handlers,
        prompt_files,
        prompt_alias_files,
        installed_prompt_files,
        workflow_prompts,
        git_ignored_generated_files,
    })
}

fn validate_config(codex_dir: &Path) -> Result<ConfigValidationReport> {
    let path = codex_dir.join("config.toml");
    let config = read_toml(&path)?;
    let model = required_toml_string(&config, "model", &path)?;
    let effort = required_toml_string(&config, "model_reasoning_effort", &path)?;
    let model_catalog_json = required_toml_string(&config, "model_catalog_json", &path)?;
    let approval_policy = required_toml_string(&config, "approval_policy", &path)?;
    let approvals_reviewer = required_toml_string(&config, "approvals_reviewer", &path)?;
    let goals_enabled = required_toml_bool(&config, &["features", "goals"], &path)?;
    if model != "gpt-5.5" {
        bail!(
            "{} must default Codex to gpt-5.5, found {model}",
            path.display()
        );
    }
    if effort != "high" {
        bail!(
            "{} must default Codex reasoning effort to high, found {effort}",
            path.display()
        );
    }
    if model_catalog_json != REQUIRED_CODEX_MODEL_CATALOG {
        bail!(
            "{} must set model_catalog_json to {REQUIRED_CODEX_MODEL_CATALOG}, found {model_catalog_json}",
            path.display()
        );
    }
    validate_repo_model_catalog(codex_dir)?;
    if approval_policy != "on-request" {
        bail!(
            "{} must set Codex approval_policy to on-request, found {approval_policy}",
            path.display()
        );
    }
    if approvals_reviewer != "auto_review" {
        bail!(
            "{} must set Codex approvals_reviewer to auto_review, found {approvals_reviewer}",
            path.display()
        );
    }
    if !goals_enabled {
        bail!(
            "{} must enable Codex Goal mode with features.goals = true",
            path.display()
        );
    }
    let mcp_servers = validate_mcp_servers(&config, &path)?;
    let configured_agents = configured_agent_files(&config, codex_dir, &path)?;
    Ok(ConfigValidationReport {
        model,
        reasoning_effort: effort,
        model_catalog_json,
        approval_policy,
        approvals_reviewer,
        goals_enabled,
        mcp_servers,
        configured_agents,
    })
}

fn validate_mcp_servers(config: &toml::Value, config_path: &Path) -> Result<Vec<String>> {
    let servers = config
        .get("mcp_servers")
        .and_then(toml::Value::as_table)
        .ok_or_else(|| {
            anyhow!(
                "{} is missing required mcp_servers table",
                config_path.display()
            )
        })?;

    for required in REQUIRED_MCP_SERVERS {
        let Some(server) = servers.get(*required).and_then(toml::Value::as_table) else {
            bail!(
                "{} is missing required MCP server {required}",
                config_path.display()
            );
        };
        if *required == "exa" {
            let url = server
                .get("url")
                .and_then(toml::Value::as_str)
                .ok_or_else(|| {
                    anyhow!(
                        "{} MCP server {required} must define a url",
                        config_path.display()
                    )
                })?;
            if !url.starts_with("https://") {
                bail!(
                    "{} MCP server {required} must use an https URL, found {url}",
                    config_path.display()
                );
            }
        } else {
            let command = server
                .get("command")
                .and_then(toml::Value::as_str)
                .ok_or_else(|| {
                    anyhow!(
                        "{} MCP server {required} must define a command",
                        config_path.display()
                    )
                })?;
            if command.trim().is_empty() {
                bail!(
                    "{} MCP server {required} has an empty command",
                    config_path.display()
                );
            }
        }
    }

    let claude_flow = servers
        .get("claude-flow")
        .and_then(toml::Value::as_table)
        .ok_or_else(|| {
            anyhow!(
                "{} is missing claude-flow MCP config",
                config_path.display()
            )
        })?;
    let env = claude_flow
        .get("env")
        .and_then(toml::Value::as_table)
        .ok_or_else(|| {
            anyhow!(
                "{} MCP server claude-flow is missing required env table",
                config_path.display()
            )
        })?;
    for (key, expected) in REQUIRED_CLAUDE_FLOW_ENV {
        let value = env.get(*key).and_then(toml::Value::as_str).ok_or_else(|| {
            anyhow!(
                "{} MCP server claude-flow env is missing {key}",
                config_path.display()
            )
        })?;
        if value != *expected {
            bail!(
                "{} MCP server claude-flow env {key} must be {expected}, found {value}",
                config_path.display()
            );
        }
    }

    Ok(servers.keys().cloned().collect())
}

fn validate_repo_model_catalog(codex_dir: &Path) -> Result<()> {
    let path = codex_dir.join(REQUIRED_CODEX_MODEL_CATALOG);
    let catalog = serde_json::from_slice::<JsonValue>(
        &fs::read(&path).with_context(|| format!("failed to read {}", path.display()))?,
    )
    .with_context(|| format!("failed to parse {}", path.display()))?;
    let models = catalog
        .get("models")
        .and_then(JsonValue::as_array)
        .ok_or_else(|| anyhow!("{} must contain a models array", path.display()))?;
    let Some(model) = models
        .iter()
        .find(|model| model.get("slug").and_then(JsonValue::as_str) == Some(REQUIRED_CODEX_MODEL))
    else {
        bail!(
            "{} must contain model {}",
            path.display(),
            REQUIRED_CODEX_MODEL
        );
    };
    let context_window = model
        .get("context_window")
        .and_then(JsonValue::as_i64)
        .ok_or_else(|| anyhow!("{} model must set context_window", path.display()))?;
    let max_context_window = model
        .get("max_context_window")
        .and_then(JsonValue::as_i64)
        .ok_or_else(|| anyhow!("{} model must set max_context_window", path.display()))?;
    if context_window < REQUIRED_CODEX_CONTEXT_WINDOW
        || max_context_window < REQUIRED_CODEX_CONTEXT_WINDOW
    {
        bail!(
            "{} model {} must set context_window and max_context_window >= {}, found {context_window}/{max_context_window}",
            path.display(),
            REQUIRED_CODEX_MODEL,
            REQUIRED_CODEX_CONTEXT_WINDOW
        );
    }
    Ok(())
}

fn validate_agents(codex_dir: &Path, configured_agents: &ConfiguredAgents) -> Result<AgentCounts> {
    let root = codex_dir.join("agents");
    let mut agent_files = 0;
    let mut models = BTreeMap::new();
    let mut efforts = BTreeMap::new();
    let mut discovered_agents = BTreeSet::new();
    for entry in WalkDir::new(&root).into_iter() {
        let entry = entry?;
        let path = entry.path();
        if !entry.file_type().is_file()
            || path.extension().and_then(|value| value.to_str()) != Some("toml")
        {
            continue;
        }

        let toml = read_toml(path)?;
        for key in ["name", "description", "developer_instructions"] {
            let value = required_toml_string(&toml, key, path)?;
            if value.trim().is_empty() {
                bail!("{} has empty required key {key}", path.display());
            }
        }
        let model = required_toml_string(&toml, "model", path)?;
        let effort = required_toml_string(&toml, "model_reasoning_effort", path)?;
        *models.entry(model).or_insert(0) += 1;
        *efforts.entry(effort).or_insert(0) += 1;
        discovered_agents.insert(strip_repo_prefix(codex_dir, path));
        agent_files += 1;
    }

    if agent_files == 0 {
        bail!("{} has no custom agent TOML files", root.display());
    }
    for model in ["gpt-5.5", "gpt-5.4-mini"] {
        if !models.contains_key(model) {
            bail!("{} has no custom agents routed to {model}", root.display());
        }
    }
    let configured_agent_files = configured_agents.values().cloned().collect::<BTreeSet<_>>();
    if discovered_agents != configured_agent_files {
        let missing_from_config: Vec<_> = discovered_agents
            .difference(&configured_agent_files)
            .take(5)
            .map(|path| path.display().to_string())
            .collect();
        let missing_from_disk: Vec<_> = configured_agent_files
            .difference(&discovered_agents)
            .take(5)
            .map(|path| path.display().to_string())
            .collect();
        bail!(
            "{} custom agent config is out of sync: {} file(s) missing from config [{}], {} config entry/entries missing from disk [{}]",
            root.display(),
            discovered_agents.difference(&configured_agent_files).count(),
            missing_from_config.join(", "),
            configured_agent_files.difference(&discovered_agents).count(),
            missing_from_disk.join(", ")
        );
    }
    Ok((agent_files, models, efforts))
}

fn configured_agent_files(
    config: &toml::Value,
    codex_dir: &Path,
    config_path: &Path,
) -> Result<ConfiguredAgents> {
    let agents = config
        .get("agents")
        .and_then(toml::Value::as_table)
        .ok_or_else(|| anyhow!("{} is missing required agents table", config_path.display()))?;
    let mut configured = BTreeMap::new();
    for (name, value) in agents {
        let Some(table) = value.as_table() else {
            continue;
        };
        let Some(config_file) = table.get("config_file").and_then(toml::Value::as_str) else {
            continue;
        };
        if config_file.trim().is_empty() {
            bail!(
                "{} agent {name} has an empty config_file",
                config_path.display()
            );
        }
        let config_file = PathBuf::from(config_file);
        if config_file.is_absolute()
            || config_file
                .components()
                .any(|component| matches!(component, Component::ParentDir))
        {
            bail!(
                "{} agent {name} has unsafe config_file {}",
                config_path.display(),
                config_file.display()
            );
        }
        configured.insert(
            name.clone(),
            strip_repo_prefix(codex_dir, &codex_dir.join(config_file)),
        );
    }
    if configured.is_empty() {
        bail!("{} has no configured custom agents", config_path.display());
    }
    Ok(configured)
}

fn validate_agent_teams(
    codex_dir: &Path,
    configured_agents: &ConfiguredAgents,
) -> Result<(usize, usize)> {
    let path = codex_dir.join("agent-teams.json");
    let manifest: AgentTeamsManifest = serde_json::from_slice(
        &fs::read(&path).with_context(|| format!("failed to read {}", path.display()))?,
    )
    .with_context(|| format!("failed to parse {}", path.display()))?;
    if manifest.schema_version != 1 {
        bail!(
            "{} must use agent team schemaVersion 1, found {}",
            path.display(),
            manifest.schema_version
        );
    }

    let mut seen_teams = BTreeSet::new();
    let mut referenced_members = 0;
    let mut referenced_models = BTreeSet::new();
    let mut mixed_model_teams = 0;

    for team in &manifest.teams {
        if team.name.trim().is_empty() {
            bail!(
                "{} contains an agent team with an empty name",
                path.display()
            );
        }
        if !seen_teams.insert(team.name.as_str()) {
            bail!(
                "{} contains duplicate agent team {}",
                path.display(),
                team.name
            );
        }
        for (field, value) in [
            ("description", team.description.as_str()),
            ("strategy", team.strategy.as_str()),
            ("consolidationOwner", team.consolidation_owner.as_str()),
        ] {
            if value.trim().is_empty() {
                bail!(
                    "{} agent team {} has an empty {field}",
                    path.display(),
                    team.name
                );
            }
        }
        if !team.parallel {
            bail!(
                "{} agent team {} must be marked parallel",
                path.display(),
                team.name
            );
        }
        if team.consolidation_owner != "parent" {
            bail!(
                "{} agent team {} must consolidate through parent, found {}",
                path.display(),
                team.name,
                team.consolidation_owner
            );
        }
        if team.agents.len() < 2 {
            bail!(
                "{} agent team {} must reference at least two agents",
                path.display(),
                team.name
            );
        }

        let mut team_agents = BTreeSet::new();
        let mut team_models = BTreeSet::new();
        for agent in &team.agents {
            if !team_agents.insert(agent.as_str()) {
                bail!(
                    "{} agent team {} contains duplicate agent {}",
                    path.display(),
                    team.name,
                    agent
                );
            }
            let Some(config_file) = configured_agents.get(agent) else {
                bail!(
                    "{} agent team {} references unknown configured agent {}",
                    path.display(),
                    team.name,
                    agent
                );
            };
            let agent_toml = read_toml(&codex_dir.join(config_file))?;
            let model = required_toml_string(&agent_toml, "model", &codex_dir.join(config_file))?;
            team_models.insert(model.clone());
            referenced_models.insert(model);
            referenced_members += 1;
        }
        if team_models.len() > 1 {
            mixed_model_teams += 1;
        }
    }

    for required in REQUIRED_AGENT_TEAMS {
        if !seen_teams.contains(required) {
            bail!(
                "{} is missing required agent team {required}",
                path.display()
            );
        }
    }
    for model in ["gpt-5.5", "gpt-5.4-mini"] {
        if !referenced_models.contains(model) {
            bail!(
                "{} agent teams do not reference any custom agents routed to {model}",
                path.display()
            );
        }
    }
    if mixed_model_teams == 0 {
        bail!(
            "{} agent teams must include at least one mixed-model parallel team",
            path.display()
        );
    }

    Ok((manifest.teams.len(), referenced_members))
}

fn validate_hooks(codex_dir: &Path) -> Result<HookValidationReport> {
    let path = codex_dir.join("hooks.json");
    let hooks_root: serde_json::Value = serde_json::from_slice(
        &fs::read(&path).with_context(|| format!("failed to read {}", path.display()))?,
    )
    .with_context(|| format!("failed to parse {}", path.display()))?;
    let hooks = hooks_root
        .get("hooks")
        .and_then(serde_json::Value::as_object)
        .ok_or_else(|| anyhow!("{} must contain an object at hooks", path.display()))?;
    let supported: BTreeSet<_> = SUPPORTED_HOOK_EVENTS.iter().copied().collect();
    let mut events = Vec::new();
    let mut handlers = 0;
    let mut shim_handlers = 0;

    for (event, groups) in hooks {
        if !supported.contains(event.as_str()) {
            bail!(
                "{} contains unsupported Codex hook event {event}",
                path.display()
            );
        }
        let groups = groups
            .as_array()
            .ok_or_else(|| anyhow!("{} hook event {event} must be an array", path.display()))?;
        if groups.is_empty() {
            continue;
        }
        events.push(event.clone());
        for group in groups {
            let entries = group
                .get("hooks")
                .and_then(serde_json::Value::as_array)
                .ok_or_else(|| {
                    anyhow!("{} hook event {event} has no hooks array", path.display())
                })?;
            for entry in entries {
                if validate_hook_entry(codex_dir, &path, event, entry)? {
                    shim_handlers += 1;
                }
                handlers += 1;
            }
        }
    }

    if handlers == 0 {
        bail!("{} has no active Codex hook handlers", path.display());
    }
    events.sort();
    Ok(HookValidationReport {
        events,
        handlers,
        shim_handlers,
    })
}

fn validate_hook_entry(
    codex_dir: &Path,
    path: &Path,
    event: &str,
    entry: &serde_json::Value,
) -> Result<bool> {
    if entry.get("type").and_then(serde_json::Value::as_str) != Some("command") {
        bail!(
            "{} hook event {event} contains a non-command hook",
            path.display()
        );
    }
    if entry.get("async").and_then(serde_json::Value::as_bool) == Some(true) {
        bail!(
            "{} hook event {event} still contains async=true",
            path.display()
        );
    }
    if let Some(timeout) = entry.get("timeout").and_then(serde_json::Value::as_u64) {
        if timeout == 0 || timeout > 600 {
            bail!(
                "{} hook event {event} has invalid timeout {timeout}; Codex expects seconds",
                path.display()
            );
        }
    }
    let command = entry
        .get("command")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| anyhow!("{} hook event {event} has no command", path.display()))?;
    if command.contains(".claude/helpers") && !command.contains(".codex/helpers/run-claude-hook.sh")
    {
        bail!(
            "{} hook event {event} calls .claude/helpers without the Codex helper shim",
            path.display()
        );
    }
    validate_hook_shim_command(codex_dir, path, event, command)
}

fn validate_hook_shim_command(
    codex_dir: &Path,
    path: &Path,
    event: &str,
    command: &str,
) -> Result<bool> {
    let shim_marker = ".codex/helpers/run-claude-hook.sh";
    if !command.contains(shim_marker) {
        return Ok(false);
    }

    let expected_prefix =
        r#""$(git rev-parse --show-toplevel)/.codex/helpers/run-claude-hook.sh" "#;
    let Some(args) = command.strip_prefix(expected_prefix) else {
        bail!(
            "{} hook event {event} has unsupported Codex hook shim command form: {command}",
            path.display()
        );
    };
    let helper = args.split_whitespace().next().ok_or_else(|| {
        anyhow!(
            "{} hook event {event} uses the Codex hook shim without a helper argument",
            path.display()
        )
    })?;
    match helper {
        "hook-handler.cjs" | "auto-memory-hook.mjs" => {}
        _ => {
            bail!(
                "{} hook event {event} references unsupported Claude helper {helper}",
                path.display()
            );
        }
    }

    let shim_path = codex_dir.join("helpers/run-claude-hook.sh");
    if !shim_path.is_file() {
        bail!(
            "{} is missing required Codex hook shim",
            shim_path.display()
        );
    }
    if !is_executable(&shim_path)? {
        bail!("{} must be executable", shim_path.display());
    }

    let repo_root = codex_dir.parent().unwrap_or(codex_dir);
    let claude_helper = repo_root.join(".claude/helpers").join(helper);
    if !claude_helper.is_file() {
        bail!(
            "{} hook event {event} references missing Claude helper {}",
            path.display(),
            claude_helper.display()
        );
    }

    Ok(true)
}

fn validate_prompts(
    repo_root: &Path,
    codex_dir: &Path,
    codex_home: &Path,
) -> Result<(usize, usize, usize, Vec<String>)> {
    let source_dir = codex_dir.join("prompts");
    let target_dir = codex_home.join("prompts");
    let source_prompts = prompt_files(&source_dir)?;
    if source_prompts.is_empty() {
        bail!("{} has no Codex prompt files", source_dir.display());
    }
    for required in REQUIRED_WORKFLOW_PROMPTS {
        if !source_prompts.contains(&source_dir.join(required)) {
            bail!(
                "{} is missing required workflow prompt {required}",
                source_dir.display()
            );
        }
    }

    let mut installed_prompt_files = 0;
    let expected_targets: BTreeSet<PathBuf> = source_prompts
        .iter()
        .map(|source| {
            source
                .strip_prefix(&source_dir)
                .map(|relative| target_dir.join(relative))
        })
        .collect::<std::result::Result<_, _>>()?;
    for source in &source_prompts {
        let relative = source.strip_prefix(&source_dir)?;
        let target = target_dir.join(relative);
        let source_bytes = fs::read(source)?;
        let target_bytes = fs::read(&target).with_context(|| {
            format!(
                "installed Codex prompt {} is missing for source {}",
                target.display(),
                strip_repo_prefix(repo_root, source).display()
            )
        })?;
        if source_bytes != target_bytes {
            bail!(
                "installed Codex prompt {} differs from source {}",
                target.display(),
                strip_repo_prefix(repo_root, source).display()
            );
        }
        installed_prompt_files += 1;
    }
    let installed_prompts = prompt_files(&target_dir)?;
    let stale_targets: Vec<_> = installed_prompts
        .into_iter()
        .filter(|path| !expected_targets.contains(path))
        .collect();
    if !stale_targets.is_empty() {
        bail!(
            "installed Codex prompts include {} stale file(s); first: {}",
            stale_targets.len(),
            stale_targets[0].display()
        );
    }

    Ok((
        source_prompts.len(),
        source_prompts
            .iter()
            .filter(|path| {
                path.file_name()
                    .and_then(|value| value.to_str())
                    .is_some_and(|file_name| file_name.contains(':'))
            })
            .count(),
        installed_prompt_files,
        REQUIRED_WORKFLOW_PROMPTS
            .iter()
            .map(|value| (*value).to_owned())
            .collect(),
    ))
}

fn validate_generated_files_visible_to_git(
    repo_root: &Path,
    generated_files: &[PathBuf],
) -> Result<Vec<PathBuf>> {
    if !is_git_worktree(repo_root)? {
        return Ok(Vec::new());
    }

    let mut ignored = Vec::new();
    for relative in generated_files {
        let status = Command::new("git")
            .arg("check-ignore")
            .arg("-q")
            .arg("--")
            .arg(relative)
            .current_dir(repo_root)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .with_context(|| "failed to run git check-ignore")?;
        match status.code() {
            Some(0) => ignored.push(relative.clone()),
            Some(1) => {}
            Some(code) => {
                bail!(
                    "git check-ignore failed with status {code} for {}",
                    relative.display()
                );
            }
            None => bail!("git check-ignore was terminated for {}", relative.display()),
        }
    }

    if !ignored.is_empty() {
        bail!(
            "Codex generated surface has {} gitignored file(s); first: {}",
            ignored.len(),
            ignored[0].display()
        );
    }

    Ok(ignored)
}

fn is_git_worktree(repo_root: &Path) -> Result<bool> {
    let status = Command::new("git")
        .arg("rev-parse")
        .arg("--is-inside-work-tree")
        .current_dir(repo_root)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .with_context(|| "failed to run git rev-parse")?;
    Ok(status.success())
}

fn prompt_files(root: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    if !root.exists() {
        return Ok(files);
    }
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() && path.extension().and_then(|value| value.to_str()) == Some("md") {
            files.push(path);
        }
    }
    files.sort();
    Ok(files)
}

fn read_toml(path: &Path) -> Result<toml::Value> {
    let text =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    toml::from_str(&text).with_context(|| format!("failed to parse {}", path.display()))
}

fn required_toml_string(value: &toml::Value, key: &str, path: &Path) -> Result<String> {
    value
        .get(key)
        .and_then(toml::Value::as_str)
        .map(ToOwned::to_owned)
        .ok_or_else(|| anyhow!("{} is missing required string key {key}", path.display()))
}

fn required_toml_bool(value: &toml::Value, keys: &[&str], path: &Path) -> Result<bool> {
    let mut current = value;
    for key in keys {
        current = current.get(*key).ok_or_else(|| {
            anyhow!(
                "{} is missing required boolean key {}",
                path.display(),
                keys.join(".")
            )
        })?;
    }
    current.as_bool().ok_or_else(|| {
        anyhow!(
            "{} required key {} must be a boolean",
            path.display(),
            keys.join(".")
        )
    })
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use super::{validate_mcp_servers, validate_repo_model_catalog};

    fn valid_mcp_config() -> toml::Value {
        toml::from_str(
            r#"
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
"#,
        )
        .unwrap()
    }

    #[test]
    fn validate_mcp_servers_rejects_missing_required_server() {
        let mut config = valid_mcp_config();
        config
            .get_mut("mcp_servers")
            .unwrap()
            .as_table_mut()
            .unwrap()
            .remove("github");

        let error = validate_mcp_servers(&config, Path::new(".codex/config.toml")).unwrap_err();
        assert!(error
            .to_string()
            .contains("missing required MCP server github"));
    }

    #[test]
    fn validate_mcp_servers_rejects_claude_flow_env_drift() {
        let mut config = valid_mcp_config();
        config
            .get_mut("mcp_servers")
            .unwrap()
            .get_mut("claude-flow")
            .unwrap()
            .get_mut("env")
            .unwrap()
            .as_table_mut()
            .unwrap()
            .insert(
                "CLAUDE_FLOW_MODE".to_owned(),
                toml::Value::String("v2".to_owned()),
            );

        let error = validate_mcp_servers(&config, Path::new(".codex/config.toml")).unwrap_err();
        assert!(error
            .to_string()
            .contains("claude-flow env CLAUDE_FLOW_MODE must be v3"));
    }

    #[test]
    fn validate_repo_model_catalog_rejects_fallback_context_window() {
        let temp = tempfile::tempdir().unwrap();
        let codex_dir = temp.path();
        fs::write(
            codex_dir.join("model-catalog.json"),
            r#"{
              "models": [
                {
                  "slug": "gpt-5.5",
                  "context_window": 272000,
                  "max_context_window": 272000
                }
              ]
            }"#,
        )
        .unwrap();

        let error = validate_repo_model_catalog(codex_dir).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("must set context_window and max_context_window >= 4000000"),
            "unexpected error: {error:?}"
        );
    }
}
