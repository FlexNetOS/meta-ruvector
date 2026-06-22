use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::{Context, Result};
use serde::Serialize;
use serde_json::json;
use sha2::{Digest, Sha256};
use walkdir::WalkDir;

use super::{
    first_heading, is_executable, is_ignored_path, slugify, strip_repo_prefix, PlannedFile,
};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct MirrorSymbolInventory {
    generated_by: &'static str,
    source_root: &'static str,
    raw_mirror_root: &'static str,
    source_file_count: usize,
    kind_counts: BTreeMap<String, usize>,
    entries: Vec<MirrorSymbolEntry>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct MirrorSymbolEntry {
    source: String,
    mirror: String,
    kind: String,
    executable: bool,
    bytes: usize,
    source_sha256: String,
    mirror_sha256: String,
    symbols: BTreeMap<String, serde_json::Value>,
}

pub(super) fn claude_source_files(repo_root: &Path, claude_dir: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    for entry in WalkDir::new(claude_dir)
        .into_iter()
        .filter_entry(|entry| !is_ignored_path(entry.path()))
    {
        let entry = entry?;
        if !entry.file_type().is_file() {
            continue;
        }
        files.push(strip_repo_prefix(repo_root, entry.path()));
    }
    let tracked = git_tracked_paths(repo_root, &files)?;
    let ignored = git_ignored_paths(repo_root, &files)?;
    files.retain(|file| tracked.contains(file) || !ignored.contains(file));
    files.sort();
    Ok(files)
}

fn git_tracked_paths(repo_root: &Path, files: &[PathBuf]) -> Result<BTreeSet<PathBuf>> {
    git_filter_paths(repo_root, files, &["ls-files", "--"])
}

fn git_ignored_paths(repo_root: &Path, files: &[PathBuf]) -> Result<BTreeSet<PathBuf>> {
    git_filter_paths(repo_root, files, &["check-ignore", "--stdin"])
}

fn git_filter_paths(
    repo_root: &Path,
    files: &[PathBuf],
    args: &[&str],
) -> Result<BTreeSet<PathBuf>> {
    let mut command = Command::new("git");
    command
        .args(args)
        .current_dir(repo_root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    if args.first() == Some(&"ls-files") {
        command.args(files);
    }
    let mut child = command.spawn().with_context(|| {
        format!(
            "failed to run git {} in {}",
            args.join(" "),
            repo_root.display()
        )
    })?;

    if args.first() == Some(&"check-ignore") {
        if let Some(mut stdin) = child.stdin.take() {
            for file in files {
                writeln!(stdin, "{}", file.display())?;
            }
        }
    }

    let output = child.wait_with_output()?;
    if output.status.success() || output.status.code() == Some(1) {
        return Ok(String::from_utf8_lossy(&output.stdout)
            .lines()
            .map(PathBuf::from)
            .collect());
    }
    Ok(BTreeSet::new())
}

pub(super) fn raw_claude_mirror_plan(
    repo_root: &Path,
    codex_dir: &Path,
    claude_files: &[PathBuf],
) -> Result<Vec<PlannedFile>> {
    let mut files = Vec::with_capacity(claude_files.len());
    for relative_source in claude_files {
        let source = repo_root.join(relative_source);
        files.push(PlannedFile {
            path: codex_dir.join("mirror").join(relative_source),
            bytes: fs::read(&source).with_context(|| {
                format!("failed to read raw mirror source {}", source.display())
            })?,
            executable: is_executable(&source)?,
        });
    }
    Ok(files)
}

pub(super) fn mirror_symbol_inventory_json(
    repo_root: &Path,
    codex_dir: &Path,
    claude_files: &[PathBuf],
) -> Result<String> {
    let mut kind_counts = BTreeMap::new();
    let mut entries = Vec::with_capacity(claude_files.len());

    for relative_source in claude_files {
        let source = repo_root.join(relative_source);
        let bytes = fs::read(&source)
            .with_context(|| format!("failed to read symbol source {}", source.display()))?;
        let kind = classify_claude_file(relative_source).to_owned();
        *kind_counts.entry(kind.clone()).or_insert(0) += 1;
        entries.push(MirrorSymbolEntry {
            source: relative_source.display().to_string(),
            mirror: strip_repo_prefix(repo_root, &codex_dir.join("mirror").join(relative_source))
                .display()
                .to_string(),
            kind,
            executable: is_executable(&source)?,
            bytes: bytes.len(),
            source_sha256: sha256_hex(&bytes),
            mirror_sha256: sha256_hex(&bytes),
            symbols: extract_symbols(relative_source, &bytes),
        });
    }

    let inventory = MirrorSymbolInventory {
        generated_by: "codex-env",
        source_root: ".claude",
        raw_mirror_root: ".codex/mirror/.claude",
        source_file_count: entries.len(),
        kind_counts,
        entries,
    };
    Ok(format!("{}\n", serde_json::to_string_pretty(&inventory)?))
}

pub(super) fn clean_raw_mirror(codex_dir: &Path) -> Result<()> {
    let raw_root = codex_dir.join("mirror/.claude");
    if raw_root.exists() {
        fs::remove_dir_all(&raw_root)
            .with_context(|| format!("failed to clean raw mirror {}", raw_root.display()))?;
    }
    Ok(())
}

pub(super) fn stale_raw_mirror_files(
    repo_root: &Path,
    codex_dir: &Path,
    claude_files: &[PathBuf],
) -> Result<Vec<PathBuf>> {
    let raw_root = codex_dir.join("mirror/.claude");
    if !raw_root.exists() {
        return Ok(Vec::new());
    }

    let expected: BTreeSet<PathBuf> = claude_files.iter().cloned().collect();
    let mut stale = Vec::new();
    for entry in WalkDir::new(&raw_root) {
        let entry = entry?;
        if !entry.file_type().is_file() {
            continue;
        }
        let relative = strip_repo_prefix(repo_root, entry.path());
        let source_relative = relative
            .strip_prefix(".codex/mirror")
            .unwrap_or(&relative)
            .to_path_buf();
        if !expected.contains(&source_relative) {
            stale.push(relative);
        }
    }
    stale.sort();
    Ok(stale)
}

fn classify_claude_file(relative_source: &Path) -> &'static str {
    let path = relative_source.to_string_lossy();
    if path == ".claude/settings.json" || path.ends_with("/settings.json") {
        "setting"
    } else if path == ".claude/identity.json" || path.ends_with("/identity.json") {
        "identity"
    } else if path.starts_with(".claude/agents/") {
        "agent"
    } else if path.starts_with(".claude/commands/") {
        "command"
    } else if path.starts_with(".claude/hooks/") {
        "hook"
    } else if path.starts_with(".claude/helpers/") {
        "helper"
    } else if path.starts_with(".claude/skills/") {
        "skill"
    } else if path.starts_with(".claude/intelligence/") {
        "intelligence"
    } else {
        "other"
    }
}

fn extract_symbols(relative_source: &Path, bytes: &[u8]) -> BTreeMap<String, serde_json::Value> {
    let mut symbols = BTreeMap::new();
    let path = relative_source.to_string_lossy();
    symbols.insert(
        "pathSlug".to_owned(),
        json!(slugify(path.trim_start_matches(".claude/"))),
    );

    if let Some(file_name) = relative_source.file_name().and_then(|value| value.to_str()) {
        symbols.insert("fileName".to_owned(), json!(file_name));
    }
    if let Some(extension) = relative_source.extension().and_then(|value| value.to_str()) {
        symbols.insert("extension".to_owned(), json!(extension));
    }

    if let Ok(text) = std::str::from_utf8(bytes) {
        if let Some(heading) = first_heading(text) {
            symbols.insert("firstHeading".to_owned(), json!(heading));
        }
        let frontmatter = frontmatter_symbols(text);
        if !frontmatter.is_empty() {
            symbols.insert("frontmatter".to_owned(), json!(frontmatter));
        }
    }

    if let Ok(value) = serde_json::from_slice::<serde_json::Value>(bytes) {
        if let Some(object) = value.as_object() {
            let mut keys: Vec<_> = object.keys().cloned().collect();
            keys.sort();
            symbols.insert("jsonTopLevelKeys".to_owned(), json!(keys));
        }
        if path == ".claude/settings.json" {
            if let Some(hooks) = value.get("hooks").and_then(serde_json::Value::as_object) {
                let mut hook_events: Vec<_> = hooks.keys().cloned().collect();
                hook_events.sort();
                symbols.insert("hookEvents".to_owned(), json!(hook_events));
            }
            if let Some(env) = value.get("env").and_then(serde_json::Value::as_object) {
                let mut env_keys: Vec<_> = env.keys().cloned().collect();
                env_keys.sort();
                symbols.insert("envKeys".to_owned(), json!(env_keys));
            }
        }
    }

    symbols
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut output = String::with_capacity(digest.len() * 2);
    for byte in digest {
        write!(&mut output, "{byte:02x}").expect("writing sha256 to string cannot fail");
    }
    output
}

fn frontmatter_symbols(markdown: &str) -> BTreeMap<String, String> {
    let mut symbols = BTreeMap::new();
    let Some(rest) = markdown.strip_prefix("---\n") else {
        return symbols;
    };
    let Some(end) = rest.find("\n---\n") else {
        return symbols;
    };
    for line in rest[..end].lines() {
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let key = key.trim();
        if matches!(
            key,
            "name" | "description" | "allowed-tools" | "argument-hint"
        ) {
            symbols.insert(
                key.to_owned(),
                value.trim().trim_matches('"').trim_matches('\'').to_owned(),
            );
        }
    }
    symbols
}
