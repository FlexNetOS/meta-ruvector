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
    codex_agent_profiles, codex_agent_teams_json, codex_agents_md, codex_config, codex_hooks_json,
    codex_native_workflow_prompts, codex_native_workflow_skills, command_skill_plan,
    copy_tree_plan, read_claude_env,
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
    planned.extend(codex_agent_profiles(&codex_dir));
    planned.extend(agent_role_plan.files);
    planned.extend(prompt_plan.files);
    planned.extend(codex_native_workflow_prompts(
        &codex_dir,
        &agent_role_plan.roles,
    ));
    planned.extend(codex_prompt_helpers(&codex_dir));
    planned.push(PlannedFile {
        path: codex_dir.join("hooks.json"),
        bytes: codex_hooks_json(&claude_dir)?.into_bytes(),
        executable: false,
    });
    planned.extend(copy_tree_plan(
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

    if !options.check {
        clean_raw_mirror(&codex_dir)?;
        clean_claude_agent_roles(&codex_dir)?;
        clean_codex_prompts(&codex_dir)?;
    }

    for file in &planned {
        let exists_with_same_content = fs::read(&file.path).is_ok_and(|bytes| bytes == file.bytes);
        if exists_with_same_content {
            verified_files += 1;
        } else {
            changed_files += 1;
        }
        generated.push(strip_repo_prefix(&repo_root, &file.path));
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
    let target_dir = options.codex_home.join("prompts");
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
            path: target_dir.join(entry.file_name()),
            bytes: fs::read(path)?,
            executable: false,
        });
    }
    planned.sort_by(|a, b| a.path.cmp(&b.path));

    let mut changed_files = 0;
    let mut verified_files = 0;
    for file in &planned {
        let exists_with_same_content = fs::read(&file.path).is_ok_and(|bytes| bytes == file.bytes);
        if exists_with_same_content {
            verified_files += 1;
        } else {
            changed_files += 1;
        }
        if !options.check {
            write_file(file)?;
        }
    }

    if options.check && changed_files > 0 {
        return Err(anyhow!(
            "Codex home prompts are stale: {changed_files} prompt file(s) differ"
        ));
    }

    Ok(PromptInstallReport {
        repo_root,
        source_dir,
        target_dir,
        total_files: planned.len(),
        changed_files,
        verified_files,
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
        catalog_path.to_string_lossy().as_ref(),
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
    if model_catalog_json != expected_catalog.to_string_lossy() {
        return Err(anyhow!(
            "{} must set model_catalog_json to {}, found {model_catalog_json}",
            config_path.display(),
            expected_catalog.display()
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

fn codex_prompt_helpers(codex_dir: &Path) -> Vec<PlannedFile> {
    vec![
        PlannedFile {
            path: codex_dir.join("helpers/install-prompts.sh"),
            bytes: normalize_generated_text(
                r#"#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
codex_home="${CODEX_HOME:-${HOME}/.codex}"

cargo run -p codex-env -- --repo "${repo_root}" install --codex-home "${codex_home}"

cat <<'MSG'
Installed Codex mirror surface and prompt commands.
Restart Codex, then invoke Claude command mirrors as /prompts:<name>.
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
exec node "${repo_root}/.claude/helpers/${helper}" "$@"
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
