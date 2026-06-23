use std::path::PathBuf;

use anyhow::{bail, Result};
use clap::{Parser, Subcommand};
use codex_env::{
    doctor_codex_surface, install_codex_env, install_codex_prompts, inventory_codex_surface,
    mirror_codex_surface, run_codex_auto_loop, run_codex_task, run_codex_team,
    CodexAutoLoopOptions, CodexInstallOptions, CodexInventoryOptions, CodexRunOptions,
    CodexTeamRunOptions, DoctorOptions, MirrorOptions, PromptInstallOptions,
};

#[derive(Parser)]
#[command(name = "codex-env")]
#[command(about = "Mirror the tracked .claude surface into a Codex-native env")]
struct Cli {
    /// Repository root. Defaults to the current directory.
    #[arg(long, global = true)]
    repo: Option<PathBuf>,

    /// Optional Lua policy script for repo-local transformations.
    #[arg(long, global = true)]
    lua_policy: Option<PathBuf>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Refresh the repo-local .codex surface, then run doctor.
    Install {
        /// Codex home directory for runtime settings. Defaults to this repo's .codex.
        #[arg(long)]
        codex_home: Option<PathBuf>,
    },

    /// Generate .codex hooks/config plus .agents skills from .claude.
    Mirror {
        /// Validate the generated surface without writing files.
        #[arg(long)]
        check: bool,
    },

    /// Verify generated repo-local .codex/prompts for /prompts:* usage.
    InstallPrompts {
        /// Deprecated compatibility option; prompts stay in this repo's .codex.
        #[arg(long)]
        codex_home: Option<PathBuf>,

        /// Validate installed prompts without writing files.
        #[arg(long)]
        check: bool,
    },

    /// Verify the generated Codex surface and repo-local prompt commands.
    Doctor {
        /// Codex home directory for runtime settings. Defaults to this repo's .codex.
        #[arg(long)]
        codex_home: Option<PathBuf>,

        /// Emit the doctor report as JSON.
        #[arg(long)]
        json: bool,
    },

    /// Inventory Claude-to-Codex parity coverage and fail on detected gaps.
    Inventory {
        /// Codex home directory. Defaults to CODEX_HOME or ~/.codex.
        #[arg(long)]
        codex_home: Option<PathBuf>,

        /// Emit the inventory report as JSON.
        #[arg(long)]
        json: bool,

        /// Fail if the inventory contains any parity gaps.
        #[arg(long)]
        check: bool,
    },

    /// Refresh the Codex env, then run codex exec with JSONL artifacts.
    Run {
        /// Goal to give the non-interactive Codex runner.
        goal: Option<String>,

        /// Read additional goal text from a file.
        #[arg(long)]
        prompt_file: Option<PathBuf>,

        /// Codex home directory. Defaults to CODEX_HOME or ~/.codex.
        #[arg(long)]
        codex_home: Option<PathBuf>,

        /// Directory for prompt/events/status artifacts. Defaults under .codex/harness/runs.
        #[arg(long)]
        output_dir: Option<PathBuf>,

        /// Materialize the run prompt and status without launching codex exec.
        #[arg(long)]
        dry_run: bool,

        /// Skip install and only run doctor before launching.
        #[arg(long)]
        skip_install: bool,
    },

    /// Run a generated Codex agent team in parallel with JSONL artifacts.
    TeamRun {
        /// Team name from .codex/agent-teams.json.
        #[arg(long, default_value = "core")]
        team: String,

        /// Goal to give every team member.
        goal: Option<String>,

        /// Read additional goal text from a file.
        #[arg(long)]
        prompt_file: Option<PathBuf>,

        /// Codex home directory. Defaults to CODEX_HOME or ~/.codex.
        #[arg(long)]
        codex_home: Option<PathBuf>,

        /// Directory for team artifacts. Defaults under .codex/harness/runs.
        #[arg(long)]
        output_dir: Option<PathBuf>,

        /// Sandbox for parallel team members. Defaults to read-only; parent consolidation owns writes.
        #[arg(long, default_value = "read-only")]
        member_sandbox: String,

        /// Materialize team prompts and status without launching codex exec.
        #[arg(long)]
        dry_run: bool,

        /// Skip install and only run doctor before launching.
        #[arg(long)]
        skip_install: bool,
    },

