//! TEASTASK-008 — capture each REAL run as a learning trajectory (DOMAIN_MODEL seam
//! S6, the capability-gain law).
//!
//! Every real run outcome is recorded into a [`sona`](ruvector_sona) `ReasoningBank`
//! as a one-step [`QueryTrajectory`], so each cycle can inform the next. This module
//! **reuses** ruvector-sona's trajectory machinery — it does not reimplement clustering,
//! trajectory storage, or recall.
//!
//! ## Honesty caveat (read before trusting recall)
//!
//! sona's trajectory API is *embedding-based*: a `QueryTrajectory` carries a query
//! embedding and per-step neural activations, and recall is nearest-neighbour over those
//! vectors. A shell verification (`verification_command` exits 0/non-zero) produces **no
//! neural activations**. Rather than fabricate a fake semantic signal, this recorder uses
//! a **deterministic pseudo-embedding** derived from a blake3 hash of the WorkOrder's
//! `task_id` + `objective` (see [`deterministic_embedding`]) — the same
//! deterministic-hash-embedding pattern the ruvector ledger v2 uses. There is **no model
//! and no network**.
//!
//! Consequence: the honest, meaningful signal captured here is the **reward** (`1.0` for a
//! Completed run, `0.0` for a Failed run) and the **fact of the run** (trajectory count).
//! Semantic recall — "find runs whose *objective* resembles this one" — is **deferred**:
//! it requires a real text embedder wired in front of [`deterministic_embedding`]. Until
//! then [`TrajectoryRecorder::recall`] clusters over hash vectors, which co-locates
//! identical objectives but carries no cross-objective semantic meaning.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

use ruvector_sona::reasoning_bank::{PatternConfig, ReasoningBank};
use ruvector_sona::trajectory::TrajectoryBuilder;
use ruvector_sona::types::LearnedPattern;

/// A single recorded run outcome, kept alongside the sona `ReasoningBank` for direct
/// observability (the bank's per-trajectory reward is otherwise private).
///
/// The `reward` here is the *same* honest signal handed to `ReasoningBank::add_trajectory`
/// as the trajectory's final quality — `1.0` for Completed, `0.0` for Failed.
#[derive(Clone, Debug, PartialEq)]
pub struct RecordedRun {
    /// Deterministic trajectory id assigned by the recorder.
    pub id: u64,
    /// The WorkOrder / TaskSpec id this run belongs to.
    pub task_id: String,
    /// The honest reward signal: `1.0` Completed, `0.0` Failed.
    pub reward: f32,
}

/// Records each REAL run outcome as a learning trajectory in a sona `ReasoningBank`.
///
/// Thread-safe and cheap to share behind an `Arc`: the bank sits behind a [`Mutex`] and
/// ids come from an [`AtomicU64`]. See the module docs for the honesty caveat around the
/// deterministic embedding.
#[derive(Debug)]
pub struct TrajectoryRecorder {
    /// The reused sona reasoning bank (trajectory storage + clustering/recall).
    bank: Mutex<ReasoningBank>,
    /// Monotonic trajectory id source.
    next_id: AtomicU64,
    /// Embedding width, matched to the bank's `embedding_dim` so `add_trajectory`'s
    /// internal `compute_embedding` consumes the whole query vector.
    embedding_dim: usize,
    /// Direct log of recorded rewards (the bank keeps its own copy privately).
    recorded: Mutex<Vec<RecordedRun>>,
}

impl Default for TrajectoryRecorder {
    fn default() -> Self {
        Self::new()
    }
}

impl TrajectoryRecorder {
    /// Construct a recorder over a fresh `ReasoningBank` with sona's default config.
    #[must_use]
    pub fn new() -> Self {
        Self::with_config(PatternConfig::default())
    }

    /// Construct a recorder over a `ReasoningBank` built from `config`.
    #[must_use]
    pub fn with_config(config: PatternConfig) -> Self {
        let embedding_dim = config.embedding_dim.max(1);
        Self {
            bank: Mutex::new(ReasoningBank::new(config)),
            next_id: AtomicU64::new(0),
            embedding_dim,
            recorded: Mutex::new(Vec::new()),
        }
    }

