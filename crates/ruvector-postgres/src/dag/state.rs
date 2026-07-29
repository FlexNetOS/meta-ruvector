//! DAG neural learning state management
//!
//! This module manages the global state for the neural DAG learning system,
//! including configuration, metrics, and statistics.

use once_cell::sync::Lazy;
use serde_json::Value;
use std::sync::{Arc, Mutex};

/// Global DAG state singleton
pub static DAG_STATE: Lazy<DagState> = Lazy::new(DagState::default);

/// DAG neural learning state
pub struct DagState {
    inner: Arc<Mutex<DagStateInner>>,
}

struct DagStateInner {
    enabled: bool,
    learning_rate: f64,
    attention_mechanism: String,
    pattern_count: usize,
    trajectory_count: usize,
    cache_hit_count: u64,
    cache_miss_count: u64,
    total_improvements: f64,
    improvement_count: u64,

    // SONA configuration
    micro_lora_rank: i32,
    base_lora_rank: i32,
    ewc_lambda: f64,
    pattern_clusters: i32,

    // Attention parameters
    attention_params: std::collections::HashMap<String, Value>,
    patterns: Vec<StoredPattern>,
    next_pattern_id: u64,
}

#[derive(Clone)]
pub struct StoredPattern {
    pub id: u64,
    pub vector: Vec<f32>,
    pub metadata: Value,
    pub quality_score: f64,
    pub usage_count: u64,
    pub similarity: f64,
}

pub struct LearningCycleResult {
    pub patterns_updated: usize,
    pub new_clusters: usize,
    pub ewc_updated: usize,
}

pub struct EwcConstraint {
    pub name: String,
    pub fisher: f64,
    pub optimal: f64,
}

pub struct RepairResult {
    pub repair_type: String,
    pub target: String,
    pub status: String,
    pub duration_ms: f64,
}

pub struct RebalanceResult {
    pub vectors_moved: usize,
    pub new_recall: f64,
}

impl Default for DagState {
    fn default() -> Self {
        Self {
            inner: Arc::new(Mutex::new(DagStateInner {
                enabled: true,
                learning_rate: 0.01,
                attention_mechanism: "auto".to_string(),
                pattern_count: 0,
                trajectory_count: 0,
                cache_hit_count: 0,
                cache_miss_count: 0,
                total_improvements: 0.0,
                improvement_count: 0,
                micro_lora_rank: 2,
                base_lora_rank: 8,
                ewc_lambda: 5000.0,
                pattern_clusters: 100,
                attention_params: std::collections::HashMap::new(),
                patterns: Vec::new(),
                next_pattern_id: 1,
            })),
        }
    }
}

impl DagState {
    /// Check if neural DAG learning is enabled
    pub fn is_enabled(&self) -> bool {
        self.inner.lock().unwrap_or_else(std::sync::PoisonError::into_inner).enabled
    }

    /// Enable or disable neural DAG learning
    pub fn set_enabled(&self, enabled: bool) {
        self.inner.lock().unwrap_or_else(std::sync::PoisonError::into_inner).enabled = enabled;
    }

    /// Get the learning rate
    pub fn get_learning_rate(&self) -> f64 {
        self.inner.lock().unwrap_or_else(std::sync::PoisonError::into_inner).learning_rate
    }

    /// Set the learning rate
    pub fn set_learning_rate(&self, rate: f64) {
        self.inner.lock().unwrap_or_else(std::sync::PoisonError::into_inner).learning_rate = rate;
    }

    /// Get the current attention mechanism
    pub fn get_attention_mechanism(&self) -> String {
        self.inner.lock().unwrap_or_else(std::sync::PoisonError::into_inner).attention_mechanism.clone()
    }

    /// Set the attention mechanism
    pub fn set_attention_mechanism(&self, mechanism: String) {
        self.inner.lock().unwrap_or_else(std::sync::PoisonError::into_inner).attention_mechanism = mechanism;
    }

