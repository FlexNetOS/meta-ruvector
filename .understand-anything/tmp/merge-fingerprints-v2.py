#!/usr/bin/env python3
"""Merge slice fingerprints into master, handling both dict and list slice schemas."""
import json
import sys
from pathlib import Path

if len(sys.argv) < 3:
    print("usage: merge-fingerprints-v2.py <master> <slice1> [<slice2> ...]")
    sys.exit(1)

master_path = Path(sys.argv[1])
slice_paths = [Path(p) for p in sys.argv[2:]]

with open(master_path) as f:
    master = json.load(f)
master_files = master.setdefault("files", {})
before = len(master_files)
added = 0
updated = 0

for sp in slice_paths:
    with open(sp) as f:
        d = json.load(f)
    files = d.get("files", {})
    if isinstance(files, dict):
        items = files.items()
    elif isinstance(files, list):
        items = [(f.get("path") or f.get("filePath"), f) for f in files if (f.get("path") or f.get("filePath"))]
    else:
        continue
    for path, fp in items:
        if "filePath" not in fp and "path" in fp:
            fp["filePath"] = fp["path"]
        fp.setdefault("contentHash", fp.get("sha1", ""))
        fp.setdefault("functions", [])
        fp.setdefault("classes", [])
        fp.setdefault("imports", [])
        fp.setdefault("exports", [])
        fp.setdefault("totalLines", fp.get("sizeLines", 0))
        fp.setdefault("hasStructuralAnalysis", False)
        if path in master_files:
            updated += 1
        else:
            added += 1
        master_files[path] = fp

after = len(master_files)
print(f"Master: {before} -> {after} files (+{added} new, {updated} updated)")
with open(master_path, "w") as f:
    json.dump(master, f, indent=2)
print(f"Written: {master_path}")
