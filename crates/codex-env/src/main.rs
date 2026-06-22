use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};
use codex_env::{install_codex_prompts, mirror_codex_surface, MirrorOptions, PromptInstallOptions};

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
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let repo_root = cli.repo.unwrap_or(std::env::current_dir()?);

    match cli.command {
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
    }

    Ok(())
}

fn default_codex_home() -> PathBuf {
    std::env::var_os("CODEX_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".codex")))
        .unwrap_or_else(|| PathBuf::from(".codex"))
}
