use std::fs;
use std::process::Command;

use codex_env::{
    doctor_codex_surface, ensure_codex_home_settings, install_codex_env, install_codex_prompts,
    inventory_codex_surface, mirror_codex_surface, run_codex_task, CodexAutoLoopOptions,
    CodexInstallOptions, CodexInventoryOptions, CodexRunOptions, CodexTeamRunOptions,
    DoctorOptions, MirrorOptions, PromptInstallOptions,
};

#[test]
fn mirror_generates_codex_and_skill_files() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    fs::create_dir_all(root.join(".claude/hooks")).unwrap();
    fs::create_dir_all(root.join(".claude/helpers")).unwrap();
    fs::create_dir_all(root.join(".claude/agents/core")).unwrap();
    fs::create_dir_all(root.join(".claude/agents/browser")).unwrap();
    fs::create_dir_all(root.join(".claude/skills/demo")).unwrap();
    fs::create_dir_all(root.join(".claude/commands/sparc")).unwrap();
    fs::write(
        root.join(".claude/settings.json"),
        r#"{
          "hooks": {
            "PreToolUse": [
              {
                "matcher": "Bash",
                "hooks": [
                  {
                    "type": "command",
                    "command": "node \"${CLAUDE_PROJECT_DIR:-.}/.claude/helpers/hook-handler.cjs\" pre-bash",
                    "timeout": 5000
                  }
                ]
              }
            ],
            "PostToolUse": [
              {
                "matcher": "Bash",
                "hooks": [
                  {
                    "type": "command",
                    "command": "echo done",
                    "timeout": 5
                  },
                  {
                    "type": "command",
                    "command": "echo async",
                    "timeout": 10000,
                    "async": true
                  }
                ]
              }
            ],
            "SessionEnd": [
              {
                "hooks": [
                  {
                    "type": "command",
                    "command": "node \"${CLAUDE_PROJECT_DIR:-.}/.claude/helpers/hook-handler.cjs\" session-end",
                    "timeout": 10000
                  }
                ]
              }
            ],
            "Stop": [
              {
                "hooks": [
                  {
                    "type": "command",
                    "command": "node \"${CLAUDE_PROJECT_DIR:-.}/.claude/helpers/auto-memory-hook.mjs\" sync",
                    "timeout": 10000
                  }
                ]
              }
            ],
            "Notification": [
              {
                "hooks": [
                  {
                    "type": "command",
                    "command": "echo notify",
                    "timeout": 3000
                  }
                ]
              }
            ]
          },
          "env": {"BRAIN_URL":"https://pi.ruv.io"}
        }"#,
    )
    .unwrap();
    fs::write(
        root.join(".claude/hooks/rust-check.sh"),
        "#!/bin/sh\necho exact   \n",
    )
    .unwrap();
    fs::write(
        root.join(".claude/helpers/hook-handler.cjs"),
        "#!/usr/bin/env node\nconsole.log('[OK] hook handler')\n",
    )
    .unwrap();
    fs::write(
        root.join(".claude/helpers/auto-memory-hook.mjs"),
        "#!/usr/bin/env node\nconsole.log('[OK] auto memory')\n",
    )
    .unwrap();
    fs::write(
        root.join(".claude/agents/core/coder.md"),
        "---\nname: coder\ndescription: Writes code\npriority: high\n---\n\n# Coder\nImplement carefully.\n",
    )
    .unwrap();
    for agent in ["planner", "researcher", "tester", "reviewer"] {
        fs::write(
            root.join(".claude/agents/core").join(format!("{agent}.md")),
            format!(
                "---\nname: {agent}\ndescription: Core {agent}\n---\n\n# {agent}\nWork carefully.\n"
            ),
        )
        .unwrap();
    }
    fs::write(
        root.join(".claude/agents/core/verbose.md"),
        format!(
            "---\ndescription: {}\n---\n\n# Verbose\nKeep the full source body.\n",
            "This description is intentionally long ".repeat(16)
        ),
    )
    .unwrap();
    fs::write(
        root.join(".claude/agents/browser/browser-agent.yaml"),
        "name: browser-agent\ndescription: Automates browsers\nrouting:\n  model: sonnet\n",
    )
    .unwrap();
    fs::write(root.join(".claude/skills/demo/SKILL.md"), "# Demo\n").unwrap();
    fs::write(
        root.join(".claude/commands/sparc/code.md"),
        "---\ndescription: Write code through SPARC\n---\n\n# Code\nBody with $ARGUMENTS and shell \"$FOO\".\n",
    )
    .unwrap();

    let report = mirror_codex_surface(MirrorOptions {
        repo_root: root.to_path_buf(),
        lua_policy: None,
        check: false,
    })
    .unwrap();

    assert!(report.changed_files > 0);
    let config = fs::read_to_string(root.join(".codex/config.toml")).unwrap();
    toml::from_str::<toml::Value>(&config).unwrap();
    assert!(config.contains("approval_policy = \"on-request\""));
    assert!(config.contains("approvals_reviewer = \"auto_review\""));
    assert!(config.contains("model = \"gpt-5.5\""));
    assert!(config.contains("model_reasoning_effort = \"high\""));
    assert!(config.contains("model_catalog_json = \"model-catalog.json\""));
    assert!(config.contains("model_context_window = 4000000"));
    let catalog = fs::read_to_string(root.join(".codex/model-catalog.json")).unwrap();
    assert!(catalog.contains("\"slug\": \"gpt-5.5\""));
    assert!(catalog.contains("\"context_window\": 4000000"));
    assert!(catalog.contains("\"max_context_window\": 4000000"));
    assert!(config.contains("[features]\nmulti_agent = true\ngoals = true"));
    assert!(config.contains("[skills]\ninclude_instructions = true"));
    for server in [
        "github",
        "context7",
        "exa",
        "memory",
        "playwright",
        "sequential-thinking",
        "claude-flow",
    ] {
        assert!(config.contains(&format!("[mcp_servers.{server}]")));
    }
    assert!(config.contains("CLAUDE_FLOW_MODE = \"v3\""));
    assert!(config.contains("CLAUDE_FLOW_TOPOLOGY = \"hierarchical-mesh\""));
    assert!(config.contains("[agents]\nmax_threads = 15\nmax_depth = 3"));
    assert!(config.contains("[agents.claude-browser-browser-agent]"));
    assert!(config.contains("config_file = \"agents/claude/claude-browser-browser-agent.toml\""));
    assert!(config.contains("[agents.claude-core-coder]"));
    assert!(config.contains("config_file = \"agents/claude/claude-core-coder.toml\""));
    let explorer = fs::read_to_string(root.join(".codex/agents/explorer.toml")).unwrap();
    let explorer: toml::Value = toml::from_str(&explorer).unwrap();
    assert_eq!(explorer["name"].as_str().unwrap(), "explorer");
    assert_eq!(explorer["model"].as_str().unwrap(), "gpt-5.4-mini");
    assert_eq!(
        explorer["description"].as_str().unwrap(),
        "Read-only codebase explorer for gathering evidence before changes are proposed."
    );
    let coder_role =
        fs::read_to_string(root.join(".codex/agents/claude/claude-core-coder.toml")).unwrap();
    toml::from_str::<toml::Value>(&coder_role).unwrap();
    assert!(coder_role.contains("name = \"claude-core-coder\""));
    assert!(coder_role.contains("description = \"Writes code\""));
    assert!(coder_role.contains("model = \"gpt-5.5\""));
    assert!(coder_role.contains("model_reasoning_effort = \"medium\""));
    assert!(coder_role.contains("developer_instructions = "));
    assert!(coder_role.contains("Source: `.claude/agents/core/coder.md`"));
    assert!(coder_role.contains("Implement carefully."));
    let verbose_role =
        fs::read_to_string(root.join(".codex/agents/claude/claude-core-verbose.toml")).unwrap();
    let verbose_role: toml::Value = toml::from_str(&verbose_role).unwrap();
    assert_eq!(verbose_role["model"].as_str().unwrap(), "gpt-5.4-mini");
    assert_eq!(
        verbose_role["model_reasoning_effort"].as_str().unwrap(),
        "medium"
    );
    assert!(
        verbose_role["description"]
            .as_str()
            .unwrap()
            .chars()
            .count()
            <= 240
    );
    assert!(verbose_role["description"]
        .as_str()
        .unwrap()
        .ends_with("..."));
    assert!(verbose_role["developer_instructions"]
        .as_str()
        .unwrap()
        .contains("Keep the full source body."));
    let browser_role =
        fs::read_to_string(root.join(".codex/agents/claude/claude-browser-browser-agent.toml"))
            .unwrap();
    toml::from_str::<toml::Value>(&browser_role).unwrap();
    assert!(browser_role.contains("description = \"Automates browsers\""));
    assert!(root.join(".codex/hooks/rust-check.sh").exists());
    assert!(root.join(".codex/helpers/run-claude-hook.sh").exists());
    assert!(root.join(".codex/helpers/hook-handler.cjs").exists());
    assert!(root.join(".codex/helpers/auto-memory-hook.mjs").exists());
    let hook_shim = fs::read_to_string(root.join(".codex/helpers/run-claude-hook.sh")).unwrap();
    assert!(hook_shim.contains(".codex/helpers/${helper}"));
    assert!(!hook_shim.contains(".claude/helpers/${helper}"));
    let hooks: serde_json::Value =
        serde_json::from_slice(&fs::read(root.join(".codex/hooks.json")).unwrap()).unwrap();
    assert!(hooks["hooks"]["Notification"].is_null());
    let pre_tool = &hooks["hooks"]["PreToolUse"][0]["hooks"][0];
    assert_eq!(pre_tool["timeout"], 5);
    assert!(pre_tool["command"]
        .as_str()
        .unwrap()
        .contains(".codex/helpers/run-claude-hook.sh\" hook-handler.cjs pre-bash"));
    let post_tool_hooks = hooks["hooks"]["PostToolUse"][0]["hooks"]
        .as_array()
        .unwrap();
    assert_eq!(post_tool_hooks.len(), 1);
    assert_eq!(post_tool_hooks[0]["command"], "echo done");
    assert_eq!(post_tool_hooks[0]["timeout"], 5);
    let stop_hooks = hooks["hooks"]["Stop"].as_array().unwrap();
    assert_eq!(stop_hooks.len(), 2);
    assert!(stop_hooks.iter().any(|group| group["hooks"][0]["command"]
        .as_str()
        .unwrap()
        .contains("hook-handler.cjs session-end")));
    assert!(stop_hooks.iter().any(|group| group["hooks"][0]["command"]
        .as_str()
        .unwrap()
        .contains("auto-memory-hook.mjs sync")));
    assert_eq!(
        fs::read(root.join(".codex/mirror/.claude/hooks/rust-check.sh")).unwrap(),
        fs::read(root.join(".claude/hooks/rust-check.sh")).unwrap()
    );
    assert!(root.join(".agents/skills/demo/SKILL.md").exists());
    assert!(root
        .join(".agents/skills/source-command-sparc-code/SKILL.md")
        .exists());
    let prompt = fs::read_to_string(root.join(".codex/prompts/sparc-code.md")).unwrap();
    assert!(prompt.contains("description: 'Write code through SPARC'"));
    assert!(prompt.contains("argument-hint: [ARGUMENTS]"));
    assert!(prompt.contains("Claude Code command `/sparc:code`"));
    assert!(prompt.contains("Source: `.claude/commands/sparc/code.md`"));
    assert!(prompt.contains("Arguments supplied to this prompt: $ARGUMENTS"));
    assert!(prompt.contains("Body with $ARGUMENTS and shell \"$$FOO\"."));
    let alias_prompt = fs::read_to_string(root.join(".codex/prompts/sparc:code.md")).unwrap();
    assert_eq!(alias_prompt, prompt);
    assert!(root.join(".codex/prompts/codex-agent-team.md").exists());
    assert!(root.join(".codex/prompts/codex-auto-loop.md").exists());
    assert!(root.join(".codex/prompts/codex-gap-hunt.md").exists());
    let teams: serde_json::Value =
        serde_json::from_slice(&fs::read(root.join(".codex/agent-teams.json")).unwrap()).unwrap();
    assert_eq!(teams["schemaVersion"], 1);
    assert_eq!(teams["teams"].as_array().unwrap().len(), 6);
    let core_team = teams["teams"]
        .as_array()
        .unwrap()
        .iter()
        .find(|team| team["name"] == "core")
        .unwrap();
    assert!(core_team["agents"]
        .as_array()
        .unwrap()
        .iter()
        .any(|agent| agent == "claude-core-coder"));
    assert!(root.join(".codex/helpers/install-prompts.sh").exists());
    let install_helper =
        fs::read_to_string(root.join(".codex/helpers/install-prompts.sh")).unwrap();
    assert!(install_helper.contains("codex-env -- --repo \"${repo_root}\" install"));
    assert!(root
        .join(".agents/skills/codex-agent-team/SKILL.md")
        .exists());
    assert!(root
        .join(".agents/skills/codex-auto-loop/SKILL.md")
        .exists());
    assert!(root.join(".agents/skills/codex-gap-hunt/SKILL.md").exists());

    let inventory: serde_json::Value =
        serde_json::from_slice(&fs::read(root.join(".codex/mirror-symbols.json")).unwrap())
            .unwrap();
    assert_eq!(inventory["sourceFileCount"], 13);
    let command_entry = inventory["entries"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| {
            entry["source"] == ".claude/commands/sparc/code.md" && entry["kind"] == "command"
        })
        .unwrap();
    assert_eq!(command_entry["sourceSha256"], command_entry["mirrorSha256"]);
    assert_eq!(command_entry["sourceSha256"].as_str().unwrap().len(), 64);

    let manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(root.join(".codex/mirror-manifest.json")).unwrap())
            .unwrap();
    assert_eq!(manifest["fileCount"], report.total_files);
    assert!(manifest["files"]
        .as_array()
        .unwrap()
        .iter()
        .any(|file| file == ".codex/mirror-manifest.json"));

    let check = mirror_codex_surface(MirrorOptions {
        repo_root: root.to_path_buf(),
        lua_policy: None,
        check: true,
    })
    .unwrap();
    assert_eq!(check.changed_files, 0);

    let codex_home = root.join("codex-home");
    install_codex_prompts(PromptInstallOptions {
        repo_root: root.to_path_buf(),
        codex_home: codex_home.clone(),
        check: false,
    })
    .unwrap();
    ensure_codex_home_settings(&codex_home).unwrap();
    let doctor = doctor_codex_surface(DoctorOptions {
        repo_root: root.to_path_buf(),
        lua_policy: None,
        codex_home: codex_home.clone(),
    })
    .unwrap();
    assert_eq!(doctor.config_model, "gpt-5.5");
    assert_eq!(doctor.config_reasoning_effort, "high");
    assert_eq!(doctor.config_model_catalog_json, "model-catalog.json");
    assert_eq!(doctor.config_approval_policy, "on-request");
    assert_eq!(doctor.config_approvals_reviewer, "auto_review");
    assert!(doctor.config_goals_enabled);
    assert_eq!(doctor.codex_home_settings.model_context_window, 4_000_000);
    assert!(doctor.codex_home_settings.include_skill_instructions);
    assert_eq!(doctor.config_mcp_servers.len(), 7);
    assert!(doctor
        .config_mcp_servers
        .contains(&"claude-flow".to_owned()));
    assert_eq!(doctor.config_agent_entries, doctor.agent_files);
    assert_eq!(doctor.agent_teams, 6);
    assert!(doctor.agent_team_members >= 12);
    assert_eq!(doctor.prompt_files, 5);
    assert_eq!(doctor.prompt_alias_files, 1);
    assert_eq!(doctor.installed_prompt_files, 5);
    assert_eq!(doctor.claude_helper_files, 2);
    assert!(doctor.codex_helper_files >= doctor.claude_helper_files);
    assert!(doctor.agent_models.contains_key("gpt-5.5"));
    assert!(doctor.agent_models.contains_key("gpt-5.4-mini"));
    assert!(doctor.hook_events.contains(&"Stop".to_owned()));
    assert_eq!(doctor.hook_shim_handlers, 3);

    let codex_inventory = inventory_codex_surface(CodexInventoryOptions {
        repo_root: root.to_path_buf(),
        lua_policy: None,
        codex_home,
    })
    .unwrap();
    assert!(codex_inventory.gaps.is_empty());
    assert_eq!(codex_inventory.claude.command_files, 1);
    assert_eq!(codex_inventory.codex.source_command_skills, 1);
    assert_eq!(
        codex_inventory.expected.prompt_files,
        codex_inventory.codex.prompt_files
    );
    assert_eq!(codex_inventory.claude.helper_files, 2);
    assert_eq!(codex_inventory.codex.helper_mirror_files, 2);
    assert_eq!(
        codex_inventory.expected.claude_agent_profiles,
        codex_inventory.codex.claude_agent_profiles
    );
}