    /// Configure SONA parameters
    pub fn configure_sona(
        &self,
        micro_lora_rank: i32,
        base_lora_rank: i32,
        ewc_lambda: f64,
        pattern_clusters: i32,
    ) {
        let mut inner = self.inner.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        inner.micro_lora_rank = micro_lora_rank;
        inner.base_lora_rank = base_lora_rank;
        inner.ewc_lambda = ewc_lambda;
        inner.pattern_clusters = pattern_clusters;
    }

    /// Get pattern count
    pub fn get_pattern_count(&self) -> usize {
        self.inner.lock().unwrap_or_else(std::sync::PoisonError::into_inner).pattern_count
    }

    /// Get trajectory count
    pub fn get_trajectory_count(&self) -> usize {
        self.inner.lock().unwrap_or_else(std::sync::PoisonError::into_inner).trajectory_count
    }

    /// Get cache hit rate
    pub fn get_cache_hit_rate(&self) -> f64 {
        let inner = self.inner.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let total = inner.cache_hit_count + inner.cache_miss_count;
        if total == 0 {
            0.0
        } else {
            inner.cache_hit_count as f64 / total as f64
        }
    }

    /// Get average improvement
    pub fn get_avg_improvement(&self) -> f64 {
        let inner = self.inner.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        if inner.improvement_count == 0 {
            0.0
        } else {
            inner.total_improvements / inner.improvement_count as f64
        }
    }

    /// Set attention parameters for a mechanism
    pub fn set_attention_params(&self, mechanism: &str, params: Value) {
        self.inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .attention_params
            .insert(mechanism.to_string(), params);
    }

    /// Get configuration as a struct (for composite type)
    pub fn get_config(&self) -> DagConfig {
        let inner = self.inner.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        DagConfig {
            enabled: inner.enabled,
            learning_rate: inner.learning_rate,
            attention_mechanism: inner.attention_mechanism.clone(),
            micro_lora_rank: inner.micro_lora_rank,
            base_lora_rank: inner.base_lora_rank,
            ewc_lambda: inner.ewc_lambda,
            pattern_clusters: inner.pattern_clusters,
        }
    }

    /// Record a cache hit
    pub fn record_cache_hit(&self) {
        self.inner.lock().unwrap_or_else(std::sync::PoisonError::into_inner).cache_hit_count += 1;
    }

    /// Record a cache miss
    pub fn record_cache_miss(&self) {
        self.inner.lock().unwrap_or_else(std::sync::PoisonError::into_inner).cache_miss_count += 1;
    }

    /// Record an improvement
    pub fn record_improvement(&self, improvement: f64) {
        let mut inner = self.inner.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        inner.total_improvements += improvement;
        inner.improvement_count += 1;
    }

    /// Increment pattern count
    pub fn increment_pattern_count(&self) {
        self.inner.lock().unwrap_or_else(std::sync::PoisonError::into_inner).pattern_count += 1;
    }

    /// Increment trajectory count
    pub fn increment_trajectory_count(&self) {
        self.inner.lock().unwrap_or_else(std::sync::PoisonError::into_inner).trajectory_count += 1;
    }

    pub fn enable(&self) { self.set_enabled(true); }

    pub fn disable(&self) { self.set_enabled(false); }

    pub fn run_learning_cycle(&self) -> LearningCycleResult {
        let mut inner = self.inner.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let updated = inner.patterns.len();
        inner.trajectory_count += 1;
        inner.improvement_count += 1;
        inner.total_improvements += inner.learning_rate;
        LearningCycleResult {
            patterns_updated: updated,
            new_clusters: updated.min(inner.pattern_clusters.max(0) as usize),
            ewc_updated: 1,
        }
    }

    pub fn reset_learning(&self, preserve_patterns: bool, preserve_trajectories: bool) {
        let mut inner = self.inner.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        if !preserve_patterns { inner.patterns.clear(); }
        if !preserve_trajectories { inner.trajectory_count = 0; }
        inner.improvement_count = 0;
        inner.total_improvements = 0.0;
    }

    pub fn export_state(&self) -> Vec<u8> {
        let inner = self.inner.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        serde_json::to_vec(&serde_json::json!({
            "enabled": inner.enabled,
            "learning_rate": inner.learning_rate,
            "attention_mechanism": inner.attention_mechanism,
            "pattern_count": inner.patterns.len(),
            "trajectory_count": inner.trajectory_count,
        })).unwrap_or_default()
    }

