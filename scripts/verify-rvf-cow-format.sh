#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
rvf_manifest="${repo_root}/crates/rvf/Cargo.toml"
rtk_bin="${RTK_BIN:-/home/flexnetos/.nix-profile/bin/rtk}"

cd "${repo_root}"

"${rtk_bin}" cargo fmt \
  --manifest-path "${rvf_manifest}" \
  --package rvf-types \
  --package rvf-wire \
  --package rvf-runtime \
  --package rvf-cli \
  --package rvf-integration-tests \
  -- --check

"${rtk_bin}" cargo test --manifest-path crates/rvf/rvf-types/Cargo.toml --lib
"${rtk_bin}" cargo test --manifest-path crates/rvf/rvf-wire/Cargo.toml --lib
"${rtk_bin}" cargo test --manifest-path crates/rvf/rvf-runtime/Cargo.toml --lib
"${rtk_bin}" cargo test --manifest-path crates/rvf/rvf-cli/Cargo.toml --bin rvf
"${rtk_bin}" cargo test \
  --manifest-path crates/rvf/tests/rvf-integration/Cargo.toml \
  --test filter_traversal
"${rtk_bin}" cargo test \
  --manifest-path crates/rvf/tests/rvf-integration/Cargo.toml \
  --test cow_benchmarks

# Evidence guards: the legacy digest is reader-only, while both public CLI
# consumers route through manifest-integrated store APIs.
if "${rtk_bin}" rg -n "legacy_content_hash" \
  crates/rvf/rvf-runtime/src/write_path.rs \
  crates/rvf/rvf-cli/src/cmd/filter.rs \
  crates/rvf/rvf-cli/src/cmd/rebuild_refcounts.rs; then
  echo "legacy hash referenced by a canonical writer" >&2
  exit 1
fi

"${rtk_bin}" rg -n "append_membership_filter" crates/rvf/rvf-cli/src/cmd/filter.rs
"${rtk_bin}" rg -n "cow_stats|append_segment" crates/rvf/rvf-cli/src/cmd/rebuild_refcounts.rs

echo "RVF canonical COW/MEMBERSHIP verification passed"
