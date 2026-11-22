#!/bin/sh
# The test map's counting build's rustc (scripts/testMap.ts, ADR 1181):
# each of the compiler's crates counts its runs, `-C instrument-coverage`,
# and nothing else does. Cargo runs it as `RUSTC_WRAPPER`, `<this> rustc
# ..`: a compile whose `--crate-name` is one of `RUST_JS_COUNTED_CRATES`,
# the compiler's crates by their names, is counted, and never cached, as a
# cache keyed on what it's told would hand back a build that doesn't count;
# any other's, a dependency's or a build script's, goes to
# `RUST_JS_CACHE_WRAPPER` (sccache, where the machine has it), as it would
# without this.
crate=""
previous=""
for arg in "$@"; do
  if [ "$previous" = "--crate-name" ]; then crate="$arg"; fi
  previous="$arg"
done
case " $RUST_JS_COUNTED_CRATES " in
  *" $crate "*) if [ -n "$crate" ]; then exec "$@" -C instrument-coverage; fi ;;
esac
if [ -n "$RUST_JS_CACHE_WRAPPER" ]; then exec "$RUST_JS_CACHE_WRAPPER" "$@"; fi
exec "$@"
