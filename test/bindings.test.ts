// The binding language, and the js and webapi crates written in it: a
// binding's call is the JS it names, and JS's globals, objects and types are
// JS's own. Each test is a program of its own, compiled and run.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildReact, compiler, fixture, root, run, target } from "./support";

beforeAll(buildReact, 600_000);

// ADR 0223: an event's name gives its listener the event its target takes,
// and a tag's name the element it makes, from `@webref/events` and
// `@webref/elements`, as TypeScript's `HTMLElementEventMap` and
// `HTMLElementTagNameMap` do. Each name is a type whose value is its string.
test("webapi's event and tag maps type a listener and an element", () => {
  const withWeb = ["--", "--extern", `webapi=${join(target, "libwebapi.rmeta")}`, "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("webapi-maps");
  const source = `use webapi::events::{Click, Keydown};
use webapi::tags::Button;
use webapi::{EventTargetExt, document};
pub fn wire() -> &'static webapi::HTMLButtonElement {
    let button = document.create_element(Button);
    button.set_disabled(false);
    button.add_event_listener(Click, |e| {
        let _ = e.client_x();
    });
    document.add_event_listener(Keydown, |e| {
        let _ = e.key();
    });
    button
}
`;
  writeFileSync(join(dir, "lib.rs"), source);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withWeb]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('const button = document.createElement("button");');
  expect(js).toContain("button.disabled = false;");
  expect(js).toContain('button.addEventListener("click", (e) => {');
  expect(js).toContain('document.addEventListener("keydown", (e) => {');
  // A key's event isn't a mouse's, and a button takes no window's message.
  for (const [wrong, error, message] of [
    ["document.add_event_listener(Keydown, |e| {\n        let _ = e.key();", "document.add_event_listener(Keydown, |e| {\n        let _ = e.client_x();", "no method named `client_x`"],
    ["use webapi::events::{Click, Keydown};", "use webapi::events::{Click, Keydown, Message};\nfn message(b: &webapi::HTMLButtonElement) { b.add_event_listener(Message, |_| ()); }", "the trait `webapi::Listen<webapi::events::Message>` is not implemented for `webapi::HTMLButtonElement`"],
  ]) {
    writeFileSync(join(dir, "lib.rs"), source.replace(wrong, error));
    const result = Bun.spawnSync([compiler, join(dir, "lib.rs"), "-o", join(dir, "wrong.js"), ...withWeb], { cwd: dir });
    expect(result.exitCode).not.toBe(0);
    expect(result.stderr.toString()).toContain(message);
  }
});

// SVG's elements are bound, and each SVG tag is a type of its own, as
// TypeScript's `SVGElementTagNameMap` has them: `createElementNS` of SVG's
// namespace and a tag makes its element, `svg_tags::Circle` an
// `SVGCircleElement`, which is an `SVGGeometryElement`, an
// `SVGGraphicsElement`, an `SVGElement` and an `Element`.
test("webapi's SVG elements are made by their tags", () => {
  const withWeb = ["--", "--extern", `webapi=${join(target, "libwebapi.rmeta")}`, "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("webapi-svg");
  writeFileSync(join(dir, "lib.rs"), `use webapi::namespaces::Svg;
use webapi::svg_tags::{Circle, Svg as SvgTag};
use webapi::{SVGCircleElement, SVGSVGElement, document};
pub fn draw() -> (&'static SVGSVGElement, &'static SVGCircleElement, bool) {
    let svg = document.create_element_ns(Svg, SvgTag);
    let circle = document.create_element_ns(Svg, Circle);
    circle.set_attribute("r", "4");
    svg.append(circle);
    (svg, circle, circle.is_point_in_fill())
}
pub fn named() -> &'static webapi::Element {
    document.create_element_ns_named("http://www.w3.org/2000/svg", "g")
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withWeb]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('const svg = document.createElementNS("http://www.w3.org/2000/svg", "svg");');
  expect(js).toContain('const circle = document.createElementNS("http://www.w3.org/2000/svg", "circle");');
  expect(js).toContain("return [svg, circle, circle.isPointInFill()];");
  expect(js).toContain('return document.createElementNS("http://www.w3.org/2000/svg", "g");');
});

// ADR 0229: a binding's union parameter is `impl` a sealed trait of its
// members, so each member is passed as it is, `upload("hello")`, with no
// `.into()` and nothing in the JS; another type is an error that names the
// union, and the enum of one, `BodyInit::of`, is matched.
test("a union parameter takes each member as it is", async () => {
  const withWeb = ["--", "--extern", `webapi=${join(target, "libwebapi.rmeta")}`, "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("union-parameters");
  const source = `use webapi::{Blob, BodyInit, Element, IntoBodyInit, IntoFloat32List, Response, response, window};

#[cfg_attr(rust_js, rust_js::untagged)]
pub enum UploadBody<'a> {
    Text(&'a str),
    Blob(&'a Blob),
}

impl<'a> From<&'a str> for UploadBody<'a> {
    fn from(value: &'a str) -> Self {
        UploadBody::Text(value)
    }
}

impl<'a> From<&'a Blob> for UploadBody<'a> {
    fn from(value: &'a Blob) -> Self {
        UploadBody::Blob(value)
    }
}

mod sealed {
    pub trait Sealed {}
    impl Sealed for &str {}
    impl Sealed for &webapi::Blob {}
    impl Sealed for super::UploadBody<'_> {}
    impl<T: Sealed> Sealed for Option<T> {}
}

#[diagnostic::on_unimplemented(message = "\`{Self}\` is not a \`string | Blob\`")]
pub trait IntoUploadBody: sealed::Sealed {}
impl IntoUploadBody for &str {}
impl IntoUploadBody for &Blob {}
impl IntoUploadBody for UploadBody<'_> {}
impl<T: IntoUploadBody> IntoUploadBody for Option<T> {}

impl<'a> UploadBody<'a> {
    #[cfg_attr(rust_js, rust_js::link_name = "this")]
    #[allow(unused_variables)]
    pub fn of(this: impl IntoUploadBody + 'a) -> UploadBody<'a> {
        unreachable!()
    }
}

#[cfg_attr(rust_js, rust_js::link_name = "globalThis.upload")]
#[allow(unused_variables)]
pub fn upload(body: impl IntoUploadBody) {
    unreachable!()
}

pub fn send(blob: &Blob, body: UploadBody, maybe: Option<&str>) {
    upload("hello");
    upload(blob);
    upload(body);
    upload(maybe);
}

pub fn kind<'a>(body: impl IntoBodyInit + 'a) -> String {
    match BodyInit::of(body) {
        BodyInit::Str(text) => text.to_string(),
        BodyInit::Blob(_) => "blob".to_string(),
        _ => "other".to_string(),
    }
}

pub fn count<'a>(values: impl IntoFloat32List + 'a) -> u32 {
    let _ = values;
    0
}

pub fn page(el: &Element, blob: &Blob, url: &str) -> (&'static Response, js::Promise<&'static Response>) {
    el.before("text");
    (response::new_with_body(blob), window.fetch(url))
}
`;
  writeFileSync(join(dir, "Cargo.toml"), '[package]\nname = "app"\nversion = "0.0.0"\nedition = "2024"\n\n[package.metadata.rust-js]\ndeclarations = true\n');
  writeFileSync(join(dir, "lib.rs"), source);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withWeb]);
  // To TypeScript, the trait is the union: by TypeScript's own name for it,
  // `Float32List`, where Rust takes each of its members; else each member,
  // `BodyInit`'s, whose `Float16Array` Rust has no type of.
  expect(readFileSync(join(dir, "lib.d.ts"), "utf8")).toContain("export function count(values: Float32List): number;");
  expect(readFileSync(join(dir, "lib.d.ts"), "utf8")).toContain("export function kind(body: ReadableStream | Blob | Int8Array | Int16Array | Int32Array | Uint8Array | Uint16Array | Uint32Array | Uint8ClampedArray | BigInt64Array | BigUint64Array | Float32Array | Float64Array | DataView | ArrayBuffer | FormData | URLSearchParams | string): string;");
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('globalThis.upload("hello");\n  globalThis.upload(blob);\n  globalThis.upload(body);\n  globalThis.upload(maybe);');
  // Of webapi's union, a binding crate's trait, no dictionary: `kind(body)`.
  expect(js).toContain("export function kind(body) {\n  const match = body;\n  if (typeof match === \"string\") {");
  expect(js).toContain('el.before("text");');
  expect(js).toContain("return [new Response(blob), window.fetch(url)];");
  const { send, kind } = await import(join(dir, "lib.js"));
  const sent: unknown[] = [];
  const previous = (globalThis as any).upload;
  (globalThis as any).upload = (body: unknown) => sent.push(body);
  try {
    const blob = new Blob(["b"]);
    send(blob, "body", undefined);
    expect(sent).toEqual(["hello", blob, "body", undefined]);
  } finally {
    (globalThis as any).upload = previous;
  }
  expect([kind("text"), kind(new Blob())]).toEqual(["text", "blob"]);
  writeFileSync(join(dir, "lib.rs"), source.replace('upload("hello");', "upload(1.5);"));
  const wrong = Bun.spawnSync([compiler, join(dir, "lib.rs"), "-o", join(dir, "wrong.js"), ...withWeb], { cwd: dir });
  expect([wrong.exitCode === 0, wrong.stderr.toString().includes("is not a `string | Blob`")], wrong.stderr.toString()).toEqual([false, true]);
});

// ADR 0260: a binding's callback parameter is a closure, given what JS
// calls it with as a function's parameters are, as react.dev's TopNav
// watches whether the page has scrolled.
test("a binding's callback parameter is a closure", async () => {
  const withWeb = ["--", "--extern", `webapi=${join(target, "libwebapi.rmeta")}`, "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("callback-parameters");
  writeFileSync(join(dir, "lib.rs"), `use webapi::{Element, IntersectionObserver, IntersectionObserverInit, intersection_observer, window};

pub fn watch(target: &'static Element, seen: &'static dyn Fn(bool)) -> &'static IntersectionObserver {
    let observer = intersection_observer::new_with_options(
        Box::new(move |entries, _| {
            entries.iter().for_each(|entry| seen(entry.is_intersecting()));
        }),
        IntersectionObserverInit {
            root_margin: Some("0px 0px"),
            threshold: Some(0.0.into()),
            ..Default::default()
        },
    );
    observer.observe(target);
    observer
}

pub fn next_frame(then: &'static dyn Fn(f64)) -> u32 {
    window.request_animation_frame(Box::new(move |time| then(time)))
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withWeb]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("new IntersectionObserver(");
  expect(js).toContain("return window.requestAnimationFrame((time) => {\n    then(time);\n  });");
  const globals = globalThis as any;
  const previous = [globals.IntersectionObserver, globals.window];
  const given: unknown[] = [];
  globals.IntersectionObserver = class {
    constructor(private callback: (entries: unknown[], observer: unknown) => void, options: unknown) {
      given.push(options);
    }
    observe(target: unknown) {
      this.callback([{ isIntersecting: false, target }, { isIntersecting: true, target }], this);
    }
  };
  globals.window = { requestAnimationFrame: (callback: (time: number) => void) => (callback(16), 7) };
  try {
    const { watch, next_frame } = await import(join(dir, "lib.js"));
    const seen: unknown[] = [];
    watch({}, (value: boolean) => seen.push(value));
    expect([seen, given]).toEqual([[false, true], [{ rootMargin: "0px 0px", threshold: 0 }]]);
    const times: number[] = [];
    expect([next_frame((time: number) => times.push(time)), times]).toEqual([7, [16]]);
  } finally {
    [globals.IntersectionObserver, globals.window] = previous;
  }
});