    /// Record a REAL run outcome as a one-step learning trajectory.
    ///
    /// Builds a [`QueryTrajectory`](ruvector_sona::types::QueryTrajectory) via sona's
    /// [`TrajectoryBuilder`]: a deterministic query embedding seeded from
    /// `task_id` + `objective`, a single named step `"verify"` carrying `verdict_reward`,
    /// finalized with `verdict_reward` as the trajectory quality, then handed to
    /// `ReasoningBank::add_trajectory`.
    ///
    /// `verdict_reward` is the honest signal: `1.0` for a Completed run, `0.0` for a
    /// Failed run. Returns the assigned trajectory id on success.
    ///
    /// # Errors
    /// Returns [`LearningError::Poisoned`] only if an internal lock was poisoned by a
    /// panic in another thread — a state the engine treats as best-effort and swallows.
    pub fn record_run(
        &self,
        task_id: &str,
        objective: &str,
        verdict_reward: f32,
    ) -> Result<u64, LearningError> {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let seed = format!("{task_id}\n{objective}");
        let embedding = deterministic_embedding(&seed, self.embedding_dim);

        // One named step "verify" carrying the real reward. Its activations reuse the
        // deterministic embedding so the trajectory's stored vector stays stable; there
        // are no real neural activations to record (see module docs). Attention weights
        // are empty for the same reason.
        let mut builder = TrajectoryBuilder::new(id, embedding.clone());
        builder.add_named_step("verify", embedding, Vec::new(), verdict_reward);
        let trajectory = builder.build(verdict_reward);

        let mut bank = self.bank.lock().map_err(|_| LearningError::Poisoned)?;
        bank.add_trajectory(&trajectory);
        drop(bank);

        let mut recorded = self.recorded.lock().map_err(|_| LearningError::Poisoned)?;
        recorded.push(RecordedRun {
            id,
            task_id: task_id.to_string(),
            reward: verdict_reward,
        });

        Ok(id)
    }

    /// Number of trajectories currently held by the underlying `ReasoningBank`.
    ///
    /// This is the honest proof that a run was captured — it increments once per
    /// [`record_run`](Self::record_run), independent of reward. Recovers a poisoned lock
    /// rather than panicking.
    #[must_use]
    pub fn trajectory_count(&self) -> usize {
        match self.bank.lock() {
            Ok(bank) => bank.trajectory_count(),
            Err(poison) => poison.into_inner().trajectory_count(),
        }
    }

    /// Snapshot of the rewards recorded so far, in record order.
    #[must_use]
    pub fn recorded(&self) -> Vec<RecordedRun> {
        match self.recorded.lock() {
            Ok(r) => r.clone(),
            Err(poison) => poison.into_inner().clone(),
        }
    }

    /// Recall the learned patterns nearest to a `task_id` + `objective` query.
    ///
    /// Crystallizes patterns (`ReasoningBank::extract_patterns`) over the recorded
    /// trajectories, then returns the top-`k` nearest via cosine similarity. **This proves
    /// a recorded run is retrievable**, but — per the module honesty caveat — the geometry
    /// is over deterministic hash vectors, so it co-locates *identical* objectives without
    /// carrying cross-objective semantic meaning. Meaningful semantic recall is deferred to
    /// a real embedder. Recovers a poisoned lock rather than panicking.
    #[must_use]
    pub fn recall(&self, task_id: &str, objective: &str, k: usize) -> Vec<LearnedPattern> {
        let seed = format!("{task_id}\n{objective}");
        let query = deterministic_embedding(&seed, self.embedding_dim);

        let mut bank = match self.bank.lock() {
            Ok(bank) => bank,
            Err(poison) => poison.into_inner(),
        };
        bank.extract_patterns();
        bank.find_similar(&query, k)
            .into_iter()
            .cloned()
            .collect()
    }
}

