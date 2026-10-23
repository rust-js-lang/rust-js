// What the compiler writes of an `Option`: `null` of a nullable, an optional
// chain where it's `None`, a field of `None` left out, a test by its truth,
// and `unwrap_or_else` and the rest of its methods. Each test is a program of
// its own, compiled and run.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildReact, compiler, fixture, root, run, target } from "./support";

beforeAll(buildReact, 600_000);

// A binding's parameter it names `#[rust_js::nullable(..)]` is `T | null`,
// as a field is (ADR 0275): its `None` is `null`, `JSON.stringify(value,
// null, 2)` as react.dev's Sandpack template writes it.
test("a binding's nullable parameter's None is null", async () => {
  const dir = fixture("nullable-param");
  writeFileSync(join(dir, "lib.rs"), `#[cfg_attr(rust_js, rust_js::link_name = "JSON.stringify")]
#[cfg_attr(rust_js, rust_js::nullable(replacer))]
#[allow(unused_variables)]
fn stringify(value: &[u32], replacer: Option<&[&str]>, space: u32) -> String {
    unreachable!()
}

pub fn plain(items: &[u32]) -> String {
    stringify(items, None, 2)
}

pub fn given(items: &[u32], keys: Option<&[&str]>) -> String {
    stringify(items, keys, 2)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("return JSON.stringify(items, null, 2);");
  expect(js).toContain("return JSON.stringify(items, keys ?? null, 2);");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.plain([1, 2]), lib.given([3], undefined)]).toEqual(["[\n  1,\n  2\n]", "[\n  3\n]"]);
});

// An Option compared to a literal is only that comparison, which none fails
// anyway: `segments.at(-1) === "index"`, as react.dev's page route drops a
// path's last `index`.
test("an Option compared to a literal needs no test that it's there", async () => {
  const dir = fixture("option-literal");
  writeFileSync(join(dir, "lib.rs"), `pub fn trimmed(mut segments: Vec<String>) -> Vec<String> {
    if segments.last().is_some_and(|last| last == "index") {
        segments.pop();
    }
    segments
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  // Read once, where it's tested (ADR 0591).
  expect(js).toContain('if (segments.at(-1) === "index") {');
  const lib = await import(join(dir, "mir.js"));
  expect([lib.trimmed(["a", "index"]), lib.trimmed(["a"]), lib.trimmed([])]).toEqual([["a"], ["a"], []]);
});

// ADR 0030: a property chain of an option's value ends where it's `None`,
// `o?.inner.v`, one chain, as `map` does: `(o?.inner).v` would read `.v` of
// `undefined`.
test("an option's property chain ends where it's None", async () => {
  const dir = fixture("optional-chains");
  writeFileSync(join(dir, "lib.rs"), `pub struct Inner {
    pub v: u32,
}

pub struct Outer {
    pub inner: Inner,
}

pub fn deep(o: Option<&Outer>) -> Option<u32> {
    o.map(|x| x.inner.v)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  expect(readFileSync(join(dir, "lib.js"), "utf8")).toContain("return o?.inner.v;");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.deep({ inner: { v: 3 } }), lib.deep(undefined)]).toEqual([3, undefined]);
});

// ADR 0030: a method of what's in an option, of a JS object, is `o?.m(x)`,
// which calls it, its arguments too, only where `o` is: as a person writes
// `document.body?.appendChild(a)`. A Rust type's method takes its value as
// an argument, `Node.add(o, x)`, which `?.` can't skip.
test("an option's method call ends where it's None", async () => {
  const dir = fixture("optional-calls");
  writeFileSync(join(dir, "lib.rs"), `pub struct Node {
    pub n: u32,
}

impl Node {
    pub fn add(&self, x: u32) -> u32 {
        self.n + x
    }
}

pub fn tested(o: Option<&js::RegExp>, x: &str) -> Option<bool> {
    o.map(|r| r.test(x))
}

pub fn ran(o: Option<&js::RegExp>, x: &str) {
    o.map(|r| r.test(x));
}

pub fn added(o: Option<&Node>, x: u32) -> Option<u32> {
    o.map(|node| node.add(x))
}

pub fn or_not(o: Option<&js::RegExp>, x: &str) -> bool {
    o.map_or(false, |r| r.test(x))
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("return o?.test(x);");
  expect(js).toContain("  o?.test(x);");
  expect(js).toContain("return o ? Node.add(o, x) : undefined;");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.tested(/a/, "a"), lib.tested(undefined, "a"), lib.ran(undefined, "a"), lib.added({ n: 1 }, 2), lib.added(undefined, 2), lib.orNot(undefined, "a")])
    .toEqual([true, undefined, undefined, 3, undefined, false]);
});

// ADR 0280: a field that's `undefined` is no key wherever its object is
// written: returned, in a `format!`'s template, or in an optional call,
// as a JS callback that asks `"value" in options` is answered the same.
test("an undefined field is no key in a template or an optional call", async () => {
  const dir = fixture("undefined-fields-anywhere");
  writeFileSync(join(dir, "lib.rs"), `pub struct Options {
    pub value: Option<u32>,
}

pub fn direct(f: impl Fn(Options) -> u32) -> u32 {
    f(Options { value: None })
}

pub fn formatted(f: impl Fn(Options) -> u32) -> String {
    format!("value {}", f(Options { value: None }))
}

#[cfg_attr(rust_js, rust_js::link_name = "keyed")]
#[allow(unused_variables)]
fn keyed(this: &js::JsObject, options: Options) -> u32 {
    unreachable!()
}

pub fn optional(o: Option<&js::JsObject>) -> Option<u32> {
    o.map(|o| keyed(o, Options { value: None }))
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).not.toContain("value: undefined");
  const lib = await import(join(dir, "lib.js"));
  const keyed = (options: object) => ("value" in options ? 1 : 0);
  expect(js).toContain("return o?.keyed({});");
  expect([lib.direct(keyed), lib.formatted(keyed), lib.optional({ keyed })]).toEqual([0, "value 0", 0]);
});

// ADR 0275: a `#[rust_js::nullable]` field is TypeScript's `T | null`: its
// `None` is `null`, as Next.js's `getStaticProps` gives react.dev's errors
// page its `errorCode`, which JSON has no `undefined` for. Another such
// field's value is one already, passed on as it is.
test("a nullable field's None is null", async () => {
  const dir = fixture("nullable-null");
  writeFileSync(join(dir, "lib.rs"), `pub struct Props {
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub code: Option<String>,
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub message: Option<String>,
    pub title: Option<String>,
}

pub fn none() -> Props {
    Props { code: None, message: Some("m".to_string()), title: None }
}

pub fn given(code: Option<String>) -> Props {
    Props { code, message: None, title: None }
}

pub fn passed(Props { code, message, .. }: Props) -> Props {
    Props { code, message, title: None }
}

pub fn chosen(code: Option<&str>) -> Props {
    Props {
        code: match code {
            Some(code) if !code.is_empty() => Some(code.to_string()),
            _ => None,
        },
        message: None,
        title: None,
    }
}

pub fn moved(p: Props) -> Props {
    Props { code: p.code, message: Some(p.title.unwrap_or_default()), title: None }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain('return { code: null, message: "m" };');
  expect(js).toContain("return { code: code ?? null, message: null };");
  expect(js).toContain("return { code, message };");
  // A conditional of `Some` or `None` is one of the value or `null`.
  expect(js).toContain("return { code: code || null, message: null };");
  expect(js).toContain('return { code: p.code, message: p.title ?? "" };');
  const lib = await import(join(dir, "mir.js"));
  expect([lib.given(undefined), lib.given("1")]).toEqual([{ code: null, message: null }, { code: "1", message: null }]);
  expect(JSON.stringify(lib.passed(lib.given(undefined)))).toBe('{"code":null,"message":null}');
  expect([lib.chosen("1").code, lib.chosen("").code, lib.chosen(undefined).code]).toEqual(["1", null, null]);
});

// ADR 0030: a getter's option mapped to a property of what's in it,
// `o.map(|e| e.id)`, is `o?.id`, which reads `o` once, as a `const` of it
// would, as react.dev's SocialBanner has `ref.current?.offsetHeight`.
test("a getter's option mapped to a property is an optional chain", async () => {
  const withWeb = ["--", "--extern", `webapi=${join(target, "libwebapi.rmeta")}`, "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("getter-chain");
  writeFileSync(join(dir, "lib.rs"), `use webapi::Element;

pub fn first_id(parent: &Element) -> Option<String> {
    parent.first_element_child().map(|child| child.id())
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withWeb]);
  expect(readFileSync(join(dir, "lib.js"), "utf8")).toContain("return parent.firstElementChild?.id;");
  const { firstId } = await import(join(dir, "lib.js"));
  expect([firstId({ firstElementChild: { id: "a" } }), firstId({ firstElementChild: null })]).toEqual(["a", undefined]);
});

// ADR 0280: a field made of a literal `None` is no key, `{ code }`, as
// hand-written JS leaves it out, and as react.dev's sandboxes give Sandpack
// `{ code, hidden, active }`. Rust can't tell: it reads a key that isn't
// there as `None`, compares the two alike, and hashes them alike.
test("a field of a literal None is left out", async () => {
  const dir = fixture("none-fields");
  writeFileSync(join(dir, "lib.rs"), `use std::collections::HashSet;

#[derive(PartialEq, Eq, Hash, Clone, Debug, Default)]
pub struct File {
    pub code: String,
    pub hidden: Option<bool>,
    pub read_only: Option<bool>,
}

pub fn literal(code: &str) -> File {
    File { code: code.to_string(), hidden: Some(true), read_only: None }
}

pub fn given(code: &str, read_only: Option<bool>) -> File {
    File { code: code.to_string(), hidden: Some(true), read_only }
}

pub fn defaulted(code: &str) -> File {
    File { code: code.to_string(), ..Default::default() }
}

#[derive(Clone, Copy)]
pub struct Flags {
    pub on: Option<bool>,
    pub off: Option<bool>,
}

pub fn reset(flags: &Flags) -> Flags {
    Flags { on: None, ..*flags }
}

pub fn alike(code: &str) -> (bool, usize, String) {
    let a = literal(code);
    let b = given(code, None);
    let set: HashSet<File> = [a.clone(), b.clone()].into_iter().collect();
    (a == b, set.len(), format!("{a:?}"))
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("return { code, hidden: true };");
  const lib = await import(join(dir, "lib.js"));
  expect(Object.keys(lib.literal("a"))).toEqual(["code", "hidden"]);
  expect(Object.keys(lib.defaulted("a"))).toEqual(["code"]);
  expect(lib.given("a", undefined).hidden).toBe(true);
  // Not after a spread, whose field it would be.
  expect(js).toContain("return { ...flags, on: undefined };");
  expect(lib.reset({ on: true, off: false })).toEqual({ on: undefined, off: false });
  expect(lib.alike("a")).toEqual([true, 1, 'File { code: "a", hidden: Some(true), read_only: None }']);
});

// A property of what may be `None`, `file.and_then(|f| js::get(f, "visible"))`,
// is `file?.visible`, and of a dictionary's, `files["/styles.css"]?.visible`,
// as react.dev's SandpackRoot reads it, and one of a variable's key,
// `files[name]?.active`: a key that's a name. Set where it's none, it's
// `counts[name] ??= ..`, as react.dev's CustomPreset counts lines. Another stays a test, `file != null ? file["a-b"] : undefined`.
test("a property read of what may be None is an optional chain", async () => {
  const withJs = ["--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("optional-property");
  writeFileSync(join(dir, "lib.rs"), `use js::{Dict, Unknown, dict};
pub struct File {
    pub active: Option<bool>,
}
pub fn active(files: &Dict<File>, name: &str) -> bool {
    dict::get(files, name).and_then(|f| f.active) == Some(true)
}
pub fn counted(counts: &Dict<f64>, name: &str, code: &str) -> f64 {
    if dict::get(counts, name).is_none() {
        dict::set(counts, name, code.split('\\n').count() as f64);
    }
    *dict::get(counts, name).unwrap()
}
pub fn styled(files: &Dict<&Unknown>) -> bool {
    !js::truthy(dict::get(files, "/styles.css").and_then(|f| js::get(*f, "visible")))
}
pub fn visible(file: Option<&Unknown>) -> bool {
    !js::truthy(file.and_then(|f| js::get(f, "visible")))
}
pub fn dashed(file: Option<&Unknown>) -> bool {
    js::truthy(file.and_then(|f| js::get(f, "a-b")))
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("return !file?.visible;");
  expect(js).toContain('return !files["/styles.css"]?.visible;');
  expect(js).toContain("return !!files[name]?.active;");
  expect(js).toContain('counts[name] ??= code.split("\\n").length;');
  expect(js).toContain('file != null ? file["a-b"] : undefined');
  const { visible, dashed, styled, active, counted } = await import(join(dir, "lib.js"));
  expect([counted({}, "a", "x\ny"), counted({ a: 7 }, "a", "x"), active({ a: { active: true } }, "a"), active({}, "a"), styled({}), styled({ "/styles.css": { visible: true } }), visible(undefined), visible({}), visible({ visible: 1 }), dashed({ "a-b": 1 }), dashed(undefined)]).toEqual([2, 7, true, false, true, false, true, true, false, true, false]);
});

// ADR 0266: text is falsy in JS only where it's empty, so text an option
// keeps where it isn't, `filter(|s| !s.is_empty())`, then another's or a
// default, is JS's `||`: `meta.title || route?.title || ""`, as react.dev's
// Page writes it. An array, which is truthy empty, isn't.
test("text kept where it isn't empty, or another, is ||", async () => {
  const dir = fixture("text-or");
  writeFileSync(join(dir, "lib.rs"), `pub struct Meta<'a> {
    pub title: Option<&'a str>,
}

pub struct Route {
    pub title: String,
}

pub fn title<'a>(meta: &Meta<'a>, route: Option<&'a Route>) -> &'a str {
    (meta.title.filter(|title| !title.is_empty()))
        .or(route.map(|route| route.title.as_str()).filter(|title| !title.is_empty()))
        .unwrap_or("")
}

pub fn items(list: Option<Vec<u32>>) -> Vec<u32> {
    list.filter(|list| !list.is_empty()).unwrap_or(vec![1])
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('return meta.title || route?.title || "";');
  expect(js).not.toContain("list ||");
  const { title, items } = await import(join(dir, "lib.js"));
  expect([title({ title: "" }, { title: "R" }), title({}, undefined), title({ title: "M" }, { title: "R" }), title({ title: "" }, { title: "" })]).toEqual(["R", "", "M", ""]);
  expect([items([]), items([2]), items(undefined)]).toEqual([[1], [2], [1]]);
});

// `Some(Direction::Up)` of an `Option` of a unit variant is `d === "Up"`:
// `undefined === "Up"` is false too, so it's said without `d != null`, as
// a constant is. Where the variant is an object's, `d.TAG`, it's needed.
test("a Some of a unit variant is tested as the variant, without a null test", async () => {
  const dir = fixture("option-unit-variant");
  writeFileSync(join(dir, "lib.rs"), 'pub enum Direction {\n    Up,\n    Down,\n}\npub enum Shape {\n    Dot,\n    Line(u32),\n}\npub fn turn(d: Option<Direction>) -> u32 {\n    match d {\n        Some(Direction::Up) => 1,\n        Some(Direction::Down) => 2,\n        None => 0,\n    }\n}\npub fn size(s: Option<Shape>) -> u32 {\n    match s {\n        Some(Shape::Dot) => 1,\n        Some(Shape::Line(n)) => n,\n        None => 0,\n    }\n}\n');
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect([js.includes('if (d === "Up")'), js.includes("d != null"), js.includes('if (s === "Dot")'), js.includes("s != null && s.TAG")]).toEqual([true, true, true, false]);
  const lib = await import(join(dir, "lib.js"));
  expect([lib.turn("Up"), lib.turn("Down"), lib.turn(undefined), lib.size("Dot"), lib.size({ TAG: "Line", _0: 7 }), lib.size(undefined)]).toEqual([1, 2, 0, 1, 7, 0]);
});

// `Option::as_deref` of a `String` or a `Vec` is the option itself: a
// `&str` is the string a `String` is, a slice the array (ADR 0211).
test("Option::as_deref of a String or a Vec is the option itself", async () => {
  const dir = fixture("option-as-deref");
  writeFileSync(join(dir, "lib.rs"), `pub fn path_is(path: Option<String>, want: &str) -> bool {
    path.as_deref() == Some(want)
}
pub fn first(items: Option<Vec<u32>>) -> Option<u32> {
    items.as_deref().and_then(|items| items.first().copied())
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).not.toMatch(/as_?deref/i);
  const lib = await import(join(dir, "lib.js"));
  expect([lib.pathIs("/learn", "/learn"), lib.pathIs("/a", "/learn"), lib.pathIs(undefined, "/learn")]).toEqual([true, false, false]);
  expect([lib.first([4, 5]), lib.first([]), lib.first(undefined)]).toEqual([4, undefined, undefined]);
});

// ADR 0578: a `#[rust_js::nullable]` function's result is `null` where it's
// written, so each `return` gives it as written, `return last;`, not a
// variable set in each branch and `return result ?? null;`. And an item's
// place through `index_mut` the body's ranges show in bounds is unchecked:
// React's heap's `pop`, `heap[0] = last`.
test("a nullable result is written where it's made, and an index_mut known in bounds is unchecked", async () => {
  const dir = fixture("nullable-written");
  writeFileSync(join(dir, "lib.rs"), `#[rust_js::nullable]
pub fn pop(heap: &mut Vec<u32>) -> Option<u32> {
    let last = heap.pop()?;
    if heap.is_empty() {
        return Some(last);
    }
    let first = std::mem::replace(&mut heap[0], last);
    Some(first)
}

#[rust_js::nullable]
pub fn first(items: &[u32]) -> Option<&u32> {
    items.first()
}

#[rust_js::nullable]
pub fn found(items: &[u32], flag: bool) -> Option<&u32> {
    let first = items.first();
    if flag && first.is_some() {
        return first;
    }
    items.last()
}

#[rust_js::nullable]
pub fn kept(items: &[u32]) -> Option<&u32> {
    let first = items.first();
    first
}

pub fn replaced(items: &mut Vec<u32>, i: usize, x: u32) -> u32 {
    if i < items.len() { std::mem::replace(&mut items[i], x) } else { x }
}

pub fn replaced_any(items: &mut Vec<u32>, i: usize, x: u32) -> u32 {
    std::mem::replace(&mut items[i], x)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("export function pop(heap) {\n  const last = heap.pop();\n  if (last == null) {\n    return null;\n  }\n  if (heap.length === 0) {\n    return last;\n  }\n  const first = heap[0];\n  heap[0] = last;\n  return first;\n}");
  expect(js).toContain("  return items[0] ?? null;\n");
  expect(js).toContain("    return first;\n");
  expect(js).toContain("export function replaced(items, i, x) {\n  if (i < items.length) {\n    const old = items[i];\n    items[i] = x;\n");
  expect(js).toContain("export function replacedAny(items, i, x) {\n  $index(items, i);\n");
  const lib = await import(join(dir, "lib.js"));
  const heap = [1, 2, 3];
  expect([lib.pop(heap), heap, lib.pop([4]), lib.pop([]), lib.first([]), lib.first([5]), lib.kept([])]).toEqual([1, [3, 2], 4, null, null, 5, null]);
  const items = [1, 2];
  expect([lib.replaced(items, 1, 7), lib.replaced(items, 5, 8), items]).toEqual([2, 8, [1, 7]]);
  expect(() => lib.replacedAny([1], 2, 0)).toThrow("index out of bounds");
});

// ADR 0638: a call of a `#[rust_js::nullable]` function, itself or
// another, a binding too, is `null` where it's `None` already, so a nullable
// result of it is the call, `return getComponentNameFromType(resolved);`,
// with no `?? null`; a call of any other function's `Option` keeps it.
test("a nullable result of a nullable function's call is the call", async () => {
  const dir = fixture("nullable-call");
  writeFileSync(join(dir, "lib.rs"), `#[cfg_attr(rust_js, rust_js::nullable)]
pub fn named(n: u32) -> Option<String> {
    if n == 0 {
        return None;
    }
    if n > 5 {
        return named(n - 1);
    }
    Some(format!("n{n}"))
}

#[cfg_attr(rust_js, rust_js::nullable)]
pub fn other(n: u32) -> Option<String> {
    named(n * 2)
}

#[cfg_attr(rust_js, rust_js::nullable)]
#[cfg_attr(rust_js, rust_js::link_name = "globalThis.lookup")]
#[allow(unused_variables)]
fn lookup(key: &str) -> Option<String> {
    unreachable!()
}

#[cfg_attr(rust_js, rust_js::nullable)]
pub fn looked(key: &str) -> Option<String> {
    lookup(key)
}

fn plain(n: u32) -> Option<u32> {
    if n > 1 { Some(n) } else { None }
}

#[cfg_attr(rust_js, rust_js::nullable)]
pub fn unnamed(n: u32) -> Option<u32> {
    plain(n)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("    return named(n - 1);\n");
  expect(js).toContain("export function other(n) {\n  return named((n * 2) >>> 0);\n}");
  expect(js).toContain("export function looked(key) {\n  return globalThis.lookup(key);\n}");
  // Of a function whose `None` is `undefined`, `null` for it.
  expect(js).toContain("export function unnamed(n) {\n  return plain(n) ?? null;\n}");
  const lib = await import(join(dir, "lib.js"));
  (globalThis as any).lookup = (key: string) => (key === "a" ? "A" : null);
  expect([lib.named(0), lib.named(3), lib.named(7), lib.other(0), lib.other(2), lib.other(4)]).toEqual([null, "n3", "n5", null, "n4", "n5"]);
  expect([lib.looked("a"), lib.looked("b"), lib.unnamed(0), lib.unnamed(2)]).toEqual(["A", null, null, 2]);
});

// ADR 0620: a nullable result of an `Option` a test it's under says is
// there, `typeof x === "function"`, is written as it is, with no `?? null`:
// React's `getIteratorFn`. Where the test fails, `x == null`'s, so after it.
test("an Option a test says is there is written as it is", async () => {
  const dir = fixture("present-coalesce");
  writeFileSync(join(dir, "lib.rs"), `use js::Unknown;

#[cfg_attr(rust_js, rust_js::nullable)]
pub fn callable(value: Option<&'static Unknown>) -> Option<&'static Unknown> {
    if js::type_of(value) == "function" {
        return value;
    }
    None
}

#[cfg_attr(rust_js, rust_js::nullable)]
pub fn logged<'a>(items: &'a [u32], log: &mut Vec<u32>) -> Option<&'a u32> {
    let first = items.first();
    if first.is_none() {
        log.push(0);
        return None;
    }
    log.push(1);
    first
}

pub fn shadowed(value: Option<u32>, values: &[Option<u32>]) -> Vec<u32> {
    if value.is_some() {
        let or_seven = |value: Option<u32>| value.unwrap_or(7);
        return values.iter().copied().map(or_seven).collect();
    }
    Vec::new()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("if (typeof value === \"function\") {\n    return value;\n  }\n  return null;");
  expect(js).toContain("  log.push(1);\n  return first;\n");
  expect(js).toContain("(value) => value ?? 7;");
  const lib = await import(join(dir, "lib.js"));
  expect(lib.shadowed(1, [2, undefined])).toEqual([2, 7]);
  const f = () => 1;
  const log: number[] = [];
  expect([lib.callable(f), lib.callable(undefined), lib.callable(3), lib.logged([], log), lib.logged([4], log), log]).toEqual([f, null, null, null, 4, [0, 1]]);
});

// ADR 0591: \`is_some_and\` of a comparison of its \`Some\` with a value of
// a type that's never \`undefined\` nor \`null\` is that comparison, which a
// \`None\` fails: React's scheduler's \`peek(taskQueue) === currentTask\`.
test("is_some_and of an equality with a defined value is the equality", async () => {
  const dir = fixture("is-some-and-equal");
  writeFileSync(join(dir, "lib.rs"), `use std::rc::Rc;

pub struct Task {
    pub id: u32,
}

pub fn first(tasks: &[Rc<Task>], task: &Rc<Task>) -> bool {
    tasks.first().is_some_and(|first| Rc::ptr_eq(first, task))
}

pub fn found(numbers: &[u32], n: u32) -> bool {
    numbers.iter().position(|&x| x == n).is_some_and(|at| at == 0)
}

pub fn big(numbers: &[u32]) -> bool {
    numbers.first().is_some_and(|&x| x > 9)
}

pub fn any_when(numbers: &[u32], n: u32) -> bool {
    numbers.first().is_some_and(|_| n == 5)
}

pub fn task(id: u32) -> Rc<Task> {
    Rc::new(Task { id })
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("export function first(tasks, task) {\n  return tasks[0] === task;\n}");
  expect(js).toContain("=== 0;");
  expect(js).toMatch(/!= null && .* > 9/);
  const lib = await import(join(dir, "lib.js"));
  const a = lib.task(1);
  expect([lib.first([a], a), lib.first([], a), lib.first([lib.task(1)], a)]).toEqual([true, false, false]);
  expect([lib.found([5, 6], 5), lib.found([5, 6], 6), lib.found([5], 7)]).toEqual([true, false, false]);
  expect([lib.big([10]), lib.big([]), lib.big([1])]).toEqual([true, false, false]);
  expect([lib.anyWhen([1], 5), lib.anyWhen([], 5)]).toEqual([true, false]);
});

// ADR 0294: `unwrap_unchecked()` is TypeScript's `x!`: the value, unchecked,
// as react.dev's Preview has `iframeRef.current!`. Rust leaves a `None`
// undefined behavior, so JS's `undefined` read on is the program's.
test("unwrap_unchecked is the value itself", async () => {
  const dir = fixture("unwrap-unchecked");
  writeFileSync(join(dir, "lib.rs"), `pub fn first(v: Option<&'static str>) -> &'static str {
    unsafe { v.unwrap_unchecked() }
}

fn nested(v: Option<Option<u32>>) -> Option<u32> {
    unsafe { v.unwrap_unchecked() }
}

pub fn inner() -> (Option<u32>, Option<u32>) {
    (nested(Some(None)), nested(Some(Some(3))))
}

thread_local! {
    static DROPS: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
}

struct Guard(u32);

impl Drop for Guard {
    fn drop(&mut self) {
        DROPS.with(|d| d.set(d.get() + self.0));
    }
}

pub fn guarded() -> u32 {
    {
        let _guard = unsafe { Some(Guard(5)).unwrap_unchecked() };
    }
    DROPS.with(|d| d.get())
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("return v;");
  expect(js).not.toContain("$unwrap");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.first("a"), lib.inner(), lib.guarded()]).toEqual(["a", [undefined, 3], 5]);
});

// ADR 0298: an Option whose value is never falsy, an object, an array or a
// function, is tested by its truth, as react.dev's Preview writes `{error &&
// ..}`, `!error` and `rawError && rawError.message`; and `o == Some(true)` of
// an `Option<bool>` is `o` in a test, `if (message.firstLoad)`. A number's
// or a string's, which can be falsy, is tested against null still.
test("an Option of an object is tested by its truth", async () => {
  const dir = fixture("truthy-options");
  writeFileSync(join(dir, "lib.rs"), `pub struct Failure {
    pub message: String,
}

pub fn shown(error: Option<&Failure>) -> bool {
    error.is_some()
}

pub fn hidden(error: Option<&Failure>) -> bool {
    error.is_none()
}

pub fn tested(error: Option<&Failure>) -> u32 {
    if error.is_some() { 1 } else { 0 }
}

pub fn chained(error: Option<&Failure>) -> bool {
    if let Some(e) = error
        && e.message == "x"
    {
        return true;
    }
    false
}

pub fn mapped(error: Option<&Failure>) -> Option<usize> {
    error.map(|e| e.message.len())
}

pub fn number(n: Option<u32>) -> bool {
    n.is_some()
}

pub fn text(s: Option<&str>) -> bool {
    s.is_none()
}

pub fn first(f: Option<bool>) -> u32 {
    if f == Some(true) { 1 } else { 0 }
}

pub fn first_value(f: Option<bool>) -> bool {
    f == Some(true)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  for (const written of ["return !!error;", "return !error;", "return error ? 1 : 0;", "if (error && error.message === \"x\") {",
    "return error ? $byteLen(error.message) : undefined;", "return n != null;", "return s == null;", "return f ? 1 : 0;", "return !!f;"]) {
    expect(js).toContain(written);
  }
  const lib = await import(join(dir, "lib.js"));
  const failure = { message: "x" };
  expect([lib.shown(failure), lib.shown(undefined), lib.hidden(undefined), lib.tested(failure), lib.chained(failure), lib.chained(undefined),
    lib.mapped(failure), lib.number(0), lib.text(""), lib.first(true), lib.first(undefined), lib.firstValue(false)])
    .toEqual([true, false, true, 1, true, false, 1, true, false, 1, 0, false]);
  // A JS value of unknown kind may be `""` or `0`: tested against null still.
  writeFileSync(join(dir, "unknown.rs"), `use js::Unknown;

pub fn has(value: Option<&Unknown>) -> bool {
    value.is_some()
}
`);
  run([compiler, join(dir, "unknown.rs"), "-o", join(dir, "unknown.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const unknown = readFileSync(join(dir, "unknown.js"), "utf8");
  expect(unknown).toContain("return value != null;");
  expect((await import(join(dir, "unknown.js"))).has("")).toBe(true);
});

// ADR 0298: a `const` only tests read is a test itself, as react.dev's
// Preview has `const hideContent = error || !iframeComputedHeight ||
// !bundlerIsReady;`: what it's made of is read by its truth. One read as a
// value keeps its boolean.
test("a const only tests read is read by its truth", async () => {
  const dir = fixture("tested-consts");
  writeFileSync(join(dir, "lib.rs"), `pub struct Failure {
    pub message: String,
}

pub fn shown(error: Option<&Failure>, ready: bool) -> u32 {
    let hide = error.is_some() || !ready;
    let mut n = 0;
    if hide {
        n += 1;
    }
    if !hide && ready {
        n += 10;
    }
    n + if hide { 100 } else { 0 }
}

pub fn kept(error: Option<&Failure>, ready: bool) -> (bool, u32) {
    let has = error.is_some() || !ready;
    (has, if has { 1 } else { 0 })
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("const hide = error || !ready;");
  expect(js).toContain("const has = !!error || !ready;");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.shown({ message: "" }, true), lib.shown(undefined, true), lib.shown(undefined, false), lib.kept({ message: "" }, true)])
    .toEqual([101, 10, 101, [true, 1]]);
});

// An `Option` compared with a `Some(..)`, of what's never `undefined` or
// `null`, is `===`, as react.dev's getRouteMeta writes `currentRoute.path
// === searchPath`: either answers the same (ADR 0030). Two `Option`s keep
// `==`, as `null` and `undefined` are both `None`.
test("an Option compared with a Some is ===", async () => {
  const dir = fixture("option-strict-eq");
  writeFileSync(join(dir, "lib.rs"), `pub fn same(p: &Option<String>, s: &str) -> bool {
    p.as_deref() == Some(s)
}

pub fn other(p: Option<u32>, n: u32) -> bool {
    p != Some(n)
}

pub fn both(p: Option<u32>, q: Option<u32>) -> bool {
    p == q
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("  return p === s;\n");
  expect(js).toContain("  return p !== n;\n");
  expect(js).toContain("  return p == q;\n");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.same("a", "a"), lib.same(null, "a"), lib.other(undefined, 1), lib.other(1, 1), lib.both(null, undefined)]).toEqual([true, false, true, false, true]);
});

// A callback that tests its argument's truth is `Boolean`, as react.dev
// writes `.filter(Boolean)`: what `filter_map` keeps of a value never falsy
// (ADR 0298), and `!s.is_empty()` of a string. A number's keeps `!= null`.
test("a truth test as a callback is Boolean", async () => {
  const dir = fixture("filter-boolean");
  writeFileSync(join(dir, "lib.rs"), `pub struct Item {
    pub n: u32,
}
pub fn items(xs: &[u32]) -> Vec<Item> {
    xs.iter().filter_map(|x| (*x > 1).then(|| Item { n: *x })).collect()
}
pub fn words(text: &str) -> Vec<String> {
    text.split(',').map(|g| g.trim().to_string()).filter(|g| !g.is_empty()).collect()
}
pub fn less(xs: &[u32]) -> Vec<u32> {
    xs.iter().filter_map(|x| x.checked_sub(1)).collect()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("return xs.map((x) => (x > 1 ? { n: x } : undefined)).filter(Boolean);");
  expect(js).not.toContain("(g) => !!g");
  expect(js).toContain(".filter(Boolean);");
  expect(js).toContain(".filter((item) => item != null)");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.items([1, 2]), lib.words("a, ,b"), lib.less([0, 2])]).toEqual([[{ n: 2 }], ["a", "b"], [1]]);
});

// `?` of an Option of a value never falsy tests it by its truth, as
// react.dev's runESLint writes `if (!error) return undefined;` (ADR 0298).
// A number's tests it against `null`.
test("? of an Option never falsy tests its truth", async () => {
  const dir = fixture("question-truth");
  writeFileSync(join(dir, "lib.rs"), `pub struct Item {
    pub n: u32,
}
pub fn first(item: Option<&Item>) -> Option<u32> {
    let item = item?;
    Some(item.n + 1)
}
pub fn counted(n: Option<u32>) -> Option<u32> {
    let n = n?;
    Some(n + 1)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("if (!item) {\n    return;\n  }");
  expect(js).toContain("if (n == null) {\n    return;\n  }");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.first({ n: 1 }), lib.first(undefined), lib.counted(0), lib.counted(undefined)]).toEqual([2, undefined, 1, undefined]);
});

// A nullable field given another's value, or a copy of it, is given it as
// it is: it's `null` already where it's `None`, as react.dev's runESLint
// gives `ruleId: error.ruleId` (ADR 0275).
test("a nullable field given a nullable field's copy takes it as it is", async () => {
  const dir = fixture("nullable-copied");
  writeFileSync(join(dir, "lib.rs"), `pub struct Message {
    #[rust_js::nullable]
    pub id: Option<String>,
}
pub struct Diagnostic {
    #[rust_js::nullable]
    pub id: Option<String>,
}
pub fn copied(message: &Message) -> Diagnostic {
    Diagnostic { id: message.id.clone() }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("return { id: message.id };");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.copied({ id: null }), lib.copied({ id: "x" })]).toEqual([{ id: null }, { id: "x" }]);
});

// `unwrap_or_default()` of a call reads the call once, `get(n) ?? ""`, as
// `unwrap_or` does, where it was kept in a `const option` first. A boxed
// `Some`, which `??` can't read, is still kept first.
test("unwrap_or_default of a call is its value or the default", async () => {
  const dir = fixture("unwrap-or-default");
  writeFileSync(join(dir, "lib.rs"), `pub fn get(n: u32) -> Option<String> {
    if n > 0 { Some(n.to_string()) } else { None }
}

pub fn shown(n: u32) -> String {
    get(n).unwrap_or_default()
}

pub fn nested(n: u32) -> Option<u32> {
    let inner = if n > 1 { Some(Some(n)) } else if n > 0 { Some(None) } else { None };
    inner.map(|x| x).unwrap_or_default()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('export function shown(n) {\n  return get(n) ?? "";\n}');
  const lib = await import(join(dir, "lib.js"));
  expect([lib.shown(3), lib.shown(0), lib.nested(2), lib.nested(1), lib.nested(0)]).toEqual(["3", "", 2, undefined, undefined]);
});

// `unwrap_or_else(f)` of a call reads it once too, and calls `f` only where
// it's `None`, as react.dev's calculateNestedToc writes
// `currentAncestors.get(item.depth - 1) || root`; a closure of statements
// is its arrow, made first.
test("unwrap_or_else of a call is its value or the closure's", async () => {
  const dir = fixture("unwrap-or-else");
  writeFileSync(join(dir, "lib.rs"), `use std::collections::HashMap;

pub fn parent(ancestors: &HashMap<u32, String>, depth: u32, root: &str) -> String {
    ancestors.get(&depth).cloned().unwrap_or_else(|| root.to_string())
}

pub fn counted(ancestors: &HashMap<u32, String>, depth: u32, log: &mut Vec<u32>) -> String {
    ancestors.get(&depth).cloned().unwrap_or_else(|| {
        log.push(depth);
        String::from("none")
    })
}

pub fn run() -> (String, String, String, String, Vec<u32>) {
    let mut ancestors = HashMap::new();
    ancestors.insert(1, String::from("one"));
    let mut log = Vec::new();
    let found = counted(&ancestors, 1, &mut log);
    let missing = counted(&ancestors, 2, &mut log);
    (parent(&ancestors, 1, "root"), parent(&ancestors, 2, "root"), found, missing, log)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("return ancestors.get(depth) ?? root;");
  expect(js).toContain("return ancestors.get(depth) ?? fallback();");
  const lib = await import(join(dir, "lib.js"));
  expect(lib.run()).toEqual(["one", "root", "one", "none", [2]]);
});

// ADR 0275: a nullable field given `Some(..)` or `None` by a branch, as
// react.dev's errors page gives `errorMessage`: the branch's `null`, `code
// ? errorCodes[code] : null`, not `(c ? v : undefined) ?? null`.
test("a nullable field of Some or None by a branch is the branch's null", async () => {
  const dir = fixture("nullable-branch");
  writeFileSync(join(dir, "lib.rs"), `pub struct Props {
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub message: Option<String>,
}

pub fn props(code: Option<&str>) -> Props {
    Props {
        message: match code {
            Some(code) if !code.is_empty() => Some(code.to_uppercase()),
            _ => None,
        },
    }
}

pub fn later(code: Option<&str>) -> Props {
    Props {
        message: match code {
            Some(c) if c.is_empty() => None,
            Some(c) => c.strip_prefix("x").map(|s| s.to_string()),
            None => None,
        },
    }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain("return { message: code ? code.toUpperCase() : null };");
  const lib = await import(join(dir, "mir.js"));
  expect([lib.props("a"), lib.props(""), lib.props(undefined)]).toEqual([{ message: "A" }, { message: null }, { message: null }]);
  expect([lib.later("xa"), lib.later("a"), lib.later(""), lib.later(undefined)]).toEqual([{ message: "a" }, { message: null }, { message: null }, { message: null }]);
});

// ADR 0298: `is_some()` of an `Option` never falsy, `!!o`, given as a
// value, a `bool`'s: `!!o && f()` is `false` where it's `None`, never the
// `undefined` of `o && f()`, as react.dev's DownloadButton keeps whether
// import maps are supported.
test("a truthy option's is_some given as a value is a boolean", async () => {
  const dir = fixture("is-some-value");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::Cell;
use js::JsObject;
#[cfg_attr(rust_js, rust_js::link_name = "get globalThis.maybeSupports")]
fn scriptSupports() -> Option<&'static JsObject> {
    unreachable!()
}

#[cfg_attr(rust_js, rust_js::link_name = "globalThis.supportsIt")]
#[allow(unused_variables)]
fn supports(what: &str) -> bool {
    unreachable!()
}
thread_local! {
    static SUPPORTED: Cell<bool> = const { Cell::new(false) };
}
pub fn update() -> bool {
    SUPPORTED.set(scriptSupports().is_some() && supports("importmap"));
    SUPPORTED.get()
}
pub fn negated() -> bool {
    if scriptSupports().is_none() { false } else { true }
}
pub fn or_none() -> bool {
    if scriptSupports().is_none() { supports("x") } else { true }
}
`);
  const globals = globalThis as typeof globalThis & { maybeSupports?: object; supportsIt?: (what: string) => boolean };
  globals.supportsIt = () => true;
  try {
    run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
    const js = readFileSync(join(dir, "mir.js"), "utf8");
    expect(js).toContain('  supported = !!globalThis.maybeSupports && globalThis.supportsIt("importmap");\n');
    const lib = await import(join(dir, "mir.js"));
    delete globals.maybeSupports;
    const absent = [lib.update(), lib.negated(), lib.orNone()];
    globals.maybeSupports = {};
    expect([absent, [lib.update(), lib.negated(), lib.orNone()]]).toEqual([[false, false, true], [true, true, true]]);
  } finally {
    delete globals.maybeSupports;
    delete globals.supportsIt;
  }
});

// A clone of a nullable field's `Option`, `m.rule_id.clone()`, is its JS
// value, null or the string, so given to another nullable field it's read
// as it is, as react.dev's runESLint gives each diagnostic its rule.
test("a nullable field's clone is given to a nullable field as it is", async () => {
  const dir = fixture("nullable-clone");
  writeFileSync(join(dir, "lib.rs"), `pub struct Msg {
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub rule_id: Option<String>,
    pub line: u32,
}
pub struct Diag {
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub rule_id: Option<String>,
    pub line: u32,
}
pub fn diags(msgs: Vec<&Msg>) -> Vec<Diag> {
    msgs.iter().map(|m| Diag { rule_id: m.rule_id.clone(), line: m.line }).collect()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain("({ ruleId: m.ruleId, line: m.line })");
  const lib = await import(join(dir, "mir.js"));
  expect(lib.diags([{ ruleId: null, line: 1 }, { ruleId: "r", line: 2 }])).toEqual([{ ruleId: null, line: 1 }, { ruleId: "r", line: 2 }]);
});

// `x != null && typeof x === "string"`, an `Option`'s test before a
// classification's: the `typeof` test, which holds of no `null`, as
// react.dev's TerminalBlock reads its children's text from MIR.
test("a typeof test after a null check is the typeof test from MIR", async () => {
  const dir = fixture("typeof-after-null");
  writeFileSync(join(dir, "lib.rs"), `pub fn text(ok: bool, given: Option<&'static js::Unknown>) -> &'static str {
    if ok
        && let Some(t) = given
        && let js::Kind::String(s) = js::classify(t)
    {
        s
    } else {
        "none"
    }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target], 600_000);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain('return ok && typeof given === "string" ? given : "none";');
  const lib = await import(join(dir, "mir.js"));
  expect([lib.text(true, "a"), lib.text(true, undefined), lib.text(true, 3), lib.text(false, "a")]).toEqual(["a", "none", "none", "none"]);
});

// An untagged enum is its payload (ADR 0214): an `Option` of one whose every
// payload is never falsy, an array or a JS object, as react.dev's
// NavigationBar's `contentBoxSize` is, is there where it's truthy, `if
// (size)`; of one that may be text, `""`, it's `size != null`.
test("an option of an untagged enum never falsy is tested by its truth", async () => {
  const dir = fixture("untagged-never-falsy");
  writeFileSync(join(dir, "lib.rs"), `pub struct Size { pub w: u32 }
#[cfg_attr(rust_js, rust_js::untagged)]
pub enum Boxes<'a> {
    Many(&'a [Size]),
    #[cfg_attr(rust_js, rust_js::otherwise)]
    One(&'a Size),
}
#[cfg_attr(rust_js, rust_js::untagged)]
pub enum Label<'a> {
    Text(&'a str),
    #[cfg_attr(rust_js, rust_js::otherwise)]
    At(&'a Size),
}
pub fn width(size: Option<Boxes>) -> u32 {
    if let Some(size) = size {
        match size { Boxes::Many(all) => all[0].w, Boxes::One(one) => one.w }
    } else {
        0
    }
}
pub fn labeled(label: Option<Label>) -> bool {
    label.is_some()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain("  if (size) {\n");
  expect(js).toContain("return label != null;");
  const lib = await import(join(dir, "mir.js"));
  expect([lib.width([{ w: 3 }]), lib.width({ w: 4 }), lib.width(undefined), lib.labeled(""), lib.labeled(undefined)]).toEqual([3, 4, 0, true, false]);
});

// ADR 0635: an `Option` filtered by the truth of what it holds, then a
// fallback, is JS's `||` of values, as React writes its names: `x || y`
// gives `x` where it's truthy, else `y`, and the filtered `Option` gives `x`
// where `js::truthy(x)`, else what's after. A predicate that's anything
// else is its own test.
test("a filter by truth then a fallback is JS's `||`", async () => {
  const dir = fixture("truthy-or");
  writeFileSync(join(dir, "lib.rs"), `use js::Unknown;

#[rust_js::nullable]
pub fn display_name(t: &Unknown) -> Option<&'static Unknown> {
    js::get(t, "displayName")
        .filter(|v| js::truthy(Some(*v)))
        .or_else(|| js::get(t, "name"))
        .filter(|v| js::truthy(Some(*v)))
}

pub fn function_name(inner: &Unknown) -> String {
    js::get(inner, "displayName")
        .filter(|v| js::truthy(Some(*v)))
        .or_else(|| js::get(inner, "name"))
        .filter(|v| js::truthy(Some(*v)))
        .map(|v| unsafe { js::cast::<String>(Some(v)) })
        .unwrap_or_default()
}

pub fn component_name(t: Option<&Unknown>) -> Option<String> {
    t.and_then(|t| js::get(t, "displayName")).map(|v| unsafe { js::cast::<String>(Some(v)) })
}

pub fn memo_name(t: &Unknown) -> String {
    component_name(js::get(t, "type"))
        .filter(|name| js::truthy(Some(name)))
        .unwrap_or_else(|| "Memo".to_string())
}

pub fn either(a: Option<&'static Unknown>, b: Option<&'static Unknown>) -> Option<&'static Unknown> {
    a.filter(|v| js::truthy(Some(*v))).or(b)
}

pub fn either_or(
    a: Option<&'static Unknown>,
    b: Option<&'static Unknown>,
    c: Option<&'static Unknown>,
) -> Option<&'static Unknown> {
    a.filter(|v| js::truthy(Some(*v))).or(b).or(c)
}

pub fn given(a: Option<&'static Unknown>, b: &'static Unknown) -> &'static Unknown {
    a.filter(|&v| js::truthy(Some(v))).unwrap_or(b)
}

pub fn first(
    a: Option<&'static Unknown>,
    b: Option<&'static Unknown>,
    c: Option<&'static Unknown>,
) -> Option<&'static Unknown> {
    a.filter(|v| js::truthy(Some(*v)))
        .or_else(|| b.filter(|v| js::truthy(Some(*v))))
        .or_else(|| c)
}

pub fn falsy_or(a: Option<&'static Unknown>, b: Option<&'static Unknown>) -> Option<&'static Unknown> {
    a.filter(|v| !js::truthy(Some(*v))).or(b)
}

#[rust_js::nullable]
pub fn kept(a: Option<&'static Unknown>) -> Option<&'static Unknown> {
    a.filter(|v| js::truthy(Some(*v)))
}

pub fn shout(a: Option<&'static str>) -> Option<String> {
    a.filter(|s| js::truthy(Some(*s))).map(|s| s.to_uppercase())
}

pub fn named(a: Option<&'static Unknown>) -> Option<&'static Unknown> {
    a.filter(|v| js::truthy(js::get(v, "name")))
}

#[rust_js::nullable]
pub fn label(a: Option<&'static str>) -> Option<&'static str> {
    a.filter(|s| js::truthy(Some(*s))).or(Some("Memo"))
}

fn logged(log: &mut Vec<u32>, n: u32) -> Option<u32> {
    log.push(n);
    if n > 1 { Some(n) } else { None }
}

pub fn ordered() -> Vec<u32> {
    let mut log = Vec::new();
    let found = logged(&mut log, 1).or(logged(&mut log, 2));
    log.push(found.unwrap_or(0));
    log
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("export function displayName(t) {\n  return t.displayName || t.name || null;\n}");
  expect(js).toContain('export function functionName(inner) {\n  return inner.displayName || inner.name || "";\n}');
  expect(js).toContain('export function memoName(t) {\n  return componentName(t.type) || "Memo";\n}');
  expect(js).toContain("export function either(a, b) {\n  return a || b;\n}");
  expect(js).toContain("export function eitherOr(a, b, c) {\n  return (a || b) ?? c;\n}");
  expect(js).toContain("export function given(a, b) {\n  return a || b;\n}");
  expect(js).toContain("export function first(a, b, c) {\n  return a || b || c;\n}");
  expect(js).toContain("export function kept(a) {\n  return a || null;\n}");
  expect(js).toContain('export function label(a) {\n  return a || "Memo";\n}');
  expect(js).toContain("export function shout(a) {\n  return a ? a.toUpperCase() : undefined;\n}");
  // A predicate that isn't the truth of what it holds is its own test.
  expect(js).toContain("export function falsyOr(a, b) {\n  return a != null && !a ? a : b;\n}");
  expect(js).toContain("export function named(a) {\n  return a != null && a.name ? a : undefined;\n}");
  const lib = await import(join(dir, "lib.js"));
  const object = { name: "object" };
  const values = ["", 0, undefined, null, "n", object];
  for (const x of values) {
    for (const y of values) {
      expect(lib.displayName({ displayName: x, name: y })).toEqual(x || y || null);
      expect(lib.functionName({ displayName: x, name: y })).toEqual(x || y || "");
      expect(lib.memoName({ type: { displayName: x } })).toEqual(x || "Memo");
      expect(lib.either(x ?? undefined, y ?? undefined)).toEqual(x || (y ?? undefined));
      expect(lib.eitherOr(x ?? undefined, y ?? undefined, object)).toEqual(x || (y ?? object));
      expect(lib.given(x ?? undefined, y)).toEqual(x || y);
      expect(lib.first(x ?? undefined, y ?? undefined, object)).toEqual(x || y || object);
      expect(lib.falsyOr(x ?? undefined, y ?? undefined)).toEqual(x != null && !x ? x : (y ?? undefined));
      expect(lib.kept(x ?? undefined)).toEqual(x || null);
      expect(lib.named(x ?? undefined)).toEqual(x != null && (x as { name?: string }).name ? x : undefined);
    }
  }
  expect(lib.memoName({})).toEqual("Memo");
  expect(["", "ab", undefined].map(lib.shout)).toEqual([undefined, "AB", undefined]);
  expect([lib.label(""), lib.label("x"), lib.label(undefined)]).toEqual(["Memo", "x", "Memo"]);
  // `or`'s `b` is worked out after its `Option`, as Rust does both first.
  expect(lib.ordered()).toEqual([1, 2, 2]);
});
