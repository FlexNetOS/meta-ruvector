import { readFileSync, writeFileSync, existsSync, FEEDBACK_FILE } from '../shared-kernel.js';

/**
 * Feedback Loop - track if suggestions were followed
 */
class FeedbackLoop {
  constructor() {
    this.data = this.load();
  }

  load() {
    if (existsSync(FEEDBACK_FILE)) {
      try { return JSON.parse(readFileSync(FEEDBACK_FILE, 'utf-8')); }
      catch { return { suggestions: [], followRates: {} }; }
    }
    return { suggestions: [], followRates: {} };
  }

  save() {
    writeFileSync(FEEDBACK_FILE, JSON.stringify(this.data, null, 2));
  }

  recordSuggestion(suggestionId, suggested, confidence) {
    this.data.suggestions.push({
      id: suggestionId,
      suggested,
      confidence,
      followed: null,
      outcome: null,
      timestamp: new Date().toISOString()
    });
    this.save();
    return suggestionId;
  }

  recordOutcome(suggestionId, actualUsed, success) {
    const suggestion = this.data.suggestions.find(s => s.id === suggestionId);
    if (suggestion) {
      suggestion.followed = suggestion.suggested === actualUsed;
      suggestion.outcome = success;

      // Update follow rates
      const key = suggestion.suggested;
      if (!this.data.followRates[key]) {
        this.data.followRates[key] = { total: 0, followed: 0, followedSuccess: 0, ignoredSuccess: 0 };
      }
      const r = this.data.followRates[key];
      r.total++;
      if (suggestion.followed) {
        r.followed++;
        if (success) r.followedSuccess++;
      } else {
        if (success) r.ignoredSuccess++;
      }

      this.save();
    }
  }

  getAdviceValue() {
    const result = {};
    for (const [key, r] of Object.entries(this.data.followRates)) {
      if (r.total >= 5) {
        const followRate = r.followed / r.total;
        const followedSuccessRate = r.followed > 0 ? r.followedSuccess / r.followed : 0;
        const ignoredSuccessRate = (r.total - r.followed) > 0
          ? r.ignoredSuccess / (r.total - r.followed) : 0;

        result[key] = {
          followRate: followRate.toFixed(3),
          followedSuccessRate: followedSuccessRate.toFixed(3),
          ignoredSuccessRate: ignoredSuccessRate.toFixed(3),
          adviceValue: (followedSuccessRate - ignoredSuccessRate).toFixed(3)
        };
      }
    }
    return result;
  }
}

export { FeedbackLoop };
