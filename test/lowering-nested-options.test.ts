// What the compiler writes of an `Option` of an `Option` a variable holds
// as its payload (ADR 1017): its `None` `undefined`, its `Some(None)`
// `null`. Each test is a program of its own, compiled.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { runSync } from "./child";
import { buildReact, compiler, fixture, root, run, target } from "./support";

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

// The user's decision (ADRs 0275, 0810, amended): an optional nullable
// member of a dictionary a function only takes, `BodyInit? body` of
// `RequestInit`, `EventTarget? relatedTarget` of `MouseEventInit`, is an
// `Option<Option<T>>` as #470's are, so a program can give the `null` the
// member takes: `None` leaves it out, `Some(None)` is `null`, `Some(Some(v))`
// the value; a read of one tells the three apart.
test("a dictionary a function only takes gives its optional nullable member's null", async () => {
  const dir = fixture("param-dictionary-nullable-member");
  writeFileSync(
    join(dir, "lib.rs"),
    `use webapi::{AbortSignal, BodyInit, EventTarget, MouseEventInit, RequestInit};

pub fn request<'a>(at: u32, text: &'a str, signal: &'a AbortSignal) -> RequestInit<'a> {
    RequestInit {
        method: Some("POST"),
        body: match at {
            0 => None,
            1 => Some(None),
            _ => Some(Some(BodyInit::Str(text))),
        },
        signal: if at == 1 { Some(None) } else { Some(Some(signal)) },
        ..Default::default()
    }
}

pub fn mouse(target: Option<&EventTarget>) -> MouseEventInit<'_> {
    MouseEventInit {
        related_target: Some(target),
        client_x: Some(3.0),
        ..Default::default()
    }
}

pub fn body(init: &RequestInit) -> &'static str {
    match init.body {
        None => "missing",
        Some(None) => "null",
        Some(Some(_)) => "value",
    }
}
`,
  );
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "--extern", `webapi=${join(target, "libwebapi.rmeta")}`, "-L", target]);
  const lib = await import(join(dir, "lib.js"));
  const signal = { aborted: false };
  const made = [0, 1, 2].map((at) => lib.request(at, "t", signal));
  // `None` is `undefined`, which WebIDL reads as left out (ADR 0810).
  expect(made.map((init: { body?: unknown }) => init.body)).toStrictEqual([undefined, null, "t"]);
  expect(made.map((init: { signal?: unknown }) => init.signal)).toEqual([signal, null, signal]);
  const related = {};
  expect(lib.mouse(undefined)).toEqual({ relatedTarget: null, clientX: 3 });
  expect(lib.mouse(related).relatedTarget).toBe(related);
  expect([{}, { body: null }, { body: "t" }].map(lib.body)).toEqual(["missing", "null", "value"]);
});

