// What the compiler writes of an `Option` of an `Option` a variable holds
// as its payload (ADR 1017): its `None` `undefined`, its `Some(None)`
// `null`. Each test is a program of its own, compiled.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildReact, compiler, fixture, run, target } from "./support";

beforeAll(buildReact, 600_000);

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

// rd547: React's Flight client's chunk cache, a `Map<string, null |
// Promise<any>>`, read in its thread-local's `with_borrow`, whose closure's
// result its own body holds as the value, `null` once loaded, and gives back
// to the local that holds it so: `chunkCache.get(chunkId)`, tested `===
// undefined` and `!== null`, as upstream's (ADR 1017, amended).
test("a thread-local's with_borrow gives back a map's value held as it is", async () => {
  const dir = fixture("held-with-borrow");
  writeFileSync(
    join(dir, "lib.rs"),
    `use std::cell::RefCell;
use std::collections::HashMap;

use js::{Promise, Unknown};

#[derive(Clone)]
#[cfg_attr(rust_js, rust_js::untagged)]
pub enum ChunkEntry {
    Pending(Promise<Option<&'static Unknown>>),
    #[cfg_attr(rust_js, rust_js::null)]
    Loaded,
}

thread_local! {
    static CHUNK_CACHE: RefCell<HashMap<String, ChunkEntry>> = RefCell::new(HashMap::new());
}

pub fn preload(
    chunk_id: &str,
    promises: &mut Vec<Promise<Option<&'static Unknown>>>,
    make: &dyn Fn() -> Promise<Option<&'static Unknown>>,
) {
    let entry = CHUNK_CACHE.with_borrow(|chunk_cache| chunk_cache.get(chunk_id).cloned());
    match entry {
        None => {
            let thenable = make();
            CHUNK_CACHE.with_borrow_mut(|chunk_cache| {
                chunk_cache.insert(chunk_id.to_string(), ChunkEntry::Pending(thenable.clone()))
            });
            promises.push(thenable);
        }
        Some(ChunkEntry::Pending(entry)) => promises.push(entry),
        Some(ChunkEntry::Loaded) => {}
    }
}

pub fn loaded(chunk_id: &str) {
    CHUNK_CACHE.with_borrow_mut(|chunk_cache| chunk_cache.insert(chunk_id.to_string(), ChunkEntry::Loaded));
}
`,
  );
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("(chunkCache) => chunkCache.get(chunkId)");
  expect(js).toContain("if (entry === undefined) {");
  expect(js).toContain("promises.push(entry);");
  expect(js).not.toContain("$someValue");
  const lib = await import(join(dir, "lib.js"));
  const made = Promise.resolve(1);
  const promises: unknown[] = [];
  lib.preload("a", promises, () => made);
  lib.preload("a", promises, () => Promise.resolve(2));
  lib.loaded("b");
  lib.preload("b", promises, () => Promise.resolve(3));
  expect(promises).toStrictEqual([made, made]);
});
