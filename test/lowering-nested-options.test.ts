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

// #470 (P0, ADRs 0051 and 0810, amended, the user's decision): a
// dictionary's `#[rust_js::nullable]` `Option<Option<T>>` field, WebIDL's
// optional nullable member, is what JS holds: no key for `None`, `null` for
// `Some(None)`, the value for `Some(Some(v))`. `reader.read()`'s `{ value:
// null }` is `Some(None)`, `{}` is `None`, its `flatten()` the read itself,
// a destructured copy too; and a candidate made `Some(None)` gives the
// browser `sdpMid: null`, not a box.
test("a dictionary's nullable Option of an Option is what JS holds", async () => {
  const dir = fixture("nested-option-dictionary");
  writeFileSync(
    join(dir, "lib.rs"),
    `use js::Unknown;
use webapi::{RTCIceCandidateInit, ReadableStreamReadResult};

pub fn has_value(entry: &ReadableStreamReadResult) -> bool {
    entry.value.is_some()
}

pub fn describe(entry: &ReadableStreamReadResult) -> &'static str {
    match entry.value {
        None => "missing",
        Some(None) => "null",
        Some(Some(_)) => "value",
    }
}

pub fn nothing(entry: &ReadableStreamReadResult) -> bool {
    matches!(entry.value, Some(None))
}

pub fn value_of(entry: &ReadableStreamReadResult) -> Option<&'static Unknown> {
    entry.value.flatten()
}

pub fn present(entry: &ReadableStreamReadResult) -> Option<&'static Unknown> {
    if let Some(Some(value)) = &entry.value {
        return Some(*value);
    }
    None
}

pub fn chunk(ReadableStreamReadResult { done, value }: ReadableStreamReadResult) -> Option<&'static Unknown> {
    if done.unwrap_or(false) {
        return None;
    }
    value.flatten()
}

pub fn candidate(at: u32) -> RTCIceCandidateInit {
    RTCIceCandidateInit {
        candidate: Some("c".to_string()),
        sdp_mid: if at == 0 {
            None
        } else if at == 1 {
            Some(None)
        } else {
            Some(Some("m".to_string()))
        },
        ..Default::default()
    }
}

pub fn mid(init: &RTCIceCandidateInit) -> String {
    format!("{:?}", init.sdp_mid)
}
`,
  );
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "--extern", `webapi=${join(target, "libwebapi.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("return entry.value !== undefined;");
  expect(js).toContain("return entry.value === null;");
  expect(js).toContain("export function valueOf(entry) {\n  return entry.value;\n}");
  expect(js).toContain("export function present(entry) {\n  if (entry.value != null) {\n    return entry.value;\n  }\n}");
  expect(js).toContain("export function chunk({ done, value }) {\n  if (done) {\n    return;\n  }\n  return value;\n}");
  const lib = await import(join(dir, "lib.js"));
  const entries = [{ done: true }, { value: undefined, done: true }, { value: null, done: false }, { value: 0, done: false }, { value: "a", done: false }];
  expect(entries.map(lib.hasValue)).toEqual([false, false, true, true, true]);
  expect(entries.map(lib.describe)).toEqual(["missing", "missing", "null", "value", "value"]);
  expect(entries.map(lib.nothing)).toEqual([false, false, true, false, false]);
  expect(entries.map(lib.valueOf)).toEqual([undefined, undefined, null, 0, "a"]);
  expect(entries.map(lib.present)).toEqual([undefined, undefined, undefined, 0, "a"]);
  expect(entries.map(lib.chunk)).toEqual([undefined, undefined, null, 0, "a"]);
  expect([0, 1, 2].map((at) => JSON.stringify(lib.candidate(at)))).toEqual([
    '{"candidate":"c"}',
    '{"candidate":"c","sdpMid":null}',
    '{"candidate":"c","sdpMid":"m"}',
  ]);
  expect([{}, { sdpMid: null }, { sdpMid: "m" }].map(lib.mid)).toEqual(["None", "Some(None)", 'Some(Some("m"))']);
});

// A dictionary's required nullable member, `required DOMRectInit?
// rootBounds`, is an `Option` whose `None` is `null`, as JS requires it
// there, `#[rust_js::nullable]` (ADR 0275): a `None` the program gives is
// `rootBounds: null`, never left out.
test("a dictionary's required nullable member's None is null", async () => {
  const dir = fixture("required-nullable-member");
  writeFileSync(
    join(dir, "lib.rs"),
    `use webapi::{DOMRectInit, Element, IntersectionObserverEntryInit};

pub fn init(target: &'static Element, root: Option<f64>) -> IntersectionObserverEntryInit<'static> {
    IntersectionObserverEntryInit {
        time: 1.0,
        root_bounds: root.map(|width| DOMRectInit { width: Some(width), ..Default::default() }),
        bounding_client_rect: DOMRectInit::default(),
        intersection_rect: DOMRectInit::default(),
        is_intersecting: false,
        is_visible: false,
        intersection_ratio: 0.0,
        target,
    }
}
`,
  );
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "--extern", `webapi=${join(target, "libwebapi.rmeta")}`, "-L", target]);
  const lib = await import(join(dir, "lib.js"));
  const element = {};
  expect(lib.init(element, undefined).rootBounds).toBe(null);
  expect("rootBounds" in lib.init(element, undefined)).toBe(true);
  expect(lib.init(element, 4).rootBounds).toEqual({ width: 4 });
});
