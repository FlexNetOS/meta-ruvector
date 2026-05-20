#!/usr/bin/env node
/* Slice 6b scanner — restricted to crates/rvAgent/** */
'use strict';

const fs = require('fs');
const path = require('path');
const { execSync, spawnSync } = require('child_process');

const PROJECT_ROOT = process.argv[2];
const OUT_PATH = process.argv[3];

if (!PROJECT_ROOT || !OUT_PATH) {
  console.error('Usage: node ua-project-scan-slice6b.js <project-root> <out-json>');
  process.exit(1);
}

// ---------------- File discovery ----------------
function gitLsFiles(root) {
  try {
    const out = execSync('git ls-files', { cwd: root, encoding: 'utf8', maxBuffer: 64 * 1024 * 1024 });
    return out.split('\n').filter(Boolean);
  } catch (e) {
    console.error('git ls-files failed:', e.message);
    return null;
  }
}

let allFiles = gitLsFiles(PROJECT_ROOT) || [];

// Slice 6b scope: only crates/rvAgent/**
const SLICE_PREFIX = 'crates/rvAgent/';
allFiles = allFiles.filter(p => p.startsWith(SLICE_PREFIX));

// ---------------- Load .understandignore via ignore pkg ----------------
let ignoreFilter = null;
try {
  const ignorePkgPath = path.join(PROJECT_ROOT, 'node_modules', 'ignore');
  let ignoreModule;
  if (fs.existsSync(ignorePkgPath)) {
    ignoreModule = require(ignorePkgPath);
  } else {
    // try to require globally; fall back to manual matching
    try { ignoreModule = require('ignore'); } catch (_) { ignoreModule = null; }
  }
  if (ignoreModule) {
    const ig = ignoreModule();
    const candidatePaths = [
      path.join(PROJECT_ROOT, '.understand-anything', '.understandignore'),
      path.join(PROJECT_ROOT, '.understandignore'),
    ];
    for (const p of candidatePaths) {
      if (fs.existsSync(p)) {
        ig.add(fs.readFileSync(p, 'utf8'));
      }
    }
    ignoreFilter = ig;
  }
} catch (e) {
  console.error('ignore package load failed (continuing with hardcoded filtering):', e.message);
}

// Hardcoded exclusion logic (mirrors spec; used when .understandignore is absent or as default)
const EXCLUDED_DIR_SEGMENTS = new Set([
  'node_modules', '.git', 'vendor', 'venv', '.venv', '__pycache__',
  'dist', 'build', 'out', 'coverage', '.next', '.cache', '.turbo', 'target', 'obj',
  '.idea', '.vscode',
]);

const EXCLUDED_BASENAMES = new Set([
  'LICENSE', '.gitignore', '.editorconfig', '.prettierrc',
  'package-lock.json', 'yarn.lock', 'pnpm-lock.yaml',
]);

const BINARY_EXTS = new Set([
  '.png', '.jpg', '.jpeg', '.gif', '.svg', '.ico', '.woff', '.woff2', '.ttf', '.eot',
  '.mp3', '.mp4', '.pdf', '.zip', '.tar', '.gz',
]);

function basenameMatchesGenerated(name) {
  if (name.endsWith('.min.js') || name.endsWith('.min.css') || name.endsWith('.map')) return true;
  if (/\.generated\./i.test(name)) return true;
  return false;
}

function isExcludedByDefaults(rel) {
  const parts = rel.split('/');
  for (const seg of parts.slice(0, -1)) {
    if (EXCLUDED_DIR_SEGMENTS.has(seg)) return true;
  }
  const base = parts[parts.length - 1];
  if (EXCLUDED_BASENAMES.has(base)) return true;
  // *.lock (but allow Cargo.lock... wait Cargo.lock IS a lockfile, spec says *.lock — exclude)
  if (base.endsWith('.lock')) return true;
  if (base.startsWith('.eslintrc')) return true;
  const ext = path.extname(base).toLowerCase();
  if (BINARY_EXTS.has(ext)) return true;
  if (ext === '.log') return true;
  if (basenameMatchesGenerated(base)) return true;
  return false;
}

const originalList = allFiles.slice();
const afterDefaults = originalList.filter(p => !isExcludedByDefaults(p));
let filteredFiles;
let filteredByIgnore = 0;

if (ignoreFilter) {
  const afterIgnore = ignoreFilter.filter(originalList);
  filteredFiles = afterIgnore;
  filteredByIgnore = Math.max(0, afterDefaults.length - afterIgnore.length);
} else {
  filteredFiles = afterDefaults;
}

