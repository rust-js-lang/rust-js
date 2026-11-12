#!/bin/bash
# run-one.sh <name>: compile p/<name>.rs natively and with rust-js, run both.
# Needs: RUSTJS (the rust-js checkout), PROBES (this directory, with p/ made by the probes-N.ts scripts),
# JFLAGS="$(serde/build.sh | tail -1)" and NFLAGS="$(serde/build.sh --rlib | tail -1)" from the checkout.
# Writes out/<name>.native, out/<name>.js, and the compilers' messages beside them.
name=$1
cd "$RUSTJS"
mkdir -p target/jp "$PROBES/out" "$PROBES/bin"
if rustc --edition=2024 -A warnings $NFLAGS "$PROBES/p/$name.rs" -o "$PROBES/bin/$name" 2> "$PROBES/out/$name.native-err"; then
  timeout 20 "$PROBES/bin/$name" > "$PROBES/out/$name.native" 2>&1; echo "exit $?" >> "$PROBES/out/$name.native"
else
  echo "NATIVE COMPILE FAILED" > "$PROBES/out/$name.native"
fi
if timeout 120 target/debug/rust-js "$PROBES/p/$name.rs" -o target/jp/$name.js -- --edition=2024 -A warnings $JFLAGS > "$PROBES/out/$name.rsjs-err" 2>&1; then
  cp target/jp/$name.js target/jp/$name.run.js; echo "main();" >> target/jp/$name.run.js
  timeout 20 bun target/jp/$name.run.js > "$PROBES/out/$name.js" 2>&1; echo "exit $?" >> "$PROBES/out/$name.js"
else
  echo "RUSTJS REFUSED" > "$PROBES/out/$name.js"
fi
