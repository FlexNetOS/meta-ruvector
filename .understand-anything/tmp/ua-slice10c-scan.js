#!/usr/bin/env node
// Slice 10c scanner: ruQu, ruqu-{core,algorithms,exotic,wasm}, ruvector-hailo{,-cluster}, hailort-sys,
// ruvector-sparse-inference{,-wasm}, ruvector-fpga-transformer{,-wasm}, ruvector-mmwave
// Uses slice-10c-files.txt as authoritative input list.

const fs = require('fs');
const path = require('path');
const crypto = require('crypto');

const projectRoot = process.argv[2];
const outPath = process.argv[3];

if (!projectRoot || !outPath) {
  console.error('Usage: node ua-slice10c-scan.js <project-root> <out-json>');
  process.exit(1);
}

const filesListPath = path.join(projectRoot, '.understand-anything/tmp/slice-10c-files.txt');
if (!fs.existsSync(filesListPath)) {
  console.error('Missing slice-10c-files.txt at: ' + filesListPath);
  process.exit(1);
}

const rawList = fs.readFileSync(filesListPath, 'utf8').split('\n').map(s => s.trim()).filter(Boolean);

// Hardcoded exclusion filters (defensive; list should already be filtered)
const EXCLUDE_DIR_SEGMENTS = new Set([
  'node_modules', '.git', 'vendor', 'venv', '.venv', '__pycache__',
  'dist', 'build', 'out', 'coverage', '.next', '.cache', '.turbo', 'target', 'obj',
  '.idea', '.vscode'
]);
const EXCLUDE_BASENAMES = new Set(['LICENSE', '.gitignore', '.editorconfig', '.prettierrc']);
const EXCLUDE_EXTS = new Set([
  '.png', '.jpg', '.jpeg', '.gif', '.svg', '.ico',
  '.woff', '.woff2', '.ttf', '.eot',
  '.mp3', '.mp4', '.pdf', '.zip', '.tar', '.gz',
  '.lock', '.log'
]);

function isExcluded(rel) {
  const parts = rel.split('/');
  for (const p of parts) {
    if (EXCLUDE_DIR_SEGMENTS.has(p)) return true;
  }
  const base = parts[parts.length - 1];
  if (EXCLUDE_BASENAMES.has(base)) return true;
  if (/\.eslintrc/.test(base)) return true;
  if (base === 'package-lock.json' || base === 'yarn.lock' || base === 'pnpm-lock.yaml') return true;
  if (/\.min\.(js|css)$/.test(base)) return true;
  if (/\.map$/.test(base)) return true;
  if (/\.generated\./.test(base)) return true;
  const ext = path.extname(base).toLowerCase();
  if (EXCLUDE_EXTS.has(ext)) return true;
  return false;
}