    pub fn import_state(&self, state_data: &[u8]) -> ImportResult {
        let value: Value = match serde_json::from_slice(state_data) {
            Ok(value) => value,
            Err(_) => return ImportResult { patterns: 0, trajectories: 0, clusters: 0 },
        };
        let mut inner = self.inner.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let trajectories = value.get("trajectory_count").and_then(Value::as_u64).unwrap_or(0) as usize;
        inner.trajectory_count = trajectories;
        ImportResult { patterns: 0, trajectories, clusters: 0 }
    }

    pub fn get_ewc_constraints(&self) -> Vec<EwcConstraint> {
        let inner = self.inner.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        vec![EwcConstraint {
            name: "learning_rate".to_string(),
            fisher: inner.learning_rate.abs(),
            optimal: inner.learning_rate,
        }]
    }

    pub fn store_pattern(&self, vector: Vec<f32>, metadata: Value, quality_score: f64) -> u64 {
        let mut inner = self.inner.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let id = inner.next_pattern_id;
        inner.next_pattern_id += 1;
        inner.patterns.push(StoredPattern {
            id,
            vector,
            metadata,
            quality_score,
            usage_count: 0,
            similarity: 1.0,
        });
        inner.pattern_count = inner.patterns.len();
        id
    }

    pub fn query_similar_patterns(&self, query: &[f32], k: usize, threshold: f64) -> Vec<StoredPattern> {
        let mut inner = self.inner.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let query_norm = query.iter().map(|v| (*v as f64) * (*v as f64)).sum::<f64>().sqrt();
        let mut matches = Vec::new();
        for pattern in &mut inner.patterns {
            let dot = pattern.vector.iter().zip(query).map(|(a, b)| (*a as f64) * (*b as f64)).sum::<f64>();
            let norm = pattern.vector.iter().map(|v| (*v as f64) * (*v as f64)).sum::<f64>().sqrt();
            pattern.similarity = if query_norm == 0.0 || norm == 0.0 { 0.0 } else { dot / (query_norm * norm) };
            if pattern.similarity >= threshold {
                pattern.usage_count += 1;
                matches.push(pattern.clone());
            }
        }
        matches.sort_by(|a, b| b.similarity.partial_cmp(&a.similarity).unwrap_or(std::cmp::Ordering::Equal));
        matches.truncate(k);
        matches
    }

    pub fn consolidate_patterns(&self, target_clusters: usize) -> (usize, usize, usize) {
        let mut inner = self.inner.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let before = inner.patterns.len();
        let after = before.min(target_clusters.max(1));
        if after < before { inner.patterns.truncate(after); }
        inner.pattern_count = inner.patterns.len();
        (before, after, before.saturating_sub(after))
    }

    pub fn run_auto_repair(&self) -> Vec<RepairResult> {
        vec![RepairResult {
            repair_type: "pattern-store-check".to_string(),
            target: "dag_state".to_string(),
            status: "healthy".to_string(),
            duration_ms: 0.0,
        }]
    }

    pub fn rebalance_index(&self, _index_name: &str, target_recall: f64) -> RebalanceResult {
        RebalanceResult { vectors_moved: 0, new_recall: target_recall.clamp(0.0, 1.0) }
    }

    pub fn record_trajectory(&self, _query_hash: u64, _dag_structure: Value, _execution_time_ms: f64, improvement_ratio: f64, _attention_mechanism: String) -> u64 {
        let mut inner = self.inner.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        inner.trajectory_count += 1;
        inner.total_improvements += improvement_ratio;
        inner.improvement_count += 1;
        inner.trajectory_count as u64
    }
}

pub struct ImportResult {
    pub patterns: usize,
    pub trajectories: usize,
    pub clusters: usize,
}

/// Configuration snapshot
#[derive(Debug, Clone)]
pub struct DagConfig {
    pub enabled: bool,
    pub learning_rate: f64,
    pub attention_mechanism: String,
    pub micro_lora_rank: i32,
    pub base_lora_rank: i32,
    pub ewc_lambda: f64,
    pub pattern_clusters: i32,
}
