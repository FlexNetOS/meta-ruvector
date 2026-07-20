import { readFileSync, writeFileSync, existsSync, join, SEQUENCES_FILE } from '../shared-kernel.js';

/**
 * File Sequence Tracker - learns which files are often edited together
 */
class SequenceTracker {
  constructor() {
    this.data = this.load();
    this.sessionEdits = []; // Track edits in current session
  }

  load() {
    if (existsSync(SEQUENCES_FILE)) {
      try { return JSON.parse(readFileSync(SEQUENCES_FILE, 'utf-8')); }
      catch { return { sequences: {}, coedits: {}, testPairs: {} }; }
    }
    return { sequences: {}, coedits: {}, testPairs: {} };
  }

  save() {
    writeFileSync(SEQUENCES_FILE, JSON.stringify(this.data, null, 2));
  }

  /**
   * Record a file edit and learn sequences
   */
  recordEdit(file) {
    const timestamp = Date.now();
    const normalizedFile = this.normalizePath(file);

    // Check for sequence from previous edit
    if (this.sessionEdits.length > 0) {
      const lastEdit = this.sessionEdits[this.sessionEdits.length - 1];
      const timeDiff = timestamp - lastEdit.timestamp;

      // If edited within 5 minutes, consider it a sequence
      if (timeDiff < 5 * 60 * 1000) {
        this.recordSequence(lastEdit.file, normalizedFile);
      }
    }

    // Detect test file pairing
    this.detectTestPair(normalizedFile);

    this.sessionEdits.push({ file: normalizedFile, timestamp });

    // Keep session to last 20 edits
    if (this.sessionEdits.length > 20) {
      this.sessionEdits = this.sessionEdits.slice(-20);
    }

    this.save();
  }

  normalizePath(file) {
    // Normalize to relative path from crates/ or src/
    const match = file.match(/(crates\/[^/]+\/.*|src\/.*|tests\/.*)/);
    return match ? match[1] : file.split('/').slice(-3).join('/');
  }

  recordSequence(from, to) {
    if (from === to) return;

    if (!this.data.sequences[from]) {
      this.data.sequences[from] = {};
    }
    if (!this.data.sequences[from][to]) {
      this.data.sequences[from][to] = { count: 0, lastSeen: null };
    }
    this.data.sequences[from][to].count++;
    this.data.sequences[from][to].lastSeen = new Date().toISOString();

    // Also record as co-edit (bidirectional)
    const pairKey = [from, to].sort().join('|');
    if (!this.data.coedits[pairKey]) {
      this.data.coedits[pairKey] = { count: 0, files: [from, to] };
    }
    this.data.coedits[pairKey].count++;
  }

  detectTestPair(file) {
    // Match source file to test file patterns
    let testFile = null;
    let sourceFile = null;

    if (file.includes('/tests/') || file.includes('.test.') || file.includes('_test.')) {
      testFile = file;
      // Try to find corresponding source
      sourceFile = file
        .replace('/tests/', '/src/')
        .replace('.test.', '.')
        .replace('_test.', '.');
    } else if (file.includes('/src/')) {
      sourceFile = file;
      // Construct potential test file paths
      const ext = file.split('.').pop();
      testFile = file
        .replace('/src/', '/tests/')
        .replace(`.${ext}`, `.test.${ext}`);
    }

    if (testFile && sourceFile) {
      const pairKey = [sourceFile, testFile].sort().join('|');
      if (!this.data.testPairs[pairKey]) {
        this.data.testPairs[pairKey] = { source: sourceFile, test: testFile, editCount: 0 };
      }
      this.data.testPairs[pairKey].editCount++;
    }
  }

  /**
   * Suggest next files based on current file
   */
  suggestNextFiles(currentFile, limit = 3) {
    const normalized = this.normalizePath(currentFile);
    const sequences = this.data.sequences[normalized] || {};

    const suggestions = Object.entries(sequences)
      .sort((a, b) => b[1].count - a[1].count)
      .slice(0, limit)
      .map(([file, data]) => ({
        file,
        probability: Math.min(0.9, data.count / 10),
        timesSequenced: data.count
      }));

    // Also check for test file suggestion
    const testSuggestion = this.suggestTestFile(currentFile);
    if (testSuggestion && !suggestions.find(s => s.file === testSuggestion.file)) {
      suggestions.push(testSuggestion);
    }

    return suggestions.slice(0, limit);
  }

  /**
   * Suggest test file for a source file
   */
  suggestTestFile(sourceFile) {
    const normalized = this.normalizePath(sourceFile);

    // Find matching test pair
    for (const [, pair] of Object.entries(this.data.testPairs)) {
      if (pair.source === normalized || normalized.includes(pair.source)) {
        return {
          file: pair.test,
          type: 'test-file',
          probability: 0.8,
          reason: 'Corresponding test file'
        };
      }
    }

    // Generate test file path if not found
    if (sourceFile.includes('/src/') && !sourceFile.includes('test')) {
      const ext = sourceFile.split('.').pop();
      const testPath = sourceFile
        .replace('/src/', '/tests/')
        .replace(`.${ext}`, ext === 'rs' ? `_test.${ext}` : `.test.${ext}`);
      return {
        file: this.normalizePath(testPath),
        type: 'suggested-test',
        probability: 0.5,
        reason: 'Suggested test location'
      };
    }

    return null;
  }

  /**
   * Suggest running tests after editing source files
   */
  shouldSuggestTests(file) {
    const normalized = this.normalizePath(file);

    // Always suggest tests for Rust source files
    if (file.endsWith('.rs') && file.includes('/src/') && !file.includes('test')) {
      const crateMatch = file.match(/crates\/([^/]+)/);
      const crate = crateMatch ? crateMatch[1] : null;
      return {
        suggest: true,
        command: crate ? `cargo test -p ${crate}` : 'cargo test',
        reason: 'Source file modified'
      };
    }

    // Suggest tests for TypeScript source files
    if ((file.endsWith('.ts') || file.endsWith('.tsx')) && !file.includes('.test.')) {
      return {
        suggest: true,
        command: 'npm test',
        reason: 'TypeScript source modified'
      };
    }

    return { suggest: false };
  }

  getStats() {
    return {
      totalSequences: Object.keys(this.data.sequences).length,
      totalCoedits: Object.keys(this.data.coedits).length,
      testPairs: Object.keys(this.data.testPairs).length,
      sessionEdits: this.sessionEdits.length
    };
  }
}

export { SequenceTracker };
