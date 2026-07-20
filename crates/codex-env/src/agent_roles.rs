use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::Serialize;
use walkdir::WalkDir;

use super::{
    first_heading, is_ignored_path, normalize_generated_text, slugify, strip_leading_frontmatter,
    strip_repo_prefix, PlannedFile,
};

const AGENT_DESCRIPTION_LIMIT: usize = 240;

#[derive(Debug, Clone)]
pub(super) struct CodexAgentRole {
    pub role_name: String,
    pub description: String,
    pub config_file: PathBuf,
}

#[derive(Debug, Default)]
pub(super) struct CodexAgentRolePlan {
    pub roles: Vec<CodexAgentRole>,
    pub files: Vec<PlannedFile>,
}

#[derive(Debug, Serialize)]
struct AgentRoleFile<'a> {
    name: &'a str,
    description: &'a str,
    model: &'static str,
    model_reasoning_effort: &'static str,
    developer_instructions: &'a str,
}

pub(super) fn claude_agent_role_plan(
    claude_agents_dir: &Path,
    codex_dir: &Path,
) -> Result<CodexAgentRolePlan> {
    if !claude_agents_dir.exists() {
        return Ok(CodexAgentRolePlan::default());
    }

    let mut roles = Vec::new();
    let mut files = Vec::new();
    for entry in WalkDir::new(claude_agents_dir)
        .into_iter()
        .filter_entry(|entry| !is_ignored_path(entry.path()))
    {
        let entry = entry?;
        if !entry.file_type().is_file() {
            continue;
        }

        let relative = entry.path().strip_prefix(claude_agents_dir)?;
        let source = fs::read_to_string(entry.path()).with_context(|| {
            format!(
                "failed to read Claude agent source {}",
                entry.path().display()
            )
        })?;
        let stem = relative.with_extension("");
        let role_name = format!("claude-{}", slugify(&stem.to_string_lossy()));
        let config_file = PathBuf::from("agents/claude").join(format!("{role_name}.toml"));
        let description = agent_description(&source, relative);
        let developer_instructions = agent_developer_instructions(relative, &source);
        let model_profile = agent_model_profile(relative, &role_name, &description);
        let role_file = AgentRoleFile {
            name: &role_name,
            description: &description,
            model: model_profile.model,
            model_reasoning_effort: model_profile.reasoning_effort,
            developer_instructions: &developer_instructions,
        };
        let toml = toml::to_string_pretty(&role_file)?;

        files.push(PlannedFile {
            path: codex_dir.join(&config_file),
            bytes: normalize_generated_text(&toml).into_bytes(),
            executable: false,
        });
        roles.push(CodexAgentRole {
            role_name,
            description,
            config_file,
        });
    }

    roles.sort_by(|a, b| a.role_name.cmp(&b.role_name));
    files.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(CodexAgentRolePlan { roles, files })
}

#[derive(Debug, Clone, Copy)]
struct AgentModelProfile {
    model: &'static str,
    reasoning_effort: &'static str,
}

fn agent_model_profile(relative: &Path, role_name: &str, description: &str) -> AgentModelProfile {
    let haystack =
        format!("{} {} {}", relative.display(), role_name, description).to_ascii_lowercase();

    let heavy_keywords = [
        "architect",
        "architecture",
        "auditor",
        "byzantine",
        "claims",
        "code-review",
        "consensus",
        "coordinator",
        "database",
        "ddd",
        "github",
        "hive-mind",
        "injection",
        "integration",
        "memory-specialist",
        "performance-engineer",
        "pii",
        "planner",
        "queen",
        "reasoning",
        "release",
        "reviewer",
        "security",
        "sparc",
        "swarm",
        "tdd",
        "test-architect",
        "validator",
        "v3",
    ];
    if heavy_keywords
        .iter()
        .any(|keyword| haystack.contains(keyword))
    {
        return AgentModelProfile {
            model: "gpt-5.5",
            reasoning_effort: "high",
        };
    }

    let medium_keywords = [
        "analyzer",
        "backend",
        "browser",
        "coder",
        "data",
        "devops",
        "documentation",
        "flow-nexus",
        "learning",
        "mobile",
        "optimizer",
        "payments",
        "python",
        "researcher",
        "specialist",
        "typescript",
        "workflow",
    ];
    if medium_keywords
        .iter()
        .any(|keyword| haystack.contains(keyword))
    {
        return AgentModelProfile {
            model: "gpt-5.5",
            reasoning_effort: "medium",
        };
    }

    AgentModelProfile {
        model: "gpt-5.4-mini",
        reasoning_effort: "medium",
    }
}

pub(super) fn clean_claude_agent_roles(codex_dir: &Path) -> Result<()> {
    let root = codex_dir.join("agents/claude");
    if root.exists() {
        fs::remove_dir_all(&root)
            .with_context(|| format!("failed to clean generated agent roles {}", root.display()))?;
    }
    Ok(())
}

pub(super) fn stale_claude_agent_role_files(
    repo_root: &Path,
    codex_dir: &Path,
    planned_files: &[PlannedFile],
) -> Result<Vec<PathBuf>> {
    let root = codex_dir.join("agents/claude");
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

fn agent_description(source: &str, relative: &Path) -> String {
    let description = extract_scalar(source, "description")
        .or_else(|| first_heading(strip_leading_frontmatter(source)))
        .unwrap_or_else(|| {
            format!(
                "Claude agent role mirrored from .claude/agents/{}.",
                relative.display()
            )
        });
    compact_description(&description)
}

fn agent_developer_instructions(relative: &Path, source: &str) -> String {
    let body = strip_leading_frontmatter(source).trim_start();
    let mut instructions = format!(
        "You are a Codex-native agent role generated from `.claude/agents/{}`.\n\
Preserve the source agent's responsibilities, constraints, routing hints, and operating style while using Codex tools and project instructions.\n\
Do not claim to be Claude; treat Claude-specific hooks or MCP names in the source as compatibility context unless the repo exposes an equivalent local command.\n\n\
Source: `.claude/agents/{}`\n\n",
        relative.display(),
        relative.display()
    );
    if body.is_empty() {
        instructions.push_str(source.trim_start());
    } else {
        instructions.push_str(body);
    }
    if !instructions.ends_with('\n') {
        instructions.push('\n');
    }
    instructions
}

fn extract_scalar(source: &str, key: &str) -> Option<String> {
    let frontmatter = source
        .strip_prefix("---\n")
        .and_then(|body| body.split_once("\n---"))
        .map_or(source, |(frontmatter, _)| frontmatter);

    for line in frontmatter.lines() {
        let trimmed = line.trim();
        let Some((candidate, value)) = trimmed.split_once(':') else {
            continue;
        };
        if candidate.trim() != key {
            continue;
        }
        let value = value.trim();
        if value.is_empty() || matches!(value, "|" | ">") {
            return None;
        }
        return Some(unquote_scalar(value));
    }
    None
}

fn unquote_scalar(value: &str) -> String {
    value.trim_matches('"').trim_matches('\'').trim().to_owned()
}

fn compact_description(value: &str) -> String {
    let normalized = value.split_whitespace().collect::<Vec<_>>().join(" ");
    if normalized.chars().count() <= AGENT_DESCRIPTION_LIMIT {
        return normalized;
    }

    let mut truncated = normalized
        .chars()
        .take(AGENT_DESCRIPTION_LIMIT.saturating_sub(3))
        .collect::<String>();
    truncated = truncated
        .trim_end_matches(|c: char| c.is_ascii_punctuation() || c.is_whitespace())
        .to_owned();
    truncated.push_str("...");
    truncated
}
