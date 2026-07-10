#!/usr/bin/env node
/**
 * Claude Flow Agent Router
 * Routes tasks to optimal agents based on learned patterns
 */

const AGENT_CAPABILITIES = {
  coder: ['code-generation', 'refactoring', 'debugging', 'implementation'],
  tester: ['unit-testing', 'integration-testing', 'coverage', 'test-generation'],
  reviewer: ['code-review', 'security-audit', 'quality-check', 'best-practices'],
  researcher: ['web-search', 'documentation', 'analysis', 'summarization'],
  architect: ['system-design', 'architecture', 'patterns', 'scalability'],
  'backend-dev': ['api', 'database', 'server', 'authentication'],
  'frontend-dev': ['ui', 'react', 'css', 'components'],
  devops: ['ci-cd', 'docker', 'deployment', 'infrastructure'],
};

const TASK_PATTERNS = {
  // Code patterns
  'implement|create|build|add|write code': 'coder',
  'test|spec|coverage|unit test|integration': 'tester',
  'review|audit|check|validate|security': 'reviewer',
  'research|find|search|documentation|explore': 'researcher',
  'design|architect|structure|plan': 'architect',

  // Domain patterns
  'api|endpoint|server|backend|database': 'backend-dev',
  'ui|frontend|component|react|css|style': 'frontend-dev',
  'deploy|docker|ci|cd|pipeline|infrastructure': 'devops',
};

// ── RuvLTRA tier (FastGRNN, WASM-accelerated) ─────────────────────────────
// Operator directive 2026-07-09: RuvLTRA is installed and proven (complex→sonnet,
// simple→haiku) but the harness routed by keywords only. This adds the real
// complexity classifier via the PINNED agentdb runtime (never bunx @latest),
// fail-open to keyword routing on any error/timeout. Decisions accrue to the
// coordinator cognitive container (.rvf) so `route feedback` can learn.
const RUVLTRA_BIN =
  '/home/flexnetos/lifeos/var/lib/ruvector/runtime/node_modules/.bin/agentdb';
const RUVLTRA_DB =
  '/home/flexnetos/lifeos/var/lib/ruvector/agents/coordinator.rvf.db';

function ruvltraRoute(task) {
  try {
    const { execFileSync } = require('child_process');
    const fs = require('fs');
    if (!fs.existsSync(RUVLTRA_BIN)) return null;
    const out = execFileSync(
      RUVLTRA_BIN,
      ['route', '--prompt', task.slice(0, 2000), '--json'],
      {
        timeout: 2500,
        encoding: 'utf8',
        env: { ...process.env, AGENTDB_FORCE_SQLJS: '1', AGENTDB_PATH: RUVLTRA_DB },
        stdio: ['ignore', 'pipe', 'ignore'],
      }
    );
    const jsonStart = out.indexOf('{');
    const jsonEnd = out.lastIndexOf('}');
    if (jsonStart < 0 || jsonEnd <= jsonStart) return null;
    const d = JSON.parse(out.slice(jsonStart, jsonEnd + 1));
    if (!d || !d.model) return null;
    return {
      modelTier: d.model,
      tierConfidence: d.confidence,
      tierReason: d.reasoning || d.reason || '',
      wasm: d.wasmAccelerated !== false,
    };
  } catch (e) {
    return null; // fail-open: keyword routing still answers
  }
}

// ── RuvLTRA calibration layer (R4, ADR-0004) ──────────────────────────────
// The FastGRNN gates are uncalibrated on this box (near-constant activations;
// T4 fixture: trivial renames → opus, consensus design → haiku). This maps the
// tier through semantic anchor calibration over the R3 MiniLM embedder
// (ruvltra-calibrate.mjs): decisive margin overrides the GRNN tier, mid-band
// (|margin| < tau) keeps the GRNN answer. Fail-open like everything else here.
function ruvltraCalibrate(task) {
  try {
    const { execFileSync } = require('child_process');
    const fs = require('fs');
    const path = require('path');
    const script = path.join(__dirname, 'ruvltra-calibrate.mjs');
    if (!fs.existsSync(script)) return null;
    const out = execFileSync('bun', [script, task.slice(0, 2000)], {
      timeout: 15000,
      encoding: 'utf8',
      stdio: ['ignore', 'pipe', 'ignore'],
    });
    const s = out.indexOf('{');
    const t = out.lastIndexOf('}');
    if (s < 0 || t <= s) return null;
    const d = JSON.parse(out.slice(s, t + 1));
    return d && d.tier ? d : null;
  } catch (e) {
    return null; // fail-open: GRNN/keyword tier stands
  }
}

function routeTask(task) {
  const taskLower = task.toLowerCase();

  // Agent selection: learned keyword patterns (fast, always available).
  let base = null;
  for (const [pattern, agent] of Object.entries(TASK_PATTERNS)) {
    const regex = new RegExp(pattern, 'i');
    if (regex.test(taskLower)) {
      base = { agent, confidence: 0.8, reason: `Matched pattern: ${pattern}` };
      break;
    }
  }
  if (!base) {
    base = {
      agent: 'coder',
      confidence: 0.5,
      reason: 'Default routing - no specific pattern matched',
    };
  }

  // Complexity tier: RuvLTRA FastGRNN (upgrade layer; absent = pure keyword),
  // mapped through the MiniLM anchor calibration (decisive margin overrides the
  // uncalibrated GRNN tier; mid-band keeps the GRNN answer — R4, ADR-0004).
  const tier = ruvltraRoute(task);
  if (tier) {
    const cal = ruvltraCalibrate(task);
    if (cal) {
      base.modelTier = cal.tier;
      base.tierConfidence = Math.min(0.99, Math.abs(cal.margin) * 2);
      base.reason = `${base.reason} | RuvLTRA-cal[minilm margin=${cal.margin}]: ${cal.tier} (grnn said ${tier.modelTier})`.slice(0, 200);
      base.backend = 'ruvltra-fastgrnn+minilm-cal';
    } else {
      base.modelTier = tier.modelTier;
      base.tierConfidence = tier.tierConfidence;
      base.reason = `${base.reason} | RuvLTRA[${tier.wasm ? 'wasm' : 'heuristic'}]: ${tier.modelTier} — ${tier.tierReason}`.slice(0, 200);
      base.backend = tier.wasm ? 'ruvltra-fastgrnn' : 'ruvltra-heuristic';
    }
  } else {
    base.backend = 'keyword-fallback';
  }
  return base;
}

// CLI
const task = process.argv.slice(2).join(' ');

if (task) {
  const result = routeTask(task);
  console.log(JSON.stringify(result, null, 2));
} else {
  console.log('Usage: router.js <task description>');
  console.log('\nAvailable agents:', Object.keys(AGENT_CAPABILITIES).join(', '));
}

module.exports = { routeTask, AGENT_CAPABILITIES, TASK_PATTERNS };
