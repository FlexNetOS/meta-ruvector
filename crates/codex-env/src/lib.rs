use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use mlua::{Lua, Value};
use serde::Serialize;
use serde_json::json;

mod generated;
mod raw_mirror;

use generated::{
    codex_agent_profiles, codex_agents_md, codex_config, codex_hooks_json, command_skill_plan,
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

    if !options.check {
        clean_raw_mirror(&codex_dir)?;
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