    /// Run bounded autonomous Codex team iterations until complete or max iterations.
    AutoLoop {
        /// Team name from .codex/agent-teams.json.
        #[arg(long, default_value = "core")]
        team: String,

        /// Goal to pursue through the auto-loop.
        goal: Option<String>,

        /// Read additional goal text from a file.
        #[arg(long)]
        prompt_file: Option<PathBuf>,

        /// Codex home directory. Defaults to CODEX_HOME or ~/.codex.
        #[arg(long)]
        codex_home: Option<PathBuf>,

        /// Directory for auto-loop artifacts. Defaults under .codex/harness/runs.
        #[arg(long)]
        output_dir: Option<PathBuf>,

        /// Maximum non-dry-run iterations before stopping.
        #[arg(long, default_value_t = 3)]
        max_iterations: usize,

        /// Sandbox for parallel team members. Defaults to read-only; parent consolidation owns writes.
        #[arg(long, default_value = "read-only")]
        member_sandbox: String,

        /// Materialize the first iteration prompts and status without launching codex exec.
        #[arg(long)]
        dry_run: bool,

        /// Skip install and only run doctor before launching.
        #[arg(long)]
        skip_install: bool,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let repo_root = cli.repo.unwrap_or(std::env::current_dir()?);

    match cli.command {
        Commands::Install { codex_home } => {
            let codex_home = codex_home.unwrap_or_else(|| default_codex_home(&repo_root));
            let report = install_codex_env(CodexInstallOptions {
                repo_root,
                lua_policy: cli.lua_policy,
                codex_home,
            })?;
            println!(
                "codex-env install ok: mirrored {} files ({} changed), verified {} repo-local prompts ({} changed), runtime settings {} at {}, doctor verified config {}/{}, {} MCP server(s), {} agents ({} config entries), {} team(s), {} team member reference(s), {} hook handler(s), {} shim-backed hook handler(s), {} helper mirrors, {} prompts ({} aliases) in {}",
                report.mirror.total_files,
                report.mirror.changed_files,
                report.prompts.total_files,
                report.prompts.changed_files,
                if report.home_settings.changed {
                    "updated"
                } else {
                    "verified"
                },
                report.home_settings.config_path.display(),
                report.doctor.config_model,
                report.doctor.config_reasoning_effort,
                report.doctor.config_mcp_servers.len(),
                report.doctor.agent_files,
                report.doctor.config_agent_entries,
                report.doctor.agent_teams,
                report.doctor.agent_team_members,
                report.doctor.hook_handlers,
                report.doctor.hook_shim_handlers,
                report.doctor.claude_helper_files,
                report.doctor.installed_prompt_files,
                report.doctor.prompt_alias_files,
                report.doctor.codex_dir.join("prompts").display()
            );
        }
        Commands::Mirror { check } => {
            let report = mirror_codex_surface(MirrorOptions {
                repo_root,
                lua_policy: cli.lua_policy,
                check,
            })?;
            println!(
                "codex-env mirrored {} files ({} changed, {} verified) from {}",
                report.total_files,
                report.changed_files,
                report.verified_files,
                report.claude_dir.display()
            );
        }
        Commands::InstallPrompts { codex_home, check } => {
            let codex_home = codex_home.unwrap_or_else(|| default_codex_home(&repo_root));
            let report = install_codex_prompts(PromptInstallOptions {
                repo_root,
                codex_home,
                check,
            })?;
            println!(
                "codex-env verified {} repo-local prompt files ({} changed, {} verified, {} stale removed) in {}",
                report.total_files,
                report.changed_files,
                report.verified_files,
                report.removed_files.len(),
                report.target_dir.display()
            );
        }
        Commands::Doctor { codex_home, json } => {
            let codex_home = codex_home.unwrap_or_else(|| default_codex_home(&repo_root));
            let report = doctor_codex_surface(DoctorOptions {
                repo_root,
                lua_policy: cli.lua_policy,
                codex_home,
            })?;
            if json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                println!(
                    "codex-env doctor ok: config {}/{}, approvals {}/{}, goals {}, home context {}, skills {}, {} MCP server(s), {} agents ({} config entries; {}), {} team(s), {} team member reference(s), {} hook event(s), {} hook handler(s), {} shim-backed hook handler(s), {} helper mirrors, {} repo-local prompts ({} aliases) in {}",
                    report.config_model,
                    report.config_reasoning_effort,
                    report.config_approval_policy,
                    report.config_approvals_reviewer,
                    report.config_goals_enabled,
                    report.codex_home_settings.model_context_window,
                    report.codex_home_settings.include_skill_instructions,
                    report.config_mcp_servers.len(),
                    report.agent_files,
                    report.config_agent_entries,
                    format_counts(&report.agent_models),
                    report.agent_teams,
                    report.agent_team_members,
                    report.hook_events.len(),
                    report.hook_handlers,
                    report.hook_shim_handlers,
                    report.claude_helper_files,
                    report.installed_prompt_files,
                    report.prompt_alias_files,
                    report.codex_dir.join("prompts").display()
                );
            }
        }
        Commands::Inventory {
            codex_home,
            json,
            check,
        } => {
            let codex_home = codex_home.unwrap_or_else(|| default_codex_home(&repo_root));
            let report = inventory_codex_surface(CodexInventoryOptions {
                repo_root,
                lua_policy: cli.lua_policy,
                codex_home,
            })?;
            if json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                println!(
                    "codex-env inventory: commands {} -> {} prompts ({} aliases, {} installed), agents {} -> {} Claude profiles / {} total, helpers {} -> {} mirrored ({} helper files), hooks {} -> {}, skills {} source-command / {} total, teams {} ({} members), MCP {}, gaps {}",
                    report.claude.command_files,
                    report.codex.prompt_files,
                    report.codex.prompt_alias_files,
                    report.codex.installed_prompt_files,
                    report.claude.agent_files,
                    report.codex.claude_agent_profiles,
                    report.codex.agent_profiles,
                    report.claude.helper_files,
                    report.codex.helper_mirror_files,
                    report.codex.helper_files,
                    report.claude.hook_files,
                    report.codex.hook_files,
                    report.codex.source_command_skills,
                    report.codex.skill_entrypoints,
                    report.codex.agent_teams,
                    report.codex.agent_team_members,
                    report.codex.mcp_servers,
                    report.gaps.len()
                );
                for gap in &report.gaps {
                    println!("gap: {gap}");
                }
            }
            if check && !report.gaps.is_empty() {
                bail!(
                    "codex-env inventory found {} parity gap(s)",
                    report.gaps.len()
                );
            }
        }
        Commands::Run {
            goal,
            prompt_file,
            codex_home,
            output_dir,
            dry_run,
            skip_install,
        } => {
            let codex_home = codex_home.unwrap_or_else(|| default_codex_home(&repo_root));
            let report = run_codex_task(CodexRunOptions {
                repo_root,
                lua_policy: cli.lua_policy,
                codex_home,
                goal,
                prompt_file,
                output_dir,
                dry_run,
                skip_install,
            })?;
            println!(
                "codex-env run {}: run_dir={}, prompt={}, events={}, stderr={}, last_message={}, status={}, exit_code={}",
                if report.dry_run { "prepared" } else { "ok" },
                report.run_dir.display(),
                report.prompt_path.display(),
                report.events_path.display(),
                report.stderr_path.display(),
                report.last_message_path.display(),
                report.status_path.display(),
                report
                    .exit_code
                    .map_or_else(|| "not-run".to_owned(), |code| code.to_string())
            );
        }
        Commands::TeamRun {
            team,
            goal,
            prompt_file,
            codex_home,
            output_dir,
            member_sandbox,
            dry_run,
            skip_install,
        } => {
            let codex_home = codex_home.unwrap_or_else(|| default_codex_home(&repo_root));
            let report = run_codex_team(CodexTeamRunOptions {
                repo_root,
                lua_policy: cli.lua_policy,
                codex_home,
                team,
                goal,
                prompt_file,
                output_dir,
                member_sandbox_mode: member_sandbox,
                dry_run,
                skip_install,
            })?;
            println!(
                "codex-env team-run {}: team={}, strategy={}, members={}, member_sandbox={}, run_dir={}, consolidation_prompt={}, consolidation_last_message={}, status={}",
                if report.dry_run { "prepared" } else { "ok" },
                report.team,
                report.strategy,
                report.members.len(),
                report.member_sandbox_mode,
                report.run_dir.display(),
                report.consolidation_prompt_path.display(),
                report.consolidation_run.last_message_path.display(),
                report.status_path.display()
            );
        }
        Commands::AutoLoop {
            team,
            goal,
            prompt_file,
            codex_home,
            output_dir,
            max_iterations,
            member_sandbox,
            dry_run,
            skip_install,
        } => {
            let codex_home = codex_home.unwrap_or_else(|| default_codex_home(&repo_root));
            let report = run_codex_auto_loop(CodexAutoLoopOptions {
                repo_root,
                lua_policy: cli.lua_policy,
                codex_home,
                team,
                goal,
                prompt_file,
                output_dir,
                max_iterations,
                member_sandbox_mode: member_sandbox,
                dry_run,
                skip_install,
            })?;
            println!(
                "codex-env auto-loop {}: team={}, iterations={}/{}, completed={}, marker={}, run_dir={}, status={}",
                if report.dry_run { "prepared" } else { "ok" },
                report.team,
                report.iterations.len(),
                report.max_iterations,
                report.completed,
                report
                    .completion_marker
                    .clone()
                    .unwrap_or_else(|| "none".to_owned()),
                report.run_dir.display(),
                report.status_path.display()
            );
        }
    }

    Ok(())
}

fn format_counts(counts: &std::collections::BTreeMap<String, usize>) -> String {
    counts
        .iter()
        .map(|(key, count)| format!("{key}={count}"))
        .collect::<Vec<_>>()
        .join(", ")
}

fn default_codex_home(repo_root: &std::path::Path) -> PathBuf {
    repo_root.join(".codex")
}
