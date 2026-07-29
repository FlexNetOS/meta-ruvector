//! TEASTASK-005 — the git-kb `ready` → handoff `claim` selection wire (DOMAIN_MODEL seam S2).
//!
//! git-kb ranks the ready backlog (all dependencies satisfied) and emits the top
//! candidate as JSON; handoff's `hf claim <id>` reserves an **atomic witnessed lease**
//! on that task so exactly one agent owns the next unit of work. This module joins the
//! two: read git-kb's deterministic selection, then issue the lease claim — treating a
//! refused claim (the lease is already held by another peer) as a first-class conflict,
//! never as a silent success.
//!
//! ## Wired commands (verified against `/home/flexnetos/lifeos/src/handoff`)
//! - **ready:** `git-kb ready --json --limit 1` — the JSON shape is
//!   `{"tasks":[{"slug":"tasks/foo","doc_type":"task","priority":"high","score":75.7,
//!   "reasons":{…}}]}`. Unknown fields (`reasons`, …) are ignored.
//! - **claim:** `hf claim <slug>` — the id is a **positional** argument
//!   (`hf/src/main.rs`: `HF_CLAIM_HELP = "usage: hf claim ID|--next|--batch"`, and
//!   `cmd_claim` at `main.rs:407` `std::process::exit(1)`s when the claim is
//!   refused/blocked). The lease holder identity is `HF_LEASE_HOLDER` (else hostname),
//!   per `handoff-lease/src/lib.rs::local_holder`; we set it on the child explicitly.
//!
//! Exit 0 from `hf claim` ⇒ the lease is ours ([`ClaimedTask`]). Any non-zero exit ⇒
//! [`SelectionError::ClaimConflict`] — the lease belongs to someone else and the task is
//! NOT ours to run.
//!
//! Subprocess execution is behind the [`CommandRunner`] trait so the wire is testable
//! without the real `git-kb`/`hf` binaries.

use serde::Deserialize;

/// A single ready-task candidate from `git-kb ready --json`.
///
/// Only the fields the selection wire consumes are named; git-kb also emits `reasons`
/// (and may add more) — those are ignored (no `deny_unknown_fields`), so a richer
/// git-kb release never breaks the parse (upgrade-only).
#[derive(Debug, Clone, Deserialize)]
pub struct ReadyTask {
    /// git-kb document slug, e.g. `tasks/foo`. This is the id handed to `hf claim`.
    pub slug: String,
    /// Document type (e.g. `task`, `spec`) when git-kb supplies it.
    #[serde(default)]
    pub doc_type: Option<String>,
    /// Declared priority (e.g. `high`) when git-kb supplies it.
    #[serde(default)]
    pub priority: Option<String>,
    /// git-kb's readiness/value score. Higher wins.
    #[serde(default)]
    pub score: f64,
}

/// Wrapper matching the top-level `{"tasks":[…]}` object of `git-kb ready --json`.
#[derive(Debug, Clone, Deserialize)]
struct ReadyResponse {
    #[serde(default)]
    tasks: Vec<ReadyTask>,
}

/// Captured result of a subprocess invocation.
#[derive(Debug, Clone)]
pub struct CommandOutput {
    /// Decoded stdout (lossy UTF-8).
    pub stdout: String,
    /// Decoded stderr (lossy UTF-8).
    pub stderr: String,
    /// Process exit code, or `None` if the process was terminated by a signal.
    pub exit_code: Option<i32>,
}

/// Injectable subprocess boundary so the selection wire is testable without the real
/// `git-kb`/`hf` binaries. `env` entries are set on the child (upgrade, not replace, of
/// the inherited environment).
pub trait CommandRunner {
    /// Run `program` with `args`, setting each `(key, value)` in `env` on the child, and
    /// capture its stdout/stderr/exit code.
    ///
    /// # Errors
    /// Returns the underlying [`std::io::Error`] if the process could not be spawned.
    fn run(
        &self,
        program: &str,
        args: &[&str],
        env: &[(&str, &str)],
    ) -> std::io::Result<CommandOutput>;
}