#[test]
fn install_refreshes_mirror_prompts_and_doctor_in_one_step() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("repo");
    let codex_home = temp.path().join("codex-home");
    fs::create_dir_all(root.join(".claude/commands")).unwrap();
    fs::create_dir_all(root.join(".claude/hooks")).unwrap();
    fs::write(
        root.join(".claude/settings.json"),
        r#"{
          "hooks": {
            "Stop": [
              {
                "hooks": [
                  {
                    "type": "command",
                    "command": "echo stop",
                    "timeout": 5
                  }
                ]
              }
            ]
          },
          "env": {}
        }"#,
    )
    .unwrap();
    fs::write(
        root.join(".claude/commands/demo.md"),
        "---\ndescription: Demo prompt\n---\n\n# Demo\nUse $ARGUMENTS.\n",
    )
    .unwrap();

    let report = install_codex_env(CodexInstallOptions {
        repo_root: root.clone(),
        lua_policy: None,
        codex_home: codex_home.clone(),
    })
    .unwrap();

    assert!(report.mirror.changed_files > 0);
    assert_eq!(report.prompts.total_files, 4);
    assert_eq!(report.doctor.prompt_files, 4);
    assert_eq!(report.doctor.prompt_alias_files, 0);
    assert!(report.home_settings.changed);
    assert_eq!(report.home_settings.approvals_reviewer, "auto_review");
    assert_eq!(
        report.home_settings.model_catalog_json,
        codex_home.join("model-catalog.json").to_string_lossy()
    );
    assert_eq!(report.home_settings.model_context_window, 4_000_000);
    assert!(report.home_settings.goals_enabled);
    assert!(report.home_settings.include_skill_instructions);
    assert_eq!(
        report.doctor.config_agent_entries,
        report.doctor.agent_files
    );
    assert_eq!(report.doctor.installed_prompt_files, 4);
    assert!(root.join(".codex/config.toml").exists());
    assert!(root.join(".codex/model-catalog.json").exists());
    assert!(codex_home.join("model-catalog.json").exists());
    assert!(codex_home.join("prompts/demo.md").exists());
    fs::write(codex_home.join("prompts/stale-command.md"), "stale").unwrap();

    let stale_doctor = doctor_codex_surface(DoctorOptions {
        repo_root: root.clone(),
        lua_policy: None,
        codex_home: codex_home.clone(),
    })
    .unwrap_err();
    assert!(stale_doctor.to_string().contains("stale file"));

    let cleaned = install_codex_prompts(PromptInstallOptions {
        repo_root: root.clone(),
        codex_home: codex_home.clone(),
        check: false,
    })
    .unwrap();
    assert_eq!(cleaned.removed_files.len(), 1);
    assert!(!codex_home.join("prompts/stale-command.md").exists());

    let checked = doctor_codex_surface(DoctorOptions {
        repo_root: root,
        lua_policy: None,
        codex_home,
    })
    .unwrap();
    assert_eq!(checked.installed_prompt_files, 4);
}

