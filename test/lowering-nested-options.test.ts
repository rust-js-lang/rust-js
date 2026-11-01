// What the compiler writes of an `Option` of an `Option` a variable holds
// as its payload (ADR 1017): its `None` `undefined`, its `Some(None)`
// `null`. Each test is a program of its own, compiled.

import { expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { compiler, fixture, run } from "./support";

// `matches!(last, Some(None))` is `last === null`, its two tests one where
// they're of one place; a test of `undefined` beside another place's of
// `null` stays two (ADR 1017, amended).
test("an Option of an Option's Some(None) is === null", () => {
  const dir = fixture("nested-option-null");
  writeFileSync(
    join(dir, "lib.rs"),
    `fn counted(values: &[Option<u32>], plain: Option<u32>, wanted: usize) -> u32 {
    let mut last = None;
    let mut seen = 0;
    for (i, value) in values.iter().enumerate() {
        if i == wanted {
            last = Some(*value);
        }
        if matches!(last, Some(None)) {
            seen += 1;
        }
        if matches!(last, Some(_)) && plain.is_none() {
            seen += 10;
        }
    }
    seen
}

pub fn shown(values: &[Option<u32>], plain: Option<u32>) -> String {
    format!("{}", counted(values, plain, 1))
}
`,
  );
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("if (last === null) {");
  expect(js).toContain("last !== undefined && plain == null");
});

// rd489 (P0): the inner `Option` of one held as its payload, given on, is
// the `None` it is, `undefined`, never the `null` its `Some(None)` is held
// as: `nums(undefined, f)` called `f(null)`. One only tested keeps its
// payload, `inner.is_some()` read `last != null` (ADR 1017, amended).
test("the None an Option of an Option gives on is undefined", async () => {
  const dir = fixture("nested-option-given-on");
  writeFileSync(
    join(dir, "lib.rs"),
    `pub struct Point {
    pub x: i32,
}

pub fn nums(v: Option<u32>, f: &dyn Fn(Option<u32>)) {
    let o = if v.is_some() { Some(v) } else { Some(None) };
    if let Some(v) = o {
        f(v);
    }
}

pub fn strs(v: Option<&'static str>, f: &dyn Fn(Option<&'static str>)) {
    let o = if v.is_some() { Some(v) } else { Some(None) };
    if let Some(v) = o {
        f(v);
    }
}

pub fn points(points: &'static [Option<Point>], f: &dyn Fn(&Option<Point>)) {
    let mut last = None;
    for point in points {
        last = Some(point);
    }
    if let Some(point) = last {
        f(point);
    }
}

pub fn derefs(values: &[Option<u32>], f: &dyn Fn(Option<u32>)) {
    let mut last = None;
    for value in values {
        last = Some(value);
    }
    if let Some(&value) = last {
        f(value);
    }
}

pub fn returned(v: Option<u32>) -> Option<u32> {
    let o = if v.is_some() { Some(v) } else { Some(None) };
    match o {
        Some(inner) => inner,
        None => Some(0),
    }
}

pub fn tested(values: &[Option<u32>]) -> bool {
    let mut last = None;
    for value in values {
        last = Some(*value);
    }
    if let Some(inner) = last { inner.is_some() } else { false }
}
`,
  );
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("last = value ?? null;");
  expect(js).toContain("return last != null;");
  const lib = await import(join(dir, "lib.js"));
  const given = (call: (f: (v: unknown) => void) => void) => {
    const seen: unknown[] = [];
    call((v) => seen.push(v));
    return seen;
  };
  expect([undefined, 0, 2].map((v) => given((f) => lib.nums(v, f)))).toStrictEqual([[undefined], [0], [2]]);
  expect([undefined, "", "a"].map((v) => given((f) => lib.strs(v, f)))).toStrictEqual([[undefined], [""], ["a"]]);
  expect([[{ x: 1 }, undefined], [undefined, { x: 2 }], []].map((p) => given((f) => lib.points(p, f)))).toStrictEqual([
    [undefined],
    [{ x: 2 }],
    [],
  ]);
  expect([[1, undefined], [undefined, 2], []].map((v) => given((f) => lib.derefs(v, f)))).toStrictEqual([[undefined], [2], []]);
  expect([undefined, 0, 3].map(lib.returned)).toStrictEqual([undefined, 0, 3]);
  expect([[], [1, undefined], [undefined, 1]].map(lib.tested)).toStrictEqual([false, false, true]);
});