/// Production [`CommandRunner`] over [`std::process::Command`].
#[derive(Debug, Clone, Default)]
pub struct SystemRunner;

impl CommandRunner for SystemRunner {
    fn run(
        &self,
        program: &str,
        args: &[&str],
        env: &[(&str, &str)],
    ) -> std::io::Result<CommandOutput> {
        let mut command = std::process::Command::new(program);
        command.args(args);
        for (key, value) in env {
            command.env(key, value);
        }
        let output = command.output()?;
        Ok(CommandOutput {
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            exit_code: output.status.code(),
        })
    }
}

/// A task whose handoff lease this engine now holds — the successful outcome of the
/// git-kb→handoff selection wire.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaimedTask {
    /// The git-kb slug we claimed (the `hf claim` positional id).
    pub slug: String,
    /// The lease holder identity recorded for the claim.
    pub holder: String,
}

/// Errors from the selection→claim wire.
#[derive(Debug, thiserror::Error)]
pub enum SelectionError {
    /// A subprocess could not be spawned.
    #[error("selection I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// `git-kb ready --json` output could not be parsed.
    #[error("failed to parse git-kb ready JSON: {0}")]
    Json(#[from] serde_json::Error),

    /// git-kb reported no ready task to claim.
    #[error("git-kb ready returned no ready task")]
    NoReadyTask,

    /// `git-kb ready` itself exited non-zero — selection could not even be read.
    #[error("git-kb ready command failed (exit {code:?}): {stderr}")]
    ReadyCommandFailed {
        /// The `git-kb ready` exit code (`None` if signal-terminated).
        code: Option<i32>,
        /// The captured stderr, for diagnosis.
        stderr: String,
    },

    /// `hf claim` refused the lease — it is held by another peer; the task is NOT ours.
    #[error("hf claim refused for {slug} (exit {code:?}): {stderr}")]
    ClaimConflict {
        /// The slug we tried and failed to claim.
        slug: String,
        /// The `hf claim` exit code (`None` if signal-terminated).
        code: Option<i32>,
        /// The captured stderr explaining the refusal.
        stderr: String,
    },
}

/// Parse `git-kb ready --json` output into its ready-task list.
///
/// # Errors
/// Returns [`SelectionError::Json`] if the payload is not the expected `{"tasks":[…]}`
/// object.
pub fn parse_ready(json: &str) -> Result<Vec<ReadyTask>, SelectionError> {
    let response: ReadyResponse = serde_json::from_str(json)?;
    Ok(response.tasks)
}

/// Pick the top ready task defensively: the maximum `score`, with a deterministic
/// tie-break to the lexicographically smallest `slug` (git-kb pre-sorts, but we never
/// rely on input order). Returns `None` for an empty slice.
#[must_use]
pub fn top_ready(tasks: &[ReadyTask]) -> Option<&ReadyTask> {
    // `max_by` yields the greatest element under this ordering; on equal scores we make
    // the SMALLER slug compare greater (reversed slug order) so ties resolve
    // deterministically to the smallest slug regardless of input order.
    tasks.iter().max_by(|a, b| {
        a.score
            .total_cmp(&b.score)
            .then_with(|| b.slug.cmp(&a.slug))
    })
}

/// The git-kb `ready` → handoff `claim` selection wire.
///
/// Reads git-kb's deterministic top ready task and issues an atomic `hf claim` lease so
/// exactly one agent owns it. Generic over [`CommandRunner`] so tests drive it without
/// the real binaries.
#[derive(Debug, Clone)]
pub struct Selector<R: CommandRunner> {
    runner: R,
    gitkb_bin: String,
    hf_bin: String,
    holder: String,
}

impl<R: CommandRunner> Selector<R> {
    /// Construct a selector over `runner`, defaulting the binary names to `git-kb` and
    /// `hf` and the lease holder to `HF_LEASE_HOLDER` (else the hostname, else the stable
    /// fallback `rvagent-engine`).
    #[must_use]
    pub fn new(runner: R) -> Self {
        Self {
            runner,
            gitkb_bin: "git-kb".to_string(),
            hf_bin: "hf".to_string(),
            holder: default_holder(),
        }
    }

