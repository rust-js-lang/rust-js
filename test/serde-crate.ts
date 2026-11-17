// serde's crates for the tests that use them, a module of its own: in test/support.ts, which every test
// imports, its reads of serde/build.sh made each test a reader of them, so the
// selector ran the whole suite for a change to them (ADR 1101, amended).

import { join } from "node:path";

import { keptSerdeFlags } from "./serde-kept";
import { run, target } from "./support";

/** serde, serde_derive and serde_json (ADR 0077): the flags that find them,
 * as metadata for rust-js, as libraries for a native build, for one on
 * wasm32-wasip1, whose 32-bit `usize` is the JS's, or built optimized, for
 * a native benchmark. */
const serdeFlags: Record<string, string[]> = {};
export function buildSerde(kind: "rmeta" | "rlib" | "rlib-wasm32" | "rlib-release" = "rmeta"): string[] {
  // Of a snapshot of what Cargo built, which a rebuild of `target/serde`
  // by another run never takes from under a compile (#1711).
  serdeFlags[kind] ??= keptSerdeFlags(
    run(["serde/build.sh", ...(kind === "rmeta" ? [] : [`--${kind}`])]).trim().split(/\s+/),
    join(target, "serde-kept"),
  );
  return serdeFlags[kind];
}