// ADR 0269: what every JS global scope has, a window's, a worker's or Node's,
// `fetch` and `queueMicrotask`, is called bare, as react.dev's errors page
// fetches in Next.js's `getStaticProps`, where there's no `window`.
test("the global scope's functions are called bare", async () => {
  const withWeb = ["--", "--extern", `webapi=${join(target, "libwebapi.rmeta")}`, "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("global-scope");
  writeFileSync(join(dir, "lib.rs"), `use js::Promise;
use webapi::{Response, global};

pub fn codes(url: &str) -> Promise<&'static Response> {
    global::fetch(url)
}

pub fn later(then: &'static dyn Fn()) {
    global::queue_microtask(Box::new(move || then()));
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withWeb]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("return fetch(url);");
  expect(js).toContain("queueMicrotask(");
  expect(js).not.toContain("window");
  const fetched: string[] = [];
  const previous = globalThis.fetch;
  globalThis.fetch = ((url: string) => (fetched.push(url), Promise.resolve(new Response("{}")))) as typeof fetch;
  try {
    const { codes, later } = await import(join(dir, "lib.js"));
    await codes("https://example.com/codes.json");
    const ran: number[] = [];
    later(() => ran.push(1));
    await Promise.resolve();
    expect([fetched, ran]).toEqual([["https://example.com/codes.json"], [1]]);
  } finally {
    globalThis.fetch = previous;
  }
});

// ADR 0035: what a JS function threw is shown as JS shows it, `String(error)`,
// `SyntaxError: ..`, by `{:?}` and by an `unwrap`'s panic.
test("a JS error is shown as JS shows it", async () => {
  const withJs = ["--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("js-error");
  writeFileSync(join(dir, "lib.rs"), `use js::{Unknown, json};
pub fn parsed(text: &str) -> Option<&'static Unknown> {
    json::parse(text).unwrap()
}
pub fn kept(text: &str) -> Option<&'static Unknown> {
    let value = match json::parse(text) {
        Ok(value) => value,
        Err(_) => {
            eprintln!("invalid");
            None
        }
    };
    value
}
pub fn valid(text: &str) -> bool {
    match json::parse(text) {
        Ok(_) => true,
        Err(_) => false,
    }
}
pub fn told(text: &str) -> String {
    let told = match json::parse(text) {
        Ok(_) => String::new(),
        Err(error) => format!("{error:?}"),
    };
    told
}
pub fn kept_or_told(text: &str) -> Option<&'static Unknown> {
    let value = match json::parse(text) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("{error:?}");
            None
        }
    };
    value
}
pub fn shown(text: &str) -> String {
    match json::parse(text) {
        Ok(_) => "parsed".to_string(),
        Err(error) => format!("{error:?}"),
    }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("return String(match._0);");
  // A `match` of what it threw, whose `Ok` only keeps the value and
  // whose `Err` reads no error, is JS's `try` (ADR 0035).
  expect(js).toContain('  let value;\n  try {\n    value = JSON.parse(text);\n  } catch {\n    console.error("invalid");\n    value = undefined;\n  }');
  const lib = await import(join(dir, "lib.js"));
  expect(lib.parsed("[1]")).toEqual([1]);
  expect([lib.kept("[1]"), lib.kept("{")]).toEqual([[1], undefined]);
  // One whose `Err` reads the error keeps it, `$try`'s.
  expect([lib.kept_or_told("[1]"), lib.kept_or_told("{")]).toEqual([[1], undefined]);
  expect([lib.told("1"), lib.told("{").startsWith("SyntaxError")]).toEqual(["", true]);
  expect([lib.valid("[1]"), lib.valid("{")]).toEqual([true, false]);
  expect(() => lib.parsed("{")).toThrow("called `Result::unwrap()` on an `Err` value: SyntaxError");
  expect([lib.shown("{").startsWith("SyntaxError: "), lib.shown("1")]).toEqual([true, "parsed"]);
});

