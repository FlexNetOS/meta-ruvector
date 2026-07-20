import { readFileSync, writeFileSync, existsSync, TRAJECTORIES_FILE, PATTERNS_FILE } from '../shared-kernel.js';

/**
 * ReasoningBank with A/B Testing, Decay, and Active Learning
 */
class ReasoningBank {
  constructor() {
    this.trajectories = this.loadTrajectories();
    this.qTable = this.loadPatterns();
    this.alpha = 0.1;
    this.gamma = 0.9;
    this.epsilon = 0.1;
    // A/B testing: Use environment override, or persistent session-based assignment
    // INTELLIGENCE_MODE=treatment forces learning mode (for development/testing)
    // INTELLIGENCE_MODE=control forces control mode (for baseline comparison)
    this.abTestGroup = process.env.INTELLIGENCE_MODE ||
      (this.getSessionId() % 100 < 5 ? 'control' : 'treatment'); // 5% holdout
    this.decayHalfLife = 7 * 24 * 60 * 60 * 1000; // 7 days in ms
  }

  loadTrajectories() {
    if (existsSync(TRAJECTORIES_FILE)) {
      try { return JSON.parse(readFileSync(TRAJECTORIES_FILE, 'utf-8')); }
      catch { return []; }
    }
    return [];
  }

  loadPatterns() {
    if (existsSync(PATTERNS_FILE)) {
      try { return JSON.parse(readFileSync(PATTERNS_FILE, 'utf-8')); }
      catch { return {}; }
    }
    return {};
  }

  /**
   * Get persistent session ID for consistent A/B assignment
   * Uses process PID + startup time hash for session-stable assignment
   */
  getSessionId() {
    // Combine PID with a time bucket (hourly) for session-stable but varied assignment
    const hourBucket = Math.floor(Date.now() / (60 * 60 * 1000));
    return (process.pid || 0) + hourBucket;
  }

  save() {
    writeFileSync(TRAJECTORIES_FILE, JSON.stringify(this.trajectories.slice(-1000), null, 2));
    writeFileSync(PATTERNS_FILE, JSON.stringify(this.qTable, null, 2));
  }

  stateKey(state) {
    // Preserve hyphens in crate names (e.g., ruvector-core, micro-hnsw-wasm)
    return state.toLowerCase().replace(/[^a-z0-9-]+/g, '_').slice(0, 80);
  }

  /**
   * Calculate decay weight based on trajectory age
   */
  getDecayWeight(timestamp) {
    const age = Date.now() - new Date(timestamp).getTime();
    return Math.pow(0.5, age / this.decayHalfLife);
  }

  /**
   * Record trajectory with time-weighted learning
   */
  recordTrajectory(state, action, outcome, reward) {
    const stateKey = this.stateKey(state);
    const trajectory = {
      id: `traj-${Date.now()}`,
      state: stateKey,
      action, outcome, reward,
      timestamp: new Date().toISOString(),
      abGroup: this.abTestGroup
    };
    this.trajectories.push(trajectory);

    // Time-weighted Q-learning with decay
    if (!this.qTable[stateKey]) this.qTable[stateKey] = { _meta: { lastUpdate: null, updateCount: 0 } };

    const meta = this.qTable[stateKey]._meta || { lastUpdate: null, updateCount: 0 };
    const decayWeight = meta.lastUpdate ? this.getDecayWeight(meta.lastUpdate) : 1.0;

    // Decayed current Q + new update
    const currentQ = (this.qTable[stateKey][action] || 0) * decayWeight;
    const updateCount = (meta.updateCount || 0) + 1;
    const adaptiveLR = Math.max(0.01, this.alpha / Math.sqrt(updateCount));

    this.qTable[stateKey][action] = Math.min(0.8, Math.max(-0.5,
      currentQ + adaptiveLR * (reward - currentQ)
    ));

    this.qTable[stateKey]._meta = {
      lastUpdate: new Date().toISOString(),
      updateCount
    };

    this.save();
    return trajectory.id;
  }

  /**
   * Get best action with A/B testing and active learning
   */
  getBestAction(state, availableActions) {
    const stateKey = this.stateKey(state);
    const qValues = this.qTable[stateKey] || {};

    // A/B Testing: Control group gets random actions
    if (this.abTestGroup === 'control') {
      const action = availableActions[Math.floor(Math.random() * availableActions.length)];
      return { action, confidence: 0, reason: 'control-group', qValues, abGroup: 'control' };
    }

    // Exploration with probability ε
    if (Math.random() < this.epsilon) {
      const action = availableActions[Math.floor(Math.random() * availableActions.length)];
      return { action, confidence: 0, reason: 'exploration', qValues, abGroup: 'treatment' };
    }

    // Exploitation
    let bestAction = availableActions[0];
    let bestQ = -Infinity;
    let secondBestQ = -Infinity;

    for (const action of availableActions) {
      const q = qValues[action] || 0;
      if (q > bestQ) {
        secondBestQ = bestQ;
        bestQ = q;
        bestAction = action;
      } else if (q > secondBestQ) {
        secondBestQ = q;
      }
    }

    const confidence = 1 / (1 + Math.exp(-bestQ * 2));

    // Active Learning: flag uncertain states
    const uncertainty = bestQ - secondBestQ;
    const isUncertain = uncertainty < 0.1 && bestQ < 0.5;

    return {
      action: bestAction,
      confidence: bestQ > 0 ? confidence : 0,
      reason: bestQ > 0 ? 'learned-preference' : 'no-data',
      qValues,
      abGroup: 'treatment',
      isUncertain,
      uncertaintyGap: uncertainty.toFixed(3)
    };
  }

