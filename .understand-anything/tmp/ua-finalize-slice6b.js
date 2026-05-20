#!/usr/bin/env node
'use strict';

const fs = require('fs');
const path = require('path');
const crypto = require('crypto');

const ROOT = '/home/drdave/repos/RuVector';
const SCAN = require(path.join(ROOT, '.understand-anything/tmp/slice-6b-scan-results.json'));

// ---- Description for scan summary ----
let description;
if (SCAN.rawDescription) {
  description = SCAN.rawDescription;
} else if (SCAN.readmeHead) {
  description = 'rvAgent is RuVector\'s production-grade AI agent runtime crate written in Rust, providing native performance, built-in security controls, true parallel tool execution, and compile-time type safety for building coding assistants and enterprise-secure agents.';
} else {
  description = 'No description available';
}
if (SCAN.totalFiles > 100) {
  description += ' Note: this project has over 100 source files; consider scoping analysis to a subdirectory for faster results.';
}

const finalScan = {
  name: SCAN.name,
  description,
  languages: SCAN.languages,
  frameworks: SCAN.frameworks,
  files: SCAN.files,
  totalFiles: SCAN.totalFiles,
  filteredByIgnore: SCAN.filteredByIgnore,
  estimatedComplexity: SCAN.estimatedComplexity,
  importMap: SCAN.importMap,
};

fs.writeFileSync(
  path.join(ROOT, '.understand-anything/tmp/slice-6b-scan-summary.json'),
  JSON.stringify(finalScan, null, 2)
);

// ---- slice-6b-all-file-paths.json (flat path list) ----
fs.writeFileSync(
  path.join(ROOT, '.understand-anything/tmp/slice-6b-all-file-paths.json'),
  JSON.stringify(SCAN.files.map(f => f.path), null, 2)
);

// ---- fingerprints.slice-6b.json — match slice-6a shape exactly ----
function sha256OfFile(rel) {
  const buf = fs.readFileSync(path.join(ROOT, rel));
  return crypto.createHash('sha256').update(buf).digest('hex');
}

const filesMap = {};
for (const f of SCAN.files) {
  filesMap[f.path] = {
    filePath: f.path,
    contentHash: sha256OfFile(f.path),
    functions: [],
    classes: [],
    imports: (SCAN.importMap[f.path] || []).slice(),
    exports: [],
    totalLines: f.sizeLines,
    hasStructuralAnalysis: false,
  };
}

const fingerprints = {
  version: '1.0.0',
  gitCommitHash: '9054c2cc6793ff11175460694a6479be3ac5b0af',
  generatedAt: new Date().toISOString(),
  slice: '6b',
  scope: 'crates/rvAgent',
  files: filesMap,
};

fs.writeFileSync(
  path.join(ROOT, '.understand-anything/fingerprints.slice-6b.json'),
  JSON.stringify(fingerprints, null, 2)
);

console.error('Wrote slice-6b outputs:', SCAN.totalFiles, 'files');