// ADR 0283: a regular expression's match, an `Error`'s parts, symbols, the
// global functions and the weak collections are the js crate's, each a
// method or a function of the JS it names; a weak key is an object.
test("RegExp, Error, Symbol, weak collections and globals are JS's", async () => {
  const withJs = ["--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("js-objects");
  writeFileSync(join(dir, "lib.rs"), `use js::{WeakKey, dict, js_error, parse_int, reg_exp, symbol, weak_map, weak_ref};
pub fn matched(text: &str) -> Option<(String, u32, String, bool)> {
    let pattern = reg_exp::new(r"(?<word>[a-z]+)(\\d)", "g");
    let m = pattern.exec(text)?;
    let word = m.groups().and_then(|groups| dict::get(groups, "word").cloned()).unwrap_or_default();
    Some((m.get(2).unwrap_or_default(), m.index(), word + &m.full_match(), pattern.global()))
}
pub fn errors() -> (String, String, f64) {
    let e = js_error::new("boom");
    (e.name(), e.message(), parse_int("42px", 10))
}
pub fn symbols() -> (bool, Option<String>, Option<String>) {
    let a = symbol::for_("app");
    (js::object::is(a, symbol::for_("app")), symbol::key_for(a), symbol::new("local").description())
}
pub struct State {
    pub n: u32,
}
unsafe impl WeakKey for State {}
pub fn weak(state: &'static State) -> (Option<u32>, bool) {
    let seen = weak_map::new::<State, u32>();
    seen.set(state, state.n);
    let r = weak_ref::new(state);
    (seen.get(state), r.deref().is_some())
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("const m = pattern.exec(text);");
  expect(js).toContain('const e = new Error("boom");');
  expect(js).toContain('const a = Symbol.for("app");');
  expect(js).toContain("const seen = new WeakMap();\n  seen.set(state, state.n);\n  const r = new WeakRef(state);");
  const lib = await import(join(dir, "lib.js"));
  expect(lib.matched("ab cd7")).toEqual(["7", 3, "cdcd7", true]);
  expect(lib.matched("none")).toBe(undefined);
  expect(lib.errors()).toEqual(["Error", "boom", 42]);
  expect(lib.symbols()).toEqual([true, "app", "local"]);
  expect(lib.weak({ n: 5 })).toEqual([5, true]);
});

// ADR 0283: JS's Promise's statics and methods are the js crate's, as
// ReScript's Stdlib has them; `allSettled`'s results a union tagged by
// `status` (ADR 0284).
test("a Promise's statics and methods are JS's", async () => {
  const withJs = ["--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("js-promises");
  writeFileSync(join(dir, "lib.rs"), `use js::{PromiseSettledResult, PromiseWithResolvers, js_error, promise};
pub async fn made() -> u32 {
    promise::new(|resolve, _| resolve(1)).then(|n: u32| promise::resolve(n + 1)).await
}
pub async fn caught() -> String {
    promise::reject::<String>(&js_error::new("boom")).catch(|e| promise::resolve(e.message())).finally(|| {}).await
}
pub async fn paired() -> (u32, String) {
    promise::all2((promise::resolve(1), promise::resolve("a".to_string()))).await
}
pub async fn settled() -> Vec<String> {
    let all = promise::all_settled(vec![promise::resolve(1), promise::reject(&js_error::new("no"))]).await;
    all.into_iter()
        .map(|r| match r {
            PromiseSettledResult::Fulfilled { value } => value.to_string(),
            PromiseSettledResult::Rejected { reason } => reason.message(),
        })
        .collect()
}
pub async fn resolvers() -> u32 {
    let PromiseWithResolvers { promise, resolve, .. } = promise::with_resolvers::<u32>();
    resolve(7);
    promise::race(vec![promise, promise::any(vec![promise::resolve(8)])]).await
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("return await new Promise((resolve) => {\n    resolve(1);\n  }).then(");
  expect(js).toContain('Promise.reject(new Error("boom"))\n    .catch((e) => Promise.resolve(e.message))');
  expect(js).toContain('Promise.all([Promise.resolve(1), Promise.resolve("a")])');
  expect(js).toContain('r.status === "fulfilled"');
  const lib = await import(join(dir, "lib.js"));
  expect(await lib.made()).toBe(2);
  expect(await lib.caught()).toBe("boom");
  expect(await lib.paired()).toEqual([1, "a"]);
  expect(await lib.settled()).toEqual(["1", "no"]);
  expect(await lib.resolvers()).toBe(7);
});

// ADR 0283: Reflect's functions and Proxy's traps are the js crate's, of a JS
// value of any shape.
test("Reflect and Proxy are JS's", async () => {
  const withJs = ["--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("js-reflect");
  writeFileSync(join(dir, "lib.rs"), `use js::{Unknown, property_descriptor, proxy, proxy_handler, reflect, string, unknown};
pub fn reflected(o: &Unknown) -> (bool, String, usize, bool, bool) {
    let d = property_descriptor::new();
    d.set_value(Some(unknown("fixed")));
    d.set_writable(false);
    let defined = reflect::define_property(o, "id", d);
    let written = reflect::set(o, "id", "other");
    let own = reflect::get_own_property_descriptor(o, "id").and_then(|d| d.writable()) == Some(false);
    (defined, string(reflect::get(o, "id")), reflect::own_keys(o).len(), written, own || !reflect::set_prototype_of(o, None))
}
pub fn proxied(o: &'static Unknown) -> (String, String, bool) {
    let handler = proxy_handler::new();
    handler.set_get(Box::new(|target, key, _| match reflect::get(target, key) {
        Some(value) => Some(value),
        None => Some(unknown("missing")),
    }));
    let p = proxy::revocable(o, handler);
    let seen = (string(reflect::get(p.proxy(), "a")), string(reflect::get(p.proxy(), "b")));
    p.revoke();
    (seen.0, seen.1, reflect::get_prototype_of(o).is_none())
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("const d = {};");
  expect(js).toContain("d.writable = false;");
  expect(js).toContain("Reflect.setPrototypeOf(o, null)");
  expect(js).toContain("const p = Proxy.revocable(o, handler);");
  const lib = await import(join(dir, "lib.js"));
  expect(lib.reflected({ a: 1 })).toEqual([true, "fixed", 2, false, true]);
  expect(lib.proxied(Object.assign(Object.create(null), { a: 1 }))).toEqual(["1", "missing", true]);
});

// ADR 0283: Atomics' functions take an integer typed array, its element a
// Rust number of its kind, and one that a thread may wait on a
// SharedArrayBuffer's.
test("Atomics are JS's, of a view of a shared buffer", async () => {
  const withJs = ["--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("js-atomics");
  writeFileSync(join(dir, "lib.rs"), `use js::{WaitAsyncValue, WaitResult, atomics, big_int64_array, int32_array, shared_array_buffer};
pub fn counted() -> (i32, i32, i32, i64, bool, bool) {
    let shared = shared_array_buffer::new(16);
    let counts = int32_array::new_with_buffer(shared);
    atomics::add(counts, 0, 5);
    let before = atomics::sub(counts, 0, 2);
    atomics::compare_exchange(counts, 1, 0, 9);
    let wide = big_int64_array::new(1);
    atomics::store(wide, 0, 7);
    let waited = matches!(atomics::wait(counts, 0, 0), WaitResult::NotEqual);
    (before, atomics::load(counts, 0), atomics::load(counts, 1), atomics::or(wide, 0, 8), waited, counts.buffer().growable())
}
pub async fn waited() -> bool {
    let counts = int32_array::new_with_buffer(shared_array_buffer::new(4));
    match atomics::wait_async_with_timeout(counts, 0, 0, 1.0).value {
        WaitAsyncValue::Waiting(promise) => matches!(promise.await, WaitResult::TimedOut),
        WaitAsyncValue::Ended(_) => false,
    }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("const counts = new Int32Array(shared);");
  expect(js).toContain("Atomics.compareExchange(counts, 1, 0, 9);");
  expect(js).toContain('Atomics.wait(counts, 0, 0) === "not-equal"');
  const lib = await import(join(dir, "lib.js"));
  expect(lib.counted()).toEqual([5, 3, 9, 7n, true, false]);
  expect(await lib.waited()).toBe(true);
});

// ADR 0283: Intl's formatters are the js crate's, their options structs whose
// `None` isn't given, each string union an enum.
test("Intl's formatters are JS's, their options typed", async () => {
  const withJs = ["--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("js-intl");
  writeFileSync(join(dir, "lib.rs"), `use js::date;
use js::intl::{self, *};
pub fn numbers() -> (String, String, bool, Vec<String>, String) {
    let price = number_format::new(&["en-US"], &NumberFormatOptions { style: Some(NumberStyle::Currency), currency: Some("USD"), ..Default::default() });
    let plain = number_format::new(&["en-US"], &NumberFormatOptions { use_grouping: Some(UseGrouping::Bool(false)), ..Default::default() });
    let options = plain.resolved_options();
    let parts = price.format_to_parts(-1.5).into_iter().filter(|p| p.r#type != NumberFormatPartType::Literal).map(|p| p.value).collect();
    (price.format(1234.5), plain.format(1234.0), options.use_grouping == UseGrouping::Bool(false), parts, price.format_range(3.0, 5.0))
}
pub fn dates() -> (String, String, Vec<String>) {
    let d = date::new_with_time(0.0);
    let utc = DateTimeFormatOptions { time_zone: Some("UTC"), year: Some(NumericStyle::Numeric), month: Some(MonthStyle::Long), day: Some(NumericStyle::Numeric), ..Default::default() };
    let format = date_time_format::new(&["en-US"], &utc);
    let kinds = format.format_to_parts(d).into_iter().filter(|p| p.r#type != DateTimeFormatPartType::Literal).map(|p| p.value).collect();
    (format.format(d), d.to_locale_date_string_with(&["en-GB"], &utc), kinds)
}
pub fn words() -> (Vec<String>, PluralCategory, String, String, Option<String>, usize) {
    let collator = collator::new(&["en"], &CollatorOptions { numeric: Some(true), ..Default::default() });
    let mut files = vec!["file10", "file2", "File1"];
    files.sort_by(|a, b| collator.compare(a, b));
    let ordinal = plural_rules::new(&["en-US"], &PluralRulesOptions { r#type: Some(PluralRuleType::Ordinal), ..Default::default() });
    let ago = relative_time_format::new(&["en"], &RelativeTimeFormatOptions { numeric: Some(RelativeTimeNumeric::Auto), ..Default::default() });
    let list = list_format::new(&["en"], &ListFormatOptions { r#type: Some(ListFormatType::Disjunction), ..Default::default() });
    let names = display_names::new(&["en"], &DisplayNamesOptions { r#type: DisplayNamesType::Region, locale_matcher: None, style: None, language_display: None, fallback: None });
    let words = segmenter::new(&["en"], &SegmenterOptions { granularity: Some(Granularity::Word), ..Default::default() });
    let count = segments::iter(words.segment("Hello, big world")).filter(|s| s.is_word_like == Some(true)).count();
    (files.iter().map(|f| f.to_string()).collect(), ordinal.select(2.0), ago.format(-1.0, RelativeTimeUnit::Day), list.format(&["a", "b"]), names.of("US"), count)
}
pub fn locales() -> (String, bool, Vec<String>, bool) {
    let maximal = locale::new("en", &LocaleOptions::default()).map(|l| l.maximize().to_string()).unwrap_or_default();
    let canonical = intl::get_canonical_locales(&["EN-us"]).unwrap_or_default();
    (maximal, locale::new("not a tag!", &LocaleOptions::default()).is_err(), canonical, intl::supported_values_of(SupportedValuesKey::Currency).len() > 10)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('new Intl.NumberFormat(["en-US"], { style: "currency", currency: "USD"');
  expect(js).toContain('const format = new Intl.DateTimeFormat(["en-US"], utc);');
  const lib = await import(join(dir, "lib.js"));
  expect(lib.numbers()).toEqual(["$1,234.50", "1234", true, ["-", "$", "1", ".", "50"], "$3.00 – $5.00"]);
  expect(lib.dates()).toEqual(["January 1, 1970", "1 January 1970", ["January", "1", "1970"]]);
  expect(lib.words()).toEqual([["File1", "file2", "file10"], "two", "yesterday", "a or b", "United States", 3]);
  expect(lib.locales()).toEqual(["en-Latn-US", true, ["en-US"], true]);
});

// ADR 0283: JS's typed arrays are the js crate's, each element a Rust number
// of its kind, a `BigInt64Array`'s an `i64`, the `BigInt` rust-js makes one.
test("typed arrays and their buffers are JS's", async () => {
  const withJs = ["--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("js-typed-arrays");
  writeFileSync(join(dir, "lib.rs"), `use js::{array_buffer, big_int64_array, data_view, float64_array, int8_array, uint8_array};
pub fn floats() -> (f64, Option<f64>, u32, String) {
    let a = float64_array::from(&[1.5, -2.0, 3.25]);
    a.set_index(1, 4.0);
    let doubled = a.map(|x| x * 2.0);
    let sorted = a.to_sorted_by(|x, y| y.partial_cmp(&x).unwrap());
    let shifted = sorted.map_with_index(|x, i| x + i as f64);
    (doubled.reduce(|sum, x| sum + x, 0.0), a.at(-1), float64_array::BYTES_PER_ELEMENT, shifted.slice_to_end(1).join("|"))
}
pub fn bytes() -> (u32, u32, i8, Vec<u8>) {
    let buffer = array_buffer::new(8);
    let view = data_view::new(buffer);
    view.set_int16(0, -2, true);
    let bytes = uint8_array::new_with_buffer(buffer);
    let signed = int8_array::of(&[1, -1, 127]);
    (buffer.byte_length(), bytes.length(), signed.get(1).unwrap_or(0), bytes.subarray(0, 2).values().collect())
}
pub fn bigs() -> (i64, bool) {
    let a = big_int64_array::from(&[1, -2, 3]);
    (a.reduce(|sum, x| sum + x, 0), a.includes(-2))
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("const a = Float64Array.from([1.5, -2, 3.25]);\n  a[1] = 4;");
  expect(js).toContain("const signed = Int8Array.of(1, -1, 127);");
  expect(js).toContain("view.setInt16(0, -2, true);");
  const lib = await import(join(dir, "lib.js"));
  expect(lib.floats()).toEqual([17.5, 3.25, 8, "4.25|3.5"]);
  expect(lib.bytes()).toEqual([8, 8, -1, [254, 255]]);
  expect(lib.bigs()).toEqual([2n, true]);
});

// ADR 0283: JS's `Date` is the js crate's, a type with its members as
// methods and its constructors and statics a module's, each the JS it names.
test("a Date is JS's, its members methods", async () => {
  const withJs = ["--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("js-date");
  writeFileSync(join(dir, "lib.rs"), `use js::{Date, date};
pub fn moment(time: f64) -> (f64, f64, f64, String) {
    let d = date::new_with_time(time);
    d.set_utc_hours(12);
    d.set_utc_minutes_s(30, 15);
    (d.get_utc_full_year(), d.get_utc_month(), d.get_time(), d.to_iso_string().unwrap())
}
pub fn invalid() -> (bool, bool, Option<String>) {
    let d = date::new_with_text("not a date");
    (d.get_time().is_nan(), d.to_iso_string().is_err(), d.to_json())
}
pub fn utc() -> f64 {
    date::utc_ymd(2026, 9, 8)
}
pub fn now_is_recent(d: &Date) -> bool {
    date::now() - d.get_time() < 60_000.0
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("const d = new Date(time);\n  d.setUTCHours(12);\n  d.setUTCMinutes(30, 15);");
  expect(js).toContain("return Date.UTC(2026, 9, 8);");
  expect(js).toContain("return Date.now() - d.getTime() < 60000;");
  const lib = await import(join(dir, "lib.js"));
  expect(lib.moment(0)).toEqual([1970, 0, 45_015_000, "1970-01-01T12:30:15.000Z"]);
  expect(lib.invalid()).toEqual([true, true, null]);
  expect(lib.utc()).toBe(Date.UTC(2026, 9, 8));
  expect(lib.now_is_recent(new Date())).toBe(true);
});

// A struct whose JS is its JSON, `unsafe impl JsonText`, is written by
// `JSON.stringify`, indented, as react.dev's Sandpack template writes its
// package.json: `JSON.stringify({ .. }, null, 2)`.
test("a struct whose JS is its JSON is stringified as it is", async () => {
  const withJs = ["--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("json-text");
  writeFileSync(join(dir, "lib.rs"), `use js::{JsonText, json};
pub struct Dependencies {
    pub react: &'static str,
    #[cfg_attr(rust_js, rust_js::name = "react-dom")]
    pub react_dom: &'static str,
}
pub struct Package {
    pub name: &'static str,
    pub version: Option<&'static str>,
    pub files: Vec<String>,
    pub dependencies: Dependencies,
}
unsafe impl JsonText for Dependencies {}
unsafe impl JsonText for Package {}
pub fn manifest(version: Option<&'static str>) -> String {
    json::stringify_with(
        &Package {
            name: "react.dev",
            version,
            files: vec!["a.js".to_string()],
            dependencies: Dependencies { react: "19", react_dom: "19" },
        },
        None,
        2,
    )
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('dependencies: { react: "19", "react-dom": "19" },\n    },\n    null,\n    2,\n  );');
  const lib = await import(join(dir, "lib.js"));
  const want = { name: "react.dev", files: ["a.js"], dependencies: { react: "19", "react-dom": "19" } };
  expect(lib.manifest(undefined)).toBe(JSON.stringify(want, null, 2));
  expect(JSON.parse(lib.manifest("1")).version).toBe("1");
});

// ADR 0272: Node's modules, as @types/node types them, are the JS a person
// writes in Node: an import of `fs`'s, and `process`, a global, as
// react.dev's errors page reads its Markdown in `getStaticProps`.
test("node's modules are imported, and its globals called", async () => {
  const dir = fixture("node");
  run(["node/build.sh", "-o", join(dir, "libnode.rmeta")]);
  writeFileSync(join(dir, "lib.rs"), `use node::{BufferEncoding, fs, process};

pub fn read(path: &str) -> String {
    match fs::read_file_sync(path, BufferEncoding::Utf8) {
        Ok(text) => text,
        Err(_) => "missing".to_string(),
    }
}

pub fn here() -> String {
    process::cwd()
}

pub fn twice(path: &str) -> String {
    let text = match fs::read_file_sync(path, BufferEncoding::Utf8) {
        Ok(text) => text,
        Err(_) => "missing".to_string(),
    };
    format!("{text}{text}")
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `node=${join(dir, "libnode.rmeta")}`, "-L", dir]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('import { readFileSync } from "fs";');
  // A `match` of what it threw is JS's `try`, as the page has it (ADR 0035).
  expect(js).toContain('export function read(path) {\n  try {\n    return readFileSync(path, "utf8");\n  } catch {\n    return "missing";\n  }\n}');
  expect(js).toContain("return process.cwd();");
  // One whose value is a variable's too, as react.dev's errors page reads
  // its `.md` or the generic one, not a conditional of `$try` (ADR 0309).
  expect(js).toContain('  let text;\n  try {\n    text = readFileSync(path, "utf8");\n  } catch {\n    text = "missing";\n  }');
  writeFileSync(join(dir, "note.md"), "# Note");
  const { read, here } = await import(join(dir, "lib.js"));
  expect([read(join(dir, "note.md")), read(join(dir, "none.md")), here()]).toEqual(["# Note", "missing", process.cwd()]);
  const lib = await import(join(dir, "lib.js"));
  expect([lib.twice(join(dir, "note.md")), lib.twice(join(dir, "none.md"))]).toEqual(["# Note# Note", "missingmissing"]);
});

// ADR 0272: `path`, each of @types/node's: its functions, its constants,
// and a platform's own, `win32.join`, as Node's `path` has them.
test("node's path module is bound whole", async () => {
  const dir = fixture("node-path");
  run(["node/build.sh", "-o", join(dir, "libnode.rmeta")]);
  writeFileSync(join(dir, "lib.rs"), `use node::path::{self, FormatInputPathObject, ParsedPath, PlatformPath};

fn parts(parsed: ParsedPath) -> String {
    format!("{}|{}|{}|{}|{}", parsed.root, parsed.dir, parsed.base, parsed.ext, parsed.name)
}

pub fn all() -> Vec<String> {
    vec![
        path::normalize("/a//b/../c"),
        path::join(&["a", "b", "../c"]),
        path::resolve(&["/a", "b"]),
        path::relative("/a/b", "/a/c"),
        path::dirname("/a/b.txt"),
        path::basename("/a/b.txt"),
        path::basename_with_suffix("/a/b.txt", ".txt"),
        path::extname("x.md"),
        path::sep.to_string(),
        path::delimiter.to_string(),
        parts(path::parse("/home/u/file.txt")),
        path::format(FormatInputPathObject { dir: Some("/x"), base: Some("y.js"), ..Default::default() }),
        path::to_namespaced_path("/a"),
        format!("{} {}", path::is_absolute("/a"), path::matches_glob("a/b.js", "a/*.js")),
    ]
}

pub fn platform(platform: &PlatformPath) -> Vec<String> {
    vec![
        platform.join(&["a", "b"]),
        platform.sep(),
        platform.delimiter(),
        platform.basename_with_suffix("c:/x/y.txt", ".txt"),
        parts(platform.parse("/a/b.c")),
        platform.posix().sep(),
    ]
}

pub fn platforms() -> Vec<Vec<String>> {
    vec![platform(path::posix), platform(path::win32)]
}

pub fn spread(parts: &[&str]) -> String {
    path::join(parts)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `node=${join(dir, "libnode.rmeta")}`, "-L", dir]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('join("a", "b", "../c")');
  expect(js).toContain("return join(...parts);");
  expect(js).toContain('platform.join("a", "b")');
  const lib = await import(join(dir, "lib.js"));
  const path = await import("node:path");
  const parts = (p: { root: string; dir: string; base: string; ext: string; name: string }) => [p.root, p.dir, p.base, p.ext, p.name].join("|");
  expect(lib.all()).toEqual([
    path.normalize("/a//b/../c"),
    path.join("a", "b", "../c"),
    path.resolve("/a", "b"),
    path.relative("/a/b", "/a/c"),
    path.dirname("/a/b.txt"),
    path.basename("/a/b.txt"),
    "b",
    ".md",
    path.sep,
    path.delimiter,
    "/|/home/u|file.txt|.txt|file",
    "/x/y.js",
    path.toNamespacedPath("/a"),
    "true true",
  ]);
  const platform = (p: typeof path.posix) => [p.join("a", "b"), p.sep, p.delimiter, p.basename("c:/x/y.txt", ".txt"), parts(p.parse("/a/b.c")), "/"];
  expect(lib.platforms()).toEqual([platform(path.posix), platform(path.win32)]);
  expect(lib.spread(["x", "y"])).toBe(path.join("x", "y"));
});

// ADR 0272: `process`, the global, read and written, run by Node itself,
// whose own `process` says what each should be.
test("node's process global is read and written", () => {
  const dir = fixture("node-process");
  run(["node/build.sh", "-o", join(dir, "libnode.rmeta")]);
  writeFileSync(join(dir, "lib.rs"), `use node::js::dict;
use node::process::{self, NumberOrStr, ProcessFeaturesTypeScript};

pub fn all() -> Vec<String> {
    let here = process::cwd();
    let _ = process::chdir("/");
    let root = process::cwd();
    let _ = process::chdir(&here);
    dict::set(process::env(), "RUST_JS_TEST", "set".to_string());
    let since = process::cpu_usage_with_previous_value(process::cpu_usage());
    let time = process::hrtime_with_time(process::hrtime());
    process::set_exit_code(3.0);
    let code = match process::exit_code() {
        Some(NumberOrStr::Number(n)) => n.to_string(),
        _ => "?".to_string(),
    };
    process::set_exit_code(0.0);
    process::set_trace_process_warnings(true);
    process::next_tick(Box::new(|| println!("tick")));
    println!("now");
    vec![
        process::argv().len().to_string(),
        format!("{:?}", process::platform()),
        format!("{:?}", process::arch()),
        root,
        (process::cwd() == here).to_string(),
        dict::get(process::env(), "RUST_JS_TEST").cloned().unwrap_or_default(),
        process::exec_path(),
        process::version(),
        dict::get(process::versions(), "node").cloned().unwrap_or_default(),
        (process::pid() > 0.0 && process::ppid() > 0.0 && process::uptime() > 0.0).to_string(),
        (time.0 >= 0.0 && process::hrtime_bigint() > 0).to_string(),
        (process::memory_usage().heap_used > 0.0 && process::memory_usage_rss() > 0.0).to_string(),
        (since.user >= 0.0 && process::resource_usage().max_rss > 0.0).to_string(),
        process::release().name,
        match process::features().typescript {
            ProcessFeaturesTypeScript::Strip => "strip".to_string(),
            ProcessFeaturesTypeScript::Transform => "transform".to_string(),
            ProcessFeaturesTypeScript::None(_) => "false".to_string(),
        },
        process::config().variables.host_arch,
        code,
        process::kill_with_signal(process::pid(), 0.0).map(|sent| sent.to_string()).unwrap_or_default(),
        format!("{} {}", process::get_builtin_module("fs").is_some(), process::get_builtin_module("nope").is_some()),
        process::allowed_node_environment_flags().contains("--max-old-space-size").to_string(),
        process::trace_process_warnings().to_string(),
        process::has_uncaught_exception_capture_callback().to_string(),
        process::getuid().to_string(),
        process::argv0(),
        process::exec_argv().len().to_string(),
    ]
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `node=${join(dir, "libnode.rmeta")}`, "-L", dir]);
  writeFileSync(join(dir, "package.json"), '{ "type": "module" }');
  writeFileSync(join(dir, "run.js"), `import { all } from "./lib.js";
const cap = (s) => s[0].toUpperCase() + s.slice(1);
const got = all();
const want = [
  String(process.argv.length), cap(process.platform), cap(process.arch), "/", "true", "set", process.execPath, process.version,
  process.versions.node, "true", "true", "true", "true", process.release.name, String(process.features.typescript),
  process.config.variables.host_arch, "3", "true", "true false", "true", "true", "false", String(process.getuid()), process.argv0,
  String(process.execArgv.length),
];
console.log(JSON.stringify({ got, want }));
`);
  const p = Bun.spawnSync(["node", "run.js"], { cwd: dir, stderr: "pipe" });
  const out = p.stdout.toString();
  expect([p.exitCode, p.stderr.toString()]).toEqual([0, ""]);
  const lines = out.trim().split("\n");
  // `nextTick`'s callback runs once the script's own code is done.
  expect([lines[0], lines[2]]).toEqual(["now", "tick"]);
  const { got, want } = JSON.parse(lines[1]);
  expect(got).toEqual(want);
});

// ADR 0272: `querystring`, a query's text from a dictionary of values and
// back, each value one text or a list of them.
test("node's querystring module is bound whole", async () => {
  const dir = fixture("node-querystring");
  run(["node/build.sh", "-o", join(dir, "libnode.rmeta")]);
  writeFileSync(join(dir, "lib.rs"), `use node::js::dict;
use node::querystring::{
    self, ParseOptions, ParsedUrlQuery, ParsedUrlQueryInput, ParsedUrlQueryInputItem, ParsedUrlQueryInputValue,
    ParsedUrlQueryValue, StringifyOptions,
};

fn shown(query: &ParsedUrlQuery) -> String {
    dict::entries(query)
        .into_iter()
        .map(|(key, value)| match value {
            ParsedUrlQueryValue::One(one) => format!("{key}={one}"),
            ParsedUrlQueryValue::Many(many) => format!("{key}=[{}]", many.join(",")),
        })
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn all() -> Vec<String> {
    let input: &ParsedUrlQueryInput = dict::new();
    dict::set(input, "name", Some(ParsedUrlQueryInputValue::Str("a b".to_string())));
    dict::set(input, "n", Some(ParsedUrlQueryInputValue::Number(2.0)));
    dict::set(input, "ok", Some(ParsedUrlQueryInputValue::Bool(true)));
    dict::set(input, "big", Some(ParsedUrlQueryInputValue::BigInt(7)));
    dict::set(input, "tags", Some(ParsedUrlQueryInputValue::List(vec![ParsedUrlQueryInputItem::Str("x".to_string()), ParsedUrlQueryInputItem::Number(1.0)])));
    dict::set(input, "none", None);
    vec![
        querystring::stringify(input),
        querystring::stringify_with_sep(input, ";"),
        querystring::stringify_with_sep_and_eq(input, ";", ":"),
        querystring::stringify_with_sep_and_eq_and_options(input, "&", "=", StringifyOptions { encode_uri_component: Some(Box::new(|s: &str| s.to_uppercase())) }),
        querystring::encode(input),
        shown(querystring::parse("a=1&b=2&a=3")),
        shown(querystring::parse_with_sep("a=1;b=2", ";")),
        shown(querystring::parse_with_sep_and_eq("a:1;b:2", ";", ":")),
        shown(querystring::parse_with_sep_and_eq_and_options("a=1&b=2&c=3", "&", "=", ParseOptions { max_keys: Some(2.0), ..Default::default() })),
        shown(querystring::decode("x=%20y")),
        querystring::escape("a b&c"),
        querystring::unescape("a%20b"),
    ]
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `node=${join(dir, "libnode.rmeta")}`, "-L", dir]);
  expect(readFileSync(join(dir, "lib.js"), "utf8")).toContain('from "querystring";');
  const lib = await import(join(dir, "lib.js"));
  const qs = await import("node:querystring");
  const input = { name: "a b", n: 2, ok: true, big: 7n, tags: ["x", 1], none: undefined };
  expect(lib.all()).toEqual([
    qs.stringify(input),
    qs.stringify(input, ";"),
    qs.stringify(input, ";", ":"),
    qs.stringify(input, "&", "=", { encodeURIComponent: (s: string) => s.toUpperCase() }),
    qs.stringify(input),
    "a=[1,3] b=2",
    "a=1 b=2",
    "a=1 b=2",
    "a=1 b=2",
    "x= y",
    qs.escape("a b&c"),
    "a b",
  ]);
});

// ADR 0272: `url`, its WHATWG classes webapi's, as Node's are the globals,
// and its own: file URLs, domains, and a URL written without its parts.
test("node's url module binds its own and the WHATWG classes", async () => {
  const dir = fixture("node-url");
  run(["node/build.sh", "-o", join(dir, "libnode.rmeta")]);
  writeFileSync(join(dir, "lib.rs"), `use node::url::{self, FileUrlToPathOptions, PathToFileUrlOptions, URLFormatOptions, URLSearchParams};
use node::webapi;

pub fn all() -> Vec<String> {
    let file = url::path_to_file_url("/tmp/a b.txt");
    let link = webapi::url::new("https://user:pw@example.com/p?q=1#top");
    let params: &URLSearchParams = link.search_params();
    vec![
        file.href(),
        url::file_url_to_path("file:///tmp/a%20b.txt"),
        url::file_url_to_path(file),
        url::file_url_to_path_with_options("file:///C:/x", FileUrlToPathOptions { windows: Some(true) }),
        url::path_to_file_url_with_options("C:\\\\x y", PathToFileUrlOptions { windows: Some(true) }).href(),
        url::domain_to_ascii("español.com"),
        url::domain_to_unicode("xn--espaol-zwa.com"),
        url::resolve("/one/two", "three"),
        url::format(link),
        url::format_with_options(link, URLFormatOptions { auth: Some(false), fragment: Some(false), ..Default::default() }),
        params.get("q").unwrap_or_default(),
    ]
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `node=${join(dir, "libnode.rmeta")}`, "-L", dir]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('from "url";');
  const lib = await import(join(dir, "lib.js"));
  const url = await import("node:url");
  const link = new URL("https://user:pw@example.com/p?q=1#top");
  expect(lib.all()).toEqual([
    url.pathToFileURL("/tmp/a b.txt").href,
    "/tmp/a b.txt",
    "/tmp/a b.txt",
    url.fileURLToPath("file:///C:/x", { windows: true }),
    url.pathToFileURL("C:\\x y", { windows: true }).href,
    url.domainToASCII("español.com"),
    url.domainToUnicode("xn--espaol-zwa.com"),
    url.resolve("/one/two", "three"),
    url.format(link),
    url.format(link, { auth: false, fragment: false }),
    "1",
  ]);
});

// ADR 0272: `url`'s legacy `parse` and `format`, a URL's parts, `null`
// where it hasn't one, and its query a text or parsed.
test("node's url module parses and formats legacy URL objects", async () => {
  const dir = fixture("node-url-legacy");
  run(["node/build.sh", "-o", join(dir, "libnode.rmeta")]);
  writeFileSync(join(dir, "lib.rs"), `use node::js::dict;
use node::querystring::{ParsedUrlQueryInput, ParsedUrlQueryInputValue, ParsedUrlQueryValue};
use node::url::{self, Url, UrlObject, UrlObjectPort, UrlObjectQuery, UrlQuery, UrlWithStringQuery};

fn or(part: Option<String>) -> String {
    part.unwrap_or_else(|| "-".to_string())
}

pub fn parsed() -> Vec<String> {
    let link: UrlWithStringQuery = url::parse("https://u:p@example.com:8080/a/b?x=1&x=2#h");
    let bare: UrlWithStringQuery = url::parse("/only/path");
    let query: Url = url::parse_with_parse_query_string("https://example.com/?x=1&x=2&y=3", true);
    let host: Url = url::parse_with_parse_query_string_and_slashes_denote_host("//example.com/p", false, true);
    let parsed = match query.query {
        Some(UrlQuery::Parsed(parsed)) => match dict::get(parsed, "x") {
            Some(ParsedUrlQueryValue::Many(many)) => many.join(","),
            _ => "?".to_string(),
        },
        _ => "?".to_string(),
    };
    vec![
        link.href,
        or(link.protocol),
        or(link.auth),
        or(link.host),
        or(link.port),
        or(link.hostname),
        or(link.hash),
        or(link.search),
        or(link.query),
        or(link.pathname),
        or(link.path),
        format!("{:?}", link.slashes),
        or(bare.host),
        or(bare.query),
        parsed,
        or(host.host),
    ]
}

pub fn formatted() -> Vec<String> {
    let query: &ParsedUrlQueryInput = dict::new();
    dict::set(query, "q", Some(ParsedUrlQueryInputValue::Str("a b".to_string())));
    vec![
        url::format_with_url_object(UrlObject {
            protocol: Some("https"),
            hostname: Some("example.com"),
            port: Some(UrlObjectPort::Number(8080.0)),
            pathname: Some("/x"),
            query: Some(UrlObjectQuery::Input(query)),
            ..Default::default()
        }),
        url::format_with_url_object(UrlObject { host: Some("example.com"), query: Some(UrlObjectQuery::Str("a=1")), ..Default::default() }),
        url::format_with_str("https://example.com/a?b#c"),
    ]
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `node=${join(dir, "libnode.rmeta")}`, "-L", dir]);
  const lib = await import(join(dir, "lib.js"));
  const url = await import("node:url");
  const link = url.parse("https://u:p@example.com:8080/a/b?x=1&x=2#h");
  const or = (part: string | null) => part ?? "-";
  expect(lib.parsed()).toEqual([
    link.href, or(link.protocol), or(link.auth), or(link.host), or(link.port), or(link.hostname), or(link.hash), or(link.search),
    or(link.query), or(link.pathname), or(link.path), "Some(true)", "-", "-", "1,2", "example.com",
  ]);
  expect(lib.formatted()).toEqual([
    url.format({ protocol: "https", hostname: "example.com", port: 8080, pathname: "/x", query: { q: "a b" } }),
    url.format({ host: "example.com", query: "a=1" }),
    url.format("https://example.com/a?b#c"),
  ]);
});

// Node's http request and response, as a server's handler is given them,
// and Next.js's Pages Router: what's read of one and written to the other.
test("node's http request is read and its response written", async () => {
  const dir = fixture("node-http");
  run(["node/build.sh", "-o", join(dir, "libnode.rmeta")]);
  writeFileSync(join(dir, "lib.rs"), `use node::http::{IncomingMessage, ServerResponse};

pub fn handle(req: &IncomingMessage, res: &ServerResponse) {
    res.set_status_code(201.0);
    res.set_header("x-method", req.method().unwrap_or_default().as_str());
    res.set_header("x-many", &["a", "b"][..]);
    let seen = req.has_header_named("x-seen");
    res.end_with(&format!("{} {} {}", req.url().unwrap_or_default(), req.http_version(), seen));
}
`.replace("let seen = req.has_header_named(\"x-seen\");", "let seen = req.raw_headers().iter().any(|h| h == \"x-seen\");"));
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `node=${join(dir, "libnode.rmeta")}`, "-L", dir]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("res.statusCode = 201;");
  expect(js).toContain('res.setHeader("x-many", ["a", "b"]);');
  const { handle } = await import(join(dir, "lib.js"));
  const { createServer } = await import("node:http");
  const server = createServer(handle);
  await new Promise<void>((done) => server.listen(0, done));
  try {
    const { port } = server.address() as { port: number };
    const response = await fetch(`http://localhost:${port}/search?q=rust`, { headers: { "x-seen": "1" } });
    expect([response.status, response.headers.get("x-method"), response.headers.get("x-many"), await response.text()]).toEqual([201, "GET", "a, b", "/search?q=rust 1.1 true"]);
  } finally {
    server.close();
  }
});

// The `history` global, a window's, as `document` is, as react.dev's _app
// sets `history.scrollRestoration` where the browser is Safari.
test("the history global is the page's history", async () => {
  const withWeb = ["--", "--extern", `webapi=${join(target, "libwebapi.rmeta")}`, "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("history-global");
  writeFileSync(join(dir, "lib.rs"), `use webapi::history;

pub fn restore() {
    history.set_scroll_restoration("auto");
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withWeb]);
  expect(readFileSync(join(dir, "lib.js"), "utf8")).toContain('history.scrollRestoration = "auto";');
  const previous = (globalThis as any).history;
  (globalThis as any).history = { scrollRestoration: "manual" };
  try {
    (await import(join(dir, "lib.js"))).restore();
    expect((globalThis as any).history.scrollRestoration).toBe("auto");
  } finally {
    (globalThis as any).history = previous;
  }
});

// ADR 0271: JS's truthiness of a value, `!value` as a person tests one, and
// a value of any shape given a type it's vouched to have, the value itself.
test("a JS value is tested as JS tests it, and cast as it's vouched", async () => {
  const withJs = ["--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("truthy");
  writeFileSync(join(dir, "lib.rs"), `use js::Unknown;
pub fn missing(value: Option<&Unknown>) -> bool {
    !js::truthy(value)
}
pub fn present(value: Option<&Unknown>) -> bool {
    js::truthy(value)
}
pub fn shown(value: Option<&Unknown>) -> &'static str {
    if js::truthy(value) { "yes" } else { "no" }
}
pub fn named(name: Option<&str>) -> &'static str {
    if !js::truthy(name) { "none" } else { "some" }
}
pub fn text(value: Option<&'static Unknown>) -> Option<String> {
    unsafe { js::cast(value) }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("export function missing(value) {\n  return !value;\n}");
  expect(js).toContain("export function present(value) {\n  return !!value;\n}");
  expect(js).toContain('export function shown(value) {\n  if (value) {\n    return "yes";\n  }');
  expect(js).toContain('export function named(name) {\n  if (!name) {\n    return "none";\n  }');
  expect(js).toContain("export function text(value) {\n  return value;\n}");
  const lib = await import(join(dir, "lib.js"));
  const values = [undefined, null, 0, "", false, NaN, "a", 1, {}, [], -1];
  expect(values.map(lib.missing)).toEqual([true, true, true, true, true, true, false, false, false, false, false]);
  expect(values.map(lib.present)).toEqual(values.map(Boolean));
  expect(values.map(lib.shown)).toEqual(values.map((v) => (v ? "yes" : "no")));
  expect(["", "a", undefined].map(lib.named)).toEqual(["none", "some", "none"]);
  expect([lib.text("x"), lib.text(undefined)]).toEqual(["x", undefined]);
});

// What's never nullish is a `js::Unknown` too, as any value is TypeScript's
// `unknown`; and `js::string` is JS's `String(value)`, as `result += value`
// makes one: react.dev's console line joins its children's text so.
test("a defined value is an unknown one, and any value is a string as JS makes it", async () => {
  const withWeb = ["--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("unknown-upcast");
  writeFileSync(join(dir, "lib.rs"), `use js::{Unknown, string, unknown};
pub fn message<'a>(text: &'a str, other: Option<&'a Unknown>, use_text: bool) -> Option<&'a Unknown> {
    if use_text { Some(unknown(text)) } else { other }
}
pub fn joined(parts: Vec<Option<&Unknown>>) -> String {
    let mut result = String::new();
    for part in parts {
        result.push_str(&string(part));
    }
    result
}
pub fn counted(n: u32) -> String {
    string(Some(unknown(&n)))
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withWeb]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("return text;");
  expect(js).toContain("String(part)");
  expect(js).toContain("return String(n);");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.message("a", 1, true), lib.message("a", 1, false), lib.message("a", undefined, false)]).toEqual(["a", 1, undefined]);
  expect(lib.joined(["a", 1, undefined, null, { b: 1 }])).toBe("a1undefinednull[object Object]");
  expect(lib.counted(7)).toBe("7");
});

// ADR 0225: a WebIDL parameter typed `any` takes a value as JS has it, and
// one the browser copies, `postMessage`'s, `pushState`'s and
// `structuredClone`'s, only a value it copies as it is: `js::StructuredClone`.
// A closure, which it can't copy, and `Some(None)`, which would arrive as
// rust-js's box, are rustc's errors; `reportError` takes anything still.
test("webapi's any parameters take any value as JS has it, and a clone only one it copies", async () => {
  const withWeb = ["--", "--extern", `webapi=${join(target, "libwebapi.rmeta")}`, "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("any-params");
  const source = `use js::StructuredClone;
use webapi::window;
pub struct Saved {
    pub page: u32,
}
// Its fields are numbers, which the browser copies.
unsafe impl StructuredClone for Saved {}
pub fn save(page: u32) {
    window.history().push_state(Saved { page }, "");
}
pub fn copy(text: &str) -> Option<&'static js::Unknown> {
    window.structured_clone(text)
}
pub fn copies(blob: &webapi::Blob) -> Option<&'static js::Unknown> {
    window.structured_clone((vec![Some(1.5), None], "a", blob))
}
pub fn send(text: String) {
    window.post_message(Some(text), "*");
}
pub fn report(error: &js::JsError) {
    window.report_error(error);
    window.report_error(|| ());
}
`;
  writeFileSync(join(dir, "lib.rs"), source);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withWeb]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('window.history.pushState({ page }, "");');
  expect(js).toContain("return window.structuredClone(text);");
  expect(js).toContain('return window.structuredClone([[1.5, undefined], "a", blob]);');
  expect(js).toContain('window.postMessage(text, "*");');
  expect(js).toContain("window.reportError(error);");
  for (const [written, error] of [
    ["window.structured_clone(|| ())", "the trait `js::StructuredClone` is not implemented for closure"],
    ["window.structured_clone(Some(None::<i32>))", "the trait `js::Defined` is not implemented for `std::option::Option<i32>`"],
    ["window.structured_clone(window)", "the trait `js::StructuredClone` is not implemented for `webapi::Window`"],
  ]) {
    writeFileSync(join(dir, "lib.rs"), source.replace("window.structured_clone(text)", written));
    const result = Bun.spawnSync([compiler, join(dir, "lib.rs"), "-o", join(dir, "wrong.js"), ...withWeb], { cwd: dir });
    expect(result.exitCode).not.toBe(0);
    expect(result.stderr.toString()).toContain(error);
  }
});

// ADR 0225: JSON is a typed value, `js::Json`, as ReScript's `JSON.t` is,
// and an object of it a `js::Dict`, ReScript's `dict`, TypeScript's
// `Record<string, T>`: a JSON `null` is `None`, a key that isn't there too,
// told apart by `dict::get`.
test("JSON is a typed value, and its objects dictionaries", async () => {
  const withWeb = ["--", "--extern", `webapi=${join(target, "libwebapi.rmeta")}`, "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("json");
  writeFileSync(join(dir, "lib.rs"), `use js::{Dict, Json, dict};
fn show(value: Option<Json>) -> String {
    match value {
        None => "null".to_string(),
        Some(Json::String(text)) => format!("'{text}'"),
        Some(Json::Number(n)) => n.to_string(),
        Some(Json::Bool(b)) => b.to_string(),
        Some(Json::Array(items)) => format!("[{}]", items.iter().map(|item| show(*item)).collect::<Vec<_>>().join(",")),
        Some(Json::Object(fields)) => format!(
            "{{{}}}",
            dict::entries(fields).into_iter().map(|(key, value)| format!("{key}:{}", show(value))).collect::<Vec<_>>().join(",")
        ),
    }
}
pub fn parsed(text: &str) -> String {
    match Json::parse(text) {
        Ok(value) => show(value),
        Err(_) => "invalid".to_string(),
    }
}
pub fn lookup(text: &str, key: &str) -> String {
    match Json::parse(text) {
        Ok(Some(Json::Object(fields))) => match dict::get(fields, key) {
            None => "missing".to_string(),
            Some(value) => show(*value),
        },
        _ => "not an object".to_string(),
    }
}
pub fn round_trip(text: &str) -> String {
    match Json::parse(text) {
        Ok(Some(value)) => Json::stringify(&value),
        _ => "null".to_string(),
    }
}
pub fn built() -> Vec<String> {
    let numbers: &Dict<f64> = dict::from_entries(vec![("a".to_string(), 1.0)]);
    dict::set(numbers, "b", 2.0);
    dict::keys(numbers)
}
pub fn counted(key: &str) -> f64 {
    let counts: &Dict<f64> = dict::from_entries(vec![("a".to_string(), 1.0)]);
    dict::get(counts, key).copied().unwrap_or(0.0)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withWeb]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  // A lookup is the runtime's `$dictGet`, imported with its other helpers.
  expect(js).toContain('import { $dictGet, $displayF64, $someValue, $try } from "@rust-js/runtime";');
  expect(js).toContain("Object.entries(value)");
  expect(js).toContain("const match$1 = $dictGet(match._0, key);");
  expect(js).toContain("return JSON.stringify(match._0);");
  expect(js).toContain("const numbers = { a: 1 };\n  numbers.b = 2;");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.parsed('{"a":[1,"x",null,true],"b":{}}'), lib.parsed("null"), lib.parsed("nope")])
    .toEqual(["{a:[1,'x',null,true],b:{}}", "null", "invalid"]);
  expect([lib.lookup('{"a":null,"b":1}', "a"), lib.lookup('{"a":null,"b":1}', "b"), lib.lookup('{"a":null}', "c"), lib.lookup('{"a":null}', "toString"), lib.lookup("[1]", "a")])
    .toEqual(["null", "1", "missing", "missing", "not an object"]);
  expect(lib.round_trip('{"a": [1, null]}')).toBe('{"a":[1,null]}');
  expect(lib.built()).toEqual(["a", "b"]);
  // A dictionary of what isn't an Option is read as JS reads one (ADR 0225).
  expect(js).toContain("return counts[key] ?? 0;");
  expect([lib.counted("a"), lib.counted("b")]).toEqual([1, 0]);
});

// An element's constructor is WebIDL's `[HTMLConstructor]`, which only a
// custom element's class can call: `new HTMLDivElement()` in a page throws
// "Illegal constructor". So webapi binds none, and an element is made with
// `document.create_element(..)`. Other constructors, `new Event`, it binds.
test("the webapi crate binds no element constructor, which a page can't call", () => {
  const lib = readFileSync(join(root, "webapi", "src", "lib.rs"), "utf8");
  expect(lib.match(/#\[link_name = "new HTML\w*"\]/g) ?? []).toEqual([]);
  expect(lib).toContain('#[link_name = "new Event"]');
  expect(lib).toContain('#[link_name = "new TextEncoder"]');
});

// A binding's last parameter, a slice, can be JS's rest arguments,
// `#[rust_js::variadic]`: a slice written out is the arguments, as
// react.dev calls `cn("a", className)`, and another one spread (ADR 0221).
test("a variadic binding takes a slice as its rest arguments", async () => {
  const dir = fixture("variadic");
  writeFileSync(join(dir, "lib.rs"), `unsafe extern "Rust" {
    #[link_name = "Math.max"]
    #[rust_js::variadic]
    safe fn max(values: &[f64]) -> f64;
}
pub fn largest(a: f64, b: f64) -> f64 {
    max(&[a, b, 1.0])
}
pub fn largest_of(values: Vec<f64>) -> f64 {
    max(&values)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("return Math.max(a, b, 1);");
  expect(js).toContain("return Math.max(...values);");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.largest(-3, -2), lib.largest(4, 2), lib.largest_of([2, 9, 4])]).toEqual([1, 4, 9]);
});

// A variadic binding's last parameter may be a tuple, as an emitter's
// event's arguments are, of each type: the arguments themselves too, none
// of \`()\`, and of a type parameter, its type at the call.
test("a variadic binding takes a tuple as its rest arguments", async () => {
  const dir = fixture("variadic-tuple");
  writeFileSync(join(dir, "lib.rs"), `pub trait Event {
    type Args;
}

pub struct Pair;
pub struct Nothing;

impl Event for Pair {
    type Args = (f64, &'static str);
}

impl Event for Nothing {
    type Args = ();
}

#[rust_js::link_name = "globalThis.called"]
#[rust_js::variadic]
#[allow(unused_variables)]
fn called<E: Event>(event: &str, args: E::Args) -> String {
    unreachable!()
}

#[rust_js::link_name = "globalThis.called"]
#[rust_js::variadic]
#[allow(unused_variables)]
fn pair(event: &str, args: (f64, &str)) -> String {
    unreachable!()
}

pub fn calls(held: (f64, &str)) -> Vec<String> {
    vec![
        called::<Pair>("pair", (1.0, "a")),
        called::<Nothing>("nothing", ()),
        pair("held", held),
    ]
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('globalThis.called("pair", 1, "a")');
  expect(js).toContain('globalThis.called("nothing")');
  expect(js).toContain('globalThis.called("held", ...held)');
  const lib = await import(join(dir, "lib.js"));
  (globalThis as any).called = (...args: unknown[]) => JSON.stringify(args);
  expect(lib.calls([2, "b"])).toEqual(['["pair",1,"a"]', '["nothing"]', '["held",2,"b"]']);
});

// ADR 0269: a window's own functions are called bare too, as TypeScript's DOM
// declares them globals and react.dev's NavigationBar asks `confirm('Clear
// all your edits?')`. `window.confirm(..)` is still the method's.
test("a window's own functions are called bare", async () => {
  const withWeb = ["--", "--extern", `webapi=${join(target, "libwebapi.rmeta")}`, "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("window-global");
  writeFileSync(join(dir, "lib.rs"), `use webapi::{window, window_global};

pub fn cleared() -> bool {
    window_global::confirm_with_message("Clear all your edits?")
}

pub fn asked() -> bool {
    window.confirm_with_message("Sure?")
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withWeb]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('return confirm("Clear all your edits?");');
  expect(js).toContain('return window.confirm("Sure?");');
});

// ADR 0221: a binding that skips what's falsy, `#[rust_js::skips_falsy]`, as
// classnames does, is given an argument shown only if a test holds as `test
// && value`, as react.dev writes `cn('a', isExpanded && 'sp-layout-expanded')`.
test("a binding that skips what's falsy is given test && value", async () => {
  const dir = fixture("skips-falsy");
  writeFileSync(join(dir, "cn.js"), 'export default (...classes) => classes.filter(Boolean).join(" ");\n');
  writeFileSync(join(dir, "lib.rs"), `unsafe extern "Rust" {
    #[link_name = "./cn.js#default"]
    #[rust_js::variadic]
    #[rust_js::skips_falsy]
    safe fn cn(classes: &[Option<&str>]) -> String;
    #[link_name = "./cn.js#default"]
    #[rust_js::variadic]
    safe fn plain(classes: &[Option<&str>]) -> String;
}
pub fn classes(expanded: bool, shown: bool) -> String {
    cn(&[Some("a"), expanded.then_some("wide"), (!expanded && shown).then_some("tall")])
}
pub fn listed(expanded: bool) -> String {
    plain(&[Some("a"), expanded.then_some("wide")])
}
pub struct Failure(pub u32);
pub fn flagged(error: Option<&Failure>, ready: bool) -> String {
    cn(&[Some("a"), (error.is_some() || ready).then_some("b")])
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('return cn("a", expanded && "wide", !expanded && shown && "tall");');
  expect(js).toContain('return cn("a", expanded ? "wide" : undefined);');
  expect(js).toContain('return cn("a", (error || ready) && "b");');
  const lib = await import(join(dir, "lib.js"));
  expect([lib.classes(true, true), lib.classes(false, true), lib.classes(false, false), lib.listed(true), lib.flagged([1], false), lib.flagged(undefined, false)]).toEqual(["a wide", "a tall", "a", "a wide", "a b", "a"]);
});

// ADR 0301: a binding that can't throw, `#[rust_js::cannot_throw]`, as reading
// React's `ref.current` can't, read before a value is moved leaves it nothing
// to drop: a library's function takes no drop for it, and has no `try`, as
// react.dev's `useEvent` reads its ref, then calls with its arguments. So
// does `unwrap_unchecked`, which can't panic. One unmarked keeps both.
test("a binding that can't throw, read before a move, leaves nothing to drop", async () => {
  const dir = fixture("cannot-throw");
  writeFileSync(join(dir, "lib.rs"), `unsafe extern "Rust" {
    #[link_name = "get globalThis.calls"]
    #[rust_js::cannot_throw]
    safe fn calls() -> u32;
    #[link_name = "get globalThis.calls"]
    safe fn calls_unmarked() -> u32;
}
pub struct Counted<T>(pub u32, pub T);
pub fn count<T>(value: T) -> Counted<T> {
    let n = calls();
    Counted(n, value)
}
pub fn count_unmarked<T>(value: T) -> Counted<T> {
    let n = calls_unmarked();
    Counted(n, value)
}
pub fn first<T>(value: T, given: Option<u32>) -> Counted<T> {
    let n = unsafe { given.unwrap_unchecked() };
    Counted(n, value)
}
pub fn call<A: 'static>(value: A, given: Option<&'static dyn Fn(A) -> u32>) -> u32 {
    let f = unsafe { given.unwrap_unchecked() };
    f(value)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--library", "--manifest", join(dir, "lib.manifest.json"),
    "--", "--crate-name", "counted", `--emit=metadata=${join(dir, "libcounted.rmeta")}`]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("export function count(value) {\n  const n = globalThis.calls;\n  return [n, value];\n}");
  expect(js).toContain("export function first(value, given) {\n  const n = given;\n  return [n, value];\n}");
  expect(js).toContain("export function call(value, given) {\n  const f = given;\n  return f(value);\n}");
  expect(js).toContain("export function count_unmarked(value, dropT) {");
  (globalThis as { calls?: number }).calls = 3;
  const lib = await import(join(dir, "lib.js"));
  expect([lib.count("a"), lib.count_unmarked("b"), lib.first("c", 4), lib.call(5, (n: number) => n + 1)]).toEqual([[3, "a"], [3, "b"], [4, "c"], 6]);
});

// ADR 0304: `js::import!(item)` loads the module that has it when it's asked
// for, as react.dev's useSandpackLint does, `const { runESLint } = await
// import("./runESLint")`: a crate's function or a binding's, which nothing
// imports statically. `js::import_module!(item)`, of a module's default
// export, is the module itself, for React's `lazy`.
test("js::import! loads a module when it's asked for", async () => {
  const withWeb = ["--", "--extern", `webapi=${join(target, "libwebapi.rmeta")}`, "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("dynamic-import");
  writeFileSync(join(dir, "math.js"), "export const twice = (n) => n * 2;\n");
  writeFileSync(join(dir, "lint.rs"), "pub fn run(n: u32) -> u32 {\n    n + 1\n}\n");
  writeFileSync(join(dir, "root.rs"), "pub fn Root() -> u32 {\n    7\n}\njs::export_default!(Root);\n");
  writeFileSync(join(dir, "lib.rs"), `#![allow(non_snake_case)]
mod lint;
mod root;
unsafe extern "Rust" {
    #[link_name = "./math.js#twice"]
    safe fn twice(n: f64) -> f64;
}
pub async fn linted(n: u32) -> u32 {
    let run = js::import!(lint::run).await;
    run(n)
}
pub async fn doubled(n: f64) -> f64 {
    let twice = js::import!(twice).await;
    twice(n)
}
pub async fn named(n: u32) -> u32 {
    let next = js::import!(lint::run).await;
    next(n)
}
pub async fn rooted() -> u32 {
    let module = js::import_module!(root::Root).await;
    module.default()()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withWeb]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('const { run } = await import("./lint.js");');
  expect(js).toContain('const { twice } = await import("./math.js");');
  expect(js).toContain('const { run: next } = await import("./lint.js");');
  expect(js).toContain('const module = await import("./root.js");');
  expect(js).not.toMatch(/^import /m);
  const lib = await import(join(dir, "lib.js"));
  expect([await lib.linted(1), await lib.doubled(3), await lib.named(2), await lib.rooted()]).toEqual([2, 6, 3, 7]);
});

// ADR 0305: a binding marked `#[rust_js::require]` is read through
// `require(module)`, where it's read, as react.dev's runESLint reads
// `require('eslint-plugin-react-hooks').rules`: nothing imports it.
test("a required binding is read through require", async () => {
  const withWeb = ["--", "--extern", `webapi=${join(target, "libwebapi.rmeta")}`, "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("required");
  writeFileSync(join(dir, "lib.rs"), `#![allow(non_upper_case_globals)]
unsafe extern "Rust" {
    #[link_name = "get node:path#sep"]
    #[rust_js::require]
    safe fn sep() -> String;
    #[link_name = "get node:path#delimiter"]
    safe fn delimiter() -> String;
}
thread_local! {
    static separator: String = sep();
}
pub fn joined(a: &str, b: &str) -> String {
    separator.with(|s| format!("{a}{s}{b}"))
}
pub fn listed() -> String {
    delimiter()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withWeb]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('const separator = require("node:path").sep;');
  expect(js).toContain('import { delimiter } from "node:path";');
  expect(js).not.toContain("import { sep");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.joined("a", "b"), lib.listed()]).toEqual(["a/b", ":"]);
});

// ADR 0307: a trait's method that's a binding is read of the value itself,
// as react.dev's useSandpackLint reads `error.ruleId` of ESLint's errors and
// CodeMirror's diagnostics alike: generic code takes no dictionary, and an
// impl, empty, makes none.
test("a trait's binding method is read of the value, with no dictionary", async () => {
  const dir = fixture("binding-trait");
  writeFileSync(join(dir, "lib.rs"), `pub struct LintError {
    pub rule_id: Option<String>,
    pub line: f64,
}

pub struct Diagnostic {
    pub rule_id: Option<String>,
    pub from: f64,
}

pub trait RuleIdentified {
    #[rust_js::link_name = "get ruleId"]
    fn rule_id(&self) -> &Option<String> {
        unreachable!()
    }
}

impl RuleIdentified for LintError {}
impl RuleIdentified for Diagnostic {}

fn isReactRuleError<E: RuleIdentified>(error: &E) -> bool {
    error.rule_id().is_some()
}

pub fn lint(errors: Vec<LintError>, diagnostics: Vec<Diagnostic>) -> (Vec<f64>, Vec<f64>) {
    (
        errors.into_iter().filter(isReactRuleError).map(|e| e.line).collect(),
        diagnostics.into_iter().filter(isReactRuleError).map(|d| d.from).collect(),
    )
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("function isReactRuleError(error) {\n  return error.ruleId != null;\n}");
  expect(js).not.toContain("RuleIdentified");
  const { lint } = await import(join(dir, "lib.js"));
  expect(lint([{ ruleId: "react-hooks/rules-of-hooks", line: 1 }, { ruleId: null, line: 2 }], [{ from: 3 }, { ruleId: "x", from: 4 }])).toEqual([[1], [4]]);
});

// ADR 0310: what react.dev's Console reads of any value a page logs, as
// JS has it: a replace's callback of the match and its groups, `value + ""`,
// `parseInt` and `parseFloat` of any value and a number's `toString`,
// `JSON.stringify` of any value, which may throw or give `undefined`,
// `typeof`, and `Object.prototype.toString.call`.
test("any value is read as JS reads it, as react.dev's Console does", async () => {
  const withJs = ["--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("console-values");
  writeFileSync(join(dir, "lib.rs"), `use js::{JsError, Unknown, json, number, object, reg_exp};

pub fn formatted(text: &str) -> String {
    let pattern = reg_exp::new("(%?)(%([jds]))", "g");
    reg_exp::replace(text, pattern, |matched: &str, escaped: Option<&str>, _ptn: Option<&str>, flag: Option<&str>| {
        if escaped.is_some_and(|e| !e.is_empty()) {
            matched.to_string()
        } else {
            format!("<{}>", flag.unwrap_or(""))
        }
    })
}

pub fn unescaped(text: &str) -> String {
    reg_exp::replace(text, reg_exp::new("%{2,2}", "g"), "%")
}

pub fn text_of(value: Option<&Unknown>) -> String {
    js::concat_text(value)
}

pub fn int_of(value: Option<&Unknown>) -> String {
    number::to_string(js::parse_int(value, 10))
}

pub fn float_of(value: &Unknown) -> String {
    number::to_string(js::parse_float(value))
}

pub fn json_of(value: &Unknown) -> Result<Option<String>, &'static JsError> {
    json::stringify_with(value, None, 2)
}

pub fn kind_of(value: Option<&Unknown>) -> String {
    js::type_of(value)
}

pub fn tag_of(value: Option<&Unknown>) -> Result<String, &'static JsError> {
    object::to_string(value)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("text.replace(pattern, (matched, escaped, _ptn, flag) => {");
  expect(js).toContain('return text.replace(/%{2,2}/g, "%");');
  expect(js).toContain('return value + "";');
  expect(js).toContain("return parseInt(value, 10).toString();");
  expect(js).toContain("return parseFloat(value).toString();");
  expect(js).toContain("JSON.stringify(value, null, 2)");
  expect(js).toContain("return typeof value;");
  expect(js).toContain("Object.prototype.toString.call(value)");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.formatted("a %s b %%d"), lib.unescaped("100%% sure")]).toEqual(["a <s> b %%d", "100% sure"]);
  expect([lib.text_of(undefined), lib.text_of(12), lib.text_of({})]).toEqual(["undefined", "12", "[object Object]"]);
  expect([lib.int_of("42px"), lib.int_of(undefined), lib.float_of("1e21"), lib.float_of("x")]).toEqual(["42", "NaN", "1e+21", "NaN"]);
  const cyclic: any = {};
  cyclic.self = cyclic;
  expect([lib.json_of({ a: 1 }), lib.json_of(() => 1)]).toEqual([{ TAG: "Ok", _0: '{\n  "a": 1\n}' }, { TAG: "Ok", _0: undefined }]);
  expect(lib.json_of(cyclic).TAG).toBe("Err");
  expect([lib.kind_of(undefined), lib.kind_of("s"), lib.kind_of(1n)]).toEqual(["undefined", "string", "bigint"]);
  expect([lib.tag_of([]), lib.tag_of(null)]).toEqual([{ TAG: "Ok", _0: "[object Array]" }, { TAG: "Ok", _0: "[object Null]" }]);
});

// ADR 0312: any value shown as JS's template shows it, `${value}`, as
// react.dev's Console keys what it logged by `${msg}-${index}`.
test("js::shown is shown as a template shows any value", async () => {
  const withJs = ["--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("shown");
  writeFileSync(join(dir, "lib.rs"), `use js::Unknown;

pub fn key(msg: Option<&Unknown>, index: usize) -> String {
    format!("{}-{index}", js::shown(msg))
}

pub fn named(arg: Option<&Unknown>) -> String {
    format!(" {}", js::string(arg))
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("return `${msg}-${index}`;");
  // `String(arg)` stays, which names a symbol a template throws on.
  expect(js).toContain("return ` ${String(arg)}`;");
  const { key, named } = await import(join(dir, "lib.js"));
  expect(named(Symbol("s"))).toBe(" Symbol(s)");
  expect([key(undefined, 0), key(null, 1), key({}, 2), key("a", 3)]).toEqual(["undefined-0", "null-1", "[object Object]-2", "a-3"]);
});
