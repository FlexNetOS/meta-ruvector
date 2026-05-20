#!/usr/bin/env node
/**
 * Slice 10a scanner: scope is crates/ruvix only.
 *
 * Produces:
 *   - slice-10a-all-file-paths.json  (file list, in-scope only)
 *   - slice-10a-scan-summary.json    (languages/frameworks/importMap/complexity)
 *   - fingerprints.slice-10a.json    (slice-scoped fingerprint stub for downstream merge)
 */
'use strict';

const fs = require('fs');
const path = require('path');
const cp = require('child_process');

const projectRoot = process.argv[2];
const outDir = process.argv[3];
if (!projectRoot || !outDir) {
  console.error('usage: ua-scan-slice-10a.js <projectRoot> <outDir>');
  process.exit(1);
}

const SCOPE_PREFIX = 'crates/ruvix/';

// -------- Step 1: discover files via git ls-files, scoped to crates/ruvix --------
let allTracked;
try {
  const raw = cp.execSync('git ls-files crates/ruvix', {
    cwd: projectRoot,
    maxBuffer: 256 * 1024 * 1024,
    encoding: 'utf8',
  });
  allTracked = raw.split('\n').filter(Boolean);
} catch (e) {
  console.error('git ls-files failed:', e.message);
  process.exit(1);
}

// -------- Step 2: hardcoded exclusions (defensive — git ls-files already filters most) --------
const SEGMENT_EXCLUDES = new Set([
  'node_modules', '.git', 'vendor', 'venv', '.venv', '__pycache__',
  'dist', 'build', 'out', 'coverage', '.next', '.cache', '.turbo', 'target', 'obj',
  '.idea', '.vscode',
]);
const BINARY_EXTS = new Set([
  '.png', '.jpg', '.jpeg', '.gif', '.svg', '.ico', '.woff', '.woff2', '.ttf', '.eot',
  '.mp3', '.mp4', '.pdf', '.zip', '.tar', '.gz',
]);
const LOCK_FILES = new Set(['package-lock.json', 'yarn.lock', 'pnpm-lock.yaml', 'Cargo.lock']);
function isExcluded(p) {
  const segs = p.split('/');
  for (const s of segs) if (SEGMENT_EXCLUDES.has(s)) return true;
  const base = segs[segs.length - 1];
  if (LOCK_FILES.has(base)) return true;
  if (base.endsWith('.lock')) return true;
  const ext = path.extname(base).toLowerCase();
  if (BINARY_EXTS.has(ext)) return true;
  if (/\.min\.(js|css)$/.test(base)) return true;
  if (base.endsWith('.map') || base.includes('.generated.')) return true;
  if (base === 'LICENSE' || base === '.gitignore' || base === '.editorconfig') return true;
  if (base.startsWith('.prettierrc') || base.startsWith('.eslintrc')) return true;
  if (base.endsWith('.log')) return true;
  return false;
}

// -------- Step 2.5: .understandignore filter (scope is enforced by it) --------
// The .understandignore at .understand-anything/.understandignore restricts to crates/ruvix/**.
// Since we already used `git ls-files crates/ruvix`, the slice scope is satisfied. We still
// enforce SCOPE_PREFIX defensively below.
const baseFiltered = allTracked.filter((p) => {
  if (!p.startsWith(SCOPE_PREFIX)) return false;
  if (isExcluded(p)) return false;
  return true;
});

// Task spec says 300 candidates: *.rs, *.toml, *.md. Keep that filter as the slice-10a charter.
const SLICE_EXTS = new Set(['.rs', '.toml', '.md']);
const inScope = baseFiltered.filter((p) => SLICE_EXTS.has(path.extname(p).toLowerCase()));

// Track filtered-by-ignore count = (everything under crates/ruvix tracked by git) - inScope.
const allUnderScope = allTracked.filter((p) => p.startsWith(SCOPE_PREFIX));
const filteredByIgnore = allUnderScope.length - inScope.length;

// -------- Step 3: language detection --------
const EXT_LANG = {
  '.rs': 'rust',
  '.toml': 'toml',
  '.md': 'markdown',
};
function detectLang(p) {
  const base = path.basename(p);
  const ext = path.extname(base).toLowerCase();
  if (EXT_LANG[ext]) return EXT_LANG[ext];
  if (base === 'Dockerfile') return 'dockerfile';
  if (base === 'Makefile') return 'makefile';
  if (base === 'Jenkinsfile') return 'jenkinsfile';
  return ext ? ext.slice(1) : 'unknown';
}

