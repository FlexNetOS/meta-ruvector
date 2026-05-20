#!/usr/bin/env node
// Build fingerprints.slice-10c.json in the same shape as slice-10b's.
const fs = require('fs');

const summaryPath = '/home/drdave/repos/RuVector/.understand-anything/tmp/slice-10c-scan-summary.json';
const outPath = '/home/drdave/repos/RuVector/.understand-anything/fingerprints.slice-10c.json';

const s = JSON.parse(fs.readFileSync(summaryPath, 'utf8'));

const fp = {
  slice: '10c',
  scope: s.scope,
  generatedAt: new Date().toISOString(),
  totalFiles: s.totalFiles,
  files: s.filesWithSha1.map(r => ({
    path: r.path,
    language: r.language,
    sizeLines: r.sizeLines,
    fileCategory: r.fileCategory,
    sha1: r.sha1,
  })),
};

fs.writeFileSync(outPath, JSON.stringify(fp, null, 2));
console.log('Wrote: ' + outPath);
console.log('Files: ' + fp.totalFiles);
