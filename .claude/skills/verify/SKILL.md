---
name: verify
description: Build and drive the rvf and ruvector CLIs to verify a change at its real surface. Use for meta-ruvector changes instead of running the test suite.
---

# Verifying meta-ruvector

Two CLIs reach most of this repo: `rvf` for the RuVector Format / CoW
layer, and `ruvector` for the vector database and its HNSW index. Drive
those rather than running tests.

## System libraries come from the nix store, not the default path

`libudev-sys` and `yeslogic-fontconfig-sys` fail their build scripts out
of the box even though the libraries are installed. Point pkg-config at
the store paths first or the build dies before any code compiles:

```bash
export PKG_CONFIG_PATH="\
$(ls -d /nix/store/*systemd-minimal-*-dev/lib/pkgconfig | head -1):\
$(ls -d /nix/store/*fontconfig-*-dev/lib/pkgconfig | head -1):\
$(ls -d /nix/store/*freetype-*-dev/lib/pkgconfig | head -1):\
$(ls -d /nix/store/*expat-*-dev/lib/pkgconfig | head -1)"
```

fontconfig additionally needs freetype2 **and** expat on the path or it
still fails with `Package 'freetype2', required by 'fontconfig', not found`.

## crates/rvf is a separate workspace

It sits in the root `exclude` list, so `cargo check --workspace` at the
repo root does **not** cover it. Any rvf change needs its own
invocation from `crates/rvf`, otherwise a broken rvf compiles "clean".

```bash
cd crates/rvf && rtk proxy -- env PKG_CONFIG_PATH="$PKG_CONFIG_PATH" cargo build -p rvf-cli
RVF=/run/user/1001/yazelix/volatile/cargo-target/debug/rvf
```

## Drive rvf

```bash
W=$(mktemp -d); cd $W
$RVF create parent.rvf --dimension 4
cat > v.json <<'JSON'
[{"id":1,"vector":[1,0,0,0]},{"id":2,"vector":[0,1,0,0]},
 {"id":3,"vector":[0,0,1,0]},{"id":4,"vector":[0.9,0.1,0,0]}]
JSON
$RVF ingest parent.rvf --input v.json
$RVF status parent.rvf
$RVF derive parent.rvf child.rvf        # CoW branch
$RVF inspect child.rvf                  # lineage: parent id, depth, is_root
$RVF filter parent.rvf --include-ids 1,2   # writes a Membership segment
$RVF inspect parent.rvf                 # segment list should now show Membership
$RVF query parent.rvf --vector "1,0,0,0" -k 3
```

Use `--metric ip` on `create` to exercise the inner-product path.
Distances there are **signed** — a correct run looks like
`-1.0, -0.9, -0.0`, not a column of zeros. Zeros mean something
clamped the negation and destroyed the ranking.

Known dead end: `rebuild-refcounts` reports "No COW map found in file"
after derive, freeze and ingest alike. No rvf-cli flow observed so far
writes a COW_MAP segment, so the `append_cow_map` path and the
`rvf_wire` framed codec are not reachable from this CLI. Verify those
through `crates/rvf/tests/rvf-integration/tests/cow_*.rs` instead, and
say so rather than claiming CLI coverage.

## Drive ruvector (HNSW)

```bash
cd /home/flexnetos/meta/src/meta-ruvector
rtk proxy -- env PKG_CONFIG_PATH="$PKG_CONFIG_PATH" cargo build -p ruvector-cli
RV=/run/user/1001/yazelix/volatile/cargo-target/debug/ruvector
$RV create --dimensions 4 --path core.db
$RV insert --db core.db --input v.json    # --input, not --file
$RV search --db core.db --query "1,0,0,0" -k 3
$RV info --db core.db
```

For cosine, an identical vector scores 0.0, a near-parallel one ~0.006,
an orthogonal one 1.0. `create` has no metric flag — it is always
Cosine — so the DotProduct branch of `DistanceFn::eval` in
`ruvector-core/src/index/hnsw.rs` is **not** reachable from this CLI.
Do not claim to have verified it here.

## Fork discipline

This repo is a fork of ruvnet/RuVector; `upstream` is configured. RVF
and CoW are different sources — resolve merges per source, never a
whole crate tree wholesale, or upstream API silently disappears. A
no-downgrade check against the pre-merge tree cannot see that; compare
public items against upstream directly:

```bash
git show upstream/main:$F | grep -oE '^\s*pub (fn|struct|enum|trait|const) [a-zA-Z_0-9]+'
```
