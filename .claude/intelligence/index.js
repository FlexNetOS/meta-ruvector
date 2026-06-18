/**
 * RuVector Intelligence Layer v2 for Claude Code
 *
 * Enhanced with:
 * 1. Native HNSW rebuild on startup (150x faster search)
 * 2. Hyperbolic distance for hierarchical embeddings
 * 3. Confidence Calibration (track predicted vs actual)
 * 4. A/B Testing (holdout group comparison)
 * 5. Feedback Loop (learn from followed/ignored suggestions)
 * 6. Active Learning (identify uncertain states)
 * 7. Pattern Decay (time-weighted trajectories)
 */

// DDD decomposition: this god object was split into bounded-context modules.
// index.js is now the thin facade — it composes the domain classes into the
// RuVectorIntelligence aggregate and re-exports the original public interface.
import { ruvectorAvailable } from './shared-kernel.js';
import { VectorMemory } from './domains/vector-memory.js';
import { CalibrationTracker } from './domains/calibration-tracker.js';
import { FeedbackLoop } from './domains/feedback-loop.js';
import { ReasoningBank } from './domains/reasoning-bank.js';
import { ErrorPatternTracker } from './domains/error-pattern-tracker.js';
import { SequenceTracker } from './domains/sequence-tracker.js';
import { NeuralRouter } from './domains/neural-router.js';

/**
 * Main Intelligence API v2
 */
class RuVectorIntelligence {
  constructor(options = {}) {
    this.memory = new VectorMemory({ hyperbolic: options.hyperbolic ?? true });
    this.reasoning = new ReasoningBank();
    this.calibration = new CalibrationTracker();
    this.feedback = new FeedbackLoop();
    this.errorPatterns = new ErrorPatternTracker();
    this.sequences = new SequenceTracker();
    this.router = new NeuralRouter(this.memory, this.reasoning, this.calibration, this.feedback);
    this.initialized = false;
  }

  async init() {
    if (!this.initialized) {
      await this.memory.init();
      this.initialized = true;
    }
  }

  async remember(type, content, metadata = {}) {
    await this.init();
    return this.memory.store(type, content, metadata);
  }

  async recall(query, limit = 5) {
    await this.init();
    return this.memory.search(query, limit);
  }

  learn(state, action, outcome, reward) {
    return this.reasoning.recordTrajectory(state, action, outcome, reward);
  }

  suggest(state, actions) {
    return this.reasoning.getBestAction(state, actions);
  }

  // ReasoningBank-with-AgentDB: verdict judgment, memory distillation, experience replay.
  judge(rewardOrTrajectory) {
    return this.reasoning.judge(rewardOrTrajectory);
  }

  judgeAll() {
    return this.reasoning.judgeAll();
  }

  distill(opts) {
    return this.reasoning.distill(opts);
  }

  replay(n, opts) {
    return this.reasoning.replay(n, opts);
  }

  async route(task, context = {}) {
    await this.init();
    return this.router.route(task, context);
  }

  recordCalibration(predicted, actual, confidence) {
    return this.calibration.record(predicted, actual, confidence);
  }

  recordFeedback(suggestionId, actualUsed, success) {
    this.feedback.recordOutcome(suggestionId, actualUsed, success);
  }

  // === New v3 Features ===

  /**
   * Record an error from command output
   */
  recordError(command, stderr, file = null, crate = null) {
    return this.errorPatterns.recordError(command, stderr, file, crate);
  }

  /**
   * Record a fix for an error pattern
   */
  recordFix(errorCode, fixDescription) {
    this.errorPatterns.recordFix(errorCode, fixDescription);
  }

  /**
   * Get suggested fixes for an error
   */
  suggestFix(errorCode) {
    return this.errorPatterns.suggestFix(errorCode);
  }

  /**
   * Record a file edit for sequence learning
   */
  recordFileEdit(file) {
    this.sequences.recordEdit(file);
  }

  /**
   * Suggest next files based on current file
   */
  suggestNextFiles(file, limit = 3) {
    return this.sequences.suggestNextFiles(file, limit);
  }

  /**
   * Check if tests should be suggested after editing a file
   */
  shouldSuggestTests(file) {
    return this.sequences.shouldSuggestTests(file);
  }

  stats() {
    return {
      memory: this.memory.getStats(),
      trajectories: this.reasoning.trajectories.length,
      patterns: Object.keys(this.reasoning.qTable).length,
      topPatterns: this.reasoning.getTopPatterns(5),
      calibration: this.calibration.getStats(),
      abTest: this.reasoning.getABStats(),
      adviceValue: this.feedback.getAdviceValue(),
      uncertainStates: this.reasoning.getUncertainStates(0.15),
      // v3 stats
      errorPatterns: this.errorPatterns.getStats(),
      sequences: this.sequences.getStats(),
      ruvectorNative: ruvectorAvailable
    };
  }
}


export { RuVectorIntelligence, VectorMemory, ReasoningBank, NeuralRouter, CalibrationTracker, FeedbackLoop, ErrorPatternTracker, SequenceTracker };
export default RuVectorIntelligence;