#[test]
fn install_repairs_codex_home_runtime_settings_without_dropping_existing_config() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("repo");
    let codex_home = temp.path().join("codex-home");
    fs::create_dir_all(root.join(".claude/commands")).unwrap();
    fs::create_dir_all(&codex_home).unwrap();
    fs::write(
        root.join(".claude/settings.json"),
        r#"{"hooks":{"Stop":[{"hooks":[{"type":"command","command":"echo stop","timeout":5}]}]},"env":{}}"#,
    )
    .unwrap();
    fs::write(
        root.join(".claude/commands/demo.md"),
        "---\ndescription: Demo prompt\n---\n\n# Demo\nUse $ARGUMENTS.\n",
    )
    .unwrap();
    fs::write(
        codex_home.join("config.toml"),
        r#"model = "gpt-5"
model_reasoning_effort = "medium"
approvals_reviewer = "user"

[features]
memories = true

[mcp_servers.icm]
command = "/home/drdave/.local/bin/icm"
args = ["serve"]
"#,
    )
    .unwrap();

    let report = install_codex_env(CodexInstallOptions {
        repo_root: root,
        lua_policy: None,
        codex_home: codex_home.clone(),
    })
    .unwrap();

    assert!(report.home_settings.changed);
    assert_eq!(report.home_settings.model, "gpt-5.5");
    assert_eq!(report.home_settings.model_reasoning_effort, "high");
    assert_eq!(
        report.home_settings.model_catalog_json,
        codex_home.join("model-catalog.json").to_string_lossy()
    );
    assert_eq!(report.home_settings.approval_policy, "on-request");
    assert_eq!(report.home_settings.approvals_reviewer, "auto_review");
    assert_eq!(report.home_settings.model_context_window, 4_000_000);
    assert!(report.home_settings.multi_agent_enabled);
    assert!(report.home_settings.goals_enabled);
    assert!(report.home_settings.include_skill_instructions);

    let config = fs::read_to_string(codex_home.join("config.toml")).unwrap();
    assert!(config.contains("[mcp_servers.icm]"));
    assert!(config.contains("command = \"/home/drdave/.local/bin/icm\""));
    assert!(config.contains("memories = true"));
    assert!(config.contains("model_context_window = 4000000"));
    assert!(config.contains(&format!(
        "model_catalog_json = \"{}\"",
        codex_home.join("model-catalog.json").display()
    )));
    assert!(config.contains("approvals_reviewer = \"auto_review\""));
    assert!(config.contains("goals = true"));
    assert!(config.contains("[skills]\ninclude_instructions = true"));
}

