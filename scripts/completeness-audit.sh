#!/usr/bin/env bash
# RuVector completeness audit — per-crate evidence collector.
# Emits TSV: crate<TAB>src_loc<TAB>hard_stubs<TAB>partial_markers<TAB>has_tests<TAB>tier
# tier heuristic (evidence flag, not a verdict): STUB / PARTIAL / SUSPECT / LIKELY-PROD
set -uo pipefail
cd "$(dirname "$0")/.." || exit 1
printf "crate\tsrc_loc\thard\tpartial\ttests\ttier\n"
# iterate every crate root (Cargo.toml with a [package]) under crates/, excluding target
while IFS= read -r manifest; do
  dir=$(dirname "$manifest")
  case "$dir" in *"/target/"*) continue;; esac
  grep -q '^\[package\]' "$manifest" 2>/dev/null || continue
  name=$(grep -m1 '^name' "$manifest" | sed -E 's/name *= *"([^"]+)".*/\1/')
  [ -d "$dir/src" ] || { printf "%s\t0\t0\t0\tno\tNO-SRC\n" "$name"; continue; }
  # source LOC excluding tests
  loc=$(find "$dir/src" -name '*.rs' 2>/dev/null | xargs cat 2>/dev/null | wc -l)
  hard=$(grep -rIn -E 'todo!\(\)|unimplemented!\(\)|unreachable!\("not' "$dir/src" --include=*.rs 2>/dev/null | grep -vE '#\[cfg\(test|/tests/' | wc -l)
  partial=$(grep -rIn -E 'in a real|in production[^"]*would|simplified (impl|version|for)|placeholder (impl|return|value|-)' "$dir/src" --include=*.rs 2>/dev/null | wc -l)
  tests=$([ -d "$dir/tests" ] && echo yes || (grep -rqE '#\[test\]|#\[tokio::test\]' "$dir/src" --include=*.rs 2>/dev/null && echo yes || echo no))
  # heuristic tier
  if [ "$loc" -lt 60 ] && [ "$hard" -gt 0 ]; then tier="STUB"
  elif [ "$hard" -gt 0 ]; then tier="PARTIAL"
  elif [ "$partial" -ge 3 ]; then tier="PARTIAL"
  elif [ "$partial" -ge 1 ]; then tier="SUSPECT"
  else tier="LIKELY-PROD"; fi
  printf "%s\t%s\t%s\t%s\t%s\t%s\n" "$name" "$loc" "$hard" "$partial" "$tests" "$tier"
done < <(find crates examples -name Cargo.toml 2>/dev/null)
