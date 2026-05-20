#!/usr/bin/env node
// Finalize slice-10b outputs:
//   1) Produce fingerprints.slice-10b.json with sha1
//   2) Rewrite slice-10b-scan-summary.json without intermediate fields

const fs = require('fs');
const path = require('path');

const projectRoot = process.argv[2];
const summaryPath = path.join(projectRoot, '.understand-anything/tmp/slice-10b-scan-summary.json');
const fingerprintsPath = path.join(projectRoot, '.understand-anything/fingerprints.slice-10b.json');

const raw = JSON.parse(fs.readFileSync(summaryPath, 'utf8'));

// Build fingerprints file (slice-scoped, sha1-included)
const fingerprints = {
  slice: '10b',
  scope: raw.scope,
  generatedAt: new Date().toISOString(),
  totalFiles: raw.totalFiles,
  files: raw.filesWithSha1.map(f => ({
    path: f.path,
    language: f.language,
    sizeLines: f.sizeLines,
    fileCategory: f.fileCategory,
    sha1: f.sha1,
  })),
};

fs.writeFileSync(fingerprintsPath, JSON.stringify(fingerprints, null, 2));

// Build final scan-summary (strip script-only fields per project-scanner contract,
// but preserve slice/scope context useful for downstream agents)
let description;
if (raw.rawDescription && raw.rawDescription.trim()) {
  description = raw.rawDescription.trim();
} else if (raw.readmeHead && raw.readmeHead.trim()) {
  description = raw.readmeHead.split('\n').slice(0, 2).join(' ').replace(/^#+\s*/, '').trim();
} else {
  description = 'No description available';
}
if (raw.totalFiles > 100) {
  description += ' Note: this slice has over 100 source files; consider scoping analysis to a subdirectory for faster results.';
}

const finalSummary = {
  slice: '10b',
  scope: raw.scope,
  name: raw.name,
  description,
  languages: raw.languages,
  frameworks: raw.frameworks,
  files: raw.files,
  totalFiles: raw.totalFiles,
  filteredByIgnore: raw.filteredByIgnore,
  estimatedComplexity: raw.estimatedComplexity,
  importMap: raw.importMap,
};

fs.writeFileSync(summaryPath, JSON.stringify(finalSummary, null, 2));
console.log('fingerprints: ' + fingerprintsPath);
console.log('summary: ' + summaryPath);
console.log('totalFiles: ' + fingerprints.totalFiles);
