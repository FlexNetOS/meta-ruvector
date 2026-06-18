import { readFileSync, writeFileSync, existsSync, ERROR_PATTERNS_FILE } from '../shared-kernel.js';

/**
 * Error Pattern Tracker - learns from specific error types
 */
class ErrorPatternTracker {
  constructor() {
    this.data = this.load();
  }

  load() {
    if (existsSync(ERROR_PATTERNS_FILE)) {
      try { return JSON.parse(readFileSync(ERROR_PATTERNS_FILE, 'utf-8')); }
      catch { return { patterns: {}, fixes: {}, recentErrors: [] }; }
    }
    return { patterns: {}, fixes: {}, recentErrors: [] };
  }

  save() {
    writeFileSync(ERROR_PATTERNS_FILE, JSON.stringify(this.data, null, 2));
  }

  /**
   * Parse error output to extract error codes and types
   */
  parseError(stderr) {
    const errors = [];

    // Rust error codes (E0308, E0433, etc.)
    const rustErrors = stderr.match(/error\[E\d{4}\]/g) || [];
    for (const e of rustErrors) {
      const code = e.match(/E\d{4}/)[0];
      errors.push({ type: 'rust', code, category: this.categorizeRustError(code) });
    }

    // TypeScript errors (TS2304, TS2322, etc.)
    const tsErrors = stderr.match(/TS\d{4}/g) || [];
    for (const code of tsErrors) {
      errors.push({ type: 'typescript', code, category: this.categorizeTsError(code) });
    }

    // npm/node errors
    if (stderr.includes('ENOENT')) errors.push({ type: 'npm', code: 'ENOENT', category: 'file-not-found' });
    if (stderr.includes('EACCES')) errors.push({ type: 'npm', code: 'EACCES', category: 'permission' });
    if (stderr.includes('MODULE_NOT_FOUND')) errors.push({ type: 'node', code: 'MODULE_NOT_FOUND', category: 'missing-module' });

    return errors;
  }

  categorizeRustError(code) {
    const categories = {
      'E0308': 'type-mismatch',
      'E0433': 'missing-import',
      'E0412': 'undefined-type',
      'E0425': 'undefined-value',
      'E0599': 'missing-method',
      'E0277': 'trait-not-implemented',
      'E0382': 'use-after-move',
      'E0502': 'borrow-conflict',
      'E0507': 'cannot-move-out',
      'E0515': 'return-local-reference'
    };
    return categories[code] || 'other';
  }

  categorizeTsError(code) {
    const categories = {
      'TS2304': 'undefined-name',
      'TS2322': 'type-mismatch',
      'TS2339': 'missing-property',
      'TS2345': 'argument-type',
      'TS2769': 'overload-mismatch'
    };
    return categories[code] || 'other';
  }

  /**
   * Record an error occurrence
   */
  recordError(command, stderr, file = null, crate = null) {
    const errors = this.parseError(stderr);
    const timestamp = new Date().toISOString();

    for (const error of errors) {
      const key = `${error.type}:${error.code}`;
      if (!this.data.patterns[key]) {
        this.data.patterns[key] = { count: 0, category: error.category, contexts: [], lastSeen: null };
      }
      this.data.patterns[key].count++;
      this.data.patterns[key].lastSeen = timestamp;
      if (crate && !this.data.patterns[key].contexts.includes(crate)) {
        this.data.patterns[key].contexts.push(crate);
      }
    }

    // Store recent errors for sequence detection
    if (errors.length > 0) {
      this.data.recentErrors.push({ errors, command, file, crate, timestamp });
      if (this.data.recentErrors.length > 100) {
        this.data.recentErrors = this.data.recentErrors.slice(-100);
      }
    }

    this.save();
    return errors;
  }

  /**
   * Record a successful fix for an error pattern
   */
  recordFix(errorCode, fixDescription) {
    if (!this.data.fixes[errorCode]) {
      this.data.fixes[errorCode] = [];
    }
    this.data.fixes[errorCode].push({
      fix: fixDescription,
      timestamp: new Date().toISOString()
    });
    // Keep last 5 fixes per error
    if (this.data.fixes[errorCode].length > 5) {
      this.data.fixes[errorCode] = this.data.fixes[errorCode].slice(-5);
    }
    this.save();
  }

  /**
   * Suggest fixes for an error code
   */
  suggestFix(errorCode) {
    const fixes = this.data.fixes[errorCode] || [];
    const pattern = this.data.patterns[errorCode];

    return {
      errorCode,
      category: pattern?.category || 'unknown',
      occurrences: pattern?.count || 0,
      commonContexts: pattern?.contexts?.slice(0, 3) || [],
      recentFixes: fixes.slice(-3).map(f => f.fix)
    };
  }

  getStats() {
    const totalErrors = Object.values(this.data.patterns).reduce((s, p) => s + p.count, 0);
    const topErrors = Object.entries(this.data.patterns)
      .sort((a, b) => b[1].count - a[1].count)
      .slice(0, 5)
      .map(([code, p]) => ({ code, count: p.count, category: p.category }));

    return { totalErrors, topErrors, fixesRecorded: Object.keys(this.data.fixes).length };
  }
}

export { ErrorPatternTracker };
