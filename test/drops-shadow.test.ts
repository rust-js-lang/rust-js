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

// Change D's (b): a drop behind a pointer, `*it = other`'s or a capture's
// `d = ..`, compared whole, rustc's place read through `Derefer`'s copies;
// and two of rust-js's drops written alike at one place, `?`'s, compared as
// a multiset of what each side drops there, a dead one left out.
const behind = ["closure_in_closure_sets_a_capture.rs", "associated_iterator_dropped.rs"];
const alike = ["default_matches_its_tuple_whole.rs", "array_parts_moved_in_a_copied_default.rs"];

test("a drop behind a pointer is compared whole, as the writer drops it", () => {
  const lines = shadowOf("associated_iterator_dropped.rs").filter(([, path]) => path === "normal");
  const put = lines.filter(([, , , at]) => at.includes(":67:5:"));
  expect(put.map(([verdict, , , , detail]) => `${verdict} ${detail}`)).toEqual(["agree behind a pointer, whole"]);
  expect(lines.filter(([verdict]) => verdict !== "agree")).toEqual([]);
  // A capture's `d = D(..)`, rustc's `drop((*_t))` of `_t = copy ((*_1).0)`.
  const capture = shadowOf("closure_in_closure_sets_a_capture.rs").filter(([, path]) => path === "normal");
  expect(capture.filter(([verdict]) => verdict !== "agree")).toEqual([]);
  expect(capture.filter(([, , , , detail]) => detail === "behind a pointer, whole").length).toBe(1);
});

test("two drops written alike are compared as a multiset", () => {
  const lines = shadowOf("default_matches_its_tuple_whole.rs").filter(([, path]) => path === "normal");
  const at = lines.filter(([, , , where]) => where.includes(":22:37:")).map(([verdict]) => verdict);
  expect(at.sort()).toEqual(["agree", "agree"]);
});

// The normal path's verdicts over these cases, each pinned: none disagrees,
// the baseline a refusal of a disagreement (change D's (c)) starts from.
test("each normal-path verdict of the partial moves, behind a pointer and alike, is pinned", () => {
  const tally = new Map<string, number>();
  for (const name of [...partial, ...behind, ...alike]) {
    for (const [verdict, path] of shadowOf(name)) {
      if (path === "normal") tally.set(verdict, (tally.get(verdict) ?? 0) + 1);
    }
  }
  expect(Object.fromEntries(["agree", "disagree", "flag-ours", "unmatched"].map((v) => [v, tally.get(v) ?? 0]))).toEqual({
    agree: 193,
    disagree: 0,
    "flag-ours": 4,
    unmatched: 8,
  });
});
