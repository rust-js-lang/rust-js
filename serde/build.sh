#!/usr/bin/env bash
# Build serde, serde_derive and serde_json (ADR 0077), hashbrown (ADR 0423),
# indexmap (ADR 0424) and memchr (ADR 0529), with rust-js's pinned
# toolchain, and print the
# rustc flags that find them:
#
#   rust-js shared.rs -- $(serde/build.sh)           # metadata, for rust-js's target (ADR 0090)
#   rustc shared.rs $(serde/build.sh --rlib)         # libraries, for a native build
#   rustc --target=wasm32-wasip1 shared.rs $(serde/build.sh --rlib-wasm32)
#                                                    # libraries, for a native build on wasm32, whose
#                                                    # 32-bit usize is the JS's (#1532)
#   rustc -O shared.rs $(serde/build.sh --rlib-release)
#                                                    # libraries built optimized, for a native
#                                                    # benchmark (#1542)
set -euo pipefail
cd "$(dirname "$0")"
target=../target/serde
# Each crate's file where Cargo's messages say it put it (`flags.ts`).
flags() {
  local profile=$1 extension=$2
  shift 2
  cargo build --quiet --locked --message-format=json --target-dir "$target" "$@" | bun flags.ts "$target/$profile" "$extension"
}
if [ "${1:-}" = "--rlib" ]; then
  flags debug rlib
elif [ "${1:-}" = "--rlib-release" ]; then
  flags release rlib --release
elif [ "${1:-}" = "--rlib-wasm32" ]; then
  # Natively on wasm32; serde_derive, a procedural macro, runs on the host.
  flags wasm32-wasip1/debug rlib --target wasm32-wasip1
else
  # For the target; serde_derive, a procedural macro, runs on the host.
  flags wasm32-unknown-unknown/debug rmeta --target wasm32-unknown-unknown
fi
