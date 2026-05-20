#!/usr/bin/env node
// slice-6a-scan.js — Produces fingerprints + project-scan-summary for crates/rvf slice.
// Inputs: project root (argv[2]), output dir (argv[3]).
'use strict';
const fs = require('fs');
const path = require('path');
const crypto = require('crypto');
const { execSync } = require('child_process');

const ROOT = process.argv[2];
const OUTDIR = process.argv[3];
if (!ROOT || !OUTDIR) {
  console.error('Usage: node slice-6a-scan.js <project-root> <out-dir>');
  process.exit(1);
}

const SLICE_FILES_PATH = path.join(ROOT, '.understand-anything', 'tmp', 'slice-6a-files.txt');
let files;
try {
  files = fs.readFileSync(SLICE_FILES_PATH, 'utf-8').split('\n').map(s => s.trim()).filter(Boolean);
} catch (e) {
  console.error('Failed to read slice file list:', e.message);
  process.exit(1);
}
files.sort();

// ---- helpers ----
function sha256(buf) { return crypto.createHash('sha256').update(buf).digest('hex'); }
function safeRead(p) { try { return fs.readFileSync(p, 'utf-8'); } catch { return null; } }
function countLines(s) {
  if (s === null) return 0;
  if (s.length === 0) return 0;
  let n = 0;
  for (let i = 0; i < s.length; i++) if (s.charCodeAt(i) === 10) n++;
  if (s.charCodeAt(s.length - 1) !== 10) n++;
  return n;
}

// ---- language + category mapping ----
function extInfo(rel) {
  const base = path.basename(rel);
  const ext = path.extname(rel).toLowerCase();
  if (base === 'Cargo.toml' || base === 'Cargo.lock') return { language: ext === '.lock' ? 'lockfile' : 'toml', fileCategory: 'config' };
  if (base === 'Dockerfile') return { language: 'dockerfile', fileCategory: 'infra' };
  if (base === 'Makefile') return { language: 'makefile', fileCategory: 'infra' };
  switch (ext) {
    case '.rs': return { language: 'rust', fileCategory: 'code' };
    case '.toml': return { language: 'toml', fileCategory: 'config' };
    case '.md': return { language: 'markdown', fileCategory: 'docs' };
    case '.yaml':
    case '.yml': return { language: 'yaml', fileCategory: 'config' };
    case '.json': return { language: 'json', fileCategory: 'config' };
    case '.sh': return { language: 'shell', fileCategory: 'script' };
    case '.sql': return { language: 'sql', fileCategory: 'data' };
    case '.proto': return { language: 'protobuf', fileCategory: 'data' };
    case '.tf': return { language: 'terraform', fileCategory: 'infra' };
    default: {
      const lang = ext ? ext.slice(1) : 'unknown';
      return { language: lang || 'unknown', fileCategory: 'code' };
    }
  }
}

// ---- rust structural extraction (regex-based; conservative) ----
const RUST_FN_RE = /^[ \t]*(?:pub(?:\([^)]*\))?\s+)?(?:async\s+|const\s+|unsafe\s+|extern(?:\s+"[^"]*")?\s+)*fn\s+([A-Za-z_][A-Za-z0-9_]*)/gm;
const RUST_STRUCT_RE = /^[ \t]*(?:pub(?:\([^)]*\))?\s+)?struct\s+([A-Za-z_][A-Za-z0-9_]*)/gm;
const RUST_ENUM_RE = /^[ \t]*(?:pub(?:\([^)]*\))?\s+)?enum\s+([A-Za-z_][A-Za-z0-9_]*)/gm;
const RUST_TRAIT_RE = /^[ \t]*(?:pub(?:\([^)]*\))?\s+)?(?:unsafe\s+)?trait\s+([A-Za-z_][A-Za-z0-9_]*)/gm;
const RUST_USE_RE = /^[ \t]*use\s+([^;]+);/gm;
const RUST_MOD_RE = /^[ \t]*(?:pub(?:\([^)]*\))?\s+)?mod\s+([A-Za-z_][A-Za-z0-9_]*)\s*;/gm;

function extractRustSymbols(src) {
  const functions = [];
  const classes = [];
  const imports = [];
  let m;
  RUST_FN_RE.lastIndex = 0;
  while ((m = RUST_FN_RE.exec(src)) !== null) {
    functions.push({ name: m[1], kind: 'fn' });
  }
  RUST_STRUCT_RE.lastIndex = 0;
  while ((m = RUST_STRUCT_RE.exec(src)) !== null) {
    classes.push({ name: m[1], kind: 'struct' });
  }
  RUST_ENUM_RE.lastIndex = 0;
  while ((m = RUST_ENUM_RE.exec(src)) !== null) {
    classes.push({ name: m[1], kind: 'enum' });
  }
  RUST_TRAIT_RE.lastIndex = 0;
  while ((m = RUST_TRAIT_RE.exec(src)) !== null) {
    classes.push({ name: m[1], kind: 'trait' });
  }
  RUST_USE_RE.lastIndex = 0;
  while ((m = RUST_USE_RE.exec(src)) !== null) {
    const target = m[1].trim().replace(/\s+/g, ' ');
    imports.push({ raw: target });
  }
  RUST_MOD_RE.lastIndex = 0;
  while ((m = RUST_MOD_RE.exec(src)) !== null) {
    imports.push({ raw: `mod ${m[1]}` });
  }
  // de-dup names
  const dedupBy = (arr, key) => {
    const seen = new Set();
    const out = [];
    for (const item of arr) {
      const k = key(item);
      if (seen.has(k)) continue;
      seen.add(k);
      out.push(item);
    }
    return out;
  };
  return {
    functions: dedupBy(functions, x => x.name),
    classes: dedupBy(classes, x => `${x.kind}:${x.name}`),
    imports: dedupBy(imports, x => x.raw),
  };
}

