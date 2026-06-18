/**
 * Tests for the AgentDB Memory Patterns capabilities (cycle 8):
 * session memory, long-term facts, consolidation, hierarchical organization —
 * plus their exposure on the RuVectorIntelligence aggregate.
 *
 * Deterministic & non-destructive: every fixture starts from an empty in-memory
 * state with disk persistence stubbed out, so the test never reads or mutates
 * the persisted data files.
 *
 * Run: node .claude/intelligence/test/memory-patterns.test.mjs
 */
import assert from 'node:assert';
import { VectorMemory, MemoryPatterns, RuVectorIntelligence } from '../index.js';

let pass = 0, fail = 0;
const ok = (n) => { console.log('  ✓', n); pass++; };
const bad = (n, e) => { console.log('  ✗', n, e?.message ?? ''); fail++; };
const t = (n, fn) => { try { const r = fn(); if (r instanceof Promise) return r.then(() => ok(n), (e) => bad(n, e)); ok(n); } catch (e) { bad(n, e); } };

// Isolated VectorMemory: empty, never touches disk, no native db.
function freshMemory() {
  const vm = new VectorMemory({ hyperbolic: false });
  vm.memories = [];
  vm.saveMemories = () => {};
  return vm;
}

function freshPatterns() {
  const vm = freshMemory();
  const mp = new MemoryPatterns(vm);
  mp.facts = {};
  mp.saveFacts = () => {};
  return { vm, mp };
}

console.log('== memory-patterns (session / facts / consolidate / organize) ==');