#[test]
fn doctor_rejects_undeclared_custom_agent_files() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("repo");
    let codex_home = temp.path().join("codex-home");
    fs::create_dir_all(root.join(".claude/commands")).unwrap();
    fs::write(
        root.join(".claude/settings.json"),
        r#"{
          "hooks": {
            "Stop": [
              {
                "hooks": [
                  {
                    "type": "command",
                    "command": "echo stop",
                    "timeout": 5
                  }
                ]
              }
            ]
          },
          "env": {}
        }"#,
    )
    .unwrap();
    fs::write(
        root.join(".claude/commands/demo.md"),
        "---\ndescription: Demo prompt\n---\n\n# Demo\nUse $ARGUMENTS.\n",
    )
    .unwrap();

    install_codex_env(CodexInstallOptions {
        repo_root: root.clone(),
        lua_policy: None,
        codex_home: codex_home.clone(),
    })
    .unwrap();
    fs::write(
        root.join(".codex/agents/rogue.toml"),
        "name = \"rogue\"\ndescription = \"not in config\"\nmodel = \"gpt-5.5\"\nmodel_reasoning_effort = \"high\"\ndeveloper_instructions = \"missing config entry\"\n",
    )
    .unwrap();

    let error = doctor_codex_surface(DoctorOptions {
        repo_root: root,
        lua_policy: None,
        codex_home,
    })
    .unwrap_err();
    assert!(error.to_string().contains("missing from config"));
}

