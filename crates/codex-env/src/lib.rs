use std::collections::BTreeSet;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, Context, Result};
use mlua::{Lua, Value};
use serde::{Deserialize, Serialize};
use serde_json::json;
use serde_json::Value as JsonValue;
use toml_edit::{value, DocumentMut, Item, Table};

mod agent_roles;
mod command_prompts;
mod doctor;
mod generated;
mod raw_mirror;

use agent_roles::{
    claude_agent_role_plan, clean_claude_agent_roles, stale_claude_agent_role_files,
};
use command_prompts::{clean_codex_prompts, command_prompt_plan, stale_codex_prompt_files};
pub use doctor::{doctor_codex_surface, DoctorOptions, DoctorReport};
use generated::{
    codex_agent_profiles, codex_agent_teams_json, codex_agents_md, codex_automation_graph_json,
    codex_config, codex_hooks_json, codex_native_workflow_prompts, codex_native_workflow_skills,
    codex_runtime_hook_plan, command_skill_plan, copy_tree_plan, read_claude_env,
};
use raw_mirror::{
    claude_source_files, clean_raw_mirror, mirror_symbol_inventory_json, raw_claude_mirror_plan,
    stale_raw_mirror_files,
};

#[derive(Debug, Clone)]
pub struct MirrorOptions {
    pub repo_root: PathBuf,
    pub lua_policy: Option<PathBuf>,
    pub check: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct MirrorReport {
    pub repo_root: PathBuf,
    pub claude_dir: PathBuf,
    pub codex_dir: PathBuf,
    pub total_files: usize,
    pub changed_files: usize,
    pub verified_files: usize,
    pub generated: Vec<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct PromptInstallOptions {
    pub repo_root: PathBuf,
    pub codex_home: PathBuf,
    pub check: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct PromptInstallReport {
    pub repo_root: PathBuf,
    pub source_dir: PathBuf,
    pub target_dir: PathBuf,
    pub total_files: usize,
    pub changed_files: usize,
    pub verified_files: usize,
    pub removed_files: Vec<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct CodexInventoryOptions {
    pub repo_root: PathBuf,
    pub lua_policy: Option<PathBuf>,
    pub codex_home: PathBuf,
}

#[derive(Debug, Clone, Serialize)]
pub struct CodexInventoryReport {
    pub repo_root: PathBuf,
    pub codex_home: PathBuf,
    pub claude: CodexInventoryClaudeCounts,
    pub codex: CodexInventoryCodexCounts,
    pub expected: CodexInventoryExpectedCounts,
    pub gaps: Vec<String>,
    pub doctor: DoctorReport,
}

#[derive(Debug, Clone, Serialize)]
pub struct CodexInventoryClaudeCounts {
    pub command_files: usize,
    pub agent_files: usize,
    pub hook_files: usize,
    pub helper_files: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct CodexInventoryCodexCounts {
    pub prompt_files: usize,
    pub prompt_alias_files: usize,
    pub installed_prompt_files: usize,
    pub source_command_skills: usize,
    pub skill_entrypoints: usize,
    pub claude_agent_profiles: usize,
    pub agent_profiles: usize,
    pub hook_files: usize,
    pub helper_files: usize,
    pub helper_mirror_files: usize,
    pub agent_teams: usize,
    pub agent_team_members: usize,
    pub mcp_servers: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct CodexInventoryExpectedCounts {
    pub command_prompt_files: usize,
    pub workflow_prompt_files: usize,
    pub prompt_files: usize,
    pub source_command_skills: usize,
    pub workflow_skills: usize,
    pub copied_skill_files: usize,
    pub claude_agent_profiles: usize,
    pub hook_files: usize,
    pub helper_mirror_files: usize,
}

#[derive(Debug, Clone)]
pub struct CodexInstallOptions {
    pub repo_root: PathBuf,
    pub lua_policy: Option<PathBuf>,
    pub codex_home: PathBuf,
}

#[derive(Debug, Clone, Serialize)]
pub struct CodexInstallReport {
    pub mirror: MirrorReport,
    pub prompts: PromptInstallReport,
    pub home_settings: CodexHomeSettingsReport,
    pub doctor: DoctorReport,
}

#[derive(Debug, Clone)]
pub struct CodexRunOptions {
    pub repo_root: PathBuf,
    pub lua_policy: Option<PathBuf>,
    pub codex_home: PathBuf,
    pub goal: Option<String>,
    pub prompt_file: Option<PathBuf>,
    pub output_dir: Option<PathBuf>,
    pub dry_run: bool,
    pub skip_install: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct CodexRunReport {
    pub repo_root: PathBuf,
    pub codex_home: PathBuf,
    pub run_dir: PathBuf,
    pub prompt_path: PathBuf,
    pub events_path: PathBuf,
    pub stderr_path: PathBuf,
    pub last_message_path: PathBuf,
    pub status_path: PathBuf,
    pub dry_run: bool,
    pub exit_code: Option<i32>,
}

#[derive(Debug, Clone)]
pub struct CodexTeamRunOptions {
    pub repo_root: PathBuf,
    pub lua_policy: Option<PathBuf>,
    pub codex_home: PathBuf,
    pub team: String,
    pub goal: Option<String>,
    pub prompt_file: Option<PathBuf>,
    pub output_dir: Option<PathBuf>,
    pub member_sandbox_mode: String,
    pub dry_run: bool,
    pub skip_install: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct CodexTeamRunReport {
    pub repo_root: PathBuf,
    pub codex_home: PathBuf,
    pub team: String,
    pub strategy: String,
    pub run_dir: PathBuf,
    pub status_path: PathBuf,
    pub consolidation_prompt_path: PathBuf,
    pub consolidation_run: CodexRunReport,
    pub member_sandbox_mode: String,
    pub members: Vec<CodexTeamRunMemberReport>,
    pub dry_run: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct CodexTeamRunMemberReport {
    pub agent: String,
    pub description: String,
    pub model: String,
    pub reasoning_effort: String,
    pub sandbox_mode: String,
    pub profile_path: PathBuf,
    pub run: CodexRunReport,
}

#[derive(Debug, Clone)]
pub struct CodexAutoLoopOptions {
    pub repo_root: PathBuf,
    pub lua_policy: Option<PathBuf>,
    pub codex_home: PathBuf,
    pub team: String,
    pub goal: Option<String>,
    pub prompt_file: Option<PathBuf>,
    pub output_dir: Option<PathBuf>,
    pub max_iterations: usize,
    pub member_sandbox_mode: String,
    pub dry_run: bool,
    pub skip_install: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct CodexAutoLoopReport {
    pub repo_root: PathBuf,
    pub codex_home: PathBuf,
    pub team: String,
    pub run_dir: PathBuf,
    pub status_path: PathBuf,
    pub max_iterations: usize,
    pub completed: bool,
    pub completion_marker: Option<String>,
    pub iterations: Vec<CodexAutoLoopIterationReport>,
    pub dry_run: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct CodexAutoLoopIterationReport {
    pub iteration: usize,
    pub marker: Option<String>,
    pub team_run: CodexTeamRunReport,
}

#[derive(Debug, Clone)]
pub struct CodexTddWorkflowOptions {
    pub repo_root: PathBuf,
    pub lua_policy: Option<PathBuf>,
    pub codex_home: PathBuf,
    pub output_dir: Option<PathBuf>,
    pub team: String,
    pub goal: Option<String>,
    pub dry_run: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct CodexTddWorkflowReport {
    pub repo_root: PathBuf,
    pub codex_home: PathBuf,
    pub run_dir: PathBuf,
    pub status_path: PathBuf,
    pub extraction_report_path: PathBuf,
    pub extraction_plan_path: PathBuf,
    pub operator_role: String,
    pub supervision_protocol: Vec<String>,
    pub dry_run: bool,
    pub steps: Vec<CodexTddWorkflowStepReport>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodexTddExtractionPlan {
    pub schema_version: u8,
    pub generated_by: String,
    pub goal: String,
    pub target_crate: String,
    pub forbidden_target: String,
    pub source_material: String,
    pub runtime_representation: String,
    pub next_action: String,
    pub actions: Vec<CodexTddExtractionAction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodexTddExtractionAction {
    pub step: String,
    pub status: String,
    pub worker_state: String,
    pub crate_owner: String,
    pub belongs_in: String,
    pub extraction_target: String,
    pub next_action: String,
    pub evidence_stdout: PathBuf,
    pub evidence_stderr: PathBuf,
}

#[derive(Debug, Clone)]
pub struct CodexTddNextActionOptions {
    pub repo_root: PathBuf,
    pub plan_path: Option<PathBuf>,
    pub check: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct CodexTddNextActionReport {
    pub repo_root: PathBuf,
    pub plan_path: PathBuf,
    pub target_crate: String,
    pub forbidden_target: String,
    pub next_action: String,
    pub selected_actions: Vec<CodexTddExtractionAction>,
    pub ready_for_autonomous_loop: bool,
    pub status_summary: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct CodexTddAutoLoopOptions {
    pub repo_root: PathBuf,
    pub lua_policy: Option<PathBuf>,
    pub codex_home: PathBuf,
    pub plan_path: Option<PathBuf>,
    pub team: String,
    pub output_dir: Option<PathBuf>,
    pub max_iterations: usize,
    pub member_sandbox_mode: String,
    pub dry_run: bool,
    pub skip_install: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct CodexTddAutoLoopReport {
    pub next_action: CodexTddNextActionReport,
    pub auto_loop: CodexAutoLoopReport,
    pub handoff_goal: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct CodexTddWorkflowStepReport {
    pub name: String,
    pub command: String,
    pub rationale: String,
    pub does: String,
    pub why: String,
    pub crate_owner: String,
    pub belongs_in: String,
    pub extraction_target: String,
    pub supervision_action: String,
    pub worker_state: String,
    pub stdout_path: PathBuf,
    pub stderr_path: PathBuf,
    pub supervision_events: Vec<String>,
    pub started_unix_seconds: Option<u64>,
    pub ended_unix_seconds: Option<u64>,
    pub status: String,
    pub exit_code: Option<i32>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CodexHomeSettingsReport {
    pub config_path: PathBuf,
    pub changed: bool,
    pub model: String,
    pub model_reasoning_effort: String,
    pub model_catalog_json: String,
    pub approval_policy: String,
    pub approvals_reviewer: String,
    pub model_context_window: i64,
    pub multi_agent_enabled: bool,
    pub goals_enabled: bool,
    pub include_skill_instructions: bool,
}

pub const REQUIRED_CODEX_MODEL: &str = "gpt-5.5";
pub const REQUIRED_CODEX_REASONING_EFFORT: &str = "high";
pub const REQUIRED_CODEX_APPROVAL_POLICY: &str = "on-request";
pub const REQUIRED_CODEX_APPROVALS_REVIEWER: &str = "auto_review";
pub const REQUIRED_CODEX_CONTEXT_WINDOW: i64 = 4_000_000;
pub const REQUIRED_CODEX_MODEL_CATALOG: &str = "model-catalog.json";

#[derive(Debug, Default)]
struct LuaPolicy {
    config_footer: Option<String>,
    skill_prelude: Option<String>,
}

#[derive(Debug)]
struct PlannedFile {
    path: PathBuf,
    bytes: Vec<u8>,
    executable: bool,
}

pub fn mirror_codex_surface(options: MirrorOptions) -> Result<MirrorReport> {
    let repo_root = options.repo_root.canonicalize().with_context(|| {
        format!(
            "failed to canonicalize repo root {}",
            options.repo_root.display()
        )
    })?;
    let claude_dir = locate_claude_dir(&repo_root)?;
    let codex_dir = repo_root.join(".codex");
    let policy = load_lua_policy(options.lua_policy.as_deref(), &repo_root, &claude_dir)?;
    let claude_files = claude_source_files(&repo_root, &claude_dir)?;
    let agent_role_plan = claude_agent_role_plan(&claude_dir.join("agents"), &codex_dir)?;
    let prompt_plan = command_prompt_plan(&claude_dir.join("commands"), &codex_dir)?;
    let mut planned = Vec::new();

    planned.extend(raw_claude_mirror_plan(
        &repo_root,
        &codex_dir,
        &claude_files,
    )?);
    planned.push(PlannedFile {
        path: codex_dir.join("config.toml"),
        bytes: codex_config(
            &read_claude_env(&claude_dir)?,
            &agent_role_plan.roles,
            policy.config_footer.as_deref(),
        )
        .into_bytes(),
        executable: false,
    });
    planned.push(PlannedFile {
        path: codex_dir.join(REQUIRED_CODEX_MODEL_CATALOG),
        bytes: generated::codex_model_catalog_json().into_bytes(),
        executable: false,
    });
    planned.push(PlannedFile {
        path: codex_dir.join("AGENTS.md"),
        bytes: codex_agents_md().into_bytes(),
        executable: false,
    });
    planned.push(codex_agent_teams_json(&codex_dir, &agent_role_plan.roles)?);
    planned.push(codex_automation_graph_json(
        &codex_dir,
        &claude_dir,
        &claude_files,
        &agent_role_plan.roles,
    )?);
    planned.extend(codex_agent_profiles(&codex_dir));
    planned.extend(agent_role_plan.files);
    planned.extend(prompt_plan.files);
    planned.extend(codex_native_workflow_prompts(
        &codex_dir,
        &agent_role_plan.roles,
    ));
    planned.extend(copy_tree_plan(
        &claude_dir.join("helpers"),
        &codex_dir.join("helpers"),
    )?);
    planned.extend(codex_prompt_helpers(&codex_dir));
    planned.push(PlannedFile {
        path: codex_dir.join("hooks.json"),
        bytes: codex_hooks_json(&claude_dir)?.into_bytes(),
        executable: false,
    });
    planned.extend(codex_runtime_hook_plan(
        &claude_dir.join("hooks"),
        &codex_dir.join("hooks"),
    )?);
    planned.extend(copy_tree_plan(
        &claude_dir.join("skills"),
        &repo_root.join(".agents/skills"),
    )?);
    planned.extend(command_skill_plan(
        &claude_dir.join("commands"),
        &repo_root.join(".agents/skills"),
        policy.skill_prelude.as_deref(),
    )?);
    planned.extend(codex_native_workflow_skills(
        &repo_root.join(".agents/skills"),
        &agent_role_plan.roles,
    ));
    planned.push(PlannedFile {
        path: codex_dir.join("mirror-symbols.json"),
        bytes: mirror_symbol_inventory_json(&repo_root, &codex_dir, &claude_files)?.into_bytes(),
        executable: false,
    });

    let manifest_path = codex_dir.join("mirror-manifest.json");
    let manifest = manifest_json(&repo_root, &claude_dir, &planned, &manifest_path)?;
    planned.push(PlannedFile {
        path: manifest_path,
        bytes: manifest.into_bytes(),
        executable: false,
    });

    let mut changed_files = 0;
    let mut verified_files = 0;
    let mut generated = Vec::new();
    let stale_raw_mirror_files = stale_raw_mirror_files(&repo_root, &codex_dir, &claude_files)?;
    let stale_agent_role_files = stale_claude_agent_role_files(&repo_root, &codex_dir, &planned)?;
    let stale_prompt_files = stale_codex_prompt_files(&repo_root, &codex_dir, &planned)?;

    for file in &planned {
        let exists_with_same_content = fs::read(&file.path).is_ok_and(|bytes| bytes == file.bytes);
        if exists_with_same_content {
            verified_files += 1;
        } else {
            changed_files += 1;
        }
        generated.push(strip_repo_prefix(&repo_root, &file.path));
    }

    if !options.check {
        clean_raw_mirror(&codex_dir)?;
        clean_claude_agent_roles(&codex_dir)?;
        clean_codex_prompts(&codex_dir)?;
    }

    for file in &planned {
        if !options.check {
            write_file(file)?;
        }
    }

    if options.check && !stale_raw_mirror_files.is_empty() {
        return Err(anyhow!(
            "Codex raw mirror has {} stale file(s): {}",
            stale_raw_mirror_files.len(),
            stale_raw_mirror_files
                .iter()
                .take(5)
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }

    if options.check && !stale_agent_role_files.is_empty() {
        return Err(anyhow!(
            "Codex Claude agent roles have {} stale file(s): {}",
            stale_agent_role_files.len(),
            stale_agent_role_files
                .iter()
                .take(5)
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }

    if options.check && !stale_prompt_files.is_empty() {
        return Err(anyhow!(
            "Codex prompt mirror has {} stale file(s): {}",
            stale_prompt_files.len(),
            stale_prompt_files
                .iter()
                .take(5)
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }

    if options.check && changed_files > 0 {
        return Err(anyhow!(
            "Codex mirror is stale: {changed_files} generated file(s) differ"
        ));
    }

    Ok(MirrorReport {
        repo_root,
        claude_dir,
        codex_dir,
        total_files: planned.len(),
        changed_files,
        verified_files,
        generated,
    })
}

pub fn install_codex_prompts(options: PromptInstallOptions) -> Result<PromptInstallReport> {
    let repo_root = options.repo_root.canonicalize().with_context(|| {
        format!(
            "failed to canonicalize repo root {}",
            options.repo_root.display()
        )
    })?;
    let source_dir = repo_root.join(".codex/prompts");
    let target_dir = source_dir.clone();
    if !source_dir.exists() {
        return Err(anyhow!(
            "{} does not exist; run `cargo run -p codex-env -- mirror` first",
            source_dir.display()
        ));
    }

    let mut planned = Vec::new();
    for entry in fs::read_dir(&source_dir)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() || path.extension().and_then(|value| value.to_str()) != Some("md") {
            continue;
        }
        planned.push(PlannedFile {
            path: path.clone(),
            bytes: fs::read(path)?,
            executable: false,
        });
    }
    planned.sort_by(|a, b| a.path.cmp(&b.path));

    let mut changed_files = 0;
    let mut verified_files = 0;
    let planned_paths: BTreeSet<PathBuf> = planned.iter().map(|file| file.path.clone()).collect();
    let stale_files = stale_installed_prompt_files(&target_dir, &planned_paths)?;
    for file in &planned {
        let exists_with_same_content = fs::read(&file.path).is_ok_and(|bytes| bytes == file.bytes);
        if exists_with_same_content {
            verified_files += 1;
        } else {
            changed_files += 1;
        }
    }
    if !options.check {
        for path in &stale_files {
            fs::remove_file(path).with_context(|| {
                format!(
                    "failed to remove stale Codex home prompt {}",
                    path.display()
                )
            })?;
        }
    }

    if options.check && changed_files > 0 {
        return Err(anyhow!(
            "Codex repo-local prompts are stale: {changed_files} prompt file(s) differ"
        ));
    }
    if options.check && !stale_files.is_empty() {
        return Err(anyhow!(
            "Codex home prompts include {} stale file(s): {}",
            stale_files.len(),
            stale_files
                .iter()
                .take(5)
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }

    Ok(PromptInstallReport {
        repo_root,
        source_dir,
        target_dir,
        total_files: planned.len(),
        changed_files,
        verified_files,
        removed_files: stale_files,
    })
}

fn stale_installed_prompt_files(
    target_dir: &Path,
    planned_paths: &BTreeSet<PathBuf>,
) -> Result<Vec<PathBuf>> {
    let mut stale = Vec::new();
    if !target_dir.exists() {
        return Ok(stale);
    }
    for entry in fs::read_dir(target_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file()
            && path.extension().and_then(|value| value.to_str()) == Some("md")
            && !planned_paths.contains(&path)
        {
            stale.push(path);
        }
    }
    stale.sort();
    Ok(stale)
}

fn count_files_recursive(root: &Path) -> Result<usize> {
    if !root.exists() {
        return Ok(0);
    }
    let mut count = 0;
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            count += count_files_recursive(&path)?;
        } else if path.is_file() {
            count += 1;
        }
    }
    Ok(count)
}

fn count_files_with_extension(root: &Path, extension: &str) -> Result<usize> {
    if !root.exists() {
        return Ok(0);
    }
    let mut count = 0;
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            count += count_files_with_extension(&path, extension)?;
        } else if path.is_file()
            && path.extension().and_then(|value| value.to_str()) == Some(extension)
        {
            count += 1;
        }
    }
    Ok(count)
}

fn count_markdown_files(root: &Path) -> Result<usize> {
    count_files_with_extension(root, "md")
}

fn count_skill_entrypoints(root: &Path) -> Result<usize> {
    if !root.exists() {
        return Ok(0);
    }
    let mut count = 0;
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            count += count_skill_entrypoints(&path)?;
        } else if path.file_name().and_then(|value| value.to_str()) == Some("SKILL.md") {
            count += 1;
        }
    }
    Ok(count)
}

fn count_source_command_skills(skills_dir: &Path) -> Result<usize> {
    if !skills_dir.exists() {
        return Ok(0);
    }
    let mut count = 0;
    for entry in fs::read_dir(skills_dir)? {
        let path = entry?.path();
        if path.is_dir()
            && path
                .file_name()
                .and_then(|value| value.to_str())
                .is_some_and(|name| name.starts_with("source-command-"))
            && path.join("SKILL.md").is_file()
        {
            count += 1;
        }
    }
    Ok(count)
}

fn mismatched_planned_files(repo_root: &Path, planned: &[PlannedFile]) -> Result<Vec<PathBuf>> {
    let mut mismatches = Vec::new();
    for file in planned {
        let Ok(actual) = fs::read(&file.path) else {
            mismatches.push(strip_repo_prefix(repo_root, &file.path));
            continue;
        };
        if actual != file.bytes || is_executable(&file.path)? != file.executable {
            mismatches.push(strip_repo_prefix(repo_root, &file.path));
        }
    }
    mismatches.sort();
    Ok(mismatches)
}

pub fn inventory_codex_surface(options: CodexInventoryOptions) -> Result<CodexInventoryReport> {
    let repo_root = options.repo_root.canonicalize().with_context(|| {
        format!(
            "failed to canonicalize repo root {}",
            options.repo_root.display()
        )
    })?;
    let claude_dir = locate_claude_dir(&repo_root)?;
    let codex_dir = repo_root.join(".codex");
    let skills_dir = repo_root.join(".agents/skills");
    let agent_role_plan = claude_agent_role_plan(&claude_dir.join("agents"), &codex_dir)?;
    let command_prompt_plan = command_prompt_plan(&claude_dir.join("commands"), &codex_dir)?;
    let workflow_prompts = codex_native_workflow_prompts(&codex_dir, &agent_role_plan.roles);
    let copied_skill_plan = copy_tree_plan(&claude_dir.join("skills"), &skills_dir)?;
    let command_skill_plan = command_skill_plan(&claude_dir.join("commands"), &skills_dir, None)?;
    let workflow_skills = codex_native_workflow_skills(&skills_dir, &agent_role_plan.roles);
    let source_hook_files = count_files_recursive(&claude_dir.join("hooks"))?;
    let hook_plan = codex_runtime_hook_plan(&claude_dir.join("hooks"), &codex_dir.join("hooks"))?;
    let helper_plan = copy_tree_plan(&claude_dir.join("helpers"), &codex_dir.join("helpers"))?;

    let codex_home = options.codex_home;
    let doctor = doctor_codex_surface(DoctorOptions {
        repo_root: repo_root.clone(),
        lua_policy: options.lua_policy,
        codex_home: codex_home.clone(),
    })?;

    let mut skill_plan = copied_skill_plan;
    skill_plan.extend(command_skill_plan);
    skill_plan.extend(workflow_skills);

    let mut gaps = Vec::new();
    let command_prompt_files = command_prompt_plan.files.len();
    let workflow_prompt_files = workflow_prompts.len();
    let expected_prompt_files = command_prompt_files + workflow_prompt_files;
    if doctor.prompt_files != expected_prompt_files {
        gaps.push(format!(
            "expected {expected_prompt_files} Codex prompt files but found {}",
            doctor.prompt_files
        ));
    }
    if doctor.installed_prompt_files != doctor.prompt_files {
        gaps.push(format!(
            "installed prompt count {} does not match generated prompt count {}",
            doctor.installed_prompt_files, doctor.prompt_files
        ));
    }

    let expected_source_command_skills = skill_plan
        .iter()
        .filter(|file| {
            file.path
                .parent()
                .and_then(|path| path.file_name())
                .and_then(|value| value.to_str())
                .is_some_and(|name| name.starts_with("source-command-"))
                && file.path.file_name().and_then(|value| value.to_str()) == Some("SKILL.md")
        })
        .count();
    let source_command_skills = count_source_command_skills(&skills_dir)?;
    if source_command_skills != expected_source_command_skills {
        gaps.push(format!(
            "expected {expected_source_command_skills} source-command skills but found {source_command_skills}"
        ));
    }

    let skill_mismatches = mismatched_planned_files(&repo_root, &skill_plan)?;
    if !skill_mismatches.is_empty() {
        gaps.push(format!(
            "skill mirror has {} missing or stale file(s); first: {}",
            skill_mismatches.len(),
            skill_mismatches[0].display()
        ));
    }

    let hook_mismatches = mismatched_planned_files(&repo_root, &hook_plan)?;
    if !hook_mismatches.is_empty() {
        gaps.push(format!(
            "hook mirror has {} missing or stale file(s); first: {}",
            hook_mismatches.len(),
            hook_mismatches[0].display()
        ));
    }

    let helper_mismatches = mismatched_planned_files(&repo_root, &helper_plan)?;
    if !helper_mismatches.is_empty() {
        gaps.push(format!(
            "helper mirror has {} missing or stale file(s); first: {}",
            helper_mismatches.len(),
            helper_mismatches[0].display()
        ));
    }

    if doctor.agent_files != doctor.config_agent_entries {
        gaps.push(format!(
            "config has {} agent entries but {} agent files",
            doctor.config_agent_entries, doctor.agent_files
        ));
    }
    if doctor.agent_files < agent_role_plan.files.len() {
        gaps.push(format!(
            "expected at least {} Claude agent profiles but only {} agent files were verified",
            agent_role_plan.files.len(),
            doctor.agent_files
        ));
    }
    if doctor.claude_helper_files != helper_plan.len() {
        gaps.push(format!(
            "expected {} helper mirror files but doctor verified {}",
            helper_plan.len(),
            doctor.claude_helper_files
        ));
    }

    Ok(CodexInventoryReport {
        repo_root: repo_root.clone(),
        codex_home,
        claude: CodexInventoryClaudeCounts {
            command_files: count_markdown_files(&claude_dir.join("commands"))?,
            agent_files: count_files_recursive(&claude_dir.join("agents"))?,
            hook_files: source_hook_files,
            helper_files: helper_plan.len(),
        },
        codex: CodexInventoryCodexCounts {
            prompt_files: doctor.prompt_files,
            prompt_alias_files: doctor.prompt_alias_files,
            installed_prompt_files: doctor.installed_prompt_files,
            source_command_skills,
            skill_entrypoints: count_skill_entrypoints(&skills_dir)?,
            claude_agent_profiles: count_files_with_extension(
                &codex_dir.join("agents/claude"),
                "toml",
            )?,
            agent_profiles: doctor.agent_files,
            hook_files: count_files_recursive(&codex_dir.join("hooks"))?,
            helper_files: count_files_recursive(&codex_dir.join("helpers"))?,
            helper_mirror_files: doctor.claude_helper_files,
            agent_teams: doctor.agent_teams,
            agent_team_members: doctor.agent_team_members,
            mcp_servers: doctor.config_mcp_servers.len(),
        },
        expected: CodexInventoryExpectedCounts {
            command_prompt_files,
            workflow_prompt_files,
            prompt_files: expected_prompt_files,
            source_command_skills: expected_source_command_skills,
            workflow_skills: 4,
            copied_skill_files: skill_plan
                .len()
                .saturating_sub(expected_source_command_skills)
                .saturating_sub(4),
            claude_agent_profiles: agent_role_plan.files.len(),
            hook_files: hook_plan.len(),
            helper_mirror_files: helper_plan.len(),
        },
        gaps,
        doctor,
    })
}

pub fn install_codex_env(options: CodexInstallOptions) -> Result<CodexInstallReport> {
    let mirror = mirror_codex_surface(MirrorOptions {
        repo_root: options.repo_root.clone(),
        lua_policy: options.lua_policy.clone(),
        check: false,
    })?;
    let prompts = install_codex_prompts(PromptInstallOptions {
        repo_root: options.repo_root.clone(),
        codex_home: options.codex_home.clone(),
        check: false,
    })?;
    let home_settings = ensure_codex_home_settings(&options.codex_home)?;
    let doctor = doctor_codex_surface(DoctorOptions {
        repo_root: options.repo_root,
        lua_policy: options.lua_policy,
        codex_home: options.codex_home,
    })?;

    Ok(CodexInstallReport {
        mirror,
        prompts,
        home_settings,
        doctor,
    })
}

pub fn run_codex_task(options: CodexRunOptions) -> Result<CodexRunReport> {
    let repo_root = options.repo_root.canonicalize().with_context(|| {
        format!(
            "failed to canonicalize repo root {}",
            options.repo_root.display()
        )
    })?;
    let codex_home = options.codex_home;
    let goal = resolve_run_goal(options.goal.as_deref(), options.prompt_file.as_deref())?;

    if options.skip_install {
        doctor_codex_surface(DoctorOptions {
            repo_root: repo_root.clone(),
            lua_policy: options.lua_policy,
            codex_home: codex_home.clone(),
        })?;
    } else {
        install_codex_env(CodexInstallOptions {
            repo_root: repo_root.clone(),
            lua_policy: options.lua_policy,
            codex_home: codex_home.clone(),
        })?;
    }

    let run_dir = options
        .output_dir
        .unwrap_or_else(|| repo_root.join(".codex/harness/runs").join(run_id(&goal)));
    fs::create_dir_all(&run_dir)
        .with_context(|| format!("failed to create {}", run_dir.display()))?;

    let prompt = codex_run_prompt(&goal);
    let prompt_path = run_dir.join("prompt.md");
    let events_path = run_dir.join("events.jsonl");
    let stderr_path = run_dir.join("stderr.log");
    let last_message_path = run_dir.join("last-message.md");
    let status_path = run_dir.join("status.json");
    fs::write(&prompt_path, &prompt)
        .with_context(|| format!("failed to write {}", prompt_path.display()))?;

    let mut report = CodexRunReport {
        repo_root: repo_root.clone(),
        codex_home: codex_home.clone(),
        run_dir: run_dir.clone(),
        prompt_path,
        events_path,
        stderr_path,
        last_message_path,
        status_path,
        dry_run: options.dry_run,
        exit_code: None,
    };

    if options.dry_run {
        write_run_status(&report)?;
        return Ok(report);
    }

    let events = fs::File::create(&report.events_path)
        .with_context(|| format!("failed to create {}", report.events_path.display()))?;
    let stderr = fs::File::create(&report.stderr_path)
        .with_context(|| format!("failed to create {}", report.stderr_path.display()))?;
    let mut child = Command::new("codex")
        .arg("exec")
        .arg("--json")
        .arg("--cd")
        .arg(&repo_root)
        .arg("--sandbox")
        .arg("workspace-write")
        .arg("--config")
        .arg("approval_policy=\"never\"")
        .arg("--output-last-message")
        .arg(&report.last_message_path)
        .arg("-")
        .env("CODEX_HOME", &codex_home)
        .stdin(Stdio::piped())
        .stdout(Stdio::from(events))
        .stderr(Stdio::from(stderr))
        .spawn()
        .with_context(|| "failed to spawn codex exec")?;
    child
        .stdin
        .take()
        .ok_or_else(|| anyhow!("failed to open codex exec stdin"))?
        .write_all(prompt.as_bytes())
        .with_context(|| "failed to write codex exec prompt")?;
    let status = child
        .wait()
        .with_context(|| "failed to wait for codex exec")?;
    report.exit_code = status.code();
    write_run_status(&report)?;
    if !status.success() {
        return Err(anyhow!(
            "codex-env run failed with exit code {}; see {} and {}",
            status
                .code()
                .map_or_else(|| "signal".to_owned(), |code| code.to_string()),
            report.events_path.display(),
            report.stderr_path.display()
        ));
    }
    Ok(report)
}

pub fn run_codex_team(options: CodexTeamRunOptions) -> Result<CodexTeamRunReport> {
    let repo_root = options.repo_root.canonicalize().with_context(|| {
        format!(
            "failed to canonicalize repo root {}",
            options.repo_root.display()
        )
    })?;
    let codex_home = options.codex_home;
    let goal = resolve_run_goal(options.goal.as_deref(), options.prompt_file.as_deref())?;

    if options.skip_install {
        doctor_codex_surface(DoctorOptions {
            repo_root: repo_root.clone(),
            lua_policy: options.lua_policy,
            codex_home: codex_home.clone(),
        })?;
    } else {
        install_codex_env(CodexInstallOptions {
            repo_root: repo_root.clone(),
            lua_policy: options.lua_policy,
            codex_home: codex_home.clone(),
        })?;
    }

    let codex_dir = repo_root.join(".codex");
    let team = load_team(&codex_dir, &options.team)?;
    let profiles = load_team_agent_profiles(&codex_dir, &team)?;
    let member_sandbox_mode = validate_member_sandbox_mode(&options.member_sandbox_mode)?;
    let run_dir = options.output_dir.unwrap_or_else(|| {
        repo_root
            .join(".codex/harness/runs")
            .join(format!("{}-team-{}", run_id(&goal), team.name))
    });
    fs::create_dir_all(&run_dir)
        .with_context(|| format!("failed to create {}", run_dir.display()))?;

    let mut members = Vec::new();
    let mut children = Vec::new();
    for profile in profiles {
        let agent_dir = run_dir.join("agents").join(&profile.name);
        let prompt = codex_team_member_prompt(&goal, &team, &profile, &member_sandbox_mode);
        let report =
            prepared_run_report(&repo_root, &codex_home, agent_dir, prompt, options.dry_run)?;
        let child = if options.dry_run {
            None
        } else {
            Some(spawn_codex_exec(
                &repo_root,
                &codex_home,
                &report,
                &member_sandbox_mode,
                &profile.model,
                &profile.model_reasoning_effort,
            )?)
        };
        let member = CodexTeamRunMemberReport {
            agent: profile.name,
            description: profile.description,
            model: profile.model,
            reasoning_effort: profile.model_reasoning_effort,
            sandbox_mode: member_sandbox_mode.clone(),
            profile_path: profile.path,
            run: report.clone(),
        };
        if child.is_none() {
            write_run_status(&report)?;
        }
        if let Some(child) = child {
            children.push((members.len(), child));
        }
        members.push(member);
    }

    for (index, mut child) in children {
        let status = child
            .wait()
            .with_context(|| "failed to wait for codex exec team member")?;
        members[index].run.exit_code = status.code();
        write_run_status(&members[index].run)?;
        if !status.success() {
            return Err(anyhow!(
                "codex-env team-run member {} failed with exit code {}; see {} and {}",
                members[index].agent,
                status
                    .code()
                    .map_or_else(|| "signal".to_owned(), |code| code.to_string()),
                members[index].run.events_path.display(),
                members[index].run.stderr_path.display()
            ));
        }
    }

    let consolidation_prompt_path = run_dir.join("consolidation-prompt.md");
    fs::write(
        &consolidation_prompt_path,
        codex_team_consolidation_prompt(&goal, &team, &members),
    )
    .with_context(|| format!("failed to write {}", consolidation_prompt_path.display()))?;
    let consolidation_prompt = fs::read_to_string(&consolidation_prompt_path)
        .with_context(|| format!("failed to read {}", consolidation_prompt_path.display()))?;
    let mut consolidation_run = prepared_run_report(
        &repo_root,
        &codex_home,
        run_dir.join("consolidation"),
        consolidation_prompt,
        options.dry_run,
    )?;
    if options.dry_run {
        write_run_status(&consolidation_run)?;
    } else {
        let mut child = spawn_codex_exec(
            &repo_root,
            &codex_home,
            &consolidation_run,
            "workspace-write",
            REQUIRED_CODEX_MODEL,
            REQUIRED_CODEX_REASONING_EFFORT,
        )?;
        let status = child
            .wait()
            .with_context(|| "failed to wait for codex exec team consolidation")?;
        consolidation_run.exit_code = status.code();
        write_run_status(&consolidation_run)?;
        if !status.success() {
            return Err(anyhow!(
                "codex-env team-run consolidation failed with exit code {}; see {} and {}",
                status
                    .code()
                    .map_or_else(|| "signal".to_owned(), |code| code.to_string()),
                consolidation_run.events_path.display(),
                consolidation_run.stderr_path.display()
            ));
        }
    }
    let status_path = run_dir.join("team-status.json");
    let report = CodexTeamRunReport {
        repo_root,
        codex_home,
        team: team.name,
        strategy: team.strategy,
        run_dir,
        status_path,
        consolidation_prompt_path,
        consolidation_run,
        member_sandbox_mode,
        members,
        dry_run: options.dry_run,
    };
    write_team_run_status(&report)?;
    Ok(report)
}

pub fn run_codex_auto_loop(options: CodexAutoLoopOptions) -> Result<CodexAutoLoopReport> {
    if options.max_iterations == 0 || options.max_iterations > 20 {
        return Err(anyhow!(
            "codex-env auto-loop requires --max-iterations between 1 and 20"
        ));
    }
    let repo_root = options.repo_root.canonicalize().with_context(|| {
        format!(
            "failed to canonicalize repo root {}",
            options.repo_root.display()
        )
    })?;
    let codex_home = options.codex_home;
    let goal = resolve_run_goal(options.goal.as_deref(), options.prompt_file.as_deref())?;
    let member_sandbox_mode = validate_member_sandbox_mode(&options.member_sandbox_mode)?;

    if options.skip_install {
        doctor_codex_surface(DoctorOptions {
            repo_root: repo_root.clone(),
            lua_policy: options.lua_policy.clone(),
            codex_home: codex_home.clone(),
        })?;
    } else {
        install_codex_env(CodexInstallOptions {
            repo_root: repo_root.clone(),
            lua_policy: options.lua_policy.clone(),
            codex_home: codex_home.clone(),
        })?;
    }

    let run_dir = options.output_dir.unwrap_or_else(|| {
        repo_root
            .join(".codex/harness/runs")
            .join(format!("{}-auto-loop", run_id(&goal)))
    });
    fs::create_dir_all(&run_dir)
        .with_context(|| format!("failed to create {}", run_dir.display()))?;

    let iteration_count = if options.dry_run {
        1
    } else {
        options.max_iterations
    };
    let mut iterations = Vec::new();
    let mut completed = false;
    let mut completion_marker = None;

    for iteration in 1..=iteration_count {
        let iteration_goal = codex_auto_loop_goal(&goal, iteration, options.max_iterations);
        let team_run = run_codex_team(CodexTeamRunOptions {
            repo_root: repo_root.clone(),
            lua_policy: options.lua_policy.clone(),
            codex_home: codex_home.clone(),
            team: options.team.clone(),
            goal: Some(iteration_goal),
            prompt_file: None,
            output_dir: Some(run_dir.join(format!("iteration-{iteration:02}"))),
            member_sandbox_mode: member_sandbox_mode.clone(),
            dry_run: options.dry_run,
            skip_install: true,
        })?;
        let marker = if options.dry_run {
            None
        } else {
            let last_message = fs::read_to_string(&team_run.consolidation_run.last_message_path)
                .with_context(|| {
                    format!(
                        "failed to read {}",
                        team_run.consolidation_run.last_message_path.display()
                    )
                })?;
            parse_auto_loop_marker(&last_message)
        };
        if marker.as_deref() == Some("complete") {
            completed = true;
        }
        completion_marker = marker.clone().or(completion_marker);
        iterations.push(CodexAutoLoopIterationReport {
            iteration,
            marker,
            team_run,
        });
        if completed || options.dry_run {
            break;
        }
    }

    let status_path = run_dir.join("auto-loop-status.json");
    let report = CodexAutoLoopReport {
        repo_root,
        codex_home,
        team: options.team,
        run_dir,
        status_path,
        max_iterations: options.max_iterations,
        completed,
        completion_marker,
        iterations,
        dry_run: options.dry_run,
    };
    write_auto_loop_status(&report)?;
    Ok(report)
}

pub fn run_codex_tdd_workflow(options: CodexTddWorkflowOptions) -> Result<CodexTddWorkflowReport> {
    let repo_root = options.repo_root.canonicalize().with_context(|| {
        format!(
            "failed to canonicalize repo root {}",
            options.repo_root.display()
        )
    })?;
    let codex_home = options.codex_home;
    let goal = options
        .goal
        .unwrap_or_else(|| "trace, test, and execute the Codex Rust automation tools".to_owned());
    let run_dir = options.output_dir.unwrap_or_else(|| {
        repo_root
            .join(".codex/harness/runs")
            .join(format!("{}-tdd-workflow", run_id(&goal)))
    });
    fs::create_dir_all(&run_dir)
        .with_context(|| format!("failed to create {}", run_dir.display()))?;
    fs::create_dir_all(run_dir.join("steps"))
        .with_context(|| format!("failed to create {}", run_dir.join("steps").display()))?;
    let status_path = run_dir.join("tdd-workflow-status.json");
    let extraction_report_path = run_dir.join("tdd-extraction-report.md");
    let extraction_plan_path = run_dir.join("tdd-extraction-plan.json");
    let binary = repo_root.join("target/debug/codex-env");
    let repo_arg = repo_root.display().to_string();
    let codex_home_arg = codex_home.display().to_string();
    let mut steps = codex_tdd_workflow_steps(
        &repo_arg,
        &codex_home_arg,
        &binary.display().to_string(),
        &run_dir,
        &options.team,
        &goal,
    );
    if options.dry_run {
        let report = CodexTddWorkflowReport {
            repo_root,
            codex_home,
            run_dir,
            status_path,
            extraction_report_path,
            extraction_plan_path,
            operator_role: "codex-as-human-in-loop".to_owned(),
            supervision_protocol: codex_tdd_supervision_protocol(),
            dry_run: true,
            steps,
        };
        write_tdd_extraction_artifacts(&report, &goal)?;
        write_tdd_workflow_status(&report)?;
        return Ok(report);
    }

    for index in 0..steps.len() {
        let step = &mut steps[index];
        step.status = "running".to_owned();
        step.worker_state = "running".to_owned();
        step.started_unix_seconds = Some(unix_seconds_now());
        step.supervision_events.push(format!(
            "running: Codex-as-human supervisor started background terminal step {}",
            step.name
        ));
        let step_for_run = step.clone();
        write_tdd_workflow_status(&tdd_workflow_snapshot(
            &repo_root,
            &codex_home,
            &run_dir,
            &status_path,
            &extraction_report_path,
            &extraction_plan_path,
            false,
            &steps,
        ))?;
        let output = run_tdd_step(
            &repo_root,
            &codex_home,
            &binary,
            &options.team,
            &goal,
            &step_for_run,
        )?;
        let step = &mut steps[index];
        step.exit_code = output.status.code();
        step.ended_unix_seconds = Some(unix_seconds_now());
        fs::write(&step.stdout_path, &output.stdout)
            .with_context(|| format!("failed to write {}", step.stdout_path.display()))?;
        fs::write(&step.stderr_path, &output.stderr)
            .with_context(|| format!("failed to write {}", step.stderr_path.display()))?;
        if output.status.success() {
            step.status = "ok".to_owned();
            step.worker_state = "ended".to_owned();
            step.supervision_events.push(format!(
                "ended: Codex-as-human supervisor captured logs and closed background terminal step {}",
                step.name
            ));
        } else {
            step.status = "failed".to_owned();
            step.worker_state = "failed".to_owned();
            step.supervision_events.push(format!(
                "failed: Codex-as-human supervisor captured logs and stopped background terminal step {}",
                step.name
            ));
            let report = CodexTddWorkflowReport {
                repo_root,
                codex_home,
                run_dir,
                status_path,
                extraction_report_path,
                extraction_plan_path,
                operator_role: "codex-as-human-in-loop".to_owned(),
                supervision_protocol: codex_tdd_supervision_protocol(),
                dry_run: false,
                steps,
            };
            write_tdd_extraction_artifacts(&report, &goal)?;
            write_tdd_workflow_status(&report)?;
            return Err(anyhow!(
                "codex-env tdd-workflow step {} failed with exit code {}",
                report
                    .steps
                    .iter()
                    .find(|candidate| candidate.status == "failed")
                    .map_or("unknown", |candidate| candidate.name.as_str()),
                report
                    .steps
                    .iter()
                    .find(|candidate| candidate.status == "failed")
                    .and_then(|candidate| candidate.exit_code)
                    .map_or_else(|| "signal".to_owned(), |code| code.to_string())
            ));
        }
    }

    let report = CodexTddWorkflowReport {
        repo_root,
        codex_home,
        run_dir,
        status_path,
        extraction_report_path,
        extraction_plan_path,
        operator_role: "codex-as-human-in-loop".to_owned(),
        supervision_protocol: codex_tdd_supervision_protocol(),
        dry_run: false,
        steps,
    };
    write_tdd_extraction_artifacts(&report, &goal)?;
    write_tdd_workflow_status(&report)?;
    Ok(report)
}

pub fn codex_tdd_next_action(
    options: CodexTddNextActionOptions,
) -> Result<CodexTddNextActionReport> {
    let repo_root = options.repo_root.canonicalize().with_context(|| {
        format!(
            "failed to canonicalize repo root {}",
            options.repo_root.display()
        )
    })?;
    let plan_path = match options.plan_path {
        Some(path) => {
            if path.is_absolute() {
                path
            } else {
                repo_root.join(path)
            }
        }
        None => latest_tdd_extraction_plan(&repo_root)?,
    };
    let plan_bytes =
        fs::read(&plan_path).with_context(|| format!("failed to read {}", plan_path.display()))?;
    let plan: CodexTddExtractionPlan = serde_json::from_slice(&plan_bytes)
        .with_context(|| format!("failed to parse {}", plan_path.display()))?;

    validate_tdd_extraction_plan(&plan, &plan_path)?;
    let selected_actions = select_tdd_next_actions(&plan);
    let ready_for_autonomous_loop = plan
        .actions
        .iter()
        .all(|action| action.status == "ok" && action.belongs_in == plan.target_crate);
    let status_summary = plan
        .actions
        .iter()
        .map(|action| format!("{}={}", action.step, action.status))
        .collect();
    let report = CodexTddNextActionReport {
        repo_root,
        plan_path,
        target_crate: plan.target_crate,
        forbidden_target: plan.forbidden_target,
        next_action: plan.next_action,
        selected_actions,
        ready_for_autonomous_loop,
        status_summary,
    };
    if options.check && !report.ready_for_autonomous_loop {
        return Err(anyhow!(
            "{} is not ready for autonomous loop handoff: {}",
            report.plan_path.display(),
            report.status_summary.join(", ")
        ));
    }
    Ok(report)
}

pub fn run_codex_tdd_auto_loop(options: CodexTddAutoLoopOptions) -> Result<CodexTddAutoLoopReport> {
    let next_action = codex_tdd_next_action(CodexTddNextActionOptions {
        repo_root: options.repo_root.clone(),
        plan_path: options.plan_path.clone(),
        check: true,
    })?;
    let handoff_goal = codex_tdd_auto_loop_goal(&next_action);
    let auto_loop = run_codex_auto_loop(CodexAutoLoopOptions {
        repo_root: options.repo_root,
        lua_policy: options.lua_policy,
        codex_home: options.codex_home,
        team: options.team,
        goal: Some(handoff_goal.clone()),
        prompt_file: None,
        output_dir: options.output_dir,
        max_iterations: options.max_iterations,
        member_sandbox_mode: options.member_sandbox_mode,
        dry_run: options.dry_run,
        skip_install: options.skip_install,
    })?;
    Ok(CodexTddAutoLoopReport {
        next_action,
        auto_loop,
        handoff_goal,
    })
}

pub fn ensure_codex_home_settings(codex_home: &Path) -> Result<CodexHomeSettingsReport> {
    let catalog_path = codex_home.join(REQUIRED_CODEX_MODEL_CATALOG);
    write_file(&PlannedFile {
        path: catalog_path.clone(),
        bytes: generated::codex_model_catalog_json().into_bytes(),
        executable: false,
    })?;

    let config_path = codex_home.join("config.toml");
    let original = if config_path.exists() {
        fs::read_to_string(&config_path)
            .with_context(|| format!("failed to read {}", config_path.display()))?
    } else {
        String::new()
    };
    let mut document = original
        .parse::<DocumentMut>()
        .with_context(|| format!("failed to parse {}", config_path.display()))?;

    set_root_string(&mut document, "model", REQUIRED_CODEX_MODEL);
    set_root_string(
        &mut document,
        "model_reasoning_effort",
        REQUIRED_CODEX_REASONING_EFFORT,
    );
    set_root_string(
        &mut document,
        "model_catalog_json",
        model_catalog_config_value(codex_home)
            .to_string_lossy()
            .as_ref(),
    );
    set_root_string(
        &mut document,
        "approval_policy",
        REQUIRED_CODEX_APPROVAL_POLICY,
    );
    set_root_string(
        &mut document,
        "approvals_reviewer",
        REQUIRED_CODEX_APPROVALS_REVIEWER,
    );
    set_root_integer(
        &mut document,
        "model_context_window",
        REQUIRED_CODEX_CONTEXT_WINDOW,
    );
    set_root_string(&mut document, "web_search", "live");
    set_table_bool(&mut document, "features", "multi_agent", true);
    set_table_bool(&mut document, "features", "goals", true);
    set_table_bool(&mut document, "skills", "include_instructions", true);

    let rendered = document.to_string();
    let changed = rendered != original;
    if changed {
        write_file(&PlannedFile {
            path: config_path.clone(),
            bytes: rendered.into_bytes(),
            executable: false,
        })?;
    }

    validate_codex_home_settings_at(codex_home, changed)
}

fn model_catalog_config_value(codex_home: &Path) -> PathBuf {
    if codex_home.file_name().and_then(|value| value.to_str()) == Some(".codex") {
        PathBuf::from(REQUIRED_CODEX_MODEL_CATALOG)
    } else {
        codex_home.join(REQUIRED_CODEX_MODEL_CATALOG)
    }
}

pub(crate) fn validate_codex_home_settings(codex_home: &Path) -> Result<CodexHomeSettingsReport> {
    validate_codex_home_settings_at(codex_home, false)
}

fn validate_codex_home_settings_at(
    codex_home: &Path,
    changed: bool,
) -> Result<CodexHomeSettingsReport> {
    let config_path = codex_home.join("config.toml");
    let config = fs::read_to_string(&config_path)
        .with_context(|| format!("failed to read {}", config_path.display()))?;
    let parsed = toml::from_str::<toml::Value>(&config)
        .with_context(|| format!("failed to parse {}", config_path.display()))?;
    let model = required_home_string(&parsed, "model", &config_path)?;
    let model_reasoning_effort =
        required_home_string(&parsed, "model_reasoning_effort", &config_path)?;
    let model_catalog_json = required_home_string(&parsed, "model_catalog_json", &config_path)?;
    let approval_policy = required_home_string(&parsed, "approval_policy", &config_path)?;
    let approvals_reviewer = required_home_string(&parsed, "approvals_reviewer", &config_path)?;
    let model_context_window =
        required_home_integer(&parsed, "model_context_window", &config_path)?;
    let multi_agent_enabled =
        required_home_bool(&parsed, &["features", "multi_agent"], &config_path)?;
    let goals_enabled = required_home_bool(&parsed, &["features", "goals"], &config_path)?;
    let include_skill_instructions =
        required_home_bool(&parsed, &["skills", "include_instructions"], &config_path)?;

    if model != REQUIRED_CODEX_MODEL {
        return Err(anyhow!(
            "{} must set model to {REQUIRED_CODEX_MODEL}, found {model}",
            config_path.display()
        ));
    }
    if model_reasoning_effort != REQUIRED_CODEX_REASONING_EFFORT {
        return Err(anyhow!(
            "{} must set model_reasoning_effort to {REQUIRED_CODEX_REASONING_EFFORT}, found {model_reasoning_effort}",
            config_path.display()
        ));
    }
    let expected_catalog = codex_home.join(REQUIRED_CODEX_MODEL_CATALOG);
    let expected_catalog_config = model_catalog_config_value(codex_home);
    if model_catalog_json != expected_catalog_config.to_string_lossy() {
        return Err(anyhow!(
            "{} must set model_catalog_json to {}, found {model_catalog_json}",
            config_path.display(),
            expected_catalog_config.display()
        ));
    }
    validate_model_catalog(&expected_catalog)?;
    if approval_policy != REQUIRED_CODEX_APPROVAL_POLICY {
        return Err(anyhow!(
            "{} must set approval_policy to {REQUIRED_CODEX_APPROVAL_POLICY}, found {approval_policy}",
            config_path.display()
        ));
    }
    if approvals_reviewer != REQUIRED_CODEX_APPROVALS_REVIEWER {
        return Err(anyhow!(
            "{} must set approvals_reviewer to {REQUIRED_CODEX_APPROVALS_REVIEWER}, found {approvals_reviewer}",
            config_path.display()
        ));
    }
    if model_context_window < REQUIRED_CODEX_CONTEXT_WINDOW {
        return Err(anyhow!(
            "{} must set model_context_window >= {REQUIRED_CODEX_CONTEXT_WINDOW}, found {model_context_window}",
            config_path.display()
        ));
    }
    if !multi_agent_enabled {
        return Err(anyhow!(
            "{} must enable features.multi_agent",
            config_path.display()
        ));
    }
    if !goals_enabled {
        return Err(anyhow!(
            "{} must enable features.goals",
            config_path.display()
        ));
    }
    if !include_skill_instructions {
        return Err(anyhow!(
            "{} must enable skills.include_instructions",
            config_path.display()
        ));
    }

    Ok(CodexHomeSettingsReport {
        config_path,
        changed,
        model,
        model_reasoning_effort,
        model_catalog_json,
        approval_policy,
        approvals_reviewer,
        model_context_window,
        multi_agent_enabled,
        goals_enabled,
        include_skill_instructions,
    })
}

fn validate_model_catalog(path: &Path) -> Result<()> {
    let catalog =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    let parsed = serde_json::from_str::<JsonValue>(&catalog)
        .with_context(|| format!("failed to parse {}", path.display()))?;
    let models = parsed
        .get("models")
        .and_then(JsonValue::as_array)
        .ok_or_else(|| anyhow!("{} must contain a models array", path.display()))?;
    let Some(model) = models
        .iter()
        .find(|model| model.get("slug").and_then(JsonValue::as_str) == Some(REQUIRED_CODEX_MODEL))
    else {
        return Err(anyhow!(
            "{} must contain model {}",
            path.display(),
            REQUIRED_CODEX_MODEL
        ));
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
        return Err(anyhow!(
            "{} model {} must set context_window and max_context_window >= {}, found {context_window}/{max_context_window}",
            path.display(),
            REQUIRED_CODEX_MODEL,
            REQUIRED_CODEX_CONTEXT_WINDOW
        ));
    }
    Ok(())
}

fn set_root_string(document: &mut DocumentMut, key: &str, expected: &str) {
    document[key] = value(expected);
}

fn set_root_integer(document: &mut DocumentMut, key: &str, expected: i64) {
    document[key] = value(expected);
}

fn set_table_bool(document: &mut DocumentMut, table: &str, key: &str, expected: bool) {
    let table = ensure_table(document, table);
    table[key] = value(expected);
}

fn ensure_table<'a>(document: &'a mut DocumentMut, key: &str) -> &'a mut Table {
    let root = document.as_table_mut();
    let needs_table = root.get(key).is_none_or(|item| !item.is_table());
    if needs_table {
        root.insert(key, Item::Table(Table::new()));
    }
    root.get_mut(key)
        .expect("table item inserted above")
        .as_table_mut()
        .expect("table item inserted above")
}

fn required_home_string(config: &toml::Value, key: &str, path: &Path) -> Result<String> {
    config
        .get(key)
        .and_then(toml::Value::as_str)
        .map(ToOwned::to_owned)
        .ok_or_else(|| anyhow!("{} is missing required string {key}", path.display()))
}

fn required_home_integer(config: &toml::Value, key: &str, path: &Path) -> Result<i64> {
    config
        .get(key)
        .and_then(toml::Value::as_integer)
        .ok_or_else(|| anyhow!("{} is missing required integer {key}", path.display()))
}

fn required_home_bool(config: &toml::Value, keys: &[&str], path: &Path) -> Result<bool> {
    let mut current = config;
    for key in keys {
        current = current.get(*key).ok_or_else(|| {
            anyhow!(
                "{} is missing required key {}",
                path.display(),
                keys.join(".")
            )
        })?;
    }
    current.as_bool().ok_or_else(|| {
        anyhow!(
            "{} required key {} must be bool",
            path.display(),
            keys.join(".")
        )
    })
}

fn locate_claude_dir(repo_root: &Path) -> Result<PathBuf> {
    let direct = repo_root.join(".claude");
    if direct.is_dir() {
        return Ok(direct);
    }

    for ancestor in repo_root.ancestors() {
        let candidate = ancestor.join(".claude");
        if candidate.is_dir() {
            return Ok(candidate);
        }
    }

    Err(anyhow!(
        "could not locate a .claude directory from {}",
        repo_root.display()
    ))
}

fn load_lua_policy(path: Option<&Path>, repo_root: &Path, claude_dir: &Path) -> Result<LuaPolicy> {
    let Some(path) = path else {
        return Ok(LuaPolicy::default());
    };
    let script = fs::read_to_string(path)
        .with_context(|| format!("failed to read Lua policy {}", path.display()))?;
    let lua = Lua::new();
    let globals = lua.globals();
    let mirror = lua.create_table().map_err(lua_error)?;
    mirror
        .set("repo_root", repo_root.to_string_lossy().to_string())
        .map_err(lua_error)?;
    mirror
        .set("claude_dir", claude_dir.to_string_lossy().to_string())
        .map_err(lua_error)?;
    globals.set("mirror", mirror).map_err(lua_error)?;

    let value = lua
        .load(&script)
        .set_name(path.to_string_lossy().as_ref())
        .eval::<Value>()
        .map_err(|err| anyhow!("failed to evaluate Lua policy {}: {err}", path.display()))?;

    let Value::Table(table) = value else {
        return Ok(LuaPolicy::default());
    };

    Ok(LuaPolicy {
        config_footer: table.get("config_footer").map_err(lua_error)?,
        skill_prelude: table.get("skill_prelude").map_err(lua_error)?,
    })
}

fn lua_error(error: mlua::Error) -> anyhow::Error {
    anyhow!("{error}")
}

fn manifest_json(
    repo_root: &Path,
    claude_dir: &Path,
    planned: &[PlannedFile],
    manifest_path: &Path,
) -> Result<String> {
    let mut files: Vec<_> = planned
        .iter()
        .map(|file| {
            strip_repo_prefix(repo_root, &file.path)
                .display()
                .to_string()
        })
        .collect();
    files.push(
        strip_repo_prefix(repo_root, manifest_path)
            .display()
            .to_string(),
    );
    let manifest = json!({
        "generatedBy": "codex-env",
        "source": strip_repo_prefix(repo_root, claude_dir),
        "fileCount": files.len(),
        "files": files
    });
    Ok(format!("{}\n", serde_json::to_string_pretty(&manifest)?))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RunAgentTeamsManifest {
    teams: Vec<RunAgentTeam>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RunAgentTeam {
    name: String,
    description: String,
    strategy: String,
    parallel: bool,
    consolidation_owner: String,
    agents: Vec<String>,
}

#[derive(Debug, Clone)]
struct RunAgentProfile {
    name: String,
    description: String,
    model: String,
    model_reasoning_effort: String,
    developer_instructions: String,
    path: PathBuf,
}

fn load_team(codex_dir: &Path, team_name: &str) -> Result<RunAgentTeam> {
    let path = codex_dir.join("agent-teams.json");
    let manifest: RunAgentTeamsManifest = serde_json::from_slice(
        &fs::read(&path).with_context(|| format!("failed to read {}", path.display()))?,
    )
    .with_context(|| format!("failed to parse {}", path.display()))?;
    let Some(team) = manifest
        .teams
        .into_iter()
        .find(|team| team.name == team_name)
    else {
        return Err(anyhow!("{} has no team named {team_name}", path.display()));
    };
    if !team.parallel {
        return Err(anyhow!(
            "{} team {team_name} must have parallel=true for team-run",
            path.display()
        ));
    }
    if team.consolidation_owner != "parent" {
        return Err(anyhow!(
            "{} team {team_name} must use parent consolidation",
            path.display()
        ));
    }
    Ok(team)
}

fn load_team_agent_profiles(codex_dir: &Path, team: &RunAgentTeam) -> Result<Vec<RunAgentProfile>> {
    let config_path = codex_dir.join("config.toml");
    let config = toml::from_str::<toml::Value>(
        &fs::read_to_string(&config_path)
            .with_context(|| format!("failed to read {}", config_path.display()))?,
    )
    .with_context(|| format!("failed to parse {}", config_path.display()))?;
    let agents = config
        .get("agents")
        .and_then(toml::Value::as_table)
        .ok_or_else(|| anyhow!("{} is missing agents table", config_path.display()))?;
    let mut profiles = Vec::new();
    for agent in &team.agents {
        let table = agents
            .get(agent)
            .and_then(toml::Value::as_table)
            .ok_or_else(|| {
                anyhow!(
                    "{} has no config entry for agent {agent}",
                    config_path.display()
                )
            })?;
        let config_file = table
            .get("config_file")
            .and_then(toml::Value::as_str)
            .ok_or_else(|| anyhow!("{} agent {agent} has no config_file", config_path.display()))?;
        let config_file = PathBuf::from(config_file);
        if config_file.is_absolute()
            || config_file
                .components()
                .any(|component| matches!(component, std::path::Component::ParentDir))
        {
            return Err(anyhow!(
                "{} agent {agent} has unsafe config_file {}",
                config_path.display(),
                config_file.display()
            ));
        }
        profiles.push(load_agent_profile(&codex_dir.join(config_file))?);
    }
    Ok(profiles)
}

fn load_agent_profile(path: &Path) -> Result<RunAgentProfile> {
    let toml = toml::from_str::<toml::Value>(
        &fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?,
    )
    .with_context(|| format!("failed to parse {}", path.display()))?;
    let name = required_profile_string(&toml, "name", path)?;
    let description = required_profile_string(&toml, "description", path)?;
    let model = required_profile_string(&toml, "model", path)?;
    let model_reasoning_effort = required_profile_string(&toml, "model_reasoning_effort", path)?;
    let developer_instructions = required_profile_string(&toml, "developer_instructions", path)?;
    Ok(RunAgentProfile {
        name,
        description,
        model,
        model_reasoning_effort,
        developer_instructions,
        path: path.to_path_buf(),
    })
}

fn validate_member_sandbox_mode(value: &str) -> Result<String> {
    let value = value.trim();
    match value {
        "read-only" | "workspace-write" => Ok(value.to_owned()),
        _ => Err(anyhow!(
            "team-run member sandbox must be read-only or workspace-write, found {value:?}"
        )),
    }
}

fn required_profile_string(toml: &toml::Value, key: &str, path: &Path) -> Result<String> {
    toml.get(key)
        .and_then(toml::Value::as_str)
        .map(ToOwned::to_owned)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| anyhow!("{} is missing required key {key}", path.display()))
}

fn resolve_run_goal(goal: Option<&str>, prompt_file: Option<&Path>) -> Result<String> {
    let mut parts = Vec::new();
    if let Some(goal) = goal.map(str::trim).filter(|goal| !goal.is_empty()) {
        parts.push(goal.to_owned());
    }
    if let Some(path) = prompt_file {
        parts.push(
            fs::read_to_string(path)
                .with_context(|| format!("failed to read {}", path.display()))?,
        );
    }
    let goal = parts.join("\n\n");
    if goal.trim().is_empty() {
        return Err(anyhow!(
            "codex-env run requires a goal argument, --prompt-file, or both"
        ));
    }
    Ok(goal)
}

fn codex_run_prompt(goal: &str) -> String {
    normalize_generated_text(&format!(
        r#"# codex-env Run

You are running inside the repo-owned Codex harness. Do real work, not a plan.

Operating rules:
- Start by recalling ICM project memory and reading the closest AGENTS.md.
- Inspect git/branch/PR state before editing.
- Use the repo's generated `.codex` surface, installed prompts, skills, agents, hooks, and MCP settings as the local execution environment.
- Keep edits scoped to the requested goal.
- Run targeted verification plus `codex-env` mirror/doctor checks when the Codex surface changes.
- Commit and push completed publishable work, then open or update the PR.
- Store ICM memory after significant completed work.

Goal:
{goal}
"#
    ))
}

fn codex_auto_loop_goal(goal: &str, iteration: usize, max_iterations: usize) -> String {
    normalize_generated_text(&format!(
        r#"# codex-env Auto Loop

You are executing iteration {iteration} of at most {max_iterations} in the repo-owned Codex auto-loop harness.

Loop contract:
- Recall ICM memory and read the closest AGENTS.md before relying on prior context.
- Inspect current git/branch/PR/generated-surface state.
- Use read-only team members for parallel evidence and parent consolidation for writes.
- Implement only changes that move the requested final state closer to true.
- Run targeted verification and Codex mirror/doctor checks for Codex-surface changes.
- Commit, push, update the PR, and store ICM memory when publishable work is completed.
- End the parent consolidation response with exactly one marker line:
  - `CODEX_AUTO_LOOP_STATUS: complete` only when the full requested state is achieved and verified.
  - `CODEX_AUTO_LOOP_STATUS: continue` when another iteration is needed, followed by the next concrete gap.

Original goal:
{goal}
"#
    ))
}

fn codex_tdd_auto_loop_goal(next_action: &CodexTddNextActionReport) -> String {
    let selected_actions = next_action
        .selected_actions
        .iter()
        .map(|action| {
            format!(
                "- {} [{}]: {} | evidence: {}, {}",
                action.step,
                action.status,
                action.next_action,
                action.evidence_stdout.display(),
                action.evidence_stderr.display()
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    normalize_generated_text(&format!(
        r#"# Codex TDD Autonomous Handoff

Consume the validated TDD extraction plan and continue the Rust-owned autonomous loop.

Plan: {}
Target crate: {}
Forbidden target: {}
Ready for autonomous loop: {}

Next crate-owned action:
{}

Selected extraction actions:
{}

Rules:
- Treat this as the continuation after Codex-as-human supervised the background terminal trace.
- Inspect stdout/stderr evidence only when needed; the JSON plan is the low-token routing artifact.
- Apply durable behavior into `{}` or adjacent project-owned Rust crates.
- Do not move this automation into `{}`.
- End parent consolidation with `CODEX_AUTO_LOOP_STATUS: complete` only when this handoff is fully implemented and verified; otherwise use `continue` with the next concrete Rust-owned gap.
"#,
        next_action.plan_path.display(),
        next_action.target_crate,
        next_action.forbidden_target,
        next_action.ready_for_autonomous_loop,
        next_action.next_action,
        selected_actions,
        next_action.target_crate,
        next_action.forbidden_target
    ))
}

fn parse_auto_loop_marker(last_message: &str) -> Option<String> {
    last_message.lines().find_map(|line| {
        let marker = line.trim().strip_prefix("CODEX_AUTO_LOOP_STATUS:")?;
        let marker = marker.trim();
        if marker.eq_ignore_ascii_case("complete") {
            Some("complete".to_owned())
        } else if marker.eq_ignore_ascii_case("continue") {
            Some("continue".to_owned())
        } else {
            None
        }
    })
}

fn codex_team_member_prompt(
    goal: &str,
    team: &RunAgentTeam,
    profile: &RunAgentProfile,
    member_sandbox_mode: &str,
) -> String {
    normalize_generated_text(&format!(
        r#"# codex-env Team Member

You are running as Codex agent `{}` in team `{}`.

Team description: {}
Team strategy: {}

Agent description: {}
Model route: {} / {}
Execution sandbox: {}

Use your agent instructions below as the role contract. Return concrete evidence, file paths, risks, and recommended edits. Parallel team members are evidence producers; do not modify files unless the parent explicitly selected a writable member sandbox for an isolated scope.

## Agent Instructions

{}

## Goal

{}
"#,
        profile.name,
        team.name,
        team.description,
        team.strategy,
        profile.description,
        profile.model,
        profile.model_reasoning_effort,
        member_sandbox_mode,
        profile.developer_instructions.trim(),
        goal
    ))
}

fn codex_team_consolidation_prompt(
    goal: &str,
    team: &RunAgentTeam,
    members: &[CodexTeamRunMemberReport],
) -> String {
    let member_outputs = members
        .iter()
        .map(|member| {
            format!(
                "- {} ({} / {}, sandbox {}): {}",
                member.agent,
                member.model,
                member.reasoning_effort,
                member.sandbox_mode,
                member.run.last_message_path.display()
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    normalize_generated_text(&format!(
        r#"# codex-env Team Consolidation

Consolidate the completed Codex team run.

Team: {}
Strategy: {}
Goal: {}

Member outputs:
{}

Read every member output, reconcile conflicts, decide the implementation path, make parent-owned edits, verify, commit, push, and update the PR when publishing applies. Treat parallel member runs as evidence-only unless their status explicitly says they ran with `workspace-write` for an isolated scope.
"#,
        team.name, team.strategy, goal, member_outputs
    ))
}

fn prepared_run_report(
    repo_root: &Path,
    codex_home: &Path,
    run_dir: PathBuf,
    prompt: String,
    dry_run: bool,
) -> Result<CodexRunReport> {
    fs::create_dir_all(&run_dir)
        .with_context(|| format!("failed to create {}", run_dir.display()))?;
    let prompt_path = run_dir.join("prompt.md");
    let events_path = run_dir.join("events.jsonl");
    let stderr_path = run_dir.join("stderr.log");
    let last_message_path = run_dir.join("last-message.md");
    let status_path = run_dir.join("status.json");
    fs::write(&prompt_path, prompt)
        .with_context(|| format!("failed to write {}", prompt_path.display()))?;
    Ok(CodexRunReport {
        repo_root: repo_root.to_path_buf(),
        codex_home: codex_home.to_path_buf(),
        run_dir,
        prompt_path,
        events_path,
        stderr_path,
        last_message_path,
        status_path,
        dry_run,
        exit_code: None,
    })
}

fn spawn_codex_exec(
    repo_root: &Path,
    codex_home: &Path,
    report: &CodexRunReport,
    sandbox_mode: &str,
    model: &str,
    model_reasoning_effort: &str,
) -> Result<std::process::Child> {
    let events = fs::File::create(&report.events_path)
        .with_context(|| format!("failed to create {}", report.events_path.display()))?;
    let stderr = fs::File::create(&report.stderr_path)
        .with_context(|| format!("failed to create {}", report.stderr_path.display()))?;
    let prompt = fs::read(&report.prompt_path)
        .with_context(|| format!("failed to read {}", report.prompt_path.display()))?;
    let mut child = Command::new("codex")
        .arg("exec")
        .arg("--json")
        .arg("--cd")
        .arg(repo_root)
        .arg("--sandbox")
        .arg(sandbox_mode)
        .arg("--model")
        .arg(model)
        .arg("--config")
        .arg(format!(
            "model_reasoning_effort=\"{model_reasoning_effort}\""
        ))
        .arg("--config")
        .arg("approval_policy=\"never\"")
        .arg("--output-last-message")
        .arg(&report.last_message_path)
        .arg("-")
        .env("CODEX_HOME", codex_home)
        .stdin(Stdio::piped())
        .stdout(Stdio::from(events))
        .stderr(Stdio::from(stderr))
        .spawn()
        .with_context(|| "failed to spawn codex exec")?;
    child
        .stdin
        .take()
        .ok_or_else(|| anyhow!("failed to open codex exec stdin"))?
        .write_all(&prompt)
        .with_context(|| "failed to write codex exec prompt")?;
    Ok(child)
}

fn run_id(goal: &str) -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs());
    let slug = slugify(goal);
    let slug = if slug.is_empty() {
        "task".to_owned()
    } else {
        slug.chars().take(48).collect()
    };
    format!("{seconds}-{slug}")
}

fn write_run_status(report: &CodexRunReport) -> Result<()> {
    let status = json!({
        "repoRoot": report.repo_root,
        "codexHome": report.codex_home,
        "runDir": report.run_dir,
        "promptPath": report.prompt_path,
        "eventsPath": report.events_path,
        "stderrPath": report.stderr_path,
        "lastMessagePath": report.last_message_path,
        "dryRun": report.dry_run,
        "exitCode": report.exit_code,
    });
    fs::write(
        &report.status_path,
        format!("{}\n", serde_json::to_string_pretty(&status)?),
    )
    .with_context(|| format!("failed to write {}", report.status_path.display()))
}

fn write_team_run_status(report: &CodexTeamRunReport) -> Result<()> {
    fs::write(
        &report.status_path,
        format!("{}\n", serde_json::to_string_pretty(report)?),
    )
    .with_context(|| format!("failed to write {}", report.status_path.display()))
}

fn write_auto_loop_status(report: &CodexAutoLoopReport) -> Result<()> {
    fs::write(
        &report.status_path,
        format!("{}\n", serde_json::to_string_pretty(report)?),
    )
    .with_context(|| format!("failed to write {}", report.status_path.display()))
}

fn write_tdd_workflow_status(report: &CodexTddWorkflowReport) -> Result<()> {
    fs::write(
        &report.status_path,
        format!("{}\n", serde_json::to_string_pretty(report)?),
    )
    .with_context(|| format!("failed to write {}", report.status_path.display()))
}

fn write_tdd_extraction_artifacts(report: &CodexTddWorkflowReport, goal: &str) -> Result<()> {
    write_tdd_extraction_report(report, goal)?;
    write_tdd_extraction_plan(report, goal)
}

fn write_tdd_extraction_report(report: &CodexTddWorkflowReport, goal: &str) -> Result<()> {
    let mut markdown = String::from("# Codex TDD Extraction Report\n\n");
    markdown.push_str(&format!("Goal: {goal}\n\n"));
    markdown.push_str(&format!("Operator: `{}`\n\n", report.operator_role));
    markdown.push_str(
        "This report converts the supervised background-terminal trace into Rust-owned extraction actions. The target is the project crate layer, especially `crates/codex-env`; the target is not a vendor harness.\n\n",
    );
    markdown.push_str("## Next extraction action\n\n");
    if let Some(failed) = report.steps.iter().find(|step| step.status == "failed") {
        markdown.push_str(&format!(
            "- Fix `{}` in `{}` using `{}`; inspect `{}` and `{}` before editing.\n\n",
            failed.name,
            failed.belongs_in,
            failed.extraction_target,
            failed.stdout_path.display(),
            failed.stderr_path.display()
        ));
    } else if report.dry_run {
        markdown.push_str(
            "- Execute `codex-env tdd-workflow` without `--dry-run`, supervise logs, then apply any discovered behavior into `crates/codex-env`.\n\n",
        );
    } else {
        markdown.push_str(
            "- All current TDD tool gates passed; inspect the per-step logs and promote the next uncovered automation behavior into `crates/codex-env` rather than a vendor harness.\n\n",
        );
    }
    markdown.push_str("## Tool trace\n\n");
    for step in &report.steps {
        markdown.push_str(&format!(
            "### {}\n\n- Status: `{}`\n- Worker state: `{}`\n- Does: {}\n- Why: {}\n- Belongs in: `{}`\n- Extraction target: {}\n- Supervision action: {}\n- Stdout: `{}`\n- Stderr: `{}`\n\n",
            step.name,
            step.status,
            step.worker_state,
            step.does,
            step.why,
            step.belongs_in,
            step.extraction_target,
            step.supervision_action,
            step.stdout_path.display(),
            step.stderr_path.display()
        ));
    }
    fs::write(
        &report.extraction_report_path,
        normalize_generated_text(&markdown),
    )
    .with_context(|| {
        format!(
            "failed to write {}",
            report.extraction_report_path.display()
        )
    })
}

fn write_tdd_extraction_plan(report: &CodexTddWorkflowReport, goal: &str) -> Result<()> {
    let plan = CodexTddExtractionPlan {
        schema_version: 1,
        generated_by: "codex-env tdd-workflow".to_owned(),
        goal: goal.to_owned(),
        target_crate: "crates/codex-env".to_owned(),
        forbidden_target: "vendor harness".to_owned(),
        source_material: ".claude and generated .codex evidence".to_owned(),
        runtime_representation: "machine-readable Rust-owned extraction plan".to_owned(),
        next_action: tdd_next_extraction_action(report),
        actions: report
            .steps
            .iter()
            .map(|step| CodexTddExtractionAction {
                step: step.name.clone(),
                status: step.status.clone(),
                worker_state: step.worker_state.clone(),
                crate_owner: step.crate_owner.clone(),
                belongs_in: step.belongs_in.clone(),
                extraction_target: step.extraction_target.clone(),
                next_action: tdd_step_next_action(step),
                evidence_stdout: step.stdout_path.clone(),
                evidence_stderr: step.stderr_path.clone(),
            })
            .collect(),
    };
    fs::write(
        &report.extraction_plan_path,
        format!("{}\n", serde_json::to_string_pretty(&plan)?),
    )
    .with_context(|| format!("failed to write {}", report.extraction_plan_path.display()))
}

fn latest_tdd_extraction_plan(repo_root: &Path) -> Result<PathBuf> {
    let runs_dir = repo_root.join(".codex/harness/runs");
    let mut candidates = Vec::new();
    if runs_dir.exists() {
        for entry in fs::read_dir(&runs_dir)
            .with_context(|| format!("failed to read {}", runs_dir.display()))?
        {
            let entry =
                entry.with_context(|| format!("failed to read entry in {}", runs_dir.display()))?;
            let path = entry.path().join("tdd-extraction-plan.json");
            if path.exists() {
                let modified = fs::metadata(&path)
                    .and_then(|metadata| metadata.modified())
                    .unwrap_or(UNIX_EPOCH);
                candidates.push((modified, path));
            }
        }
    }
    candidates
        .into_iter()
        .max_by_key(|(modified, path)| (*modified, path.clone()))
        .map(|(_, path)| path)
        .ok_or_else(|| {
            anyhow!(
                "no tdd-extraction-plan.json found under {}; run codex-env tdd-workflow first",
                runs_dir.display()
            )
        })
}

fn validate_tdd_extraction_plan(plan: &CodexTddExtractionPlan, path: &Path) -> Result<()> {
    if plan.schema_version != 1 {
        return Err(anyhow!(
            "{} has unsupported schema_version {}",
            path.display(),
            plan.schema_version
        ));
    }
    if plan.target_crate != "crates/codex-env" {
        return Err(anyhow!(
            "{} routes target_crate to {}, expected crates/codex-env",
            path.display(),
            plan.target_crate
        ));
    }
    if plan.forbidden_target != "vendor harness" {
        return Err(anyhow!(
            "{} must keep forbidden_target as vendor harness, found {}",
            path.display(),
            plan.forbidden_target
        ));
    }
    if plan.actions.is_empty() {
        return Err(anyhow!("{} contains no extraction actions", path.display()));
    }
    for action in &plan.actions {
        if action.belongs_in == plan.forbidden_target || action.crate_owner == plan.forbidden_target
        {
            return Err(anyhow!(
                "{} action {} routes ownership to forbidden target {}",
                path.display(),
                action.step,
                plan.forbidden_target
            ));
        }
        if action.belongs_in != plan.target_crate || action.crate_owner != plan.target_crate {
            return Err(anyhow!(
                "{} action {} must belong to {}, found owner={} belongs_in={}",
                path.display(),
                action.step,
                plan.target_crate,
                action.crate_owner,
                action.belongs_in
            ));
        }
        if action.next_action.trim().is_empty() {
            return Err(anyhow!(
                "{} action {} has an empty next_action",
                path.display(),
                action.step
            ));
        }
    }
    Ok(())
}

fn select_tdd_next_actions(plan: &CodexTddExtractionPlan) -> Vec<CodexTddExtractionAction> {
    let failed = plan
        .actions
        .iter()
        .filter(|action| action.status == "failed")
        .cloned()
        .collect::<Vec<_>>();
    if !failed.is_empty() {
        return failed;
    }
    let pending = plan
        .actions
        .iter()
        .filter(|action| action.status != "ok")
        .cloned()
        .collect::<Vec<_>>();
    if !pending.is_empty() {
        return pending;
    }
    plan.actions.clone()
}

fn tdd_next_extraction_action(report: &CodexTddWorkflowReport) -> String {
    if let Some(failed) = report.steps.iter().find(|step| step.status == "failed") {
        return format!(
            "Fix {} in {} using {} and its captured stdout/stderr evidence.",
            failed.name, failed.belongs_in, failed.extraction_target
        );
    }
    if report.dry_run {
        return "Execute codex-env tdd-workflow without --dry-run, supervise logs, then apply discovered behavior into crates/codex-env.".to_owned();
    }
    "Promote the next uncovered automation behavior into crates/codex-env; do not move it into a vendor harness.".to_owned()
}

fn tdd_step_next_action(step: &CodexTddWorkflowStepReport) -> String {
    match step.status.as_str() {
        "failed" => format!(
            "Inspect {} and {}; repair {} in {}.",
            step.stdout_path.display(),
            step.stderr_path.display(),
            step.extraction_target,
            step.belongs_in
        ),
        "ok" => format!(
            "Use this passing evidence as the crate-owned guard for {}; keep it out of the vendor harness.",
            step.extraction_target
        ),
        "planned" | "running" => format!(
            "Run or supervise this step before claiming {} is covered.",
            step.extraction_target
        ),
        _ => format!(
            "Review status {} and route the finding to {}.",
            step.status, step.belongs_in
        ),
    }
}

fn codex_tdd_supervision_protocol() -> Vec<String> {
    vec![
        "build the crate-owned codex-env binary before invoking generated automation".to_owned(),
        "run the Codex Rust tools from a supervised background terminal equivalent and capture status after every step".to_owned(),
        "Codex acts as the human-in-loop operator: prompt the worker, inspect artifacts, give follow-up guidance when needed, then end the background terminal session".to_owned(),
        "extract durable behavior into project-owned Rust crates; do not move automation into a vendor harness".to_owned(),
    ]
}

fn codex_tdd_workflow_steps(
    repo: &str,
    codex_home: &str,
    binary: &str,
    run_dir: &Path,
    team: &str,
    goal: &str,
) -> Vec<CodexTddWorkflowStepReport> {
    let tool_goal = shell_quote(goal);
    let tool_run_dir = ".codex/harness/runs/tdd-workflow";
    [
        (
            "build-codex-env",
            "cargo build -p codex-env".to_owned(),
            "Build the Rust-owned automation harness first; later steps execute the built tool rather than treating generated Markdown as the runtime.",
        ),
        (
            "mirror-check",
            format!("{binary} --repo {repo} mirror --check"),
            "Prove the generated .codex extraction surface is deterministic and current before executing it.",
        ),
        (
            "install-prompts-check",
            format!("{binary} --repo {repo} install-prompts --check --codex-home {codex_home}"),
            "Prove prompt commands remain repo-local and do not pollute user-global Codex prompts.",
        ),
        (
            "doctor",
            format!("{binary} --repo {repo} doctor --codex-home {codex_home}"),
            "Validate model, MCP, hooks, teams, agents, and prompt runtime wiring before any agentic execution.",
        ),
        (
            "inventory-check",
            format!("{binary} --repo {repo} inventory --check --codex-home {codex_home}"),
            "Fail on Claude-to-Codex parity gaps so overload, MCP rot, or empty teams are not mistaken for success.",
        ),
        (
            "single-run-dry-run",
            format!("{binary} --repo {repo} run --codex-home {codex_home} --output-dir {tool_run_dir}/run --dry-run {tool_goal}"),
            "Materialize the exact non-interactive Codex prompt and artifacts for one parent-owned work pass.",
        ),
        (
            "team-run-dry-run",
            format!("{binary} --repo {repo} team-run --codex-home {codex_home} --team {team} --output-dir {tool_run_dir}/team-run --dry-run {tool_goal}"),
            "Materialize the parallel background-agent prompts while keeping member writes disabled by default.",
        ),
        (
            "auto-loop-dry-run",
            format!("{binary} --repo {repo} auto-loop --codex-home {codex_home} --team {team} --max-iterations 3 --output-dir {tool_run_dir}/auto-loop --dry-run {tool_goal}"),
            "Materialize the bounded autonomous loop contract that Codex supervises as the human-in-loop operator.",
        ),
    ]
    .into_iter()
    .map(|(name, command, rationale)| {
        let (does, why, extraction_target, supervision_action) =
            codex_tdd_step_semantics(name, rationale);
        CodexTddWorkflowStepReport {
            name: name.to_owned(),
            command,
            rationale: rationale.to_owned(),
            does,
            why,
            crate_owner: "crates/codex-env".to_owned(),
            belongs_in: "crates/codex-env".to_owned(),
            extraction_target,
            supervision_action,
            worker_state: "planned".to_owned(),
            stdout_path: run_dir.join("steps").join(format!("{name}.stdout.log")),
            stderr_path: run_dir.join("steps").join(format!("{name}.stderr.log")),
            supervision_events: vec![format!(
                "planned: Codex-as-human supervisor queued background terminal step {name}"
            )],
            started_unix_seconds: None,
            ended_unix_seconds: None,
            status: "planned".to_owned(),
            exit_code: None,
        }
    })
    .collect()
}

fn codex_tdd_step_semantics(name: &str, rationale: &str) -> (String, String, String, String) {
    let does = match name {
        "build-codex-env" => "compiles the Rust-owned Codex automation binary before any generated surface is treated as executable",
        "mirror-check" => "recomputes the .claude to .codex extraction plan and rejects stale generated files",
        "install-prompts-check" => "verifies repo-local prompt commands stay inside this repository's .codex/prompts surface",
        "doctor" => "validates runtime Codex config, MCP declarations, hooks, prompt installation, agent profiles, and nonempty teams",
        "inventory-check" => "compares source Claude assets against generated Codex runtime assets and fails on parity gaps",
        "single-run-dry-run" => "materializes the parent-owned codex exec prompt and status artifacts without launching a nested writer",
        "team-run-dry-run" => "materializes parallel evidence-agent prompts and parent consolidation artifacts with read-only members",
        "auto-loop-dry-run" => "materializes the bounded autonomous loop contract and completion-marker protocol",
        _ => "executes a Codex Rust tool workflow step",
    };
    let extraction_target = match name {
        "build-codex-env" => "Rust-owned build/test gate for crates/codex-env",
        "mirror-check" => "Rust-owned .claude to .codex extractor/compiler in crates/codex-env",
        "install-prompts-check" => {
            "Rust-owned repo-local prompt installer/verifier in crates/codex-env"
        }
        "doctor" => "Rust-owned runtime health checker in crates/codex-env",
        "inventory-check" => "Rust-owned parity inventory and gap detector in crates/codex-env",
        "single-run-dry-run" => "Rust-owned parent execution harness in crates/codex-env",
        "team-run-dry-run" => "Rust-owned team orchestration harness in crates/codex-env",
        "auto-loop-dry-run" => "Rust-owned autonomous loop harness in crates/codex-env",
        _ => "Rust-owned Codex automation in crates/codex-env",
    };
    let supervision_action = match name {
        "build-codex-env" => "supervise the background terminal until the binary is built or the compiler error is captured",
        "mirror-check" => "supervise freshness output and guide extraction fixes if the generated mirror is stale",
        "install-prompts-check" => "supervise prompt locality output and stop if user-global prompt pollution appears",
        "doctor" => "supervise doctor output and route failed health checks back into crate-owned Rust code",
        "inventory-check" => "supervise parity gaps and convert real gaps into Rust extractor/compiler work",
        "single-run-dry-run" => "supervise generated prompt/status artifacts before allowing a real parent writer run",
        "team-run-dry-run" => "supervise member prompt/status artifacts before allowing parallel background agents",
        "auto-loop-dry-run" => "supervise loop prompt/status artifacts before allowing bounded autonomous iterations",
        _ => "supervise the background terminal and capture tool evidence",
    };
    (
        does.to_owned(),
        rationale.to_owned(),
        extraction_target.to_owned(),
        supervision_action.to_owned(),
    )
}

fn tdd_workflow_snapshot(
    repo_root: &Path,
    codex_home: &Path,
    run_dir: &Path,
    status_path: &Path,
    extraction_report_path: &Path,
    extraction_plan_path: &Path,
    dry_run: bool,
    steps: &[CodexTddWorkflowStepReport],
) -> CodexTddWorkflowReport {
    CodexTddWorkflowReport {
        repo_root: repo_root.to_path_buf(),
        codex_home: codex_home.to_path_buf(),
        run_dir: run_dir.to_path_buf(),
        status_path: status_path.to_path_buf(),
        extraction_report_path: extraction_report_path.to_path_buf(),
        extraction_plan_path: extraction_plan_path.to_path_buf(),
        operator_role: "codex-as-human-in-loop".to_owned(),
        supervision_protocol: codex_tdd_supervision_protocol(),
        dry_run,
        steps: steps.to_vec(),
    }
}

fn run_tdd_step(
    repo_root: &Path,
    codex_home: &Path,
    binary: &Path,
    team: &str,
    goal: &str,
    step: &CodexTddWorkflowStepReport,
) -> Result<std::process::Output> {
    if step.name == "build-codex-env" {
        return Command::new("cargo")
            .arg("build")
            .arg("-p")
            .arg("codex-env")
            .current_dir(repo_root)
            .output()
            .with_context(|| "failed to spawn cargo build -p codex-env");
    }
    let mut command = Command::new(binary);
    command.arg("--repo").arg(repo_root);
    match step.name.as_str() {
        "mirror-check" => {
            command.arg("mirror").arg("--check");
        }
        "install-prompts-check" => {
            command
                .arg("install-prompts")
                .arg("--codex-home")
                .arg(codex_home)
                .arg("--check");
        }
        "doctor" => {
            command.arg("doctor").arg("--codex-home").arg(codex_home);
        }
        "inventory-check" => {
            command
                .arg("inventory")
                .arg("--codex-home")
                .arg(codex_home)
                .arg("--check");
        }
        "single-run-dry-run" => {
            command
                .arg("run")
                .arg("--codex-home")
                .arg(codex_home)
                .arg("--output-dir")
                .arg(repo_root.join(".codex/harness/runs/tdd-workflow/run"))
                .arg("--dry-run")
                .arg(goal);
        }
        "team-run-dry-run" => {
            command
                .arg("team-run")
                .arg("--codex-home")
                .arg(codex_home)
                .arg("--team")
                .arg(team)
                .arg("--output-dir")
                .arg(repo_root.join(".codex/harness/runs/tdd-workflow/team-run"))
                .arg("--dry-run")
                .arg(goal);
        }
        "auto-loop-dry-run" => {
            command
                .arg("auto-loop")
                .arg("--codex-home")
                .arg(codex_home)
                .arg("--team")
                .arg(team)
                .arg("--max-iterations")
                .arg("3")
                .arg("--output-dir")
                .arg(repo_root.join(".codex/harness/runs/tdd-workflow/auto-loop"))
                .arg("--dry-run")
                .arg(goal);
        }
        _ => return Err(anyhow!("unknown tdd-workflow step {}", step.name)),
    }
    command
        .current_dir(repo_root)
        .output()
        .with_context(|| format!("failed to spawn {}", step.name))
}

fn unix_seconds_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn codex_prompt_helpers(codex_dir: &Path) -> Vec<PlannedFile> {
    vec![
        PlannedFile {
            path: codex_dir.join("helpers/install-prompts.sh"),
            bytes: normalize_generated_text(
                r#"#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
codex_home="${repo_root}/.codex"

cargo run -p codex-env -- --repo "${repo_root}" install --codex-home "${codex_home}"

cat <<'MSG'
Installed Codex mirror surface and verified repo-local prompt commands.
Restart Codex from this repo, then invoke Claude command mirrors as /prompts:<name>.
Examples: /prompts:sparc-code, /prompts:sparc:code, /prompts:claude-flow-swarm
MSG
"#,
            )
            .into_bytes(),
            executable: true,
        },
        PlannedFile {
            path: codex_dir.join("helpers/run-claude-hook.sh"),
            bytes: normalize_generated_text(
                r#"#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 1 ]]; then
  echo "usage: run-claude-hook.sh <helper-file> [args...]" >&2
  exit 64
fi

repo_root="$(git rev-parse --show-toplevel)"
helper="$1"
shift

case "${helper}" in
  hook-handler.cjs|auto-memory-hook.mjs) ;;
  *)
    echo "unsupported Claude helper: ${helper}" >&2
    exit 64
    ;;
esac

export CLAUDE_PROJECT_DIR="${repo_root}"
exec node "${repo_root}/.codex/helpers/${helper}" "$@"
"#,
            )
            .into_bytes(),
            executable: true,
        },
    ]
}

fn write_file(file: &PlannedFile) -> Result<()> {
    if let Some(parent) = file.path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut temp = file.path.clone();
    temp.set_extension("tmp");
    {
        let mut handle = fs::File::create(&temp)?;
        handle.write_all(&file.bytes)?;
        handle.sync_all()?;
    }
    fs::rename(&temp, &file.path)?;
    set_executable(&file.path, file.executable)?;
    Ok(())
}

fn is_ignored_path(path: &Path) -> bool {
    path.components().any(|component| {
        let value = component.as_os_str().to_string_lossy();
        matches!(value.as_ref(), "node_modules" | ".git" | "target")
    })
}

#[cfg(unix)]
fn is_executable(path: &Path) -> Result<bool> {
    use std::os::unix::fs::PermissionsExt;
    Ok(fs::metadata(path)?.permissions().mode() & 0o111 != 0)
}

#[cfg(not(unix))]
fn is_executable(_path: &Path) -> Result<bool> {
    Ok(false)
}

#[cfg(unix)]
fn set_executable(path: &Path, executable: bool) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let mut permissions = fs::metadata(path)?.permissions();
    let mut mode = permissions.mode();
    if executable {
        mode |= 0o755;
    } else {
        mode &= !0o111;
    }
    permissions.set_mode(mode);
    fs::set_permissions(path, permissions)?;
    Ok(())
}

#[cfg(not(unix))]
fn set_executable(_path: &Path, _executable: bool) -> Result<()> {
    Ok(())
}

fn strip_repo_prefix(repo_root: &Path, path: &Path) -> PathBuf {
    path.strip_prefix(repo_root).unwrap_or(path).to_path_buf()
}

fn normalize_generated_bytes(path: &Path, bytes: Vec<u8>) -> Vec<u8> {
    let Some(extension) = path.extension().and_then(|value| value.to_str()) else {
        return bytes;
    };
    if !matches!(
        extension,
        "cjs" | "js" | "json" | "md" | "mjs" | "sh" | "toml" | "yaml" | "yml"
    ) {
        return bytes;
    }
    match String::from_utf8(bytes) {
        Ok(text) => normalize_generated_text(&text).into_bytes(),
        Err(error) => error.into_bytes(),
    }
}

fn normalize_generated_text(text: &str) -> String {
    let mut output = String::with_capacity(text.len());
    for line in text.split_inclusive('\n') {
        if let Some(line) = line.strip_suffix('\n') {
            output.push_str(line.trim_end_matches([' ', '\t', '\r']));
            output.push('\n');
        } else {
            output.push_str(line.trim_end_matches([' ', '\t', '\r']));
        }
    }
    output
}

fn escape_toml_string(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn slugify(value: &str) -> String {
    let mut slug = String::new();
    let mut last_dash = false;
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch.to_ascii_lowercase());
            last_dash = false;
        } else if !last_dash {
            slug.push('-');
            last_dash = true;
        }
    }
    slug.trim_matches('-').to_owned()
}

fn first_heading(markdown: &str) -> Option<String> {
    strip_leading_frontmatter(markdown)
        .lines()
        .find_map(|line| {
            let trimmed = line.trim();
            trimmed
                .strip_prefix("# ")
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(ToOwned::to_owned)
        })
}

fn strip_leading_frontmatter(markdown: &str) -> &str {
    let Some(rest) = markdown.strip_prefix("---\n") else {
        return markdown;
    };
    let Some(end) = rest.find("\n---\n") else {
        return markdown;
    };
    &rest[end + "\n---\n".len()..]
}

fn yaml_frontmatter_scalar(markdown: &str, key: &str) -> Option<String> {
    let rest = markdown.strip_prefix("---\n")?;
    let end = rest.find("\n---\n")?;
    let frontmatter = &rest[..end];
    for line in frontmatter.lines() {
        let Some((candidate, value)) = line.split_once(':') else {
            continue;
        };
        if candidate.trim() == key {
            let value = value.trim();
            if value.is_empty() || matches!(value, "|" | ">") {
                return None;
            }
            return Some(value.trim_matches('"').trim_matches('\'').trim().to_owned());
        }
    }
    None
}

fn yaml_scalar(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}
