use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};
use codex_env::{mirror_codex_surface, MirrorOptions};

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
    }

    Ok(())
}
