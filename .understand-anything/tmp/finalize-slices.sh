#!/bin/bash
set -euo pipefail
cd /home/drdave/repos/RuVector
UA=.understand-anything

cp $UA/tmp/slice-6a-final.json $UA/slice-6a-knowledge-graph.json
cp $UA/tmp/slice-6b-final.json $UA/slice-6b-knowledge-graph.json

python3 $UA/tmp/merge-fingerprints.py $UA/fingerprints.json $UA/fingerprints.slice-6a.json $UA/fingerprints.slice-6b.json

python3 /home/drdave/repos/Understand-Anything/understand-anything-plugin/skills/understand/merge-subdomain-graphs.py /home/drdave/repos/RuVector

python3 << 'PYEOF'
import json
import subprocess
from datetime import datetime, timezone

git_hash = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd="/home/drdave/repos/RuVector", text=True).strip()
with open(".understand-anything/fingerprints.json") as f:
    fp = json.load(f)
file_count = len(fp.get("files", {}))

meta = {
    "lastAnalyzedAt": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
    "gitCommitHash": git_hash,
    "version": "1.0.0",
    "analyzedFiles": file_count,
    "slice": "1+2+3+4+5a+5b+6a+6b+6c+7+8+9",
    "scope": "slices 1-9 (excluding slice 10 / 'Other')",
    "slicesPresent": ["1", "2", "3", "4", "5a", "5b", "6a", "6b", "6c", "7", "8", "9"],
    "remaining": ["10"],
}
with open(".understand-anything/meta.json", "w") as f:
    json.dump(meta, f, indent=2)
print(f"meta.json updated: {file_count} files, slices: {meta['slicesPresent']}")
PYEOF

python3 -c "
import json
with open('.understand-anything/knowledge-graph.json') as f:
    d = json.load(f)
print(f'Master graph: {len(d.get(\"nodes\", []))} nodes / {len(d.get(\"edges\", []))} edges / {len(d.get(\"layers\", []))} layers / {len(d.get(\"tour\", []))} tour steps')
"
