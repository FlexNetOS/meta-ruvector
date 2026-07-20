/**
 * Characterization test for the RuVector intelligence layer.
 *
 * Pins the PUBLIC interface and observable behavior of intelligence/index.js so the
 * DDD decomposition (god object -> bounded-context modules) can be proven
 * behavior-preserving: this must be green BEFORE and AFTER the refactor.
 *
 * Run: node .claude/intelligence/test/characterization.test.mjs
 */
import assert from 'node:assert';
import * as I from '../index.js';

let pass = 0, fail = 0;
const ok = (n) => { console.log('  ✓', n); pass++; };
const bad = (n, e) => { console.log('  ✗', n, e?.message ?? ''); fail++; };
const t = (n, fn) => { try { fn(); ok(n); } catch (e) { bad(n, e); } };

console.log('== intelligence characterization ==');

// 1. Public interface: the 8 named exports + default must be present and constructable.
const EXPORTS = ['RuVectorIntelligence', 'VectorMemory', 'ReasoningBank', 'NeuralRouter',
  'CalibrationTracker', 'FeedbackLoop', 'ErrorPatternTracker', 'SequenceTracker'];
for (const name of EXPORTS) t(`exports ${name} (function)`, () => assert.strictEqual(typeof I[name], 'function'));
t('default export === RuVectorIntelligence', () => assert.strictEqual(I.default, I.RuVectorIntelligence));

// 2. The aggregate composes the domains and exposes the documented method surface.
let intel;
t('construct RuVectorIntelligence', () => { intel = new I.RuVectorIntelligence(); });
const METHODS = ['init', 'remember', 'recall', 'learn', 'suggest', 'route', 'recordCalibration',
  'recordFeedback', 'recordError', 'recordFix', 'suggestFix', 'recordFileEdit',
  'suggestNextFiles', 'shouldSuggestTests', 'stats'];
for (const m of METHODS) t(`aggregate.${m} is a method`, () => assert.strictEqual(typeof intel[m], 'function'));

// 3. stats() returns the documented shape (read-only; no heavy IO mutation).
t('stats() returns the expected keys', () => {
  const s = intel.stats();
  for (const k of ['memory', 'trajectories', 'patterns', 'calibration', 'abTest',
    'adviceValue', 'errorPatterns', 'sequences', 'ruvectorNative']) {
    assert.ok(k in s, `stats missing key: ${k}`);
  }
});

// 4. Behavior: deterministic domain logic that does not require init()/HNSW.
t('suggestFix(unknown) does not throw', () => { intel.suggestFix('E_DOES_NOT_EXIST'); });
t('shouldSuggestTests returns an object', () => {
  assert.strictEqual(typeof intel.shouldSuggestTests('crates/foo/src/lib.rs'), 'object');
});
t('suggestNextFiles returns an array', () => {
  assert.ok(Array.isArray(intel.suggestNextFiles('crates/foo/src/lib.rs')));
});
t('suggest(stringState) returns an object', () => {
  // getBestAction/stateKey expect a string state key.
  const r = intel.suggest('rust', ['coder', 'tester']);
  assert.ok(r && typeof r === 'object');
});

console.log(`== result: ${pass} passed, ${fail} failed ==`);
process.exit(fail ? 1 : 0);
