// What a real crate's JS reads where no corpus case makes it (#1979):
// scripts/crate-corpus.ts's `snapshotted` functions, each crate's held to
// its snapshot by `--check` as the crate corpus measures it, which a push's
// `crate-corpus` step and the nightly's `crates` job run. Here once more,
// with the tests' compiler and target, a mutant's own (`RUST_JS_TARGET`),
// as the tests a mutation names: a mutant that only moves how itertools'
// `putBack` reads, no corpus copy of it moved, is caught here. The nightly
// alone runs this file (scripts/shard.ts's `nightly`), as its crates job
// holds the same; a mutation that names it still runs it.

import { beforeAll, expect, test } from "bun:test";
import { join } from "node:path";
import { snapshotted } from "../scripts/crate-corpus";
import { runSync } from "./child";
import { buildCompiler, compiler, root } from "./support";

beforeAll(buildCompiler, 600_000);

for (const crate of new Set(snapshotted.map((entry) => entry.split("@")[0]!))) {
  test(`${crate}'s snapshotted functions read as their snapshots`, () => {
    const p = runSync([process.execPath, join(root, "scripts/crate-corpus.ts"), "--check", crate], root, 300_000, { RUST_JS_COMPILER: compiler });
    expect(p.code, p.stdout + p.stderr).toBe(0);
  }, 320_000);
}
