use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::{anyhow, bail, Context, Result};
use serde::Serialize;
use walkdir::WalkDir;

use super::{
    install_codex_prompts, mirror_codex_surface, strip_repo_prefix, MirrorOptions,
    PromptInstallOptions,
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

type AgentCounts = (usize, BTreeMap<String, usize>, BTreeMap<String, usize>);

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
    pub config_model: String,
    pub config_reasoning_effort: String,
    pub config_approval_policy: String,
    pub config_approvals_reviewer: String,
    pub config_goals_enabled: bool,
    pub agent_files: usize,
    pub agent_models: BTreeMap<String, usize>,
    pub agent_efforts: BTreeMap<String, usize>,
    pub hook_events: Vec<String>,
    pub hook_handlers: usize,
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

    let (
        config_model,
        config_reasoning_effort,
        config_approval_policy,
        config_approvals_reviewer,
        config_goals_enabled,
    ) = validate_config(&codex_dir)?;
    let (agent_files, agent_models, agent_efforts) = validate_agents(&codex_dir)?;
    let (hook_events, hook_handlers) = validate_hooks(&codex_dir)?;
    let (prompt_files, prompt_alias_files, installed_prompt_files, workflow_prompts) =
        validate_prompts(&repo_root, &codex_dir, &codex_home)?;
    let git_ignored_generated_files =
        validate_generated_files_visible_to_git(&repo_root, &mirror_report.generated)?;

    Ok(DoctorReport {
        repo_root,
        codex_dir,
        codex_home,
        config_model,
        config_reasoning_effort,
        config_approval_policy,
        config_approvals_reviewer,
        config_goals_enabled,
        agent_files,
        agent_models,
        agent_efforts,
        hook_events,
        hook_handlers,
        prompt_files,
        prompt_alias_files,
        installed_prompt_files,
        workflow_prompts,
        git_ignored_generated_files,
    })
}

fn validate_config(codex_dir: &Path) -> Result<(String, String, String, String, bool)> {
    let path = codex_dir.join("config.toml");
    let config = read_toml(&path)?;
    let model = required_toml_string(&config, "model", &path)?;
    let effort = required_toml_string(&config, "model_reasoning_effort", &path)?;
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
    Ok((
        model,
        effort,
        approval_policy,
        approvals_reviewer,
        goals_enabled,
    ))
}

fn validate_agents(codex_dir: &Path) -> Result<AgentCounts> {
    let root = codex_dir.join("agents");
    let mut agent_files = 0;
    let mut models = BTreeMap::new();
    let mut efforts = BTreeMap::new();
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
    Ok((agent_files, models, efforts))
}

fn validate_hooks(codex_dir: &Path) -> Result<(Vec<String>, usize)> {
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
                validate_hook_entry(&path, event, entry)?;
                handlers += 1;
            }
        }
    }

    if handlers == 0 {
        bail!("{} has no active Codex hook handlers", path.display());
    }
    events.sort();
    Ok((events, handlers))
}

fn validate_hook_entry(path: &Path, event: &str, entry: &serde_json::Value) -> Result<()> {
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
    Ok(())
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