#[test]
fn doctor_rejects_hook_shim_with_missing_claude_helper() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("repo");
    let codex_home = temp.path().join("codex-home");
    fs::create_dir_all(root.join(".claude/commands")).unwrap();
    fs::write(
        root.join(".claude/settings.json"),
        r#"{
          "hooks": {
            "PreToolUse": [
              {
                "matcher": "Bash",
                "hooks": [
                  {
                    "type": "command",
                    "command": "node \"${CLAUDE_PROJECT_DIR:-.}/.claude/helpers/hook-handler.cjs\" pre-bash",
                    "timeout": 5000
                  }
                ]
              }
            ]
          },
          "env": {}
        }"#,
    )
    .unwrap();
    fs::write(
        root.join(".claude/commands/demo.md"),
        "---\ndescription: Demo prompt\n---\n\n# Demo\nUse $ARGUMENTS.\n",
    )
    .unwrap();

    mirror_codex_surface(MirrorOptions {
        repo_root: root.clone(),
        lua_policy: None,
        check: false,
    })
    .unwrap();
    install_codex_prompts(PromptInstallOptions {
        repo_root: root.clone(),
        codex_home: codex_home.clone(),
        check: false,
    })
    .unwrap();
    ensure_codex_home_settings(&codex_home).unwrap();

    let error = doctor_codex_surface(DoctorOptions {
        repo_root: root,
        lua_policy: None,
        codex_home,
    })
    .unwrap_err();
    assert!(error
        .to_string()
        .contains("references missing Codex helper"));
}

