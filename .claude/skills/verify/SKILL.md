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
cd crates/rvf
RVF=/home/flexnetos/.nix-profile/bin/rvf
rtk stat "$RVF"
```

The real CLI surface is unavailable until Yazelix packages and pins this binary.
Use `rtk cargo test` for source validation; never fall back to a workspace target
binary as a runtime frontdoor.

## Drive rvf

```bash
W=$(rtk mktemp -d); cd $W
rtk proxy -- "$RVF" create parent.rvf --dimension 4
rtk proxy -- tee v.json >/dev/null <<'JSON'
[{"id":1,"vector":[1,0,0,0]},{"id":2,"vector":[0,1,0,0]},
 {"id":3,"vector":[0,0,1,0]},{"id":4,"vector":[0.9,0.1,0,0]}]
JSON
rtk proxy -- "$RVF" ingest parent.rvf --input v.json
rtk proxy -- "$RVF" status parent.rvf
rtk proxy -- "$RVF" derive parent.rvf child.rvf
rtk proxy -- "$RVF" inspect child.rvf
rtk proxy -- "$RVF" filter parent.rvf --include-ids 1,2
rtk proxy -- "$RVF" inspect parent.rvf
rtk proxy -- "$RVF" query parent.rvf --vector "1,0,0,0" -k 3
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

### The membership filter does NOT filter `query` — by design

`filter --include-ids 1,2` followed by `query` returning every id is
**correct upstream behaviour, not a bug.** Do not "fix" it. The
membership filter has exactly two legitimate consumers:

1. **COW inheritance.** `branch()` wires a COW engine *and* a membership
   filter so the child inherits the parent's vectors; deletes in the
   child hide inherited rows by clearing their bit. It is applied to
   *parent* rows only, in `cow_exact_parent_scan` and
   `query_via_index_cow`. `derive()` deliberately does not wire it —
   see the `branch` vs `derive` docs in `@ruvector/rvf-node`'s
   `index.d.ts`.
2. **Shared-HNSW artifacts.** `rvf filter --help` says "Create a
   membership filter for shared HNSW" — the MEMBERSHIP_SEG is an
   artifact for that consumer, not an input to `rvf query`.

Upstream's `query_exact` guards its local slab on `deletion_bitmap` and
the metadata `filter` expression only. Adding a membership check there
changes upstream semantics and would break COW children, whose own
writes are absent from a parent-sized include filter by construction
(`branch()` sizes it to the parent's `vector_count`; `insert()` never
registers child-local ids). A membership filter you want to *query*
against belongs on a `branch`, i.e. `filter --output <child>`.

## Drive ruvector (HNSW)

```bash
cd /home/flexnetos/meta/src/meta-ruvector
RV=/home/flexnetos/.nix-profile/bin/ruvector
rtk stat "$RV"
rtk proxy -- "$RV" create --dimensions 4 --path core.db
rtk proxy -- "$RV" insert --db core.db --input v.json
rtk proxy -- "$RV" search --db core.db --query "1,0,0,0" -k 3
rtk proxy -- "$RV" info --db core.db
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