// ---- crate dependency extraction from Cargo.toml ----
function parseCargoDeps(src) {
  if (!src) return [];
  const deps = new Set();
  const lines = src.split('\n');
  let inDeps = false;
  for (const ln of lines) {
    const t = ln.trim();
    if (/^\[/.test(t)) {
      inDeps = /^\[(dependencies|dev-dependencies|build-dependencies)(\..+)?\]$/i.test(t)
            || /^\[workspace\.dependencies\]$/i.test(t);
      continue;
    }
    if (!inDeps || !t || t.startsWith('#')) continue;
    const m = t.match(/^([A-Za-z0-9_\-]+)\s*=/);
    if (m) deps.add(m[1]);
  }
  return [...deps].sort();
}

// ---- workspace member detection ----
function workspaceMembers(cargoSrc) {
  if (!cargoSrc) return [];
  const m = cargoSrc.match(/\[workspace\][\s\S]*?members\s*=\s*\[([\s\S]*?)\]/);
  if (!m) return [];
  return [...m[1].matchAll(/"([^"]+)"/g)].map(x => x[1]);
}

// ---- file pass ----
const filesOut = {};   // fingerprints.files
const summaryFiles = []; // project-scan files[]
const importMap = {};
const languagesSet = new Set();
const frameworksSet = new Set();
let analyzedRust = 0;
let totalLines = 0;
const crateRoots = new Set();
const cargoDepsAll = new Set();

for (const rel of files) {
  const abs = path.join(ROOT, rel);
  let st;
  try { st = fs.statSync(abs); } catch { continue; }
  if (!st.isFile()) continue;
  const buf = fs.readFileSync(abs);
  const text = buf.toString('utf-8');
  const ln = countLines(text);
  totalLines += ln;
  const meta = extInfo(rel);
  languagesSet.add(meta.language);

  // fingerprints entry (minimal baseline; structural if Rust)
  const fp = {
    filePath: rel,
    contentHash: sha256(buf),
    functions: [],
    classes: [],
    imports: [],
    exports: [],
    totalLines: ln,
    hasStructuralAnalysis: false,
  };

  if (meta.language === 'rust') {
    const sym = extractRustSymbols(text);
    fp.functions = sym.functions;
    fp.classes = sym.classes;
    fp.imports = sym.imports;
    fp.hasStructuralAnalysis = true;
    analyzedRust++;
  }

  filesOut[rel] = fp;
  summaryFiles.push({
    path: rel,
    language: meta.language,
    sizeLines: ln,
    fileCategory: meta.fileCategory,
  });

  // Cargo.toml -> framework/dep collection + crate root tracking
  if (path.basename(rel) === 'Cargo.toml') {
    const deps = parseCargoDeps(text);
    deps.forEach(d => cargoDepsAll.add(d));
    crateRoots.add(path.dirname(rel));
  }
}

// frameworks heuristic from cargo deps
const FRAMEWORK_HINTS = {
  'tokio': 'Tokio',
  'axum': 'Axum',
  'actix-web': 'Actix Web',
  'rocket': 'Rocket',
  'warp': 'Warp',
  'hyper': 'Hyper',
  'serde': 'Serde',
  'sqlx': 'SQLx',
  'diesel': 'Diesel',
  'sea-orm': 'SeaORM',
  'wasm-bindgen': 'wasm-bindgen',
  'wasmtime': 'wasmtime',
  'wasmer': 'wasmer',
  'tracing': 'tracing',
  'clap': 'clap',
  'reqwest': 'reqwest',
  'tonic': 'tonic',
  'prost': 'prost',
  'rayon': 'Rayon',
  'criterion': 'Criterion',
  'tower': 'Tower',
  'redb': 'redb',
  'rocksdb': 'RocksDB',
};
for (const d of cargoDepsAll) {
  if (FRAMEWORK_HINTS[d]) frameworksSet.add(FRAMEWORK_HINTS[d]);
}

// Import resolution: Rust `use crate::*`, `mod foo;`, etc. -- map to in-scope files when possible.
// For each rust file, find sibling files in same crate using mod refs.
// Build crate-root -> file index
const crateFiles = {};
for (const rel of files) {
  // find longest crate root prefix
  let best = '';
  for (const cr of crateRoots) {
    if (rel.startsWith(cr + '/') && cr.length > best.length) best = cr;
  }
  if (!best) continue;
  (crateFiles[best] ||= []).push(rel);
}

function resolveModRef(filePath, modName, allFiles) {
  const dir = path.dirname(filePath);
  const candidates = [
    `${dir}/${modName}.rs`,
    `${dir}/${modName}/mod.rs`,
  ];
  // Also: if file is lib.rs/main.rs/mod.rs, look in same dir
  for (const c of candidates) {
    if (allFiles.includes(c)) return c;
  }
  return null;
}

const allFilesArr = files; // sorted
for (const rel of files) {
  if (!filesOut[rel]) continue;
  const resolved = [];
  if (filesOut[rel].hasStructuralAnalysis) {
    for (const imp of filesOut[rel].imports) {
      // mod foo;
      const mm = imp.raw.match(/^mod\s+([A-Za-z_][A-Za-z0-9_]*)$/);
      if (mm) {
        const r = resolveModRef(rel, mm[1], allFilesArr);
        if (r && !resolved.includes(r)) resolved.push(r);
        continue;
      }
      // use crate::x::y; -- traverse from crate root
      const cm = imp.raw.match(/^crate::([A-Za-z_][A-Za-z0-9_:]*)/);
      if (cm) {
        // Find crate root for this file
        let bestRoot = '';
        for (const cr of crateRoots) {
          if (rel.startsWith(cr + '/') && cr.length > bestRoot.length) bestRoot = cr;
        }
        if (bestRoot) {
          const parts = cm[1].split('::');
          // crate root src dir
          const srcDir = `${bestRoot}/src`;
          // Try sequence of paths
          for (let i = parts.length; i >= 1; i--) {
            const segs = parts.slice(0, i);
            const cand1 = `${srcDir}/${segs.join('/')}.rs`;
            const cand2 = `${srcDir}/${segs.join('/')}/mod.rs`;
            if (allFilesArr.includes(cand1) && !resolved.includes(cand1)) { resolved.push(cand1); break; }
            if (allFilesArr.includes(cand2) && !resolved.includes(cand2)) { resolved.push(cand2); break; }
          }
        }
      }
    }
  }
  importMap[rel] = resolved;
}

// complexity estimate
const fileCount = summaryFiles.length;
let complexity = 'small';
if (fileCount > 500) complexity = 'very-large';
else if (fileCount > 150) complexity = 'large';
else if (fileCount > 30) complexity = 'moderate';

// readme head + raw description
const readmePath = path.join(ROOT, 'crates/rvf/README.md');
let readmeHead = '';
try {
  const r = fs.readFileSync(readmePath, 'utf-8');
  readmeHead = r.split('\n').slice(0, 10).join('\n');
} catch {}

// project name from Cargo.toml? Use rvf
const projectName = 'rvf';

// ---- write outputs ----
// 1. fingerprints slice file
const fingerprintsSlice = {
  version: '1.0.0',
  gitCommitHash: '9054c2cc6793ff11175460694a6479be3ac5b0af',
  generatedAt: new Date().toISOString(),
  slice: '6a',
  scope: 'crates/rvf',
  files: filesOut,
};
fs.writeFileSync(path.join(OUTDIR, '..', 'fingerprints.slice-6a.json'),
  JSON.stringify(fingerprintsSlice, null, 2));

// 2. all-file-paths.json
fs.writeFileSync(path.join(OUTDIR, 'all-file-paths.json'),
  JSON.stringify(files, null, 2));

// 3. project-scan-summary.json (final form, no rawDescription/readmeHead/scriptCompleted)
const summary = {
  name: projectName,
  description: (readmeHead.split('\n').find(l => l.trim() && !l.startsWith('#')) || 'RuVector flow / cognitive-container crate (rvf).').trim(),
  languages: [...languagesSet].sort(),
  frameworks: [...frameworksSet].sort(),
  files: summaryFiles.sort((a, b) => a.path.localeCompare(b.path)),
  totalFiles: summaryFiles.length,
  filteredByIgnore: 0,
  estimatedComplexity: complexity,
  importMap,
  slice: '6a',
  scope: 'crates/rvf',
  cargoDependencies: [...cargoDepsAll].sort(),
  crateRoots: [...crateRoots].sort(),
  analyzedRustFiles: analyzedRust,
  totalLines,
};
if (summary.totalFiles > 100) {
  summary.description += ' Note: this project has over 100 source files; consider scoping analysis to a subdirectory for faster results.';
}
fs.writeFileSync(path.join(OUTDIR, 'project-scan-summary.json'),
  JSON.stringify(summary, null, 2));

// brief stderr report (not captured by hooks)
console.error(`[slice-6a] files=${summary.totalFiles} rust=${analyzedRust} lines=${totalLines} crates=${crateRoots.size} deps=${cargoDepsAll.size} complexity=${complexity}`);
