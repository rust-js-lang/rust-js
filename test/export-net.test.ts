// The export net (#2481, test/export-net.ts) over its own cases, a crate
// alone and a library with the app using it, and the crate pairs' table:
// each path each crate offers JS is in its JS and its `.d.ts`, and gives
// what native Rust does. The nightly's (scripts/shard.ts).
import { beforeAll, expect, test } from "bun:test";
import { readdirSync } from "node:fs";
import { join } from "node:path";
import { exportNet } from "./export-net";
import { pairs } from "./crates";
import { buildCompiler, root } from "./support";
import { underLoad } from "../scripts/timing";

beforeAll(buildCompiler, 600_000);

const cases = join(root, "test", "export-net");
for (const name of readdirSync(cases).sort()) {
  const files = readdirSync(join(cases, name)).includes("lib.rs") ? ["lib"] : ["dep", "app"];
  test(`export net: ${name}`, () => {
    const netted = exportNet(cases, name, files);
    expect(netted.problems).toEqual([]);
    // Each of its own checks something of each kind.
    expect([netted.paths > 0, netted.calls > 0, netted.declared > 0]).toEqual([true, true, true]);
  }, underLoad(300_000));
}

// What a crate offers JS is found of Rust: each `pub` item of the root and
// each `pub use`, by the name it gives, and through each module re-exported,
// a library's root or module, by name or a glob's, what it offers; not a
// module's own `pub mod`, `dep::rng` and `dep::outer`, which only Rust
// names (#2544's shapes).
test("the paths a crate offers JS are its root's and each re-exported module's, not a pub mod's own", () => {
  const { offered } = exportNet(cases, "reexports_alone", ["dep", "app"]);
  expect(offered.dep).toEqual(["const LIMIT -> LIMIT", "type Point -> Point", "function seed -> seed", "function name_of -> nameOf"]);
  expect(offered.app).toEqual([
    "namespace f -> f",
    "namespace f::inner -> f.inner",
    "function f::inner::twice -> f.inner.twice",
    "namespace r -> r",
    "namespace r::rng -> r.rng",
    "function r::rng::next -> r.rng.next",
    "namespace i -> i",
    "namespace i::d -> i.d",
    "const i::d::LIMIT -> i.d.LIMIT",
    "type i::d::Point -> i.d.Point",
    "function i::d::seed -> i.d.seed",
    "function i::d::name_of -> i.d.nameOf",
    "type Point -> Point",
    "function total -> total",
  ]);
}, underLoad(300_000));

for (const name of readdirSync(pairs).sort()) {
  test(`export net: two crates: ${name}`, () => {
    expect(exportNet(pairs, name, ["dep", "app"]).problems).toEqual([]);
  }, underLoad(300_000));
}
