// How a `match` MIR tests as a tree reads as the chain of its arms, of the
// paths the tree takes to each: paths alike but for one value's test are
// one, an or-pattern no alternative of holds is no path, and an
// alternative tests a tuple's parts in the order written (ADR 0364). Each
// test is a program of its own, compiled and run.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildCompiler, compiler, fixture, run } from "./support";

beforeAll(buildCompiler, 600_000);

// A tuple's match whose first arm tests both values, `(0, 0) => ..`, then
// an or-pattern's, `(1, _) | (_, 1) => ..`: that arm's paths, `a === 0 &&
// b === 1`, `a === 1` and `a !== 0 && a !== 1 && b === 1`, are `a === 1 ||
// b === 1`, the first and last one path, `a !== 1 && b === 1`, whose `a
// !== 1` the second's failing says; and the later or-pattern's path past
// `a === 0` none, as none of its alternatives holds there. The chain as
// written, not MIR's labeled block (#631, ADR 0364, amended).
test("a tuple's or-pattern after an arm that tests both values is an else if", async () => {
  const dir = fixture("absorbed-or-pattern-paths");
  writeFileSync(join(dir, "lib.rs"), `pub fn assigned(a: u32, b: u32, log: &mut Vec<u32>) {
    let value = match (a, b) {
        (0, 0) => {
            log.push(0);
            1
        }
        (1, _) | (_, 1) => 2,
        (2, 3) | (3, 2) => {
            log.push(a);
            7
        }
        _ => a + b,
    };
    log.push(value);
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).not.toMatch(/\bbb\d+:/);
  expect(js).toContain(
    "  if (a === 0 && b === 0) {\n    log.push(0);\n    value = 1;\n  } else if (a === 1 || b === 1) {\n    value = 2;\n  } else if (",
  );
  const assigned = (await import(join(dir, "lib.js"))).assigned as (a: number, b: number, log: number[]) => void;
  const log: number[] = [];
  for (const [a, b] of [[0, 0], [0, 1], [1, 4], [4, 1], [2, 3], [3, 2], [0, 3], [2, 2], [4, 4]]) assigned(a, b, log);
  expect(log).toEqual([0, 1, 2, 2, 2, 2, 7, 3, 7, 3, 4, 8]);
});

// An or-pattern's alternatives of a tuple's parts, `(2, 3) | (3, 2)`, each
// test its parts in the order written, `a === 2 && b === 3`, where MIR
// tests `b` first after `(_, 1)`'s `b === 1`; a part's read neither calls
// nor throws, so none guards another. What guards them stays first, an
// `Option`'s `x != null`, a slice's `length` (#632, ADR 0364, amended).
test("an or-pattern's alternatives test a tuple's parts in the order written", async () => {
  const dir = fixture("alternatives-in-written-order");
  writeFileSync(join(dir, "lib.rs"), `pub fn pair(a: u32, b: u32) -> u32 {
    match (a, b) {
        (1, _) | (_, 1) => 1,
        (2, 3) | (3, 2) => 2,
        _ => 0,
    }
}

pub fn guarded(x: Option<(u32, u32)>) -> u32 {
    match x {
        Some((1, _)) | Some((_, 1)) => 1,
        Some((2, 3)) | Some((3, 2)) => 2,
        _ => 0,
    }
}

pub fn sliced(xs: &[u32]) -> u32 {
    match xs {
        [1, _] | [_, 1] => 1,
        [2, 3] | [3, 2] => 2,
        _ => 0,
    }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("  if ((a === 2 && b === 3) || (a === 3 && b === 2)) {\n    return 2;\n  }\n");
  expect(js).toContain("  if (x != null) {\n    if (x[0] === 1 || x[1] === 1) {");
  expect(js).toContain("    if ((x[0] === 2 && x[1] === 3) || (x[0] === 3 && x[1] === 2)) {");
  expect(js).toContain("  if (xs.length === 2) {\n    if (xs[0] === 1 || xs[1] === 1) {");
  const lib = await import(join(dir, "lib.js"));
  const pairs = [[1, 5], [5, 1], [2, 3], [3, 2], [2, 2], [3, 3], [0, 0]];
  expect(pairs.map(([a, b]) => lib.pair(a, b))).toEqual([1, 1, 2, 2, 0, 0, 0]);
  expect([...pairs.map((p) => lib.guarded(p)), lib.guarded(undefined)]).toEqual([1, 1, 2, 2, 0, 0, 0, 0]);
  expect([...pairs.map((p) => lib.sliced(p)), lib.sliced([2, 3, 4]), lib.sliced([])]).toEqual([1, 1, 2, 2, 0, 0, 0, 0, 0]);
});

// A match of three values whose or-pattern names one each, `(1, _, _) |
// (_, 1, _) | (_, _, 1)`, is the chain as written: a path whose test's
// failing says another of its arm's needs no test of it, `(a === 2 && b
// === 2) || (b === 2 && c === 2)`, not `.. || (a !== 2 && b === 2 && c ===
// 2)`; what the arms before it say failing, `a !== 1`, `b !== 1` and `c !==
// 1`, an arm after needs no test of; and the `_` is the `else`, not the
// `|`'s arm, of four values too. A test that guards a read stays, `x !=
// null` before `x[0]` (#673, ADR 0364, amended).
test("a match of three values' or-pattern is the chain as written", async () => {
  const dir = fixture("three-value-or-pattern");
  writeFileSync(join(dir, "lib.rs"), `pub fn triple(a: u8, b: u8, c: u8) -> u8 {
    match (a, b, c) {
        (0, 0, 0) => 1,
        (1, _, _) | (_, 1, _) | (_, _, 1) => 2,
        (2, 2, _) | (_, 2, 2) => 3,
        _ => 4,
    }
}

pub fn quad(a: u8, b: u8, c: u8, d: u8) -> u8 {
    match (a, b, c, d) {
        (0, 0, 0, 0) => 1,
        (1, _, _, _) | (_, 1, _, _) | (_, _, 1, _) | (_, _, _, 1) => 2,
        (2, 2, _, _) | (_, _, 2, 2) => 3,
        _ => 4,
    }
}

pub fn first_two(a: u32, b: u32) -> u32 {
    match (a, b) {
        (2, 0) => 1,
        (1, _) | (_, 1) => 2,
        (2, 3) | (3, 2) => 3,
        _ => 4,
    }
}

pub fn guarded(x: Option<(u8, u8)>, k: u8) -> u8 {
    match (x, k) {
        (None, 5) | (Some((1, _)), 5) => 1,
        (Some((_, 1)), _) | (None, 1) => 2,
        (Some((2, 2)), _) | (_, 2) => 3,
        _ => 4,
    }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).not.toMatch(/\bbb\d+:/);
  expect(js).toContain(
    "  if (a === 0 && b === 0 && c === 0) {\n    return 1;\n  }\n  if (a === 1 || b === 1 || c === 1) {\n    return 2;\n  }\n  if ((a === 2 && b === 2) || (b === 2 && c === 2)) {\n    return 3;\n  }\n  return 4;\n",
  );
  expect(js).toContain(
    "  if (a === 1 || b === 1 || c === 1 || d === 1) {\n    return 2;\n  }\n  if ((a === 2 && b === 2) || (c === 2 && d === 2)) {\n    return 3;\n  }\n  return 4;\n",
  );
  expect(js).toContain(
    "  if (a === 1 || b === 1) {\n    return 2;\n  }\n  if ((a === 2 && b === 3) || (a === 3 && b === 2)) {\n    return 3;\n  }\n  return 4;\n",
  );
  const lib = await import(join(dir, "lib.js"));
  const values = [0, 1, 2, 3];
  const triples = values.flatMap((a) => values.flatMap((b) => values.map((c) => lib.triple(a, b, c))));
  // Native Rust's, `triple` of each `(a, b, c)` of `0..4`.
  const triple = (a: number, b: number, c: number) =>
    a === 0 && b === 0 && c === 0 ? 1 : a === 1 || b === 1 || c === 1 ? 2 : (a === 2 && b === 2) || (b === 2 && c === 2) ? 3 : 4;
  expect(triples).toEqual(values.flatMap((a) => values.flatMap((b) => values.map((c) => triple(a, b, c)))));
  expect([[2, 0], [1, 4], [4, 1], [2, 3], [3, 2], [2, 2], [0, 0]].map(([a, b]) => lib.firstTwo(a, b))).toEqual([1, 2, 2, 3, 3, 4, 4]);
  expect([lib.quad(0, 0, 0, 0), lib.quad(0, 0, 0, 1), lib.quad(2, 2, 0, 0), lib.quad(0, 0, 2, 2), lib.quad(2, 0, 0, 2)]).toEqual([1, 2, 3, 3, 4]);
  const options = [undefined, [1, 0], [0, 1], [2, 2], [0, 0]];
  expect(options.flatMap((x) => [0, 1, 2, 5].map((k) => lib.guarded(x, k)))).toEqual([
    4, 2, 3, 1, 4, 4, 3, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 3, 4,
  ]);
});