  /**
   * Get uncertain states for active learning
   */
  getUncertainStates(threshold = 0.1) {
    const uncertain = [];
    for (const [state, actions] of Object.entries(this.qTable)) {
      if (state === '_meta') continue;

      const qVals = Object.entries(actions)
        .filter(([k]) => k !== '_meta')
        .map(([, v]) => v)
        .sort((a, b) => b - a);

      if (qVals.length >= 2) {
        const gap = qVals[0] - qVals[1];
        if (gap < threshold && qVals[0] < 0.5) {
          uncertain.push({ state, gap, topQ: qVals[0] });
        }
      }
    }
    return uncertain.sort((a, b) => a.gap - b.gap).slice(0, 10);
  }

  getTopPatterns(limit = 10) {
    const patterns = [];
    for (const [state, actions] of Object.entries(this.qTable)) {
      const sorted = Object.entries(actions)
        .filter(([k]) => k !== '_meta')
        .sort((a, b) => b[1] - a[1]);
      if (sorted.length > 0) {
        patterns.push({
          state,
          bestAction: sorted[0][0],
          qValue: sorted[0][1].toFixed(3),
          alternatives: sorted.slice(1, 3).map(([a, q]) => `${a}:${q.toFixed(2)}`)
        });
      }
    }
    return patterns.sort((a, b) => parseFloat(b.qValue) - parseFloat(a.qValue)).slice(0, limit);
  }

  getABStats() {
    const treatment = this.trajectories.filter(t => t.abGroup === 'treatment');
    const control = this.trajectories.filter(t => t.abGroup === 'control');

    const treatmentSuccess = treatment.filter(t => t.reward > 0).length;
    const controlSuccess = control.filter(t => t.reward > 0).length;

    return {
      treatment: { total: treatment.length, successRate: treatment.length > 0 ? (treatmentSuccess / treatment.length).toFixed(3) : 'N/A' },
      control: { total: control.length, successRate: control.length > 0 ? (controlSuccess / control.length).toFixed(3) : 'N/A' },
      lift: treatment.length > 10 && control.length > 10
        ? ((treatmentSuccess / treatment.length) - (controlSuccess / control.length)).toFixed(3)
        : 'insufficient-data'
    };
  }

  // ── ReasoningBank-with-AgentDB capabilities (verdict / distillation / replay) ──
  // Reward band that separates a success from a failure verdict (neutral in between).
  static get VERDICT_BAND() { return 0.05; }

  /**
   * Verdict judgment: classify an outcome as 'success' | 'failure' | 'neutral'.
   * Accepts a raw reward number or a trajectory-like object ({ reward }). Pure.
   */
  judge(rewardOrTrajectory) {
    const r = typeof rewardOrTrajectory === 'number'
      ? rewardOrTrajectory
      : (rewardOrTrajectory?.reward ?? 0);
    const band = ReasoningBank.VERDICT_BAND;
    if (r > band) return 'success';
    if (r < -band) return 'failure';
    return 'neutral';
  }

  /** Verdict tally across the recorded trajectories (pure). */
  judgeAll() {
    const tally = { success: 0, failure: 0, neutral: 0, total: 0 };
    for (const t of this.trajectories) { tally[this.judge(t)]++; tally.total++; }
    return tally;
  }

  /**
   * Memory distillation: consolidate the learned Q-table into a ranked set of
   * reusable "lessons" (state -> best action) above a confidence floor, each
   * carrying its verdict and how many updates back it. Read-only / pure.
   */
  distill({ minQ = 0.1, limit = 20 } = {}) {
    const lessons = [];
    for (const [state, actions] of Object.entries(this.qTable)) {
      if (state === '_meta' || !actions) continue;
      const sorted = Object.entries(actions)
        .filter(([k]) => k !== '_meta')
        .sort((a, b) => b[1] - a[1]);
      if (sorted.length && sorted[0][1] >= minQ) {
        lessons.push({
          state,
          action: sorted[0][0],
          q: Number(sorted[0][1].toFixed(3)),
          verdict: this.judge(sorted[0][1]),
          updates: actions._meta?.updateCount ?? 0,
        });
      }
    }
    return lessons.sort((a, b) => b.q - a.q).slice(0, limit);
  }

  /**
   * Experience replay: re-apply the Q-update for the most recent `n` stored
   * trajectories to reinforce learning from past experience (smaller learning
   * rate than live recording; appends NO new trajectory). In-memory by default;
   * pass { persist: true } to write the qTable through. Returns the count applied.
   */
  replay(n = 50, { persist = false } = {}) {
    const sample = this.trajectories.slice(-Math.max(0, n));
    let applied = 0;
    const replayLR = this.alpha * 0.5;
    for (const t of sample) {
      if (!t || !t.state || !t.action) continue;
      const sk = t.state; // stored trajectories already hold a stateKey
      if (!this.qTable[sk]) this.qTable[sk] = { _meta: { lastUpdate: null, updateCount: 0 } };
      const cur = this.qTable[sk][t.action] || 0;
      this.qTable[sk][t.action] = Math.min(0.8, Math.max(-0.5,
        cur + replayLR * ((t.reward ?? 0) - cur)
      ));
      applied++;
    }
    if (persist && applied) this.save();
    return applied;
  }
}

export { ReasoningBank };
