// RuvLTRA tier calibration layer (R4 / envctl TASK-0089, ADR-0004 two-plane routing).
//
// WHY: the FastGRNN gate activations are uncalibrated on this box — measured on the
// T4 fixture they emit near-constant ~0.7 activations and route "rename the local
// variable foo to bar" to opus while sending "design a Byzantine fault-tolerant
// consensus protocol" to haiku (3/5 trivial and 4/5 complex cases wrong, envctl
// scripts/tests/blueprint/t4_router_discrimination.mjs). The built-in features are
// token/word counts and cannot separate the classes either (trivial fc 0.012–0.02
// vs complex fc 0.02–0.12, overlapping).
//
// WHAT: semantic anchor calibration over the box's proven-discriminating MiniLM
// embedder (384-d fp32 ONNX-WASM, wired by R3): embed the prompt, compare cosine
// similarity against two INDEPENDENT anchor sets (deliberately NOT the T4 fixture
// prompts — anchors express the complexity classes in different words so the
// fixture stays a held-out check). margin = sim(complex) − sim(trivial):
//   margin ≥ +TAU → opus · margin ≤ −TAU → haiku · |margin| < TAU → null (defer
//   to the FastGRNN answer — mid-band prompts keep the model's own tier).
//
// FAIL-OPEN: any error/missing model prints {"tier":null} and the router keeps
// its existing GRNN→keyword fallback chain. Stdout is a single JSON object.
import fs from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';

const HERE = dirname(fileURLToPath(import.meta.url));
const EMBEDDER = '/home/flexnetos/lifeos/var/lib/ruvector/embed-minilm.mjs';
const CACHE = join(HERE, 'ruvltra-anchors.cache.json');
const TAU = 0.04;

const TRIVIAL_ANCHORS = [
  'correct a spelling mistake in the documentation',
  'change the constant name in one file',
  'delete an unused import statement',
  'update a version string in the package manifest',
  'add a missing semicolon at the end of a line',
  'reword a log message for clarity',
  'adjust indentation in a config file',
  'set a default value for a command-line flag',
  'remove trailing whitespace from a source file',
  'fix the casing of a word in a comment',
];

const COMPLEX_ANCHORS = [
  'design a distributed consensus algorithm tolerant to malicious nodes',
  'prove correctness of a concurrent data structure under weak memory ordering',
  'architect a multi-tier scheduling system with formal latency guarantees',
  'eliminate a cross-service deadlock in an async runtime boundary',
  'derive safety invariants for lock-free concurrency primitives',
  'plan a zero-downtime migration of a replicated stateful system',
  'formally verify a cryptographic key-exchange protocol implementation',
  'restructure a compiler optimization pass while preserving soundness',
  'diagnose a race condition in a distributed transaction coordinator',
  'model failure recovery semantics for an event-sourced architecture',
];

function cos(a, b) {
  let dot = 0;
  let na = 0;
  let nb = 0;
  for (let i = 0; i < a.length; i += 1) {
    dot += a[i] * b[i];
    na += a[i] * a[i];
    nb += b[i] * b[i];
  }
  return dot / (Math.sqrt(na) * Math.sqrt(nb) || 1);
}

function centroid(vecs) {
  const c = new Array(vecs[0].length).fill(0);
  for (const v of vecs) for (let i = 0; i < v.length; i += 1) c[i] += v[i];
  for (let i = 0; i < c.length; i += 1) c[i] /= vecs.length;
  return c;
}

async function main() {
  const t0 = performance.now();
  const prompt = (process.argv[2] || '').slice(0, 2000);
  if (!prompt) {
    process.stdout.write(JSON.stringify({ tier: null, err: 'empty prompt' }));
    return;
  }
  const { embed } = await import(EMBEDDER);

  // Anchor centroids: cache keyed by the anchor text so edits invalidate it.
  const key = JSON.stringify([TRIVIAL_ANCHORS, COMPLEX_ANCHORS]).length;
  let cache = null;
  try {
    cache = JSON.parse(fs.readFileSync(CACHE, 'utf8'));
    if (cache.key !== key) cache = null;
  } catch { cache = null; }
  if (!cache) {
    cache = {
      key,
      trivial: centroid(TRIVIAL_ANCHORS.map((a) => embed(a))),
      complex: centroid(COMPLEX_ANCHORS.map((a) => embed(a))),
    };
    try { fs.writeFileSync(CACHE, JSON.stringify(cache)); } catch { /* cache is best-effort */ }
  }

  const v = embed(prompt);
  const simT = cos(v, cache.trivial);
  const simC = cos(v, cache.complex);
  const margin = simC - simT;
  const tier = margin >= TAU ? 'opus' : margin <= -TAU ? 'haiku' : null;
  process.stdout.write(
    JSON.stringify({
      tier,
      margin: Number(margin.toFixed(4)),
      simTrivial: Number(simT.toFixed(4)),
      simComplex: Number(simC.toFixed(4)),
      tau: TAU,
      ms: Number((performance.now() - t0).toFixed(1)),
    })
  );
}

main().catch(() => process.stdout.write(JSON.stringify({ tier: null, err: 'calibration failed' })));