// ---------------- Language / category detection ----------------
const EXT_TO_LANG = {
  '.ts': 'typescript', '.tsx': 'typescript',
  '.js': 'javascript', '.jsx': 'javascript',
  '.py': 'python',
  '.go': 'go',
  '.rs': 'rust',
  '.java': 'java',
  '.rb': 'ruby',
  '.cpp': 'cpp', '.cc': 'cpp', '.cxx': 'cpp', '.h': 'cpp', '.hpp': 'cpp',
  '.c': 'c',
  '.cs': 'csharp',
  '.swift': 'swift',
  '.kt': 'kotlin',
  '.php': 'php',
  '.vue': 'vue',
  '.svelte': 'svelte',
  '.sh': 'shell', '.bash': 'shell',
  '.ps1': 'powershell',
  '.bat': 'batch', '.cmd': 'batch',
  '.md': 'markdown', '.rst': 'markdown',
  '.yaml': 'yaml', '.yml': 'yaml',
  '.json': 'json',
  '.jsonc': 'jsonc',
  '.toml': 'toml',
  '.sql': 'sql',
  '.graphql': 'graphql', '.gql': 'graphql',
  '.proto': 'protobuf',
  '.tf': 'terraform', '.tfvars': 'terraform',
  '.html': 'html', '.htm': 'html',
  '.css': 'css', '.scss': 'css', '.sass': 'css', '.less': 'css',
  '.xml': 'xml',
  '.cfg': 'config', '.ini': 'config', '.env': 'config',
};

function detectLanguage(rel) {
  const base = path.basename(rel);
  if (base === 'Dockerfile') return 'dockerfile';
  if (base === 'Makefile') return 'makefile';
  if (base === 'Jenkinsfile') return 'jenkinsfile';
  const ext = path.extname(base).toLowerCase();
  if (EXT_TO_LANG[ext]) return EXT_TO_LANG[ext];
  if (!ext) return 'unknown';
  return ext.slice(1).toLowerCase();
}

function detectCategory(rel) {
  const base = path.basename(rel);
  const ext = path.extname(base).toLowerCase();
  const lowerRel = rel.toLowerCase();

  // infra (most specific first)
  if (base === 'Dockerfile' || /^docker-compose(\..+)?$/i.test(base)) return 'infra';
  if (base === 'Makefile' || base === 'Jenkinsfile' || base === 'Procfile' || base === 'Vagrantfile') return 'infra';
  if (ext === '.tf' || ext === '.tfvars') return 'infra';
  if (lowerRel.startsWith('.github/workflows/')) return 'infra';
  if (base === '.gitlab-ci.yml') return 'infra';
  if (lowerRel.startsWith('.circleci/')) return 'infra';
  if (/\.k8s\.(yaml|yml)$/i.test(base)) return 'infra';
  if (lowerRel.includes('/k8s/') || lowerRel.startsWith('k8s/')) return 'infra';
  if (lowerRel.includes('/kubernetes/') || lowerRel.startsWith('kubernetes/')) return 'infra';

  // docs
  if (ext === '.md' || ext === '.rst') return 'docs';
  if (ext === '.txt' && base !== 'LICENSE') return 'docs';

  // config
  if (['.yaml', '.yml', '.json', '.jsonc', '.toml', '.xml', '.cfg', '.ini', '.env'].includes(ext)) return 'config';
  if (['tsconfig.json', 'package.json', 'pyproject.toml', 'Cargo.toml', 'go.mod'].includes(base)) return 'config';

  // data
  if (['.sql', '.graphql', '.gql', '.proto', '.prisma', '.csv'].includes(ext)) return 'data';
  if (/\.schema\.json$/i.test(base)) return 'data';

  // script
  if (['.sh', '.bash', '.ps1', '.bat'].includes(ext)) return 'script';

  // markup
  if (['.html', '.htm', '.css', '.scss', '.sass', '.less'].includes(ext)) return 'markup';

  // default code
  return 'code';
}

// ---------------- Line counts (batched wc -l) ----------------
function batchLineCounts(files, root) {
  const map = new Map();
  if (files.length === 0) return map;
  const BATCH = 200;
  for (let i = 0; i < files.length; i += BATCH) {
    const batch = files.slice(i, i + BATCH);
    const args = ['-l', ...batch];
    const r = spawnSync('wc', args, { cwd: root, encoding: 'utf8', maxBuffer: 32 * 1024 * 1024 });
    if (r.status !== 0) {
      // Fall back: zero out
      for (const f of batch) map.set(f, 0);
      continue;
    }
    const lines = r.stdout.trim().split('\n');
    for (const line of lines) {
      const m = line.trim().match(/^(\d+)\s+(.+)$/);
      if (!m) continue;
      const cnt = parseInt(m[1], 10);
      const fpath = m[2];
      if (fpath === 'total') continue;
      map.set(fpath, cnt);
    }
  }
  return map;
}

const lineMap = batchLineCounts(filteredFiles, PROJECT_ROOT);

