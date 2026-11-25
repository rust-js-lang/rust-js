// The export net (#2481, test/export-net.ts) over its own cases, a crate
// alone and a library with the app using it, and the crate pairs' table:
// each path each crate offers JS is in its JS and its `.d.ts`, and gives
// what native Rust does. In `exportNetParts` files, each the tests
// test/export-net-titles.ts puts in its part, so the pool runs them at
// once; the nightly's (scripts/shard.ts).
import { expect, test } from "bun:test";
import { exportNet } from "./export-net";
import { exportNetTitles, ownCases } from "./export-net-titles";
import { underLoad } from "../scripts/timing";

/** Each test of the export net in `part`, from 1, of `exportNetParts`. */
export function exportNetTests(part: number) {
  for (const net of exportNetTitles().filter((net) => net.part === part)) {
    if (net.kind === "own") {
      test(net.title, () => {
        const netted = exportNet(net.cases, net.name, net.files);
        expect(netted.problems).toEqual([]);
        // Each of its own checks something of each kind.
        expect([netted.paths > 0, netted.calls > 0, netted.declared > 0, netted.shapes.checked > 0]).toEqual([true, true, true, true]);
        // And oracle B decides each shape of its own: none it can't.
        expect(netted.shapes.why).toEqual({});
      }, underLoad(300_000));
    } else if (net.kind === "pair") {
      test(net.title, () => {
        const netted = exportNet(net.cases, net.name, net.files);
        expect(netted.problems).toEqual([]);
        // What no value was made of, said: a sealed struct's, a class's (#2594).
        if (Object.keys(netted.notGiven).length > 0) console.log(`${net.title}: not given ${JSON.stringify(netted.notGiven)}`);
        // And where Rust's reads weren't compared, why (#2596).
        if (netted.unread) console.log(`${net.title}: reads unread, ${netted.unread}`);
        // Oracle B decides each shape: none it can't, each said by why.
        expect(netted.shapes.why).toEqual({});
      }, underLoad(300_000));
    } else {
      // What a crate offers JS is found of Rust: each `pub` item of the root and
      // each `pub use`, by the name it gives, and through each module re-exported,
      // a library's root or module, by name or a glob's, what it offers; not a
      // module's own `pub mod`, `dep::rng` and `dep::outer`, which only Rust
      // names (#2544's shapes).
      test(net.title, () => {
        const { offered } = exportNet(ownCases, "reexports_alone", ["dep", "app"]);
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
    }
  }
}