// rd645 (the user's (B)): a `#[rust_js::nullable]` field of an `Option` of
// a value never `null` that may be `undefined`, upstream's `serverModuleMap:
// null | ServerManifest` of `type ServerManifest = void`, holds the value
// itself: `null` for `None`, `undefined` for its `Some`, tested `!== null`.
// `js::any_of` of it is what it holds, given to createResponse as
// upstream's read; of a box, what the box holds. An enum that may be `null`
// too keeps the box. A JS-made `{ serverModuleMap: undefined }` is a `Some`.
test("a nullable field of what may be undefined holds the value itself", async () => {
  const dir = fixture("nullable-field-of-undefined");
  writeFileSync(
    join(dir, "lib.rs"),
    `use js::Unknown;

#[cfg_attr(rust_js, rust_js::untagged)]
pub enum ServerManifest {
    #[cfg_attr(rust_js, rust_js::undefined)]
    Undefined,
}

#[cfg_attr(rust_js, rust_js::untagged)]
pub enum Slot {
    #[cfg_attr(rust_js, rust_js::undefined)]
    Missing,
    #[cfg_attr(rust_js, rust_js::null)]
    Cleared,
}

pub struct ServerConsumerManifest {
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub server_module_map: Option<&'static ServerManifest>,
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub slot: Option<&'static Slot>,
}

pub struct Options {
    pub server_consumer_manifest: &'static ServerConsumerManifest,
}

#[cfg_attr(rust_js, rust_js::nullable(server_reference_config))]
fn create_response(server_reference_config: Option<&'static Unknown>) -> Option<&'static Unknown> {
    server_reference_config
}

pub fn given(options: &'static Options) -> Option<&'static Unknown> {
    create_response(js::any_of(options.server_consumer_manifest.server_module_map))
}

pub fn present(manifest: &ServerConsumerManifest) -> bool {
    manifest.server_module_map.is_some()
}

pub fn slot_present(manifest: &ServerConsumerManifest) -> bool {
    manifest.slot.is_some()
}

pub fn kept(manifest: &ServerConsumerManifest) -> Option<&'static Unknown> {
    let map = manifest.server_module_map;
    let tested = map.is_some();
    if tested { js::any_of(map) } else { None }
}

pub fn boxed(some: bool) -> Option<&'static Unknown> {
    let map = if some { Some(&ServerManifest::Undefined) } else { None };
    shown(map);
    js::any_of(map)
}

fn shown<T>(_: T) {}

pub fn made(some: bool) -> ServerConsumerManifest {
    ServerConsumerManifest {
        server_module_map: if some { Some(&ServerManifest::Undefined) } else { None },
        slot: None,
    }
}

pub fn nothing() -> Option<&'static Unknown> {
    js::any_of(())
}
`,
  );
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("return createResponse(options.serverConsumerManifest.serverModuleMap);");
  expect(js).toContain("return manifest.serverModuleMap !== null;");
  expect(js).toContain("return manifest.slot != null;");
  expect(js).toContain("const map = manifest.serverModuleMap;\n  const tested = map !== null;");
  expect(js).toContain("serverModuleMap: some ? undefined : null,");
  const dts = readFileSync(join(dir, "lib.d.ts"), "utf8");
  expect(dts).toContain("serverModuleMap: ServerManifest | null;");
  const lib = await import(join(dir, "lib.js"));
  const manifests = [{ serverModuleMap: undefined }, { serverModuleMap: null }, {}];
  expect(manifests.map(lib.present)).toEqual([true, false, true]);
  expect(manifests.map((m) => lib.given({ serverConsumerManifest: m }))).toStrictEqual([undefined, null, undefined]);
  expect(manifests.map(lib.kept)).toStrictEqual([undefined, undefined, undefined]);
  expect([{ slot: undefined }, { slot: null }].map(lib.slotPresent)).toEqual([false, false]);
  expect([true, false].map(lib.boxed)).toStrictEqual([undefined, null]);
  expect([true, false].map((some) => lib.present(lib.made(some)))).toEqual([true, false]);
  expect(lib.nothing()).toBe(undefined);
});

// rd645: such a field can't be `#[rust_js::optional]`, whose absent key
// would be its `Some`, nor borrowed `&mut`, which would write a box in it;
// and `js::any_of` of a value that's always there is still refused.
test("a nullable field of what may be undefined refuses what would box it", () => {
  const dir = fixture("nullable-field-of-undefined-refused");
  const lib = (body: string) =>
    `use js::Unknown;

#[cfg_attr(rust_js, rust_js::untagged)]
pub enum ServerManifest {
    #[cfg_attr(rust_js, rust_js::undefined)]
    Undefined,
}

${body}
`;
  const refused = (name: string, body: string) => {
    writeFileSync(join(dir, `${name}.rs`), lib(body));
    const flags = ["--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target, "--crate-name", name];
    return runSync([compiler, join(dir, `${name}.rs`), "-o", join(dir, `${name}.js`), ...flags], root, 60_000).stderr;
  };
  expect(
    refused(
      "optional",
      `pub struct Manifest {
    #[cfg_attr(rust_js, rust_js::nullable, rust_js::optional)]
    pub map: Option<&'static ServerManifest>,
}`,
    ),
  ).toContain("holds its `Some` as it is, `undefined` too, so it can't be `#[rust_js::optional]`");
  expect(
    refused(
      "borrowed",
      `pub struct Manifest {
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub map: Option<&'static ServerManifest>,
}

pub fn taken(manifest: &mut Manifest) -> bool {
    manifest.map.take().is_some()
}

pub fn unknown() -> Option<&'static Unknown> {
    None
}`,
    ),
  ).toContain("a `&mut` to a `#[rust_js::nullable]` field that holds its `Option` as it's made");
  expect(
    refused(
      "known",
      `pub fn known() -> Option<&'static Unknown> {
    js::any_of(5u32)
}`,
    ),
  ).toContain("`js::any_of` is of a type parameter's value, or of one that may be `undefined` or `null` as it is");
});
