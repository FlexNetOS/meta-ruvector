use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use mlua::{Lua, Value};
use serde::Serialize;
use serde_json::json;
use walkdir::WalkDir;

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
    let mut planned = Vec::new();

    planned.push(PlannedFile {
        path: codex_dir.join("config.toml"),
        bytes: codex_config(
            &read_claude_env(&claude_dir)?,
            policy.config_footer.as_deref(),
        )
        .into_bytes(),
        executable: false,
    });
    planned.push(PlannedFile {
        path: codex_dir.join("AGENTS.md"),
        bytes: codex_agents_md().into_bytes(),
        executable: false,
    });
    planned.extend(codex_agent_profiles(&codex_dir));
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

    let manifest = manifest_json(&repo_root, &claude_dir, &planned)?;
    planned.push(PlannedFile {
        path: codex_dir.join("mirror-manifest.json"),
        bytes: manifest.into_bytes(),
        executable: false,
    });

    let mut changed_files = 0;
    let mut verified_files = 0;
    let mut generated = Vec::new();

    for file in &planned {
        let exists_with_same_content =
            fs::read(&file.path).map_or(false, |bytes| bytes == file.bytes);
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

fn read_claude_env(claude_dir: &Path) -> Result<BTreeMap<String, String>> {
    let settings_path = claude_dir.join("settings.json");
    let settings: serde_json::Value = serde_json::from_slice(
        &fs::read(&settings_path)
            .with_context(|| format!("failed to read {}", settings_path.display()))?,
    )
    .with_context(|| format!("failed to parse {}", settings_path.display()))?;

    let mut env = BTreeMap::new();
    if let Some(object) = settings.get("env").and_then(serde_json::Value::as_object) {
        for (key, value) in object {
            if let Some(value) = value.as_str() {
                env.insert(key.clone(), value.to_owned());
            }
        }
    }
    Ok(env)
}

fn codex_config(env: &BTreeMap<String, String>, footer: Option<&str>) -> String {
    let mut toml = String::from(
        r#"#:schema https://developers.openai.com/codex/config-schema.json

# Generated by `cargo run -p codex-env -- mirror`.
approval_policy = "on-request"
sandbox_mode = "workspace-write"
web_search = "live"

[mcp_servers.github]
command = "npx"
args = ["-y", "@modelcontextprotocol/server-github"]

[mcp_servers.context7]
command = "npx"
args = ["-y", "@upstash/context7-mcp@latest"]

[mcp_servers.exa]
url = "https://mcp.exa.ai/mcp"

[mcp_servers.memory]
command = "npx"
args = ["-y", "@modelcontextprotocol/server-memory"]

[mcp_servers.playwright]
command = "npx"
args = ["-y", "@playwright/mcp@latest", "--extension"]

[mcp_servers.sequential-thinking]
command = "npx"
args = ["-y", "@modelcontextprotocol/server-sequential-thinking"]

[mcp_servers.claude-flow]
command = "npx"
args = ["@claude-flow/cli@latest", "mcp", "start"]

[mcp_servers.claude-flow.env]
CLAUDE_FLOW_MODE = "v3"
CLAUDE_FLOW_HOOKS_ENABLED = "true"
CLAUDE_FLOW_TOPOLOGY = "hierarchical-mesh"
CLAUDE_FLOW_MAX_AGENTS = "15"
CLAUDE_FLOW_MEMORY_BACKEND = "hybrid"

[features]
multi_agent = true

[agents]
max_threads = 6
max_depth = 1

[agents.explorer]
description = "Read-only codebase explorer for gathering evidence before changes are proposed."
config_file = "agents/explorer.toml"

[agents.reviewer]
description = "PR reviewer focused on correctness, security, and missing tests."
config_file = "agents/reviewer.toml"

[agents.docs_researcher]
description = "Documentation specialist that verifies APIs, framework behavior, and release notes."
config_file = "agents/docs-researcher.toml"

[shell_environment_policy]
inherit = "core"

[shell_environment_policy.set]
"#,
    );

    for (key, value) in env {
        toml.push_str(&format!("{key} = \"{}\"\n", escape_toml_string(value)));
    }

    if let Some(footer) = footer {
        toml.push('\n');
        toml.push_str(footer.trim());
        toml.push('\n');
    }

    toml
}

fn codex_agents_md() -> String {
    String::from(
        r#"# Codex Mirror Surface

This directory is generated from the tracked `.claude` surface by the Rust
`codex-env` harness.

## Refresh

```bash
cargo run -p codex-env -- mirror
cargo run -p codex-env -- mirror --check
```

## Mirrored Surfaces

- `.claude/settings.json` -> `.codex/hooks.json` and shell environment defaults
- `.claude/hooks/` -> `.codex/hooks/`
- `.claude/skills/` -> `.agents/skills/`
- `.claude/commands/**/*.md` -> `.agents/skills/source-command-*`

Use `--lua-policy <path>` when a repo-local transformation is needed. The Lua
script receives a `mirror` table with `repo_root` and `claude_dir`, and may
return `{ config_footer = "...", skill_prelude = "..." }`.
"#,
    )
}

fn codex_agent_profiles(codex_dir: &Path) -> Vec<PlannedFile> {
    [
        (
            "explorer.toml",
            "gpt-5.4",
            "medium",
            "Stay in exploration mode.\nTrace the real execution path, cite files and symbols, and avoid proposing fixes unless the parent agent asks for them.\nPrefer targeted search and file reads over broad scans.\n",
        ),
        (
            "reviewer.toml",
            "gpt-5.4",
            "high",
            "Review like an owner.\nPrioritize correctness, security, behavioral regressions, and missing tests.\nLead with concrete findings and avoid style-only feedback unless it hides a real bug.\n",
        ),
        (
            "docs-researcher.toml",
            "gpt-5.4",
            "medium",
            "Verify APIs, framework behavior, and release-note claims against primary documentation before changes land.\nCite the exact docs or file paths that support each claim.\nDo not invent undocumented behavior.\n",
        ),
    ]
    .into_iter()
    .map(|(file, model, effort, instructions)| PlannedFile {
        path: codex_dir.join("agents").join(file),
        bytes: format!(
            "model = \"{model}\"\nmodel_reasoning_effort = \"{effort}\"\nsandbox_mode = \"read-only\"\n\ndeveloper_instructions = \"\"\"\n{instructions}\"\"\""
        )
        .into_bytes(),
        executable: false,
    })
    .collect()
}

fn codex_hooks_json(claude_dir: &Path) -> Result<String> {
    let settings_path = claude_dir.join("settings.json");
    let settings: serde_json::Value = serde_json::from_slice(
        &fs::read(&settings_path)
            .with_context(|| format!("failed to read {}", settings_path.display()))?,
    )?;

    let hooks = settings.get("hooks").cloned().unwrap_or_else(|| json!({}));
    let status_line = settings.get("statusLine").cloned();
    let permissions = settings.get("permissions").cloned();
    let env = settings.get("env").cloned();
    let output = json!({
        "generatedBy": "codex-env",
        "source": ".claude/settings.json",
        "hooks": hooks,
        "statusLine": status_line,
        "permissions": permissions,
        "env": env
    });
    Ok(format!("{}\n", serde_json::to_string_pretty(&output)?))
}

fn copy_tree_plan(source: &Path, target: &Path) -> Result<Vec<PlannedFile>> {
    if !source.exists() {
        return Ok(Vec::new());
    }

    let mut files = Vec::new();
    for entry in WalkDir::new(source)
        .into_iter()
        .filter_entry(|entry| !is_ignored_path(entry.path()))
    {
        let entry = entry?;
        if !entry.file_type().is_file() {
            continue;
        }
        let relative = entry.path().strip_prefix(source)?;
        let bytes = normalize_generated_bytes(entry.path(), fs::read(entry.path())?);
        files.push(PlannedFile {
            path: target.join(relative),
            bytes,
            executable: is_executable(entry.path())?,
        });
    }
    files.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(files)
}

fn command_skill_plan(
    commands_dir: &Path,
    skills_dir: &Path,
    prelude: Option<&str>,
) -> Result<Vec<PlannedFile>> {
    if !commands_dir.exists() {
        return Ok(Vec::new());
    }

    let mut files = Vec::new();
    for entry in WalkDir::new(commands_dir) {
        let entry = entry?;
        if !entry.file_type().is_file()
            || entry.path().extension().and_then(|s| s.to_str()) != Some("md")
        {
            continue;
        }

        let relative = entry.path().strip_prefix(commands_dir)?;
        let stem = relative.with_extension("");
        let name = format!("source-command-{}", slugify(&stem.to_string_lossy()));
        let source = fs::read_to_string(entry.path())?;
        let source_body = strip_leading_frontmatter(&source);
        let description = first_heading(&source)
            .unwrap_or_else(|| format!("Workflow command scaffold for {}.", stem.display()));
        let mut body = String::new();
        body.push_str("---\n");
        body.push_str(&format!("name: {name}\n"));
        body.push_str(&format!("description: {}\n", yaml_scalar(&description)));
        body.push_str("---\n\n");
        if let Some(prelude) = prelude {
            body.push_str(prelude.trim());
            body.push_str("\n\n");
        }
        body.push_str(&format!(
            "# /{}\n\nSource: `.claude/commands/{}`\n\n",
            stem.display(),
            relative.display()
        ));
        body.push_str(source_body.trim_start());
        if !body.ends_with('\n') {
            body.push('\n');
        }
        files.push(PlannedFile {
            path: skills_dir.join(name).join("SKILL.md"),
            bytes: normalize_generated_text(&body).into_bytes(),
            executable: false,
        });
    }
    files.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(files)
}

fn manifest_json(repo_root: &Path, claude_dir: &Path, planned: &[PlannedFile]) -> Result<String> {
    let files: Vec<_> = planned
        .iter()
        .map(|file| {
            strip_repo_prefix(repo_root, &file.path)
                .display()
                .to_string()
        })
        .collect();
    let manifest = json!({
        "generatedBy": "codex-env",
        "source": strip_repo_prefix(repo_root, claude_dir),
        "fileCount": files.len(),
        "files": files
    });
    Ok(format!("{}\n", serde_json::to_string_pretty(&manifest)?))
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

fn yaml_scalar(value: &str) -> String {
    format!("{:?}", value)
}

#[cfg(test)]
mod tests {
    use super::*;

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
        fs::write(root.join(".claude/hooks/rust-check.sh"), "#!/bin/sh\n").unwrap();
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
        assert!(root.join(".agents/skills/demo/SKILL.md").exists());
        assert!(root
            .join(".agents/skills/source-command-sparc-code/SKILL.md")
            .exists());

        let check = mirror_codex_surface(MirrorOptions {
            repo_root: root.to_path_buf(),
            lua_policy: None,
            check: true,
        })
        .unwrap();
        assert_eq!(check.changed_files, 0);
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
}
