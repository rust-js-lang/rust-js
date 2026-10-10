#!/usr/bin/env bash
# Compile the node crate's metadata, which is all programs need (ADR 0272),
# and the js and webapi crates' beside it, which it uses (ADR 0102), for the target
# rust-js checks programs for (ADR 0090):
#
#   node/build.sh -o target/libnode.rmeta
#
# Then compile a program with
# `rust-js app.rs -- --extern node=<that file> --extern js=<dir>/libjs.rmeta -L <dir>`.
set -euo pipefail
[ "${1:-}" = "-o" ] && [ -n "${2:-}" ] || {
  echo "usage: node/build.sh -o <dir>/libnode.rmeta [rustc flags]" >&2
  exit 1
}
out=$2
shift 2
mkdir -p "$(dirname "$out")"
dir=$(cd "$(dirname "$out")" && pwd)
cd "$(dirname "$0")"
../webapi/build.sh -o "$dir/libwebapi.rmeta" "$@"
target=(--target=wasm32-unknown-unknown)
for arg in "$@"; do
  case "$arg" in --target | --target=*) target=() ;; esac
done
# rust-js compiles it, as rustc with rust-js's tool, `rust_js`, known (ADR
# 0112): $RUST_JS_COMPILER, or this repository's own build.
compiler=${RUST_JS_COMPILER:-$PWD/../target/debug/rust-js}
# A packaged one is a JS launcher, run by the JS runtime the build is.
rust_js=("$compiler")
case "$compiler" in *.js) rust_js=("${RUST_JS_JS_RUNTIME:-node}" "$compiler") ;; esac
exec "${rust_js[@]}" --rustc --edition=2024 --crate-type=lib --crate-name=node --emit=metadata ${target[@]+"${target[@]}"} src/lib.rs \
  --extern webapi="$dir/libwebapi.rmeta" --extern js="$dir/libjs.rmeta" -o "$dir/$(basename "$out")" "$@"
