use std::fs;
use std::process::Command;

use codex_env::{
    doctor_codex_surface, install_codex_prompts, mirror_codex_surface, DoctorOptions,
    MirrorOptions, PromptInstallOptions,
};

#[test]
fn mirror_generates_codex_and_skill_files() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    fs::create_dir_all(root.join(".claude/hooks")).unwrap();
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
        root.join(".claude/agents/core/coder.md"),
        "---\nname: coder\ndescription: Writes code\npriority: high\n---\n\n# Coder\nImplement carefully.\n",
    )
    .unwrap();
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
    assert!(config.contains("model = \"gpt-5.5\""));
    assert!(config.contains("model_reasoning_effort = \"high\""));
    assert!(config.contains("model_context_window = 4000000"));
    assert!(config.contains("[skills]\ninclude_instructions = true"));
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
    assert!(prompt.contains("Source: `.claude/commands/sparc/code.md`"));
    assert!(prompt.contains("Arguments supplied to this prompt: $ARGUMENTS"));
    assert!(prompt.contains("Body with $ARGUMENTS and shell \"$$FOO\"."));
    assert!(root.join(".codex/prompts/codex-agent-team.md").exists());
    assert!(root.join(".codex/prompts/codex-auto-loop.md").exists());
    assert!(root.join(".codex/prompts/codex-gap-hunt.md").exists());
    assert!(root.join(".codex/helpers/install-prompts.sh").exists());
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
    assert_eq!(inventory["sourceFileCount"], 7);
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
    let doctor = doctor_codex_surface(DoctorOptions {
        repo_root: root.to_path_buf(),
        lua_policy: None,
        codex_home,
    })
    .unwrap();
    assert_eq!(doctor.config_model, "gpt-5.5");
    assert_eq!(doctor.config_reasoning_effort, "high");
    assert_eq!(doctor.prompt_files, 4);
    assert_eq!(doctor.installed_prompt_files, 4);
    assert!(doctor.agent_models.contains_key("gpt-5.5"));
    assert!(doctor.agent_models.contains_key("gpt-5.4-mini"));
    assert!(doctor.hook_events.contains(&"Stop".to_owned()));
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
    assert!(codex_home.join("prompts/demo.md").exists());
    assert!(codex_home.join("prompts/codex-auto-loop.md").exists());

    let check = install_codex_prompts(PromptInstallOptions {
        repo_root: root,
        codex_home,
        check: true,
    })
    .unwrap();
    assert_eq!(check.changed_files, 0);
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

    let error = doctor_codex_surface(DoctorOptions {
        repo_root: root.to_path_buf(),
        lua_policy: None,
        codex_home,
    })
    .unwrap_err();
    assert!(error.to_string().contains("gitignored file"));
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
