use std::fs;

use codex_env::{mirror_codex_surface, MirrorOptions};

#[test]
fn mirror_generates_codex_and_skill_files() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    fs::create_dir_all(root.join(".claude/hooks")).unwrap();
    fs::create_dir_all(root.join(".claude/skills/demo")).unwrap();
    fs::create_dir_all(root.join(".claude/commands/sparc")).unwrap();
    fs::write(
        root.join(".claude/settings.json"),
        r#"{"hooks":{"Stop":[{"hooks":[{"type":"command","command":"echo done"}]}]},"env":{"BRAIN_URL":"https://pi.ruv.io"}}"#,
    )
    .unwrap();
    fs::write(
        root.join(".claude/hooks/rust-check.sh"),
        "#!/bin/sh\necho exact   \n",
    )
    .unwrap();
    fs::write(root.join(".claude/skills/demo/SKILL.md"), "# Demo\n").unwrap();
    fs::write(
        root.join(".claude/commands/sparc/code.md"),
        "# Code\nBody\n",
    )
    .unwrap();

    let report = mirror_codex_surface(MirrorOptions {
        repo_root: root.to_path_buf(),
        lua_policy: None,
        check: false,
    })
    .unwrap();

    assert!(report.changed_files > 0);
    assert!(root.join(".codex/config.toml").exists());
    assert!(root.join(".codex/hooks/rust-check.sh").exists());
    assert_eq!(
        fs::read(root.join(".codex/mirror/.claude/hooks/rust-check.sh")).unwrap(),
        fs::read(root.join(".claude/hooks/rust-check.sh")).unwrap()
    );
    assert!(root.join(".agents/skills/demo/SKILL.md").exists());
    assert!(root
        .join(".agents/skills/source-command-sparc-code/SKILL.md")
        .exists());

    let inventory: serde_json::Value =
        serde_json::from_slice(&fs::read(root.join(".codex/mirror-symbols.json")).unwrap())
            .unwrap();
    assert_eq!(inventory["sourceFileCount"], 4);
    assert!(inventory["entries"]
        .as_array()
        .unwrap()
        .iter()
        .any(|entry| {
            entry["source"] == ".claude/commands/sparc/code.md" && entry["kind"] == "command"
        }));

    let check = mirror_codex_surface(MirrorOptions {
        repo_root: root.to_path_buf(),
        lua_policy: None,
        check: true,
    })
    .unwrap();
    assert_eq!(check.changed_files, 0);
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