    /// Override the git-kb binary name/path.
    #[must_use]
    pub fn with_gitkb_bin(mut self, bin: impl Into<String>) -> Self {
        self.gitkb_bin = bin.into();
        self
    }

    /// Override the hf binary name/path.
    #[must_use]
    pub fn with_hf_bin(mut self, bin: impl Into<String>) -> Self {
        self.hf_bin = bin.into();
        self
    }

    /// Override the lease holder identity recorded for the claim.
    #[must_use]
    pub fn with_holder(mut self, holder: impl Into<String>) -> Self {
        self.holder = holder.into();
        self
    }

    /// The lease holder identity this selector claims under.
    #[must_use]
    pub fn holder(&self) -> &str {
        &self.holder
    }

    /// Read git-kb's top ready task and atomically claim its handoff lease.
    ///
    /// 1. `git-kb ready --json --limit 1` → parse → [`top_ready`]. No candidate ⇒
    ///    [`SelectionError::NoReadyTask`]; a non-zero git-kb exit ⇒
    ///    [`SelectionError::ReadyCommandFailed`].
    /// 2. `hf claim <slug>` with `HF_LEASE_HOLDER=<holder>`. Exit 0 ⇒ [`ClaimedTask`];
    ///    any non-zero exit ⇒ [`SelectionError::ClaimConflict`] — the lease is held by
    ///    another peer, so the task is NOT ours (never reported as a success).
    ///
    /// # Errors
    /// See the variants above.
    pub fn select_and_claim(&self) -> Result<ClaimedTask, SelectionError> {
        // 1. Deterministic selection from git-kb.
        let ready = self
            .runner
            .run(&self.gitkb_bin, &["ready", "--json", "--limit", "1"], &[])?;
        if ready.exit_code != Some(0) {
            return Err(SelectionError::ReadyCommandFailed {
                code: ready.exit_code,
                stderr: ready.stderr,
            });
        }
        let tasks = parse_ready(&ready.stdout)?;
        let slug = top_ready(&tasks)
            .ok_or(SelectionError::NoReadyTask)?
            .slug
            .clone();

        // 2. Atomic handoff lease claim. `hf claim <id>` takes the id positionally and
        //    exits non-zero when the lease is already held (see module docs).
        let claim = self.runner.run(
            &self.hf_bin,
            &["claim", &slug],
            &[("HF_LEASE_HOLDER", &self.holder)],
        )?;
        if claim.exit_code == Some(0) {
            Ok(ClaimedTask {
                slug,
                holder: self.holder.clone(),
            })
        } else {
            // Non-zero ⇒ the lease belongs to another peer. This is a conflict, NOT an
            // acquired lease — surface it so the caller never runs a task it doesn't own.
            Err(SelectionError::ClaimConflict {
                slug,
                code: claim.exit_code,
                stderr: claim.stderr,
            })
        }
    }
}

impl Selector<SystemRunner> {
    /// Convenience constructor over the production [`SystemRunner`].
    #[must_use]
    pub fn system() -> Self {
        Self::new(SystemRunner)
    }
}

/// Resolve the default lease holder: `HF_LEASE_HOLDER`, else the hostname
/// (`HOSTNAME` env or `/etc/hostname`), else the stable fallback `rvagent-engine`.
/// Mirrors `handoff-lease::local_holder` without pulling a hostname dependency.
fn default_holder() -> String {
    if let Ok(holder) = std::env::var("HF_LEASE_HOLDER") {
        if !holder.trim().is_empty() {
            return holder;
        }
    }
    if let Ok(host) = std::env::var("HOSTNAME") {
        if !host.trim().is_empty() {
            return host.trim().to_string();
        }
    }
    if let Ok(host) = std::fs::read_to_string("/etc/hostname") {
        if !host.trim().is_empty() {
            return host.trim().to_string();
        }
    }
    "rvagent-engine".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::collections::HashMap;

    /// A recorded invocation: (program, args, env pairs).
    type RecordedCall = (String, Vec<String>, Vec<(String, String)>);

    /// A scripted [`CommandRunner`] for the tests: returns a per-program canned
    /// [`CommandOutput`] and records every invocation (program + args + env) so the wire
    /// can be asserted without the real binaries.
    struct FakeRunner {
        scripted: HashMap<String, CommandOutput>,
        calls: RefCell<Vec<RecordedCall>>,
    }

    impl FakeRunner {
        fn new() -> Self {
            Self {
                scripted: HashMap::new(),
                calls: RefCell::new(Vec::new()),
            }
        }

        fn script(mut self, program: &str, output: CommandOutput) -> Self {
            self.scripted.insert(program.to_string(), output);
            self
        }
    }

    impl CommandRunner for FakeRunner {
        fn run(
            &self,
            program: &str,
            args: &[&str],
            env: &[(&str, &str)],
        ) -> std::io::Result<CommandOutput> {
            self.calls.borrow_mut().push((
                program.to_string(),
                args.iter().map(|s| (*s).to_string()).collect(),
                env.iter()
                    .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
                    .collect(),
            ));
            self.scripted.get(program).cloned().ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!("no scripted output for {program}"),
                )
            })
        }
    }

    fn out(stdout: &str, stderr: &str, code: Option<i32>) -> CommandOutput {
        CommandOutput {
            stdout: stdout.to_string(),
            stderr: stderr.to_string(),
            exit_code: code,
        }
    }

    /// A 3-task fixture matching real `git-kb ready --json` output (unknown `reasons`
    /// field present, and NOT in score order — so `top_ready` must sort defensively).
    const READY_FIXTURE: &str = r#"{
        "tasks": [
            {"slug": "tasks/beta", "doc_type": "task", "priority": "medium", "score": 42.0, "reasons": {"deps": "clear"}},
            {"slug": "tasks/alpha", "doc_type": "task", "priority": "high", "score": 75.7, "reasons": {"deps": "clear", "age": 3}},
            {"slug": "tasks/gamma", "doc_type": "spec", "priority": "low", "score": 10.5, "reasons": {}}
        ]
    }"#;

    #[test]
    fn parse_ready_reads_slugs_and_top_picks_max_score() {
        let tasks = parse_ready(READY_FIXTURE).expect("fixture parses");
        assert_eq!(tasks.len(), 3);
        assert_eq!(tasks[0].slug, "tasks/beta");
        assert_eq!(tasks[1].doc_type.as_deref(), Some("task"));
        assert_eq!(tasks[1].priority.as_deref(), Some("high"));

        let top = top_ready(&tasks).expect("non-empty → some");
        assert_eq!(top.slug, "tasks/alpha", "max score (75.7) must win");
    }

    #[test]
    fn top_ready_ties_break_to_smallest_slug_deterministically() {
        let tasks = parse_ready(
            r#"{"tasks":[
                {"slug":"tasks/zed","score":50.0},
                {"slug":"tasks/aaa","score":50.0}
            ]}"#,
        )
        .expect("parses");
        // Equal scores → deterministic tie-break to the lexicographically smallest slug.
        assert_eq!(top_ready(&tasks).expect("some").slug, "tasks/aaa");
    }

    #[test]
    fn parse_ready_empty_list_is_empty_and_top_is_none() {
        let tasks = parse_ready(r#"{"tasks":[]}"#).expect("empty parses");
        assert!(tasks.is_empty());
        assert!(top_ready(&tasks).is_none());
    }

    #[test]
    fn parse_ready_malformed_json_is_json_error() {
        let err = parse_ready("{ this is not json").expect_err("must fail");
        assert!(matches!(err, SelectionError::Json(_)), "got {err:?}");
    }

    #[test]
    fn select_and_claim_over_empty_ready_is_no_ready_task() {
        let runner = FakeRunner::new().script("git-kb", out(r#"{"tasks":[]}"#, "", Some(0)));
        let selector = Selector::new(runner).with_holder("test-holder");
        let err = selector.select_and_claim().expect_err("no task → err");
        assert!(matches!(err, SelectionError::NoReadyTask), "got {err:?}");
    }

    #[test]
    fn select_and_claim_success_claims_top_slug_with_holder() {
        let runner = FakeRunner::new()
            .script("git-kb", out(READY_FIXTURE, "", Some(0)))
            .script("hf", out("hf claim: acquired lease", "", Some(0)));
        let selector = Selector::new(runner).with_holder("agent-7");

        let claimed = selector.select_and_claim().expect("claim succeeds");
        assert_eq!(claimed.slug, "tasks/alpha", "claims the top-scored task");
        assert_eq!(claimed.holder, "agent-7");
    }

    #[test]
    fn select_and_claim_wires_positional_slug_and_lease_holder_env() {
        let runner = FakeRunner::new()
            .script("git-kb", out(READY_FIXTURE, "", Some(0)))
            .script("hf", out("", "", Some(0)));
        let selector = Selector::new(runner).with_holder("agent-9");
        selector.select_and_claim().expect("ok");

        // Inspect the recorded calls: git-kb ready, then hf claim <slug> with the env.
        let calls = selector.runner.calls.borrow();
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[0].0, "git-kb");
        assert_eq!(calls[0].1, vec!["ready", "--json", "--limit", "1"]);

        assert_eq!(calls[1].0, "hf");
        // `hf claim <id>` — the slug is a POSITIONAL argument.
        assert_eq!(calls[1].1, vec!["claim", "tasks/alpha"]);
        assert_eq!(
            calls[1].2,
            vec![("HF_LEASE_HOLDER".to_string(), "agent-9".to_string())],
            "the lease holder must be set on the hf claim child"
        );
    }

    #[test]
    fn select_and_claim_nonzero_claim_is_conflict_not_acquired() {
        let runner = FakeRunner::new()
            .script("git-kb", out(READY_FIXTURE, "", Some(0)))
            .script(
                "hf",
                out(
                    "",
                    "hf claim: tasks/alpha BLOCKED — conflict: held by other",
                    Some(1),
                ),
            );
        let selector = Selector::new(runner).with_holder("agent-loser");

        let err = selector.select_and_claim().expect_err("conflict → err");
        match err {
            SelectionError::ClaimConflict { slug, code, stderr } => {
                // The lease was NOT treated as acquired: we get a conflict for the slug.
                assert_eq!(slug, "tasks/alpha");
                assert_eq!(code, Some(1));
                assert!(stderr.contains("conflict"), "stderr surfaced: {stderr}");
            }
            other => panic!("expected ClaimConflict, got {other:?}"),
        }
    }

    #[test]
    fn select_and_claim_ready_nonzero_exit_is_ready_command_failed() {
        let runner =
            FakeRunner::new().script("git-kb", out("", "git-kb: not a repository", Some(2)));
        let selector = Selector::new(runner).with_holder("agent-x");
        let err = selector.select_and_claim().expect_err("ready failed → err");
        match err {
            SelectionError::ReadyCommandFailed { code, stderr } => {
                assert_eq!(code, Some(2));
                assert!(stderr.contains("not a repository"));
            }
            other => panic!("expected ReadyCommandFailed, got {other:?}"),
        }
    }
}