await (async () => {
  // --- session memory ---
  await t('storeMessage assigns increasing seq', async () => {
    const { mp } = freshPatterns();
    await mp.storeMessage('s1', 'user', 'hello');
    await mp.storeMessage('s1', 'assistant', 'hi there');
    const h = mp.getSessionHistory('s1');
    assert.strictEqual(h.length, 2);
    assert.strictEqual(h[0].metadata.seq, 0);
    assert.strictEqual(h[1].metadata.seq, 1);
    assert.strictEqual(h[0].metadata.role, 'user');
  });

  await t('getSessionHistory is chronological and respects limit', async () => {
    const { mp } = freshPatterns();
    for (let i = 0; i < 5; i++) await mp.storeMessage('s1', 'user', `m${i}`);
    const last2 = mp.getSessionHistory('s1', 2);
    assert.strictEqual(last2.length, 2);
    assert.strictEqual(last2[0].content, 'm3');
    assert.strictEqual(last2[1].content, 'm4');
  });

  await t('sessions are isolated from each other', async () => {
    const { mp } = freshPatterns();
    await mp.storeMessage('a', 'user', 'in a');
    await mp.storeMessage('b', 'user', 'in b');
    assert.strictEqual(mp.getSessionHistory('a').length, 1);
    assert.strictEqual(mp.getSessionHistory('b').length, 1);
    assert.strictEqual(mp.getSessionHistory('a')[0].content, 'in a');
  });

  await t('storeMessage requires a sessionId', async () => {
    const { mp } = freshPatterns();
    await assert.rejects(() => mp.storeMessage('', 'user', 'x'), /sessionId/);
  });

  await t('listSessions counts turns per session', async () => {
    const { mp } = freshPatterns();
    await mp.storeMessage('a', 'user', '1');
    await mp.storeMessage('a', 'assistant', '2');
    await mp.storeMessage('b', 'user', '1');
    const sessions = mp.listSessions().sort((x, y) => x.sessionId.localeCompare(y.sessionId));
    assert.strictEqual(sessions.length, 2);
    assert.strictEqual(sessions[0].turns, 2);
    assert.strictEqual(sessions[1].turns, 1);
  });

  // --- long-term facts ---
  t('storeFact creates a fact retrievable by key', () => {
    const { mp } = freshPatterns();
    mp.storeFact('user_preference', 'language', 'English', { confidence: 1.0 });
    const f = mp.getFact('user_preference', 'language');
    assert.strictEqual(f.value, 'English');
    assert.strictEqual(f.confidence, 1.0);
    assert.strictEqual(f.reinforced, 1);
  });

  t('storeFact upsert updates value/confidence, bumps reinforced, keeps createdAt', () => {
    const { mp } = freshPatterns();
    mp.storeFact('user_preference', 'language', 'English', { confidence: 0.6 });
    const created = mp.getFact('user_preference', 'language').createdAt;
    mp.storeFact('user_preference', 'language', 'French', { confidence: 0.9 });
    const f = mp.getFact('user_preference', 'language');
    assert.strictEqual(f.value, 'French');
    assert.strictEqual(f.confidence, 0.9);
    assert.strictEqual(f.reinforced, 2);
    assert.strictEqual(f.createdAt, created);
  });

  t('getFacts filters by category', () => {
    const { mp } = freshPatterns();
    mp.storeFact('pref', 'lang', 'en');
    mp.storeFact('pref', 'tz', 'UTC');
    mp.storeFact('skill', 'rust', 'expert');
    assert.strictEqual(mp.getFacts('pref').length, 2);
    assert.strictEqual(mp.getFacts().length, 3);
  });

  t('forgetFact removes a fact and reports outcome', () => {
    const { mp } = freshPatterns();
    mp.storeFact('pref', 'lang', 'en');
    assert.strictEqual(mp.forgetFact('pref', 'lang'), true);
    assert.strictEqual(mp.getFact('pref', 'lang'), null);
    assert.strictEqual(mp.forgetFact('pref', 'lang'), false);
  });

  t('storeFact requires category and key', () => {
    const { mp } = freshPatterns();
    assert.throws(() => mp.storeFact('', 'k', 'v'), /category and key/);
  });

  // --- consolidation ---
  t('consolidate prunes memories below minScore', () => {
    const { vm, mp } = freshPatterns();
    const old = new Date(Date.now() - 365 * 24 * 60 * 60 * 1000).toISOString();
    const now = new Date().toISOString();
    vm.memories = [
      { id: '1', type: 'note', content: 'old', metadata: { timestamp: old, confidence: 0 } },
      { id: '2', type: 'note', content: 'fresh', metadata: { timestamp: now, confidence: 1 } },
    ];
    const r = mp.consolidate({ minScore: 0.4 });
    assert.strictEqual(r.before, 2);
    assert.strictEqual(r.after, 1);
    assert.strictEqual(vm.memories[0].id, '2');
  });

  t('consolidate caps survivors at maxSize by score', () => {
    const { vm, mp } = freshPatterns();
    const now = Date.now();
    vm.memories = [0, 1, 2, 3, 4].map(i => ({
      id: `${i}`, type: 'note', content: `m${i}`,
      metadata: { timestamp: new Date(now - i * 1000).toISOString(), confidence: 0.5 },
    }));
    const r = mp.consolidate({ maxSize: 3 });
    assert.strictEqual(r.after, 3);
    assert.strictEqual(vm.memories.length, 3);
    // The three most recent (highest recency) survive: ids 0,1,2.
    assert.deepStrictEqual(vm.memories.map(m => m.id).sort(), ['0', '1', '2']);
  });

  t('consolidate recency strategy ignores confidence', () => {
    const { vm, mp } = freshPatterns();
    const old = new Date(Date.now() - 365 * 24 * 60 * 60 * 1000).toISOString();
    vm.memories = [
      { id: '1', type: 'note', content: 'old-but-confident', metadata: { timestamp: old, confidence: 1 } },
    ];
    const r = mp.consolidate({ strategy: 'recency', minScore: 0.4 });
    assert.strictEqual(r.after, 0); // pure recency: a year-old item scores ~0
  });

  // --- hierarchical organization ---
  await t('organize returns immediate/shortTerm/longTerm/semantic layers', async () => {
    const { mp } = freshPatterns();
    for (let i = 0; i < 12; i++) await mp.storeMessage('s1', 'user', `turn ${i}`);
    mp.storeFact('pref', 'lang', 'en');
    const ctx = await mp.organize('turn', { sessionId: 's1', immediateN: 3, semanticK: 4 });
    assert.strictEqual(ctx.immediate.length, 3);
    assert.strictEqual(ctx.shortTerm.length, 12);
    assert.strictEqual(ctx.longTerm.length, 1);
    assert.ok(Array.isArray(ctx.semantic));
    assert.ok(ctx.semantic.length <= 4);
  });

  await t('organize without sessionId yields empty session layers', async () => {
    const { mp } = freshPatterns();
    mp.storeFact('pref', 'lang', 'en');
    const ctx = await mp.organize('', {});
    assert.strictEqual(ctx.immediate.length, 0);
    assert.strictEqual(ctx.shortTerm.length, 0);
    assert.strictEqual(ctx.longTerm.length, 1);
    assert.strictEqual(ctx.semantic.length, 0);
  });

  // --- stats ---
  await t('getStats reports sessions, messages, facts, categories', async () => {
    const { mp } = freshPatterns();
    await mp.storeMessage('s1', 'user', 'hi');
    mp.storeFact('pref', 'lang', 'en');
    const s = mp.getStats();
    assert.strictEqual(s.sessions, 1);
    assert.strictEqual(s.sessionMessages, 1);
    assert.strictEqual(s.facts, 1);
    assert.deepStrictEqual(s.factCategories, ['pref']);
  });

  // --- aggregate delegation ---
  await t('RuVectorIntelligence delegates the memory-pattern API', async () => {
    const intel = new RuVectorIntelligence({ hyperbolic: false });
    intel.memory.memories = [];
    intel.memory.saveMemories = () => {};
    intel.memoryPatterns.facts = {};
    intel.memoryPatterns.saveFacts = () => {};
    intel.initialized = true; // skip native init for determinism

    await intel.storeMessage('s1', 'user', 'hello');
    assert.strictEqual(intel.getSessionHistory('s1').length, 1);
    assert.strictEqual(intel.listSessions().length, 1);

    intel.storeFact('pref', 'lang', 'en', { confidence: 0.8 });
    assert.strictEqual(intel.getFact('pref', 'lang').value, 'en');
    assert.strictEqual(intel.getFacts('pref').length, 1);

    const ctx = await intel.organizeMemory('hello', { sessionId: 's1' });
    assert.strictEqual(ctx.shortTerm.length, 1);

    const r = intel.consolidateMemory({ maxSize: 100 });
    assert.strictEqual(r.before, 1);

    assert.ok(intel.stats().memoryPatterns);
    assert.strictEqual(intel.forgetFact('pref', 'lang'), true);
  });
})();

console.log(`\n${pass} passed, ${fail} failed`);
process.exit(fail === 0 ? 0 : 1);
