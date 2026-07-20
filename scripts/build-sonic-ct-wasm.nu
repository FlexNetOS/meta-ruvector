#!/usr/bin/env nu
# Build the sonic_ct WebAssembly module and stage it for the React UI.
#
# No wasm-bindgen / wasm-pack is required: the crate exports a raw C ABI, so
# this builds wasm32-unknown-unknown and stages the generated module for Vite.

const root = path self ..
const wasm_crate = ($root | path join "crates" "sonic-ct-wasm")
const ui_public = ($root | path join "examples" "sonic-ct" "public")
const output = ($wasm_crate | path join "target" "wasm32-unknown-unknown" "release" "sonic_ct_wasm.wasm")
const staged = ($ui_public | path join "sonic_ct.wasm")

print "==> ensuring wasm32 target"
if (which rustup | is-not-empty) {
  try {
    ^rustup target add wasm32-unknown-unknown
  } catch {
    print "==> rustup target setup unavailable; using the active toolchain target set"
  }
} else {
  print "==> rustup unavailable; using the active toolchain target set"
}

print "==> building sonic-ct-wasm (release)"
cd $wasm_crate
^cargo build --release --target wasm32-unknown-unknown

mkdir $ui_public
cp $output $staged
let size = (ls $staged | get 0.size)
print $"==> staged ($staged) ($size)"
