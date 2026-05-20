#!/usr/bin/env python3
"""Merge slice-scoped fingerprints into the master fingerprints.json.

Usage:
    python merge-fingerprints.py <master> <slice-file1> [<slice-file2> ...]
"""
import json
import sys
from pathlib import Path

if len(sys.argv) < 3:
    print("usage: merge-fingerprints.py <master> <slice-file1> [...]")
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
        slice_data = json.load(f)
    for path, fp in slice_data.get("files", {}).items():
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
