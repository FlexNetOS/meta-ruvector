import { readFileSync, writeFileSync, existsSync, CALIBRATION_FILE } from '../shared-kernel.js';

/**
 * Calibration Tracker - measures if confidence matches reality
 */
class CalibrationTracker {
  constructor() {
    this.data = this.load();
  }

  load() {
    if (existsSync(CALIBRATION_FILE)) {
      try { return JSON.parse(readFileSync(CALIBRATION_FILE, 'utf-8')); }
      catch { return { buckets: {}, predictions: [] }; }
    }
    return { buckets: {}, predictions: [] };
  }

  save() {
    writeFileSync(CALIBRATION_FILE, JSON.stringify(this.data, null, 2));
  }

  record(predicted, actual, confidence) {
    const correct = predicted === actual;
    const bucket = Math.floor(confidence * 10) / 10; // 0.0, 0.1, ..., 0.9

    if (!this.data.buckets[bucket]) {
      this.data.buckets[bucket] = { total: 0, correct: 0 };
    }
    this.data.buckets[bucket].total++;
    if (correct) this.data.buckets[bucket].correct++;

    this.data.predictions.push({
      predicted, actual, correct, confidence,
      timestamp: new Date().toISOString()
    });

    // Keep last 500 predictions
    if (this.data.predictions.length > 500) {
      this.data.predictions = this.data.predictions.slice(-500);
    }

    this.save();
    return correct;
  }

  getCalibrationError() {
    let totalError = 0, count = 0;
    for (const [bucket, { total, correct }] of Object.entries(this.data.buckets)) {
      if (total >= 5) {
        const expectedRate = parseFloat(bucket) + 0.05;
        const actualRate = correct / total;
        totalError += Math.abs(expectedRate - actualRate);
        count++;
      }
    }
    return count > 0 ? totalError / count : 0;
  }

  getStats() {
    const stats = {};
    for (const [bucket, { total, correct }] of Object.entries(this.data.buckets)) {
      stats[bucket] = {
        total,
        accuracy: (correct / total).toFixed(3),
        expected: (parseFloat(bucket) + 0.05).toFixed(2)
      };
    }
    return { buckets: stats, calibrationError: this.getCalibrationError().toFixed(3) };
  }
}

export { CalibrationTracker };
