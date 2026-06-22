use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};
use codex_env::{
    doctor_codex_surface, install_codex_env, install_codex_prompts, mirror_codex_surface,
    CodexInstallOptions, DoctorOptions, MirrorOptions, PromptInstallOptions,
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
    /// Refresh .codex, install prompt commands into CODEX_HOME, then run doctor.
    Install {
        /// Codex home directory. Defaults to CODEX_HOME or ~/.codex.
        #[arg(long)]
        codex_home: Option<PathBuf>,
    },

    /// Generate .codex hooks/config plus .agents skills from .claude.
    Mirror {
        /// Validate the generated surface without writing files.
        #[arg(long)]
        check: bool,
    },

    /// Install generated .codex/prompts into CODEX_HOME prompts for /prompts:* usage.
    InstallPrompts {
        /// Codex home directory. Defaults to CODEX_HOME or ~/.codex.
        #[arg(long)]
        codex_home: Option<PathBuf>,

        /// Validate installed prompts without writing files.
        #[arg(long)]
        check: bool,
    },

    /// Verify the generated Codex surface and installed prompt commands.
    Doctor {
        /// Codex home directory. Defaults to CODEX_HOME or ~/.codex.
        #[arg(long)]
        codex_home: Option<PathBuf>,

        /// Emit the doctor report as JSON.
        #[arg(long)]
        json: bool,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let repo_root = cli.repo.unwrap_or(std::env::current_dir()?);

    match cli.command {
        Commands::Install { codex_home } => {
            let codex_home = codex_home.unwrap_or_else(default_codex_home);
            let report = install_codex_env(CodexInstallOptions {
                repo_root,
                lua_policy: cli.lua_policy,
                codex_home,
            })?;
            println!(
                "codex-env install ok: mirrored {} files ({} changed), installed {} prompts ({} changed), home settings {} at {}, doctor verified config {}/{}, {} MCP server(s), {} agents ({} config entries), {} team(s), {} team member reference(s), {} hook handler(s), {} shim-backed hook handler(s), {} prompts ({} aliases) in {}",
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
                report.doctor.installed_prompt_files,
                report.doctor.prompt_alias_files,
                report.doctor.codex_home.join("prompts").display()
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
            let codex_home = codex_home.unwrap_or_else(default_codex_home);
            let report = install_codex_prompts(PromptInstallOptions {
                repo_root,
                codex_home,
                check,
            })?;
            println!(
                "codex-env installed {} prompt files ({} changed, {} verified) into {}",
                report.total_files,
                report.changed_files,
                report.verified_files,
                report.target_dir.display()
            );
        }
        Commands::Doctor { codex_home, json } => {
            let codex_home = codex_home.unwrap_or_else(default_codex_home);
            let report = doctor_codex_surface(DoctorOptions {
                repo_root,
                lua_policy: cli.lua_policy,
                codex_home,
            })?;
            if json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                println!(
                    "codex-env doctor ok: config {}/{}, approvals {}/{}, goals {}, home context {}, skills {}, {} MCP server(s), {} agents ({} config entries; {}), {} team(s), {} team member reference(s), {} hook event(s), {} hook handler(s), {} shim-backed hook handler(s), {} prompts ({} aliases) installed into {}",
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
                    report.installed_prompt_files,
                    report.prompt_alias_files,
                    report.codex_home.join("prompts").display()
                );
            }
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

fn default_codex_home() -> PathBuf {
    std::env::var_os("CODEX_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".codex")))
        .unwrap_or_else(|| PathBuf::from(".codex"))
}