#[test]
fn install_prompts_copies_generated_prompt_commands() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("repo");
    let codex_home = temp.path().join("codex-home");
    fs::create_dir_all(root.join(".claude/commands")).unwrap();
    fs::write(root.join(".claude/settings.json"), r#"{"env":{}}"#).unwrap();
    fs::write(
        root.join(".claude/commands/demo.md"),
        "---\ndescription: Demo prompt\n---\n\n# Demo\nUse $ARGUMENTS.\n",
    )
    .unwrap();

    mirror_codex_surface(MirrorOptions {
        repo_root: root.clone(),
        lua_policy: None,
        check: false,
    })
    .unwrap();

    let report = install_codex_prompts(PromptInstallOptions {
        repo_root: root.clone(),
        codex_home: codex_home.clone(),
        check: false,
    })
    .unwrap();
    assert_eq!(report.total_files, 4);
    assert_eq!(report.changed_files, 4);
    assert_eq!(report.removed_files.len(), 0);
    assert!(codex_home.join("prompts/demo.md").exists());
    assert!(codex_home.join("prompts/codex-auto-loop.md").exists());
    fs::write(codex_home.join("prompts/stale-command.md"), "stale").unwrap();

    let stale_check = install_codex_prompts(PromptInstallOptions {
        repo_root: root.clone(),
        codex_home: codex_home.clone(),
        check: true,
    })
    .unwrap_err();
    assert!(stale_check.to_string().contains("stale file"));

    let cleaned = install_codex_prompts(PromptInstallOptions {
        repo_root: root.clone(),
        codex_home: codex_home.clone(),
        check: false,
    })
    .unwrap();
    assert_eq!(cleaned.changed_files, 0);
    assert_eq!(cleaned.removed_files.len(), 1);
    assert!(!codex_home.join("prompts/stale-command.md").exists());

    let check = install_codex_prompts(PromptInstallOptions {
        repo_root: root,
        codex_home,
        check: true,
    })
    .unwrap();
    assert_eq!(check.changed_files, 0);
    assert_eq!(check.removed_files.len(), 0);
}

