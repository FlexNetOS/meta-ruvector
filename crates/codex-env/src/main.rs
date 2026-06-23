use std::{fs, path::PathBuf};

use anyhow::{bail, Result};
use clap::{Parser, Subcommand};
use codex_env::{
    codex_tdd_next_action, codex_tdd_supervise, doctor_codex_surface, install_codex_env,
    install_codex_prompts, inventory_codex_surface, mirror_codex_surface, run_codex_auto_loop,
    run_codex_task, run_codex_tdd_auto_loop, run_codex_tdd_cycle, run_codex_tdd_drive,
    run_codex_team, CodexAutoLoopOptions, CodexInstallOptions, CodexInventoryOptions,
    CodexRunOptions, CodexTddAutoLoopOptions, CodexTddCycleOptions, CodexTddDriveOptions,
    CodexTddNextActionOptions, CodexTddSuperviseOptions, CodexTddWorkflowOptions,
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

    /// Build codex-env, then execute the Rust-owned Codex TDD workflow gates.
    TddWorkflow {
        /// Team name from .codex/agent-teams.json for team and auto-loop dry-run probes.
        #[arg(long, default_value = "core")]
        team: String,

        /// Goal to trace through the TDD workflow.
        goal: Option<String>,

        /// Codex home directory. Defaults to this repo's .codex.
        #[arg(long)]
        codex_home: Option<PathBuf>,

        /// Directory for TDD workflow status artifacts.
        #[arg(long)]
        output_dir: Option<PathBuf>,

        /// Materialize the workflow plan without running commands.
        #[arg(long)]
        dry_run: bool,
    },

    /// Read a TDD extraction plan and print the next Rust-owned action.
    TddNext {
        /// Path to tdd-extraction-plan.json. Defaults to the newest TDD workflow run.
        #[arg(long)]
        plan: Option<PathBuf>,

        /// Emit the next-action report as JSON.
        #[arg(long)]
        json: bool,

        /// Fail unless the plan is ready for autonomous loop handoff.
        #[arg(long)]
        check: bool,
    },

    /// Consume a TDD extraction plan and start the autonomous auto-loop handoff.
    TddAutoLoop {
        /// Path to tdd-extraction-plan.json. Defaults to the newest TDD workflow run.
        #[arg(long)]
        plan: Option<PathBuf>,

        /// Team name from .codex/agent-teams.json.
        #[arg(long, default_value = "core")]
        team: String,

        /// Codex home directory. Defaults to this repo's .codex.
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

        /// Materialize the first autonomous handoff iteration without launching codex exec.
        #[arg(long)]
        dry_run: bool,

        /// Supervisor guidance note to inject into the autonomous handoff prompt. Repeatable.
        #[arg(long = "supervisor-note")]
        supervisor_notes: Vec<String>,

        /// File containing supervisor guidance to inject into the autonomous handoff prompt. Repeatable.
        #[arg(long = "supervisor-note-file")]
        supervisor_note_files: Vec<PathBuf>,

        /// Skip install and only run doctor before launching.
        #[arg(long)]
        skip_install: bool,
    },

    /// Run the full supervised TDD workflow, plan validation, and auto-loop handoff cycle.
    TddCycle {
        /// Team name from .codex/agent-teams.json.
        #[arg(long, default_value = "core")]
        team: String,

        /// Goal to trace through the full TDD cycle.
        goal: Option<String>,

        /// Codex home directory. Defaults to this repo's .codex.
        #[arg(long)]
        codex_home: Option<PathBuf>,

        /// Directory for cycle artifacts. Defaults under .codex/harness/runs.
        #[arg(long)]
        output_dir: Option<PathBuf>,

        /// Maximum non-dry-run auto-loop iterations before stopping.
        #[arg(long, default_value_t = 3)]
        max_iterations: usize,

        /// Sandbox for parallel team members. Defaults to read-only; parent consolidation owns writes.
        #[arg(long, default_value = "read-only")]
        member_sandbox: String,

        /// Materialize the cycle plan without building or launching nested commands.
        #[arg(long)]
        dry_run: bool,

        /// Launch the handoff auto-loop instead of only preparing its artifacts.
        #[arg(long)]
        run_handoff: bool,

        /// Supervisor guidance note to inject into the autonomous handoff prompt. Repeatable.
        #[arg(long = "supervisor-note")]
        supervisor_notes: Vec<String>,

        /// File containing supervisor guidance to inject into the autonomous handoff prompt. Repeatable.
        #[arg(long = "supervisor-note-file")]
        supervisor_note_files: Vec<PathBuf>,

        /// Skip install and only run doctor before launching nested Codex.
        #[arg(long)]
        skip_install: bool,
    },

    /// Inspect a TDD cycle status and emit the next supervisor decision.
    TddSupervise {
        /// Path to tdd-cycle-status.json. Defaults to the newest TDD cycle run.
        #[arg(long)]
        status: Option<PathBuf>,

        /// Emit the supervisor decision as JSON.
        #[arg(long)]
        json: bool,

        /// Fail if the cycle requires guidance before proceeding.
        #[arg(long)]
        check: bool,
    },

    /// Inspect the supervisor decision and drive the next safe TDD action.
    TddDrive {
        /// Path to tdd-cycle-status.json. Defaults to the newest TDD cycle run.
        #[arg(long)]
        status: Option<PathBuf>,

        /// Team name from .codex/agent-teams.json.
        #[arg(long, default_value = "core")]
        team: String,

        /// Goal to use when the decision launches a new TDD cycle.
        goal: Option<String>,

        /// Codex home directory. Defaults to this repo's .codex.
        #[arg(long)]
        codex_home: Option<PathBuf>,

        /// Directory for drive artifacts. Defaults under .codex/harness/runs.
        #[arg(long)]
        output_dir: Option<PathBuf>,

        /// Maximum non-dry-run auto-loop iterations before stopping.
        #[arg(long, default_value_t = 3)]
        max_iterations: usize,

        /// Sandbox for parallel team members. Defaults to read-only; parent consolidation owns writes.
        #[arg(long, default_value = "read-only")]
        member_sandbox: String,

        /// Only write tdd-drive-status.json; do not execute the selected next action.
        #[arg(long)]
        dry_run: bool,

        /// Launch the handoff auto-loop when the supervision decision reaches prepared handoff state.
        #[arg(long)]
        run_handoff: bool,

        /// Supervisor guidance note to inject when a driven action launches a handoff prompt. Repeatable.
        #[arg(long = "supervisor-note")]
        supervisor_notes: Vec<String>,

        /// File containing supervisor guidance to inject when a driven action launches a handoff prompt. Repeatable.
        #[arg(long = "supervisor-note-file")]
        supervisor_note_files: Vec<PathBuf>,

        /// Skip install and only run doctor before launching nested Codex.
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
        Commands::TddWorkflow {
            team,
            goal,
            codex_home,
            output_dir,
            dry_run,
        } => {
            let codex_home = codex_home.unwrap_or_else(|| default_codex_home(&repo_root));
            let report = codex_env::run_codex_tdd_workflow(CodexTddWorkflowOptions {
                repo_root,
                lua_policy: cli.lua_policy,
                codex_home,
                output_dir,
                team,
                goal,
                dry_run,
            })?;
            println!(
                "codex-env tdd-workflow {}: operator={}, steps={}, run_dir={}, status={}, extraction_report={}, extraction_plan={}",
                if report.dry_run { "planned" } else { "ok" },
                report.operator_role,
                report.steps.len(),
                report.run_dir.display(),
                report.status_path.display(),
                report.extraction_report_path.display(),
                report.extraction_plan_path.display()
            );
            for step in &report.steps {
                println!(
                    "step: {} status={} exit_code={} command={}",
                    step.name,
                    step.status,
                    step.exit_code
                        .map_or_else(|| "not-run".to_owned(), |code| code.to_string()),
                    step.command
                );
            }
        }
        Commands::TddNext { plan, json, check } => {
            let report = codex_tdd_next_action(CodexTddNextActionOptions {
                repo_root,
                plan_path: plan,
                check,
            })?;
            if json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                println!(
                    "codex-env tdd-next: plan={}, target={}, forbidden={}, ready={}, selected_actions={}, next_action={}",
                    report.plan_path.display(),
                    report.target_crate,
                    report.forbidden_target,
                    report.ready_for_autonomous_loop,
                    report.selected_actions.len(),
                    report.next_action
                );
                for action in &report.selected_actions {
                    println!(
                        "action: {} status={} target={} next={}",
                        action.step, action.status, action.extraction_target, action.next_action
                    );
                }
            }
        }
        Commands::TddAutoLoop {
            plan,
            team,
            codex_home,
            output_dir,
            max_iterations,
            member_sandbox,
            dry_run,
            supervisor_notes,
            supervisor_note_files,
            skip_install,
        } => {
            let codex_home = codex_home.unwrap_or_else(|| default_codex_home(&repo_root));
            let supervisor_guidance =
                read_supervisor_guidance(supervisor_notes, supervisor_note_files)?;
            let report = run_codex_tdd_auto_loop(CodexTddAutoLoopOptions {
                repo_root,
                lua_policy: cli.lua_policy,
                codex_home,
                plan_path: plan,
                team,
                output_dir,
                max_iterations,
                member_sandbox_mode: member_sandbox,
                supervisor_guidance,
                dry_run,
                skip_install,
            })?;
            println!(
                "codex-env tdd-auto-loop {}: plan={}, target={}, handoff_state={}, auto_loop_run_dir={}, status={}, iterations={}/{}, completed={}, next_action={}",
                if report.auto_loop.dry_run { "prepared" } else { "ok" },
                report.next_action.plan_path.display(),
                report.next_action.target_crate,
                report.handoff_state,
                report.auto_loop.run_dir.display(),
                report.status_path.display(),
                report.auto_loop.iterations.len(),
                report.auto_loop.max_iterations,
                report.auto_loop.completed,
                report.next_action.next_action
            );
        }
        Commands::TddCycle {
            team,
            goal,
            codex_home,
            output_dir,
            max_iterations,
            member_sandbox,
            dry_run,
            run_handoff,
            supervisor_notes,
            supervisor_note_files,
            skip_install,
        } => {
            let codex_home = codex_home.unwrap_or_else(|| default_codex_home(&repo_root));
            let supervisor_guidance =
                read_supervisor_guidance(supervisor_notes, supervisor_note_files)?;
            let report = run_codex_tdd_cycle(CodexTddCycleOptions {
                repo_root,
                lua_policy: cli.lua_policy,
                codex_home,
                output_dir,
                team,
                goal,
                max_iterations,
                member_sandbox_mode: member_sandbox,
                supervisor_guidance,
                dry_run,
                handoff_dry_run: !run_handoff,
                skip_install,
            })?;
            println!(
                "codex-env tdd-cycle {}: state={}, run_dir={}, status={}, guidance={}, workflow_status={}, extraction_plan={}, next_ready={}, auto_loop_status={}",
                if report.dry_run { "planned" } else { "ok" },
                report.cycle_state,
                report.run_dir.display(),
                report.status_path.display(),
                report.guidance_path.display(),
                report.workflow.status_path.display(),
                report.workflow.extraction_plan_path.display(),
                report
                    .next_action
                    .as_ref()
                    .is_some_and(|next| next.ready_for_autonomous_loop),
                report
                    .auto_loop
                    .as_ref()
                    .map(|auto_loop| auto_loop.status_path.display().to_string())
                    .unwrap_or_else(|| "not-run".to_owned())
            );
        }
        Commands::TddSupervise {
            status,
            json,
            check,
        } => {
            let report = codex_tdd_supervise(CodexTddSuperviseOptions {
                repo_root,
                status_path: status,
                check,
            })?;
            if json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                println!(
                    "codex-env tdd-supervise: decision={}, state={}, status={}, decision_path={}, next_action={}, next_command={}",
                    report.decision,
                    report.cycle_state,
                    report.status_path.display(),
                    report.decision_path.display(),
                    report.next_action,
                    report.next_command.unwrap_or_else(|| "none".to_owned())
                );
            }
        }
        Commands::TddDrive {
            status,
            team,
            goal,
            codex_home,
            output_dir,
            max_iterations,
            member_sandbox,
            dry_run,
            run_handoff,
            supervisor_notes,
            supervisor_note_files,
            skip_install,
        } => {
            let codex_home = codex_home.unwrap_or_else(|| default_codex_home(&repo_root));
            let supervisor_guidance =
                read_supervisor_guidance(supervisor_notes, supervisor_note_files)?;
            let report = run_codex_tdd_drive(CodexTddDriveOptions {
                repo_root,
                lua_policy: cli.lua_policy,
                codex_home,
                status_path: status,
                output_dir,
                team,
                goal,
                max_iterations,
                member_sandbox_mode: member_sandbox,
                supervisor_guidance,
                dry_run,
                run_handoff,
                skip_install,
            })?;
            println!(
                "codex-env tdd-drive {}: state={}, decision={}, run_dir={}, status={}, next_action={}",
                if report.dry_run { "planned" } else { "ok" },
                report.drive_state,
                report.supervision.decision,
                report.run_dir.display(),
                report.status_path.display(),
                report.supervision.next_action
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

fn read_supervisor_guidance(
    mut notes: Vec<String>,
    note_files: Vec<PathBuf>,
) -> Result<Vec<String>> {
    for path in note_files {
        let note = fs::read_to_string(&path)?;
        notes.push(note);
    }
    notes.retain(|note| !note.trim().is_empty());
    Ok(notes)
}

fn default_codex_home(repo_root: &std::path::Path) -> PathBuf {
    repo_root.join(".codex")
}