const EXT_TO_LANG = {
  '.ts': 'typescript', '.tsx': 'typescript',
  '.js': 'javascript', '.jsx': 'javascript', '.mjs': 'javascript', '.cjs': 'javascript',
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

const BASENAME_TO_LANG = {
  'Dockerfile': 'dockerfile',
  'Makefile': 'makefile',
  'Jenkinsfile': 'jenkinsfile',
};

function detectLanguage(rel) {
  const base = path.basename(rel);
  if (BASENAME_TO_LANG[base]) return BASENAME_TO_LANG[base];
  const ext = path.extname(base).toLowerCase();
  if (EXT_TO_LANG[ext]) return EXT_TO_LANG[ext];
  if (!ext) return 'unknown';
  return ext.replace(/^\./, '');
}

function detectCategory(rel) {
  const base = path.basename(rel);
  const ext = path.extname(base).toLowerCase();
  const lower = rel.toLowerCase();

  // Infra (priority over config)
  if (base === 'Dockerfile' || base.startsWith('docker-compose')) return 'infra';
  if (ext === '.tf' || ext === '.tfvars') return 'infra';
  if (base === 'Makefile' || base === 'Jenkinsfile' || base === 'Procfile' || base === 'Vagrantfile') return 'infra';
  if (lower.includes('.github/workflows/')) return 'infra';
  if (base === '.gitlab-ci.yml' || lower.includes('.circleci/')) return 'infra';
  if (/\.k8s\.ya?ml$/.test(base)) return 'infra';
  if (lower.includes('/k8s/') || lower.includes('/kubernetes/') || lower.startsWith('k8s/') || lower.startsWith('kubernetes/')) return 'infra';

  // Docs
  if (ext === '.md' || ext === '.rst') return 'docs';
  if (ext === '.txt' && base !== 'LICENSE') return 'docs';

  // Data
  if (['.sql', '.graphql', '.gql', '.proto', '.prisma', '.csv'].includes(ext)) return 'data';
  if (/\.schema\.json$/.test(base)) return 'data';

  // Script
  if (['.sh', '.bash', '.ps1', '.bat', '.cmd'].includes(ext)) return 'script';

  // Markup
  if (['.html', '.htm', '.css', '.scss', '.sass', '.less'].includes(ext)) return 'markup';

  // Config
  if (['.yaml', '.yml', '.json', '.jsonc', '.toml', '.xml', '.cfg', '.ini', '.env'].includes(ext)) return 'config';
  if (['tsconfig.json', 'package.json', 'pyproject.toml', 'Cargo.toml', 'go.mod'].includes(base)) return 'config';

  return 'code';
}

// Filter the input list
const files = [];
const seen = new Set();
for (const rel of rawList) {
  if (seen.has(rel)) continue;
  seen.add(rel);
  if (isExcluded(rel)) continue;
  const abs = path.join(projectRoot, rel);
  if (!fs.existsSync(abs)) continue;
  let stat;
  try { stat = fs.statSync(abs); } catch (e) { continue; }
  if (!stat.isFile()) continue;
  files.push(rel);
}
files.sort();

// Compute lines & sha1 per file
function countLines(absPath) {
  try {
    const buf = fs.readFileSync(absPath);
    if (buf.length === 0) return 0;
    let count = 0;
    for (let i = 0; i < buf.length; i++) if (buf[i] === 0x0a) count++;
    // Match wc -l semantics (counts newlines)
    return count;
  } catch (e) { return 0; }
}

function sha1OfFile(absPath) {
  try {
    const buf = fs.readFileSync(absPath);
    return crypto.createHash('sha1').update(buf).digest('hex');
  } catch (e) { return ''; }
}

const fileRecords = [];
for (const rel of files) {
  const abs = path.join(projectRoot, rel);
  fileRecords.push({
    path: rel,
    language: detectLanguage(rel),
    sizeLines: countLines(abs),
    fileCategory: detectCategory(rel),
    sha1: sha1OfFile(abs),
  });
}

// Language summary
const langSet = new Set(fileRecords.map(f => f.language));
const languages = [...langSet].sort();

// Framework detection
const frameworks = new Set();
// Check each crate's Cargo.toml within scope
const crateRoots = [
  'crates/ruQu',
  'crates/ruqu-core',
  'crates/ruqu-algorithms',
  'crates/ruqu-exotic',
  'crates/ruqu-wasm',
  'crates/ruvector-hailo',
  'crates/ruvector-hailo-cluster',
  'crates/hailort-sys',
  'crates/ruvector-sparse-inference',
  'crates/ruvector-sparse-inference-wasm',
  'crates/ruvector-fpga-transformer',
  'crates/ruvector-fpga-transformer-wasm',
  'crates/ruvector-mmwave',
];

const CARGO_FRAMEWORKS = {
  'actix-web': 'actix-web', 'axum': 'axum', 'rocket': 'Rocket',
  'diesel': 'Diesel', 'tokio': 'tokio', 'serde': 'serde', 'warp': 'warp',
  'wasm-bindgen': 'wasm-bindgen', 'rayon': 'rayon', 'ndarray': 'ndarray',
  'nalgebra': 'nalgebra', 'tch': 'tch', 'candle-core': 'candle',
  'criterion': 'criterion',
};

let projectName = 'ruQu';
let rawDescription = '';
let readmeHead = '';

for (const crateDir of crateRoots) {
  const cargoPath = path.join(projectRoot, crateDir, 'Cargo.toml');
  if (fs.existsSync(cargoPath)) {
    try {
      const content = fs.readFileSync(cargoPath, 'utf8');
      // crude framework detection
      for (const [key, name] of Object.entries(CARGO_FRAMEWORKS)) {
        // Match start-of-line crate name
        const re = new RegExp('^\\s*' + key.replace(/[-/\\^$*+?.()|[\]{}]/g, '\\$&') + '\\s*=', 'm');
        if (re.test(content)) frameworks.add(name);
      }
    } catch (e) {}
  }
}

// Infra framework detection from file list
for (const rec of fileRecords) {
  const base = path.basename(rec.path);
  if (base === 'Dockerfile') frameworks.add('Docker');
  if (base === 'docker-compose.yml' || base === 'docker-compose.yaml') frameworks.add('Docker Compose');
  if (rec.path.endsWith('.tf')) frameworks.add('Terraform');
  if (rec.path.includes('.github/workflows/') && (rec.path.endsWith('.yml') || rec.path.endsWith('.yaml'))) frameworks.add('GitHub Actions');
  if (base === '.gitlab-ci.yml') frameworks.add('GitLab CI');
  if (base === 'Jenkinsfile') frameworks.add('Jenkins');
}

// Use ruQu as project name/description source (largest/first crate in slice 10c)
const primeReadme = path.join(projectRoot, 'crates/ruQu/README.md');
if (fs.existsSync(primeReadme)) {
  try {
    const content = fs.readFileSync(primeReadme, 'utf8');
    readmeHead = content.split('\n').slice(0, 10).join('\n');
  } catch (e) {}
}
const primeCargo = path.join(projectRoot, 'crates/ruQu/Cargo.toml');
if (fs.existsSync(primeCargo)) {
  try {
    const content = fs.readFileSync(primeCargo, 'utf8');
    const nm = content.match(/^\s*name\s*=\s*"([^"]+)"/m);
    if (nm) projectName = nm[1];
    const desc = content.match(/^\s*description\s*=\s*"([^"]+)"/m);
    if (desc) rawDescription = desc[1];
  } catch (e) {}
}

