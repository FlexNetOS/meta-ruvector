// Shared kernel for the RuVector intelligence layer: node builtins, data-file
// constants, optional native deps (@ruvector/core HNSW, hyperbolic-attention WASM),
// and the embedding/distance math reused across the bounded contexts.
// Kept at the intelligence root so import.meta.url-relative paths are unchanged.
import { readFileSync, writeFileSync, existsSync, mkdirSync } from 'fs';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';
import { createHash } from 'crypto';

const __dirname = dirname(fileURLToPath(import.meta.url));
const DATA_DIR = join(__dirname, 'data');
const MEMORY_FILE = join(DATA_DIR, 'memory.json');
const TRAJECTORIES_FILE = join(DATA_DIR, 'trajectories.json');
const PATTERNS_FILE = join(DATA_DIR, 'patterns.json');
const CALIBRATION_FILE = join(DATA_DIR, 'calibration.json');
const FEEDBACK_FILE = join(DATA_DIR, 'feedback.json');
const ERROR_PATTERNS_FILE = join(DATA_DIR, 'error-patterns.json');
const SEQUENCES_FILE = join(DATA_DIR, 'sequences.json');

// Ensure data directory exists
if (!existsSync(DATA_DIR)) {
  mkdirSync(DATA_DIR, { recursive: true });
}

// Try to load @ruvector/core VectorDB
let VectorDB = null;
let ruvectorAvailable = false;

try {
  const ruvector = await import('@ruvector/core');
  // @ruvector/core is a CommonJS NAPI module, so ESM `import` puts its exports under
  // `.default`. The native class is `VectorDb` — NAPI lower-camel-cases the Rust
  // `VectorDB`, and the `export { VectorDb as VectorDB }` in index.d.ts is TYPE-ONLY
  // (no runtime export). The old `ruvector.VectorDB` was undefined on both counts,
  // which is why native HNSW silently stayed on the cosine fallback.
  const core = ruvector.default ?? ruvector;
  VectorDB = core.VectorDb ?? core.VectorDB; // real NAPI name first, then any alias
  ruvectorAvailable = typeof VectorDB === 'function';
  console.error(ruvectorAvailable
    ? '✅ @ruvector/core loaded - using native HNSW vector search (VectorDb)'
    : '⚠️ @ruvector/core loaded but no VectorDb export; using fallback cosine similarity');
} catch (e) {
  console.error('⚠️ @ruvector/core not available, using fallback cosine similarity');
}

// Try to load attention WASM for hyperbolic distance
let attentionWasm = null;
try {
  attentionWasm = await import('../../crates/ruvector-attention-wasm/pkg/ruvector_attention_wasm.js');
  console.error('✅ Hyperbolic attention WASM loaded');
} catch (e) {
  // Hyperbolic not available - use fallback
}

/**
 * Hyperbolic distance in Poincaré ball model
 * Better for hierarchical/tree-like data (crates, packages, file paths)
 */
function poincareDistance(u, v, c = 1.0) {
  const EPS = 1e-7;
  const sqrtC = Math.sqrt(c);

  let normDiffSq = 0, normUSq = 0, normVSq = 0;
  for (let i = 0; i < u.length; i++) {
    const diff = u[i] - (v[i] || 0);
    normDiffSq += diff * diff;
    normUSq += u[i] * u[i];
    normVSq += (v[i] || 0) * (v[i] || 0);
  }

  const lambdaU = 1.0 - c * normUSq;
  const lambdaV = 1.0 - c * normVSq;
  const numerator = 2.0 * c * normDiffSq;
  const denominator = Math.max(EPS, lambdaU * lambdaV);

  const arg = Math.max(1.0, 1.0 + numerator / denominator);
  return (1.0 / sqrtC) * Math.acosh(arg);
}

/**
 * Text to embedding with hierarchical awareness
 */
function textToEmbedding(text, dims = 128) {
  const embedding = new Float32Array(dims).fill(0);
  const normalized = text.toLowerCase().replace(/[^a-z0-9\s]/g, ' ');
  const words = normalized.split(/\s+/).filter(w => w.length > 1);

  const wordFreq = {};
  for (const word of words) {
    wordFreq[word] = (wordFreq[word] || 0) + 1;
  }

  for (const [word, freq] of Object.entries(wordFreq)) {
    const hash = createHash('sha256').update(word).digest();
    const idfWeight = 1 / Math.log(1 + freq);
    for (let i = 0; i < dims; i++) {
      const byteIdx = i % hash.length;
      const val = ((hash[byteIdx] & 0xFF) / 127.5) - 1;
      embedding[i] += val * idfWeight;
    }
  }

  // L2 normalize
  const magnitude = Math.sqrt(embedding.reduce((sum, v) => sum + v * v, 0));
  if (magnitude > 0) {
    for (let i = 0; i < dims; i++) embedding[i] /= magnitude;
  }

  // Scale down to fit in Poincaré ball (|x| < 1)
  const maxNorm = 0.95;
  for (let i = 0; i < dims; i++) embedding[i] *= maxNorm;

  return Array.from(embedding);
}

/**
 * Cosine similarity (fallback)
 */
function cosineSimilarity(a, b) {
  let dot = 0, magA = 0, magB = 0;
  for (let i = 0; i < a.length; i++) {
    dot += a[i] * (b[i] || 0);
    magA += a[i] * a[i];
    magB += (b[i] || 0) * (b[i] || 0);
  }
  return dot / (Math.sqrt(magA) * Math.sqrt(magB) || 1);
}

export {
  readFileSync, writeFileSync, existsSync, mkdirSync, join, createHash,
  DATA_DIR, MEMORY_FILE, TRAJECTORIES_FILE, PATTERNS_FILE, CALIBRATION_FILE,
  FEEDBACK_FILE, ERROR_PATTERNS_FILE, SEQUENCES_FILE,
  poincareDistance, textToEmbedding, cosineSimilarity,
  VectorDB, ruvectorAvailable, attentionWasm,
};