#[test]
fn run_dry_run_materializes_bounded_codex_exec_artifacts() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("repo");
    let codex_home = temp.path().join("codex-home");
    let run_dir = temp.path().join("run");
    fs::create_dir_all(root.join(".claude/commands")).unwrap();
    fs::write(
        root.join(".claude/settings.json"),
        r#"{
          "hooks": {
            "PreToolUse": [
              {
                "matcher": "Bash",
                "hooks": [{"type": "command", "command": "echo pre", "timeout": 5}]
              }
            ]
          },
          "env": {}
        }"#,
    )
    .unwrap();
    fs::write(root.join(".claude/commands/demo.md"), "# Demo\n").unwrap();

    let report = run_codex_task(CodexRunOptions {
        repo_root: root,
        lua_policy: None,
        codex_home,
        goal: Some("inspect the generated Codex surface".to_owned()),
        prompt_file: None,
        output_dir: Some(run_dir),
        dry_run: true,
        skip_install: false,
    })
    .unwrap();

    assert!(report.dry_run);
    assert_eq!(report.exit_code, None);
    assert!(report.prompt_path.exists());
    assert!(report.status_path.exists());
    let prompt = fs::read_to_string(report.prompt_path).unwrap();
    assert!(prompt.contains("Do real work, not a plan."));
    assert!(prompt.contains("inspect the generated Codex surface"));
    let status = fs::read_to_string(report.status_path).unwrap();
    assert!(status.contains(r#""dryRun": true"#));
}

#[test]
fn team_run_dry_run_materializes_parallel_agent_artifacts() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("repo");
    let codex_home = temp.path().join("codex-home");
    let run_dir = temp.path().join("team-run");
    fs::create_dir_all(root.join(".claude/commands")).unwrap();
    fs::write(
        root.join(".claude/settings.json"),
        r#"{
          "hooks": {
            "PreToolUse": [
              {
                "matcher": "Bash",
                "hooks": [{"type": "command", "command": "echo pre", "timeout": 5}]
              }
            ]
          },
          "env": {}
        }"#,
    )
    .unwrap();
    fs::write(root.join(".claude/commands/demo.md"), "# Demo\n").unwrap();

    let report = codex_env::run_codex_team(CodexTeamRunOptions {
        repo_root: root,
        lua_policy: None,
        codex_home,
        team: "core".to_owned(),
        goal: Some("map the generated Codex team runner".to_owned()),
        prompt_file: None,
        output_dir: Some(run_dir),
        member_sandbox_mode: "read-only".to_owned(),
        dry_run: true,
        skip_install: false,
    })
    .unwrap();

    assert!(report.dry_run);
    assert_eq!(report.team, "core");
    assert_eq!(report.member_sandbox_mode, "read-only");
    assert!(report.members.len() >= 2);
    assert!(report.consolidation_prompt_path.exists());
    assert!(report.consolidation_run.prompt_path.exists());
    assert!(report.consolidation_run.status_path.exists());
    assert_eq!(report.consolidation_run.exit_code, None);
    assert!(report.status_path.exists());
    for member in &report.members {
        assert!(member.run.prompt_path.exists());
        let prompt = fs::read_to_string(&member.run.prompt_path).unwrap();
        assert!(prompt.contains("codex-env Team Member"));
        assert!(prompt.contains(&member.agent));
        assert!(prompt.contains("Execution sandbox: read-only"));
        assert!(prompt.contains("Parallel team members are evidence producers"));
        assert_eq!(member.sandbox_mode, "read-only");
        assert_eq!(member.run.exit_code, None);
    }
    let consolidation = fs::read_to_string(report.consolidation_prompt_path).unwrap();
    assert!(consolidation.contains("Team: core"));
    assert!(consolidation.contains("Member outputs:"));
    assert!(consolidation.contains("sandbox read-only"));
    assert!(consolidation.contains("parallel member runs as evidence-only"));
    let consolidation_prompt = fs::read_to_string(report.consolidation_run.prompt_path).unwrap();
    assert!(consolidation_prompt.contains("Consolidate the completed Codex team run."));
    let consolidation_status = fs::read_to_string(report.consolidation_run.status_path).unwrap();
    assert!(consolidation_status.contains(r#""dryRun": true"#));
}

#[test]
fn auto_loop_dry_run_materializes_bounded_iteration_artifacts() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("repo");
    let codex_home = temp.path().join("codex-home");
    let run_dir = temp.path().join("auto-loop");
    fs::create_dir_all(root.join(".claude/commands")).unwrap();
    fs::write(
        root.join(".claude/settings.json"),
        r#"{
          "hooks": {
            "PreToolUse": [
              {
                "matcher": "Bash",
                "hooks": [{"type": "command", "command": "echo pre", "timeout": 5}]
              }
            ]
          },
          "env": {}
        }"#,
    )
    .unwrap();
    fs::write(root.join(".claude/commands/demo.md"), "# Demo\n").unwrap();

    let report = codex_env::run_codex_auto_loop(CodexAutoLoopOptions {
        repo_root: root,
        lua_policy: None,
        codex_home,
        team: "core".to_owned(),
        goal: Some("drive the Codex parity loop".to_owned()),
        prompt_file: None,
        output_dir: Some(run_dir),
        max_iterations: 3,
        member_sandbox_mode: "read-only".to_owned(),
        dry_run: true,
        skip_install: false,
    })
    .unwrap();

    assert!(report.dry_run);
    assert!(!report.completed);
    assert_eq!(report.max_iterations, 3);
    assert_eq!(report.iterations.len(), 1);
    assert!(report.status_path.exists());
    let team_run = &report.iterations[0].team_run;
    assert!(team_run.run_dir.ends_with("iteration-01"));
    assert!(team_run.consolidation_run.prompt_path.exists());
    let prompt = fs::read_to_string(&team_run.consolidation_run.prompt_path).unwrap();
    assert!(prompt.contains("codex-env Auto Loop"));
    assert!(prompt.contains("CODEX_AUTO_LOOP_STATUS: complete"));
    assert!(prompt.contains("CODEX_AUTO_LOOP_STATUS: continue"));
    let status = fs::read_to_string(report.status_path).unwrap();
    assert!(status.contains(r#""max_iterations": 3"#));
    assert!(status.contains(r#""completed": false"#));
}

#[test]
fn doctor_rejects_gitignored_generated_surface_files() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    Command::new("git")
        .arg("init")
        .current_dir(root)
        .status()
        .unwrap();
    fs::write(
        root.join(".gitignore"),
        ".codex/mirror/.claude/hooks/rust-check.sh\n",
    )
    .unwrap();
    fs::create_dir_all(root.join(".claude/hooks")).unwrap();
    fs::create_dir_all(root.join(".claude/commands")).unwrap();
    fs::write(
        root.join(".claude/settings.json"),
        r#"{
          "hooks": {
            "PreToolUse": [
              {
                "matcher": "Bash",
                "hooks": [{"type": "command", "command": "echo pre", "timeout": 5}]
              }
            ]
          },
          "env": {}
        }"#,
    )
    .unwrap();
    fs::write(
        root.join(".claude/hooks/rust-check.sh"),
        "#!/bin/sh\necho ok\n",
    )
    .unwrap();
    fs::write(
        root.join(".claude/commands/demo.md"),
        "---\ndescription: Demo prompt\n---\n\n# Demo\n",
    )
    .unwrap();

    mirror_codex_surface(MirrorOptions {
        repo_root: root.to_path_buf(),
        lua_policy: None,
        check: false,
    })
    .unwrap();
    let codex_home = root.join("codex-home");
    install_codex_prompts(PromptInstallOptions {
        repo_root: root.to_path_buf(),
        codex_home: codex_home.clone(),
        check: false,
    })
    .unwrap();
    ensure_codex_home_settings(&codex_home).unwrap();

    let error = doctor_codex_surface(DoctorOptions {
        repo_root: root.to_path_buf(),
        lua_policy: None,
        codex_home,
    })
    .unwrap_err();
    assert!(error.to_string().contains("gitignored file"));
}

#[test]
fn mirror_skips_ignored_untracked_claude_local_files() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    Command::new("git")
        .arg("init")
        .current_dir(root)
        .status()
        .unwrap();
    fs::write(root.join(".gitignore"), ".claude/settings.local.json\n").unwrap();
    fs::create_dir_all(root.join(".claude")).unwrap();
    fs::write(root.join(".claude/settings.json"), r#"{"env":{}}"#).unwrap();
    fs::write(
        root.join(".claude/settings.local.json"),
        r#"{"permissions":{"defaultMode":"bypassPermissions"}}"#,
    )
    .unwrap();

    mirror_codex_surface(MirrorOptions {
        repo_root: root.to_path_buf(),
        lua_policy: None,
        check: false,
    })
    .unwrap();

    assert!(root.join(".codex/mirror/.claude/settings.json").exists());
    assert!(!root
        .join(".codex/mirror/.claude/settings.local.json")
        .exists());
    let manifest = fs::read_to_string(root.join(".codex/mirror-manifest.json")).unwrap();
    assert!(!manifest.contains("settings.local.json"));
}

#[test]
fn mirror_check_rejects_stale_raw_files() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    fs::create_dir_all(root.join(".claude")).unwrap();
    fs::write(root.join(".claude/settings.json"), r#"{"env":{}}"#).unwrap();

    mirror_codex_surface(MirrorOptions {
        repo_root: root.to_path_buf(),
        lua_policy: None,
        check: false,
    })
    .unwrap();
    fs::create_dir_all(root.join(".codex/mirror/.claude/stale")).unwrap();
    fs::write(root.join(".codex/mirror/.claude/stale/file.md"), "old").unwrap();

    let error = mirror_codex_surface(MirrorOptions {
        repo_root: root.to_path_buf(),
        lua_policy: None,
        check: true,
    })
    .unwrap_err();
    assert!(error.to_string().contains("stale file"));
}

#[test]
fn mirror_check_rejects_stale_claude_agent_roles() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    fs::create_dir_all(root.join(".claude/agents/core")).unwrap();
    fs::write(root.join(".claude/settings.json"), r#"{"env":{}}"#).unwrap();
    fs::write(
        root.join(".claude/agents/core/coder.md"),
        "---\ndescription: Writes code\n---\n\n# Coder\n",
    )
    .unwrap();

    mirror_codex_surface(MirrorOptions {
        repo_root: root.to_path_buf(),
        lua_policy: None,
        check: false,
    })
    .unwrap();
    fs::write(
        root.join(".codex/agents/claude/claude-stale-agent.toml"),
        "name = \"claude-stale-agent\"\ndescription = \"stale\"\ndeveloper_instructions = \"stale\"\n",
    )
    .unwrap();

    let error = mirror_codex_surface(MirrorOptions {
        repo_root: root.to_path_buf(),
        lua_policy: None,
        check: true,
    })
    .unwrap_err();
    assert!(error.to_string().contains("stale file"));
}

#[test]
fn lua_policy_can_extend_generated_config() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    fs::create_dir_all(root.join(".claude")).unwrap();
    fs::write(root.join(".claude/settings.json"), r#"{"env":{}}"#).unwrap();
    fs::write(
        root.join("policy.lua"),
        "return { config_footer = '[tools]\\npolicy = \"lua\"' }",
    )
    .unwrap();

    mirror_codex_surface(MirrorOptions {
        repo_root: root.to_path_buf(),
        lua_policy: Some(root.join("policy.lua")),
        check: false,
    })
    .unwrap();

    let config = fs::read_to_string(root.join(".codex/config.toml")).unwrap();
    assert!(config.contains("[tools]\npolicy = \"lua\""));
}

#[test]
fn command_skill_generation_strips_source_frontmatter() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    fs::create_dir_all(root.join(".claude/commands")).unwrap();
    fs::write(root.join(".claude/settings.json"), r#"{"env":{}}"#).unwrap();
    fs::write(
        root.join(".claude/commands/demo.md"),
        "---\nname: demo\ndescription: Demo\n---\n\n# Demo\nBody\n",
    )
    .unwrap();

    mirror_codex_surface(MirrorOptions {
        repo_root: root.to_path_buf(),
        lua_policy: None,
        check: false,
    })
    .unwrap();

    let skill =
        fs::read_to_string(root.join(".agents/skills/source-command-demo/SKILL.md")).unwrap();
    assert_eq!(skill.matches("---").count(), 2);
    assert!(skill.contains("# Demo\nBody"));
}