// -------- Step 4: file category --------
function fileCategory(p) {
  const base = path.basename(p);
  const ext = path.extname(base).toLowerCase();
  if (ext === '.md' || ext === '.rst' || (ext === '.txt' && base !== 'LICENSE')) return 'docs';
  if (
    ext === '.toml' || ext === '.yaml' || ext === '.yml' ||
    ext === '.json' || ext === '.jsonc' || ext === '.xml' ||
    ext === '.cfg' || ext === '.ini' || ext === '.env' ||
    base === 'Cargo.toml' || base === 'tsconfig.json' || base === 'package.json' ||
    base === 'pyproject.toml' || base === 'go.mod'
  ) return 'config';
  return 'code';
}

// -------- Step 5: line counting (batched) --------
const sizeLines = new Map();
function countLinesBatch(paths) {
  if (paths.length === 0) return;
  // Spawn wc -l with batches to avoid argv overflow.
  const BATCH = 400;
  for (let i = 0; i < paths.length; i += BATCH) {
    const chunk = paths.slice(i, i + BATCH);
    try {
      const out = cp.execFileSync('wc', ['-l', ...chunk], {
        cwd: projectRoot,
        maxBuffer: 64 * 1024 * 1024,
        encoding: 'utf8',
      });
      const lines = out.split('\n').filter(Boolean);
      for (const line of lines) {
        const m = line.match(/^\s*(\d+)\s+(.+)$/);
        if (!m) continue;
        const fpath = m[2];
        if (fpath === 'total') continue;
        sizeLines.set(fpath, parseInt(m[1], 10));
      }
    } catch (e) {
      // fall back to per-file
      for (const fp of chunk) {
        try {
          const buf = fs.readFileSync(path.join(projectRoot, fp), 'utf8');
          sizeLines.set(fp, buf.split('\n').length - (buf.endsWith('\n') ? 1 : 0));
        } catch {
          sizeLines.set(fp, 0);
        }
      }
    }
  }
}
countLinesBatch(inScope);

// -------- Step 6: framework detection (Rust-centric for ruvix) --------
const RUST_FRAMEWORK_KEYWORDS = new Set([
  'actix-web', 'axum', 'rocket', 'diesel', 'tokio', 'serde', 'warp',
]);
const frameworks = new Set();
const cargoTomls = inScope.filter((p) => path.basename(p) === 'Cargo.toml');
for (const ct of cargoTomls) {
  try {
    const txt = fs.readFileSync(path.join(projectRoot, ct), 'utf8');
    for (const kw of RUST_FRAMEWORK_KEYWORDS) {
      const re = new RegExp('(^|[^\\w-])' + kw.replace(/[.*+?^${}()|[\\]\\\\]/g, '\\$&') + '\\s*=', 'm');
      if (re.test(txt)) frameworks.add(kw);
    }
  } catch {}
}

// -------- Step 7: project name --------
let projectName = 'ruvix';
try {
  const rootCargo = fs.readFileSync(path.join(projectRoot, 'crates/ruvix/Cargo.toml'), 'utf8');
  const m = rootCargo.match(/^\s*name\s*=\s*"([^"]+)"/m);
  if (m) projectName = m[1];
} catch {}

// -------- Step 8: rawDescription / readmeHead --------
let rawDescription = '';
let readmeHead = '';
try {
  const rootCargo = fs.readFileSync(path.join(projectRoot, 'crates/ruvix/Cargo.toml'), 'utf8');
  const m = rootCargo.match(/^\s*description\s*=\s*"([^"]+)"/m);
  if (m) rawDescription = m[1];
} catch {}
try {
  const readmePath = path.join(projectRoot, 'crates/ruvix/README.md');
  if (fs.existsSync(readmePath)) {
    readmeHead = fs.readFileSync(readmePath, 'utf8').split('\n').slice(0, 10).join('\n');
  }
} catch {}

// -------- Step 9: build files[] sorted --------
inScope.sort();
const files = inScope.map((p) => ({
  path: p,
  language: detectLang(p),
  sizeLines: sizeLines.get(p) ?? 0,
  fileCategory: fileCategory(p),
}));

// -------- Step 10: languages list --------
const languages = Array.from(new Set(files.map((f) => f.language))).sort();