// ---------------- Frameworks ----------------
const frameworks = new Set();

function readFileSafe(rel) {
  try { return fs.readFileSync(path.join(PROJECT_ROOT, rel), 'utf8'); } catch (_) { return null; }
}

// Scan top-level Cargo.toml(s) inside scope for crate framework hints
const RUST_FW_KEYWORDS = {
  'actix-web': 'Actix Web',
  'axum': 'Axum',
  'rocket': 'Rocket',
  'diesel': 'Diesel',
  'tokio': 'Tokio',
  'serde': 'Serde',
  'warp': 'Warp',
};

for (const f of filteredFiles) {
  if (path.basename(f) === 'Cargo.toml') {
    const txt = readFileSafe(f);
    if (!txt) continue;
    for (const [kw, label] of Object.entries(RUST_FW_KEYWORDS)) {
      const re = new RegExp('^\\s*' + kw.replace(/[-\/\\^$*+?.()|[\]{}]/g, '\\$&') + '\\s*=', 'm');
      if (re.test(txt)) frameworks.add(label);
      // also under [dependencies] block w/ workspace = true patterns
    }
    // Note: workspace deps come from root Cargo.toml. We'll also peek at root.
  }
}

// Also peek at the project root Cargo.toml for workspace dependencies (rvAgent inherits from workspace)
const rootCargo = readFileSafe('Cargo.toml');
if (rootCargo) {
  for (const [kw, label] of Object.entries(RUST_FW_KEYWORDS)) {
    const re = new RegExp('^\\s*' + kw.replace(/[-\/\\^$*+?.()|[\]{}]/g, '\\$&') + '\\s*=', 'm');
    if (re.test(rootCargo)) frameworks.add(label);
  }
}

// Detect Dockerfile / docker-compose / CI within scope
for (const f of filteredFiles) {
  const base = path.basename(f);
  if (base === 'Dockerfile') frameworks.add('Docker');
  if (/^docker-compose(\..+)?$/i.test(base)) frameworks.add('Docker Compose');
  if (f.endsWith('.tf')) frameworks.add('Terraform');
  if (f.startsWith('.github/workflows/') && (f.endsWith('.yml') || f.endsWith('.yaml'))) frameworks.add('GitHub Actions');
  if (base === '.gitlab-ci.yml') frameworks.add('GitLab CI');
  if (base === 'Jenkinsfile') frameworks.add('Jenkins');
}

// ---------------- Project name ----------------
let projectName = '';
// Prefer crates/rvAgent root Cargo.toml? The crate itself has no top-level Cargo.toml — it's a parent directory.
// Use the directory name as the project name for slice purposes.
projectName = 'rvAgent';
// But if a sub Cargo.toml says rvagent-core or similar we don't override.

// Description sources
let rawDescription = '';
let readmeHead = '';
const rvAgentReadme = readFileSafe('crates/rvAgent/README.md');
if (rvAgentReadme) {
  readmeHead = rvAgentReadme.split('\n').slice(0, 10).join('\n');
}

// ---------------- Build file records ----------------
const fileRecords = filteredFiles.map(p => ({
  path: p,
  language: detectLanguage(p),
  sizeLines: lineMap.get(p) || 0,
  fileCategory: detectCategory(p),
})).sort((a, b) => a.path.localeCompare(b.path));

const languages = Array.from(new Set(fileRecords.map(r => r.language))).sort();

const total = fileRecords.length;
let complexity = 'small';
if (total > 500) complexity = 'very-large';
else if (total > 150) complexity = 'large';
else if (total > 30) complexity = 'moderate';

// ---------------- Import resolution ----------------
// For Rust use crate::/super::/mod x within the crate; for shell/md/toml -> empty array.

const fileSet = new Set(fileRecords.map(r => r.path));

