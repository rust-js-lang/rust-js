#!/usr/bin/env bash
# Build rust-js, with rustc's front end, as a WASI program:
#   wasm/target/wasm32-wasip1/release/rust-js.wasm
# and stage the sysroot it type-checks against (wasm/sysroot).
# To let the deploy workflow skip this build, run `prebuilt.sh publish` after.
set -euo pipefail
cd "$(dirname "$0")"

TOOLCHAIN=$(bun ../scripts/toolchain.ts channel)
COMMIT=$(bun ../scripts/toolchain.ts commit)

# 1. rustc's source: a worktree of a rust-lang/rust clone, at the pinned commit.
if [ ! -d rustc/compiler ]; then
  echo "error: ./rustc is missing. From a rust-lang/rust clone, run:" >&2
  echo "  git worktree add --no-checkout $PWD/rustc $COMMIT" >&2
  echo "  git -C $PWD/rustc sparse-checkout set --cone $(bun ../scripts/toolchain.ts sources)" >&2
  echo "  git -C $PWD/rustc checkout" >&2
  exit 1
fi
bun ../scripts/toolchain.ts check-source "$PWD/rustc"
# The paths the build reads, a checkout made for an earlier release's too.
git -C rustc sparse-checkout set --cone $(bun ../scripts/toolchain.ts sources)

# 2. Our patches to rustc. Skip ones already applied; fail loudly on conflicts.
for patch in patches/*.patch; do
  if git -C rustc apply --reverse --check "../$patch" 2>/dev/null; then
    echo "already applied: $patch"
  else
    git -C rustc apply "../$patch"
    echo "applied: $patch"
  fi
done

# 3. On CI, rustc's tree made as old as one fixed time, `RUST_JS_RUSTC_MTIME`
#    (touch's [[CC]YY]MMDDhhmm): a fresh checkout, patched anew, is newer than
#    each output of the cached target, so Cargo built all of rustc's crates
#    again each run, 4 min 51 s of the job's 12 (docs/dev-loop.md). Sound only
#    where the target is one built from the same tree: check.yml's cache key
#    holds what decides its bytes, and the tree's hash, kept in the target,
#    is held to the checkout's here. rust-js's own sources keep their times.
if [ -n "${RUST_JS_RUSTC_MTIME:-}" ]; then
  tree=$(cd rustc && find . -path ./.git -prune -o -type f -print0 | LC_ALL=C sort -z | xargs -0 sha256sum | sha256sum | cut -c1-64)
  kept=target/rustc-sources.sha256
  if [ -f "$kept" ] && [ "$(cat "$kept")" != "$tree" ]; then
    echo "error: the cached wasm/target was built from other rustc sources ($(cat "$kept")) than this checkout's ($tree): check.yml's wasm cache key must hold what decides them" >&2
    exit 1
  fi
  find rustc -path rustc/.git -prune -o -type f -exec touch -h -t "$RUST_JS_RUSTC_MTIME" {} +
  mkdir -p target
  echo "$tree" > "$kept"
fi

cargo "+$TOOLCHAIN" build --release --locked

# 4. Record which committed inputs this build came from (`dirty` if there were
#    uncommitted changes), so `prebuilt.sh publish` can refuse a stale binary.
./prebuilt.sh stamp

# 5. The sysroot rust-js type-checks against.
./stage-sysroot.sh

echo "built: $PWD/target/wasm32-wasip1/release/rust-js.wasm"