// Complexity
const total = fileRecords.length;
let complexity = 'small';
if (total > 500) complexity = 'very-large';
else if (total > 150) complexity = 'large';
else if (total > 30) complexity = 'moderate';

// Import resolution
const fileSet = new Set(fileRecords.map(f => f.path));
function tryResolve(candidate) {
  if (fileSet.has(candidate)) return candidate;
  return null;
}

function resolveRustImport(fromRel, importPath) {
  // crate::, super::, self:: - module-level only; we map to common file conventions
  // We don't try to fully resolve; instead, downstream graph generation handles it.
  // Skip to avoid noise.
  return null;
}

function resolveJsImport(fromRel, importPath) {
  const fromDir = path.dirname(fromRel);
  let base;
  if (importPath.startsWith('.')) {
    base = path.posix.normalize(path.posix.join(fromDir, importPath));
  } else {
    return null; // external
  }
  const exts = ['.ts', '.tsx', '.js', '.jsx', '/index.ts', '/index.js', '/index.tsx', '/index.jsx'];
  if (tryResolve(base)) return tryResolve(base);
  for (const e of exts) {
    const c = base + e;
    if (tryResolve(c)) return c;
  }
  return null;
}

const importMap = {};
for (const rec of fileRecords) {
  importMap[rec.path] = [];
  if (rec.fileCategory !== 'code') continue;

  const abs = path.join(projectRoot, rec.path);
  let content;
  try { content = fs.readFileSync(abs, 'utf8'); } catch (e) { continue; }

  const resolved = new Set();

  if (rec.language === 'typescript' || rec.language === 'javascript') {
    const importRe = /(?:import[\s\S]*?from\s+|require\(\s*)['"]([^'"]+)['"]/g;
    let m;
    while ((m = importRe.exec(content)) !== null) {
      const r = resolveJsImport(rec.path, m[1]);
      if (r) resolved.add(r);
    }
  } else if (rec.language === 'rust') {
    // Rust mod resolution: look for `mod foo;` -> sibling foo.rs or foo/mod.rs
    const modRe = /^\s*(?:pub\s+)?mod\s+([a-zA-Z_][a-zA-Z0-9_]*)\s*;/gm;
    const fromDir = path.dirname(rec.path);
    let m;
    while ((m = modRe.exec(content)) !== null) {
      const modName = m[1];
      const c1 = path.posix.join(fromDir, modName + '.rs');
      const c2 = path.posix.join(fromDir, modName, 'mod.rs');
      if (tryResolve(c1)) resolved.add(c1);
      else if (tryResolve(c2)) resolved.add(c2);
    }
  }

  importMap[rec.path] = [...resolved].sort();
}

const result = {
  scriptCompleted: true,
  slice: '10c',
  scope: 'crates/ruQu + crates/ruqu-{core,algorithms,exotic,wasm} + crates/ruvector-hailo{,-cluster} + crates/hailort-sys + crates/ruvector-sparse-inference{,-wasm} + crates/ruvector-fpga-transformer{,-wasm} + crates/ruvector-mmwave',
  name: projectName,
  rawDescription,
  readmeHead,
  languages,
  frameworks: [...frameworks].sort(),
  files: fileRecords.map(({ sha1, ...rest }) => rest),
  filesWithSha1: fileRecords,
  totalFiles: fileRecords.length,
  filteredByIgnore: 0,
  estimatedComplexity: complexity,
  importMap,
};

fs.writeFileSync(outPath, JSON.stringify(result, null, 2));
console.log('Wrote: ' + outPath);
console.log('Files: ' + fileRecords.length);
console.log('Languages: ' + languages.join(', '));
console.log('Frameworks: ' + [...frameworks].sort().join(', '));