/// Errors surfaced while recording a learning trajectory.
///
/// Learning is best-effort in the engine (unlike proof, which is mandatory), so these are
/// swallowed at the call site — they exist for direct callers and tests.
#[derive(Debug, thiserror::Error)]
pub enum LearningError {
    /// An internal lock was poisoned by a panic in another thread.
    #[error("trajectory recorder lock poisoned")]
    Poisoned,
}

/// Deterministic pseudo-embedding of `seed` into a fixed-width `dim` `f32` vector.
///
/// **Placeholder, not a semantic embedding.** The bytes come from a blake3 extendable
/// output over `seed` — no model, no network. Each 4-byte chunk maps to a value in
/// `[-1.0, 1.0]`. Identical seeds always yield identical vectors (used as the query
/// embedding and step activations for a run's trajectory); *different* seeds yield
/// uncorrelated vectors, so this carries **no semantic similarity structure**. Wiring a
/// real text embedder in front of this is the deferred path to meaningful semantic recall
/// (see module docs).
#[must_use]
pub fn deterministic_embedding(seed: &str, dim: usize) -> Vec<f32> {
    if dim == 0 {
        return Vec::new();
    }
    let mut hasher = blake3::Hasher::new();
    hasher.update(seed.as_bytes());
    let mut reader = hasher.finalize_xof();

    let mut bytes = vec![0u8; dim * 4];
    reader.fill(&mut bytes);

    let mut out = Vec::with_capacity(dim);
    for chunk in bytes.chunks_exact(4) {
        let v = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
        // Map u32 -> [-1.0, 1.0].
        let unit = (v as f64) / (u32::MAX as f64);
        out.push(((unit * 2.0) - 1.0) as f32);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_embedding_is_deterministic_and_correct_length() {
        let a = deterministic_embedding("TEASTASK-008\nlearn from runs", 256);
        let b = deterministic_embedding("TEASTASK-008\nlearn from runs", 256);
        assert_eq!(a.len(), 256, "embedding must have the requested width");
        assert_eq!(a, b, "same seed must yield the same vector");

        let c = deterministic_embedding("different seed", 256);
        assert_ne!(a, c, "different seeds must yield different vectors");

        // Values stay in the documented [-1.0, 1.0] band.
        for v in &a {
            assert!((-1.0..=1.0).contains(v), "value {v} out of [-1,1]");
        }

        assert_eq!(deterministic_embedding("x", 0).len(), 0);
        assert_eq!(deterministic_embedding("x", 8).len(), 8);
    }

    #[test]
    fn records_two_runs_and_count_reflects_two() {
        let recorder = TrajectoryRecorder::new();
        let pass_id = recorder
            .record_run("TASK-A", "make the passing thing pass", 1.0)
            .expect("record pass");
        let fail_id = recorder
            .record_run("TASK-B", "the failing thing", 0.0)
            .expect("record fail");

        assert_ne!(pass_id, fail_id, "ids must be unique per run");
        assert_eq!(
            recorder.trajectory_count(),
            2,
            "both runs must be held by the reasoning bank"
        );

        let recorded = recorder.recorded();
        assert_eq!(recorded.len(), 2);
        assert_eq!(recorded[0].task_id, "TASK-A");
        assert_eq!(recorded[0].reward, 1.0);
        assert_eq!(recorded[1].task_id, "TASK-B");
        assert_eq!(recorded[1].reward, 0.0);
    }

    #[test]
    fn recall_returns_a_recorded_trajectory() {
        let recorder = TrajectoryRecorder::new();
        recorder
            .record_run("TASK-A", "make the passing thing pass", 1.0)
            .expect("record pass");

        // The passing run has reward 1.0 > the default quality threshold, so a pattern
        // crystallizes and is retrievable by the same seed.
        let hits = recorder.recall("TASK-A", "make the passing thing pass", 1);
        assert!(
            !hits.is_empty(),
            "a recorded high-reward run must be retrievable via recall"
        );
    }
}
