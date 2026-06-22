use std::collections::BTreeSet;
use std::fs;
use std::path::{Component, Path, PathBuf};

use anyhow::Result;
use walkdir::WalkDir;

use super::{
    first_heading, is_ignored_path, normalize_generated_text, slugify, strip_leading_frontmatter,
    strip_repo_prefix, yaml_scalar, PlannedFile,
};

#[derive(Debug, Default)]
pub(super) struct CodexPromptPlan {
    pub files: Vec<PlannedFile>,
}

pub(super) fn command_prompt_plan(
    commands_dir: &Path,
    codex_dir: &Path,
) -> Result<CodexPromptPlan> {
    if !commands_dir.exists() {
        return Ok(CodexPromptPlan::default());
    }

    let mut files = Vec::new();
    let mut planned_paths = BTreeSet::new();
    for entry in WalkDir::new(commands_dir)
        .into_iter()
        .filter_entry(|entry| !is_ignored_path(entry.path()))
    {
        let entry = entry?;
        if !entry.file_type().is_file()
            || entry.path().extension().and_then(|s| s.to_str()) != Some("md")
        {
            continue;
        }

        let relative = entry.path().strip_prefix(commands_dir)?;
        let stem = relative.with_extension("");
        let command_name = claude_command_name(&stem);
        let prompt_name = slugify(&stem.to_string_lossy());
        let source = fs::read_to_string(entry.path())?;
        let body = strip_leading_frontmatter(&source).trim_start();
        let description = super::yaml_frontmatter_scalar(&source, "description")
            .or_else(|| first_heading(&source))
            .unwrap_or_else(|| {
                format!(
                    "Claude command prompt mirrored from .claude/commands/{}.",
                    relative.display()
                )
            });

        let mut prompt = String::new();
        prompt.push_str("---\n");
        prompt.push_str(&format!("description: {}\n", yaml_scalar(&description)));
        prompt.push_str("argument-hint: [ARGUMENTS]\n");
        prompt.push_str("---\n\n");
        prompt.push_str(&format!(
            "You are executing the Codex-native prompt mirror for Claude Code command `/{}`.\n\n",
            command_name
        ));
        prompt.push_str("Use Codex-native tools, skills, subagents, MCP servers, and project `AGENTS.md` instructions. Treat Claude-specific tool names, hooks, or MCP names as compatibility context unless this repository exposes the same local command.\n\n");
        prompt.push_str(&format!(
            "Source: `.claude/commands/{}`\n\n",
            relative.display()
        ));
        prompt.push_str("Arguments supplied to this prompt: $ARGUMENTS\n\n");
        prompt.push_str(&escape_codex_prompt_dollars(body));
        if !prompt.ends_with('\n') {
            prompt.push('\n');
        }

        let prompt_bytes = normalize_generated_text(&prompt).into_bytes();
        push_prompt_file(
            &mut files,
            &mut planned_paths,
            codex_dir,
            &prompt_name,
            &prompt_bytes,
        );
        if let Some(alias_name) = claude_namespace_alias(&stem) {
            push_prompt_file(
                &mut files,
                &mut planned_paths,
                codex_dir,
                &alias_name,
                &prompt_bytes,
            );
        }
    }

    files.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(CodexPromptPlan { files })
}

fn push_prompt_file(
    files: &mut Vec<PlannedFile>,
    planned_paths: &mut BTreeSet<PathBuf>,
    codex_dir: &Path,
    prompt_name: &str,
    prompt_bytes: &[u8],
) {
    let path = codex_dir.join("prompts").join(format!("{prompt_name}.md"));
    if !planned_paths.insert(path.clone()) {
        return;
    }
    files.push(PlannedFile {
        path,
        bytes: prompt_bytes.to_vec(),
        executable: false,
    });
}

fn claude_command_name(stem: &Path) -> String {
    prompt_path_segments(stem).join(":")
}

fn claude_namespace_alias(stem: &Path) -> Option<String> {
    let segments = prompt_path_segments(stem);
    if segments.len() < 2 {
        return None;
    }
    Some(segments.join(":"))
}

fn prompt_path_segments(stem: &Path) -> Vec<String> {
    stem.components()
        .filter_map(|component| match component {
            Component::Normal(value) => {
                let segment = slugify(&value.to_string_lossy());
                (!segment.is_empty()).then_some(segment)
            }
            _ => None,
        })
        .collect()
}

pub(super) fn clean_codex_prompts(codex_dir: &Path) -> Result<()> {
    let root = codex_dir.join("prompts");
    if root.exists() {
        fs::remove_dir_all(&root)?;
    }
    Ok(())
}

pub(super) fn stale_codex_prompt_files(
    repo_root: &Path,
    codex_dir: &Path,
    planned_files: &[PlannedFile],
) -> Result<Vec<PathBuf>> {
    let root = codex_dir.join("prompts");
    if !root.exists() {
        return Ok(Vec::new());
    }

    let expected: BTreeSet<PathBuf> = planned_files
        .iter()
        .map(|file| strip_repo_prefix(repo_root, &file.path))
        .collect();
    let mut stale = Vec::new();
    for entry in WalkDir::new(root) {
        let entry = entry?;
        if !entry.file_type().is_file() {
            continue;
        }
        let relative = strip_repo_prefix(repo_root, entry.path());
        if !expected.contains(&relative) {
            stale.push(relative);
        }
    }
    stale.sort();
    Ok(stale)
}

fn escape_codex_prompt_dollars(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    let bytes = value.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] != b'$' {
            output.push(bytes[index] as char);
            index += 1;
            continue;
        }

        let rest = &value[index..];
        if rest.starts_with("$ARGUMENTS") {
            output.push_str("$ARGUMENTS");
            index += "$ARGUMENTS".len();
        } else {
            output.push_str("$$");
            index += 1;
        }
    }
    output
}
