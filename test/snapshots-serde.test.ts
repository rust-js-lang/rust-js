// Snapshots of the examples of serde's (see test/snapshots/), each compiled
// and compared file by file, as snapshots.test.ts' others are: a file of
// their own, as `inbox`'s compile is the snapshots' longest, so it starts
// with the pool's other files (#2465).

import { beforeAll } from "bun:test";
import { buildSerde } from "./serde-crate";
import { snapshotTests } from "./snapshot-cases";
import { buildCompiler } from "./support";

// serde's build is minutes on a cold cache: before any case, whose own time
// is its compile's alone.
beforeAll(() => {
  buildCompiler();
  buildSerde();
}, 600_000);

snapshotTests([
  ["wire", "examples/wire.rs", ["serde"]],
  ["inbox", "examples/inbox.rs", ["serde"]],
  ["api", "examples/api.rs", ["serde"]],
  ["dynamic", "examples/dynamic.rs", ["serde"]],
  ["wide", "examples/wide.rs", ["serde"]],
]);
