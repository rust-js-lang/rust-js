// The drop elaboration's shadow (#2112's change D, step 1): of each `Drop`
// of a case's bodies, whether what rust-js's elaboration drops of each part
// is what rustc's own elaborated body drops there, never, always or as a
// flag says, `--drops-shadow`'s lines. On the normal path no case may
// disagree; a cleanup's are apart, as rust-js's body has no unwinding of
// what can't throw (ADR 0301).
import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { runSync } from "./child";
import { buildCompiler, compiler, fixture, root } from "./support";

beforeAll(buildCompiler, 600_000);

/** Each line `--drops-shadow` writes of a corpus case, by its verdict and path. */
function shadowOf(name: string): string[][] {
  const dir = fixture("drops-shadow");
  const log = join(dir, `${name}.txt`);
  writeFileSync(log, "");
  const args = [compiler, "--drops-shadow", log, join(root, "test/corpus", name), "-o", join(dir, `${name}.js`), "--", "--edition=2024"];
  const out = runSync(args, dir, 120_000);
  expect(out.code, out.stderr).toBe(0);
  return readFileSync(log, "utf8").split("\n").filter(Boolean).map((line) => line.split("\t"));
}

const count = (lines: string[][], verdict: string, path: string) =>
  lines.filter(([v, p]) => v === verdict && p === path).length;

// Partial moves of enums, structs, tuples, arrays and boxes, a copied
// default's: rustc's ladder of each, read through its tests of
// discriminants and flags.
const partial = [
  "match_binding_moves_what_a_partial_arm_keeps.rs",
  "drop_partial.rs",
  "or_pattern_part_moves.rs",
  "arm_temporaries.rs",
  "array_partial_moves.rs",
  "copied_default_impl_drops.rs",
];

test("no normal path's drop disagrees with rustc's elaboration", () => {
  const disagreeing = partial.flatMap((name) =>
    shadowOf(name)
      .filter(([verdict, path]) => verdict === "disagree" && path === "normal")
      .map((line) => `${name}: ${line.slice(2).join(" ")}`),
  );
  expect(disagreeing).toEqual([]);
});

// #2064's: a `match` one arm of which moves part of an enum and another
// all of it, whose variants none takes apart are dropped as the enum's own
// flag says, as rustc's ladder tests its discriminant, then the flag.
test("a match's enum moved in part and whole is dropped where rustc drops it", () => {
  const lines = shadowOf("match_binding_moves_what_a_partial_arm_keeps.rs");
  const flatten = lines.filter(([, path, owner]) => path === "normal" && owner === "flatten");
  // Each of its drops on the normal path as rustc's: a `Text`'s bytes,
  // which JS drops nothing of, never compared.
  expect(flatten.filter(([verdict]) => verdict !== "agree").map((line) => line.slice(3).join(" "))).toEqual([]);
  expect(flatten.length).toBeGreaterThanOrEqual(10);
  expect(count(lines, "agree", "normal")).toBeGreaterThanOrEqual(40);
  // The rest of the moved enum, `(_15.0)`, as its own flag says on each
  // side, which compared at the top alone read as unalike.
  // Where the loop's `node` ends: its moved `Text` bytes and the rest of
  // the enum, each compared; and another drop written there.
  const ends = flatten.filter(([, , , at]) => at.includes(":45:5:")).map(([verdict, , , , detail]) => `${verdict} ${detail}`);
  expect(ends.sort()).toEqual(["agree 1 compared", "agree 2 compared"]);
});