// -------- Step 11: complexity --------
const totalFiles = files.length;
let complexity = 'small';
if (totalFiles > 500) complexity = 'very-large';
else if (totalFiles > 150) complexity = 'large';
else if (totalFiles > 30) complexity = 'moderate';

// -------- Step 12: importMap for Rust code files --------
// Rust intra-crate resolution is non-trivial; we approximate by recording
// `use crate::`, `use super::`, `use self::`, and `mod NAME;` references.
// For each `mod foo;` declaration in file at <dir>/X.rs (or <dir>/X/mod.rs),
// resolve to <dir>/foo.rs or <dir>/foo/mod.rs when present.
const fileSet = new Set(inScope);
const importMap = {};

function resolveMod(currentFile, modName) {
  const dir = path.posix.dirname(currentFile);
  const base = path.posix.basename(currentFile);
  // Determine the "module root" directory of the current file.
  // For lib.rs / main.rs / mod.rs the module dir is `dir`.
  // For other foo.rs files the module dir is `dir/<stem>` (Rust 2018+ allows `<stem>.rs` with sibling `<stem>/` for submods).
  const stem = base.replace(/\.rs$/, '');
  const moduleDirs = [];
  if (base === 'lib.rs' || base === 'main.rs' || base === 'mod.rs') {
    moduleDirs.push(dir);
  } else {
    moduleDirs.push(path.posix.join(dir, stem));
    moduleDirs.push(dir);
  }
  for (const md of moduleDirs) {
    const c1 = path.posix.join(md, modName + '.rs');
    const c2 = path.posix.join(md, modName, 'mod.rs');
    if (fileSet.has(c1)) return c1;
    if (fileSet.has(c2)) return c2;
  }
  return null;
}

for (const f of files) {
  if (f.fileCategory !== 'code') {
    importMap[f.path] = [];
    continue;
  }
  const resolved = new Set();
  let text;
  try {
    text = fs.readFileSync(path.join(projectRoot, f.path), 'utf8');
  } catch {
    importMap[f.path] = [];
    continue;
  }
  // mod foo;  (declares a submodule file)
  const modRe = /^\s*(?:pub(?:\s*\([^)]*\))?\s+)?mod\s+([A-Za-z_][A-Za-z0-9_]*)\s*;/gm;
  let m;
  while ((m = modRe.exec(text)) !== null) {
    const r = resolveMod(f.path, m[1]);
    if (r && r !== f.path) resolved.add(r);
  }
  importMap[f.path] = Array.from(resolved).sort();
}

// -------- assemble outputs --------
const summary = {
  scriptCompleted: true,
  name: projectName,
  rawDescription,
  readmeHead,
  languages,
  frameworks: Array.from(frameworks).sort(),
  files,
  totalFiles,
  filteredByIgnore,
  estimatedComplexity: complexity,
  importMap,
};

if (!fs.existsSync(outDir)) fs.mkdirSync(outDir, { recursive: true });

fs.writeFileSync(
  path.join(outDir, 'slice-10a-scan-summary.json'),
  JSON.stringify(summary, null, 2),
);
fs.writeFileSync(
  path.join(outDir, 'slice-10a-all-file-paths.json'),
  JSON.stringify({ scope: SCOPE_PREFIX, totalFiles, paths: inScope }, null, 2),
);

// slice-scoped fingerprints stub: structured to match other slice-scoped fingerprints
// (parent agent will merge into master). We provide a minimal, deterministic skeleton:
// per-file entry { path, language, sizeLines, fileCategory, sha1 }
const crypto = require('crypto');
const fpEntries = files.map((f) => {
  let sha1 = '';
  try {
    const buf = fs.readFileSync(path.join(projectRoot, f.path));
    sha1 = crypto.createHash('sha1').update(buf).digest('hex');
  } catch {}
  return { ...f, sha1 };
});

fs.writeFileSync(
  path.join(projectRoot, '.understand-anything', 'fingerprints.slice-10a.json'),
  JSON.stringify(
    {
      slice: '10a',
      scope: SCOPE_PREFIX,
      generatedAt: new Date().toISOString(),
      totalFiles,
      files: fpEntries,
    },
    null,
    2,
  ),
);

console.log(JSON.stringify({
  ok: true,
  totalFiles,
  filteredByIgnore,
  languages,
  frameworksCount: frameworks.size,
  complexity,
}));
