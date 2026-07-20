/**
 * Tests for the ReasoningBank-with-AgentDB capabilities (cycle 5):
 * verdict judgment, memory distillation, experience replay — plus their
 * exposure on the RuVectorIntelligence aggregate.
 *
 * Deterministic: the bank's state is injected after construction, so the test
 * does not depend on (or mutate) the persisted data files.
 *
 * Run: node .claude/intelligence/test/reasoning-bank.test.mjs
 */
import assert from 'node:assert';
import { ReasoningBank, RuVectorIntelligence } from '../index.js';

let pass = 0, fail = 0;
const ok = (n) => { console.log('  ✓', n); pass++; };
const bad = (n, e) => { console.log('  ✗', n, e?.message ?? ''); fail++; };
const t = (n, fn) => { try { fn(); ok(n); } catch (e) { bad(n, e); } };

console.log('== reasoning-bank (verdict / distill / replay) ==');

// Deterministic fixture.
const bank = new ReasoningBank();
bank.qTable = {
  'rust_lib': { coder: 0.6, tester: 0.2, _meta: { updateCount: 5 } },
  'ts_api': { coder: 0.04, _meta: { updateCount: 1 } }, // below minQ default
  'docs': { writer: 0.3, _meta: { updateCount: 2 } },
};
bank.trajectories = [
  { id: 'a', state: 'rust_lib', action: 'coder', reward: 0.5 },
  { id: 'b', state: 'ts_api', action: 'coder', reward: -0.3 },
  { id: 'c', state: 'docs', action: 'writer', reward: 0.0 },
];

// 1. Verdict judgment.
t('judge(positive number) -> success', () => assert.strictEqual(bank.judge(0.5), 'success'));
t('judge(negative number) -> failure', () => assert.strictEqual(bank.judge(-0.5), 'failure'));
t('judge(0) -> neutral', () => assert.strictEqual(bank.judge(0), 'neutral'));
t('judge(trajectory object) reads .reward', () => assert.strictEqual(bank.judge({ reward: 0.9 }), 'success'));
t('judgeAll tallies all trajectories', () => {
  const v = bank.judgeAll();
  assert.strictEqual(v.total, 3);
  assert.strictEqual(v.success, 1);
  assert.strictEqual(v.failure, 1);
  assert.strictEqual(v.neutral, 1);
});

// 2. Memory distillation.
t('distill returns lessons above the Q floor, ranked', () => {
  const lessons = bank.distill();
  // rust_lib (0.6) and docs (0.3) qualify; ts_api (0.04) is below minQ default (0.1).
  assert.strictEqual(lessons.length, 2);
  assert.strictEqual(lessons[0].state, 'rust_lib'); // highest q first
  assert.strictEqual(lessons[0].action, 'coder');
  assert.strictEqual(lessons[0].verdict, 'success');
  assert.ok(lessons.every(l => l.q >= 0.1));
});
t('distill respects minQ + limit', () => {
  assert.strictEqual(bank.distill({ minQ: 0.5 }).length, 1); // only rust_lib
  assert.strictEqual(bank.distill({ limit: 1 }).length, 1);
});

// 3. Experience replay (in-memory by default; reinforces qTable, no new trajectory).
t('replay returns the count applied and does not append trajectories', () => {
  const before = bank.trajectories.length;
  const qBefore = bank.qTable['rust_lib'].coder;     // 0.6
  const reward = 0.5;                                 // rust_lib/coder trajectory reward
  const applied = bank.replay(3);
  assert.strictEqual(applied, 3);
  assert.strictEqual(bank.trajectories.length, before); // no new trajectories
  // Q moves TOWARD the observed reward (RL update), so it gets closer to it.
  const qAfter = bank.qTable['rust_lib'].coder;
  assert.ok(Math.abs(qAfter - reward) <= Math.abs(qBefore - reward), 'Q did not move toward reward');
});
t('replay clamps to available trajectories', () => {
  assert.strictEqual(bank.replay(999), 3);
});

// 4. Aggregate exposure.
t('aggregate exposes judge/judgeAll/distill/replay', () => {
  const intel = new RuVectorIntelligence();
  for (const m of ['judge', 'judgeAll', 'distill', 'replay']) {
    assert.strictEqual(typeof intel[m], 'function', `missing ${m}`);
  }
  assert.strictEqual(intel.judge(0.2), 'success');
  assert.ok(Array.isArray(intel.distill()));
});

console.log(`== result: ${pass} passed, ${fail} failed ==`);
process.exit(fail ? 1 : 0);