function resolveRustImports(rel) {
  // Determine crate root: the Cargo.toml directory.
  // For files under crates/rvAgent/<subcrate>/src/...,
  //   subcrate root = crates/rvAgent/<subcrate>
  //   crate src root = crates/rvAgent/<subcrate>/src
  const m = rel.match(/^(crates\/rvAgent\/[^\/]+)\/(.+)$/);
  if (!m) return [];
  const crateDir = m[1];
  const srcRoot = crateDir + '/src';
  const rest = m[2];
  if (!rest.startsWith('src/')) return [];
  // current module path: from src/ onwards
  const fileSrcPath = rest; // e.g. src/foo/bar.rs
  const fileDir = path.posix.dirname(fileSrcPath); // e.g. src/foo
  const txt = readFileSafe(rel);
  if (!txt) return [];

  const imports = new Set();

  // mod x; declarations within a module file -> resolve to:
  //   <fileDir>/x.rs   OR  <fileDir>/x/mod.rs
  const modRe = /^\s*(?:pub\s+(?:\([^)]*\)\s+)?)?mod\s+([a-zA-Z_][a-zA-Z0-9_]*)\s*;/gm;
  let mm;
  while ((mm = modRe.exec(txt)) !== null) {
    const name = mm[1];
    const baseDir = path.posix.basename(fileSrcPath) === 'lib.rs' || path.posix.basename(fileSrcPath) === 'main.rs' || path.posix.basename(fileSrcPath) === 'mod.rs'
      ? fileDir
      : (() => {
          // For a non-mod.rs file foo.rs, child modules live under <dir>/foo/...
          const stem = path.posix.basename(fileSrcPath, '.rs');
          return path.posix.join(fileDir, stem);
        })();
    const candidates = [
      `${crateDir}/${baseDir}/${name}.rs`,
      `${crateDir}/${baseDir}/${name}/mod.rs`,
    ];
    for (const c of candidates) {
      if (fileSet.has(c)) { imports.add(c); break; }
    }
  }

  // use crate::a::b::c   or  use crate::{a, b::c, ...}
  // We'll extract crate::PATH bits.
  // Match `use crate::SOMETHING;` and bracket forms.
  const useStmtRe = /use\s+([^;]+);/g;
  let um;
  while ((um = useStmtRe.exec(txt)) !== null) {
    const body = um[1];
    // Split on commas at top level for grouped imports.
    // Find each `crate::PATH` or `super::PATH` segment
    // For simplicity: scan all crate:: paths.
    const crateRe = /\bcrate::([a-zA-Z_][a-zA-Z0-9_]*(?:::[a-zA-Z_][a-zA-Z0-9_]*)*)/g;
    let cm;
    while ((cm = crateRe.exec(body)) !== null) {
      const segs = cm[1].split('::');
      // Try progressively shorter prefixes as module paths
      for (let len = segs.length; len > 0; len--) {
        const sub = segs.slice(0, len).join('/');
        const cand1 = `${crateDir}/src/${sub}.rs`;
        const cand2 = `${crateDir}/src/${sub}/mod.rs`;
        if (fileSet.has(cand1)) { imports.add(cand1); break; }
        if (fileSet.has(cand2)) { imports.add(cand2); break; }
      }
    }

    // super:: resolution
    const superRe = /\bsuper::([a-zA-Z_][a-zA-Z0-9_]*(?:::[a-zA-Z_][a-zA-Z0-9_]*)*)/g;
    let sm;
    while ((sm = superRe.exec(body)) !== null) {
      const segs = sm[1].split('::');
      // current module path
      let baseModSegs;
      const bn = path.posix.basename(fileSrcPath);
      if (bn === 'lib.rs' || bn === 'main.rs') {
        baseModSegs = [];
      } else if (bn === 'mod.rs') {
        // module is fileDir relative to src/
        const rel2 = fileDir.replace(/^src\/?/, '');
        baseModSegs = rel2 ? rel2.split('/') : [];
      } else {
        const stem = path.posix.basename(fileSrcPath, '.rs');
        const rel2 = fileDir.replace(/^src\/?/, '');
        const parts = rel2 ? rel2.split('/') : [];
        parts.push(stem);
        baseModSegs = parts;
      }
      // super = parent
      const parent = baseModSegs.slice(0, -1);
      const target = parent.concat(segs);
      for (let len = target.length; len > 0; len--) {
        const sub = target.slice(0, len).join('/');
        const cand1 = `${crateDir}/src/${sub}.rs`;
        const cand2 = `${crateDir}/src/${sub}/mod.rs`;
        if (fileSet.has(cand1)) { imports.add(cand1); break; }
        if (fileSet.has(cand2)) { imports.add(cand2); break; }
      }
    }
  }

  // Remove self-reference
  imports.delete(rel);
  return Array.from(imports).sort();
}

const importMap = {};
for (const rec of fileRecords) {
  if (rec.fileCategory !== 'code') { importMap[rec.path] = []; continue; }
  if (rec.language === 'rust') {
    importMap[rec.path] = resolveRustImports(rec.path);
  } else {
    importMap[rec.path] = [];
  }
}

// ---------------- Output ----------------
const result = {
  scriptCompleted: true,
  name: projectName,
  rawDescription,
  readmeHead,
  languages,
  frameworks: Array.from(frameworks).sort(),
  files: fileRecords,
  totalFiles: fileRecords.length,
  filteredByIgnore,
  estimatedComplexity: complexity,
  importMap,
};

fs.mkdirSync(path.dirname(OUT_PATH), { recursive: true });
fs.writeFileSync(OUT_PATH, JSON.stringify(result, null, 2));
console.error(`Wrote ${result.totalFiles} files to ${OUT_PATH}`);
