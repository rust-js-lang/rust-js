// The binding language: a binding's call is the JS it names, its
// arguments as JS takes them, its values as JS has them. Each test is a
// program of its own, compiled and run.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { runSync } from "./child";
import { buildReact, buildSerde, compiler, fixture, root, run, target } from "./support";

beforeAll(buildReact, 600_000);

// A closure called by `&` that holds a value with a destructor, given to a
// binding, which JS holds and never drops: refused, as nothing would drop
// what it holds (ADR 0098).
test("a listener holding a value with a destructor is refused", () => {
  const withWeb = ["--", "--extern", `webapi=${join(target, "libwebapi.rmeta")}`, "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("webapi-listener-drop");
  const source = `use webapi::events::Click;
use webapi::{EventTargetExt, document};
pub struct Guard;
impl Drop for Guard {
    fn drop(&mut self) {}
}
pub fn wire() {
    let guard = Guard;
    document.add_event_listener(Click, move |_| {
        let _g = &guard;
    }, None);
}
`;
  writeFileSync(join(dir, "lib.rs"), source);
  const result = Bun.spawnSync([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withWeb]);
  expect(result.exitCode).not.toBe(0);
  expect(result.stderr.toString()).toContain("given away without being called once");
});

// A JS class's `Deref` to the class it extends, which webapi declares for
// each, is the object itself where it's used, and its body, a pointer cast
// that never runs, is no JS: lowered, it would be a function that makes a
// pointer to nothing.
test("a JS class's Deref to the class it extends is no JS", () => {
  const withJs = ["--", "--crate-type", "lib", "--edition", "2024", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("js-class-deref");
  writeFileSync(join(dir, "lib.rs"), `use std::marker::PhantomData;
use std::ops::Deref;
use js::JsObject;

pub struct Base(PhantomData<JsObject>);
pub struct Derived(PhantomData<JsObject>);

impl Deref for Derived {
    type Target = Base;
    fn deref(&self) -> &Base {
        unsafe { &*(self as *const Self as *const Base) }
    }
}

#[cfg_attr(rust_js, rust_js::link_name = "name")]
fn base_name(_b: &Base) -> String {
    unreachable!()
}

pub fn named(d: &Derived) -> String {
    base_name(d)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("return name(d);");
  expect(js).not.toContain("Deref_deref");
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
    el.before(("text",));
    (response::new(Some(&blob), None), window.fetch(url, None))
}
`;
  writeFileSync(join(dir, "Cargo.toml"), '[package]\nname = "app"\nversion = "0.0.0"\nedition = "2024"\n');
  writeFileSync(join(dir, "lib.rs"), source);
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(join(dir, "mir"), "lib.js"), ...withWeb]);
  // To TypeScript, the trait is the union: by TypeScript's own name for it,
  // `Float32List`, where Rust takes each of its members; else each member,
  // `BodyInit`'s, whose `Float16Array` Rust has no type of.
  expect(readFileSync(join(join(dir, "mir"), "lib.d.ts"), "utf8")).toContain("export function count(values: Float32List): number;");
  expect(readFileSync(join(join(dir, "mir"), "lib.d.ts"), "utf8")).toContain("export function kind(body: ReadableStream | Blob | Int8Array | Int16Array | Int32Array | Uint8Array | Uint16Array | Uint32Array | Uint8ClampedArray | BigInt64Array | BigUint64Array | Float32Array | Float64Array | DataView | ArrayBuffer | FormData | URLSearchParams | string): string;");
  const js = readFileSync(join(join(dir, "mir"), "lib.js"), "utf8");
  expect(js).toContain('globalThis.upload("hello");\n  globalThis.upload(blob);\n  globalThis.upload(body);\n  globalThis.upload(maybe);');
  // Of webapi's union, a binding crate's trait, no dictionary: `kind(body)`.
  expect(js).toContain("export function kind(body) {\n");
  expect(js).toContain('el.before("text");');
  expect(js).toContain("return [new Response(blob), window.fetch(url)];");
  const { send, kind } = await import(join(join(dir, "mir"), "lib.js"));
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
  expect([kind("text"), kind(new Blob()), kind(new ArrayBuffer(1))]).toEqual(["text", "blob", "other"]);
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
    let observer = intersection_observer::new(
        Box::new(move |entries, _| {
            entries.iter().for_each(|entry| seen(entry.is_intersecting()));
        }),
        Some(IntersectionObserverInit {
            root_margin: Some("0px 0px"),
            threshold: Some(0.0.into()),
            ..Default::default()
        }),
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
    const { watch, nextFrame } = await import(join(dir, "lib.js"));
    const seen: unknown[] = [];
    watch({}, (value: boolean) => seen.push(value));
    expect([seen, given]).toEqual([[false, true], [{ rootMargin: "0px 0px", threshold: 0 }]]);
    const times: number[] = [];
    expect([nextFrame((time: number) => times.push(time)), times]).toEqual([7, [16]]);
  } finally {
    [globals.IntersectionObserver, globals.window] = previous;
  }
});

// ADR 0653: a function JS gives is a `dyn Fn` of its signature, the one type
// of a JS function value: an event handler property read, called, compared
// by identity and kept, and a closure given to it, each the JS a person
// writes.
test("a function JS gives is a dyn Fn of its signature", async () => {
  const withWeb = ["--", "--extern", `webapi=${join(target, "libwebapi.rmeta")}`, "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("function-values");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::Cell;

use js::Unknown;
use webapi::{HTMLElement, PointerEvent};

thread_local! {
    static SAVED: Cell<Option<&'static dyn Fn(&PointerEvent) -> Option<&'static Unknown>>> = const { Cell::new(None) };
}

pub fn keep(button: &HTMLElement) {
    SAVED.set(button.onclick());
}

pub fn same(button: &HTMLElement) -> bool {
    match (SAVED.get(), button.onclick()) {
        (Some(saved), Some(now)) => std::ptr::eq(saved, now),
        _ => false,
    }
}

pub fn click(button: &HTMLElement, event: &PointerEvent) -> Option<&'static Unknown> {
    button.onclick().and_then(|onclick| onclick(event))
}

pub fn replace(button: &HTMLElement, seen: &'static dyn Fn(i32)) {
    button.set_onclick(Some(Box::new(move |event| seen(event.client_x()))));
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withWeb]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("  saved = button.onclick;\n");
  expect(js).toContain("    return saved === now;\n");
  expect(js).toContain("button.onclick = (event) => {\n    seen(event.clientX);\n  };");
  const { click, keep, replace, same } = await import(join(dir, "lib.js"));
  const button: any = {};
  const handler = (event: { clientX: number }) => event.clientX * 2;
  button.onclick = handler;
  keep(button);
  const other: any = {};
  other.onclick = () => 0;
  expect([same(button), same(other), click(button, { clientX: 3 }), click({ onclick: null }, { clientX: 3 })]).toEqual([true, false, 6, undefined]);
  const seen: number[] = [];
  replace(button, (x: number) => seen.push(x));
  button.onclick({ clientX: 5 });
  expect([seen, same(button)]).toEqual([[5], false]);
});

// ADR 0654: a dictionary's member of any value, `ErrorEventInit`'s `error?:
// any`, is a `dyn IntoUnknown`, as a rest parameter's item is, and so is
// `PromiseRejectionEventInit`'s `promise`, TypeScript's `Promise<any>`: React's
// `new window.ErrorEvent('error', {bubbles: true, cancelable: true,
// message, error})`, and a Rust value as a `CustomEvent`'s detail.
test("a dictionary's member of any value is a dyn IntoUnknown", async () => {
  const withWeb = ["--", "--extern", `webapi=${join(target, "libwebapi.rmeta")}`, "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("any-members");
  writeFileSync(join(dir, "lib.rs"), `use js::{Promise, Unknown};
use webapi::{CustomEvent, CustomEventInit, ErrorEvent, ErrorEventInit, PromiseRejectionEvent, PromiseRejectionEventInit};

pub struct Point {
    pub x: f64,
}

pub fn error_of(error: Option<&Unknown>, message: &str) -> &'static ErrorEvent {
    webapi::error_event::new(
        "error",
        Some(ErrorEventInit {
            bubbles: Some(true),
            cancelable: Some(true),
            message: Some(message),
            error: Some(&error),
            ..Default::default()
        }),
    )
}

pub fn moved(x: f64) -> &'static CustomEvent {
    webapi::custom_event::new("moved", Some(CustomEventInit { detail: Some(&Point { x }), ..Default::default() }))
}

pub fn rejected(promise: Promise<String>, reason: &str) -> &'static PromiseRejectionEvent {
    webapi::promise_rejection_event::new(
        "unhandledrejection",
        PromiseRejectionEventInit {
            bubbles: None,
            cancelable: None,
            composed: None,
            promise: &promise,
            reason: Some(&reason),
        },
    )
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withWeb]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('return new ErrorEvent("error", {\n    bubbles: true,\n    cancelable: true,\n    message,\n    error,\n  });');
  expect(js).toContain('return new CustomEvent("moved", { detail: { x } });');
  expect(js).toContain('return new PromiseRejectionEvent("unhandledrejection", {\n    promise,\n    reason,\n  });');
  const globals = globalThis as any;
  const previous = [globals.ErrorEvent, globals.CustomEvent, globals.PromiseRejectionEvent];
  const made = (name: string) => class {
    constructor(public type: string, public init: unknown) {}
    kind = name;
  };
  [globals.ErrorEvent, globals.CustomEvent, globals.PromiseRejectionEvent] = ["ErrorEvent", "CustomEvent", "PromiseRejectionEvent"].map(made);
  try {
    const lib = await import(join(dir, "lib.js"));
    const thrown = new Error("boom");
    const promise = Promise.resolve("x");
    expect([lib.errorOf(thrown, "boom").init, lib.errorOf(undefined, "none").init, lib.moved(2).init, lib.rejected(promise, "why").init]).toEqual([
      { bubbles: true, cancelable: true, message: "boom", error: thrown },
      { bubbles: true, cancelable: true, message: "none", error: undefined },
      { detail: { x: 2 } },
      { promise, reason: "why" },
    ]);
  } finally {
    [globals.ErrorEvent, globals.CustomEvent, globals.PromiseRejectionEvent] = previous;
  }
});

// ADR 0655: a binding of `any`, `reportError(e: any)`, as a value is a `fn`
// of `Option<&'static Unknown>`, the host's or a fallback, chosen where the
// module loads, as React's `reportGlobalError` is; and a value of it alone
// is the module's `const`.
test("a binding of any as a value is a fn of a static Unknown", async () => {
  const withWeb = ["--", "--extern", `webapi=${join(target, "libwebapi.rmeta")}`, "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("any-binding-value");
  writeFileSync(join(dir, "lib.rs"), `use js::Unknown;

thread_local! {
    static REPORT_GLOBAL_ERROR: fn(Option<&'static Unknown>) =
        if js::is_defined(webapi::global::report_error::<Option<&Unknown>>) {
            webapi::global::report_error
        } else {
            |error| webapi::console::error((error,))
        };
    static HOST_REPORT_ERROR: fn(Option<&'static Unknown>) = webapi::global::report_error;
}

js::export_default!(REPORT_GLOBAL_ERROR);

pub fn hosts(error: Option<&'static Unknown>) {
    HOST_REPORT_ERROR.with(|report| report(error));
}
`);
  const globals = globalThis as any;
  const previous = [globals.reportError, console.error];
  const reported: unknown[] = [];
  const logged: unknown[] = [];
  globals.reportError = (error: unknown) => reported.push(error);
  console.error = (error: unknown) => logged.push(error);
  try {
    run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withWeb]);
    const js = readFileSync(join(dir, "lib.js"), "utf8");
    expect(js).toContain('const reportGlobalError =\n  typeof reportError !== "undefined"\n    ? reportError\n    : (error) => {\n');
    expect(js).toContain("const hostReportError = reportError;");
    expect(js).toContain("export default reportGlobalError;");
    const lib = await import(join(dir, "lib.js"));
    lib.default("first");
    lib.hosts("second");
    globals.reportError = undefined;
    writeFileSync(join(dir, "again.js"), readFileSync(join(dir, "lib.js"), "utf8"));
    (await import(join(dir, "again.js"))).default("third");
    expect([reported, logged]).toEqual([["first", "second"], ["third"]]);
  } finally {
    [globals.reportError, console.error] = previous;
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
  expect(js).toContain('export function shown(value) {\n  return value ? "yes" : "no";\n}');
  expect(js).toContain('export function named(name) {\n  return !name ? "none" : "some";\n}');
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
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js"), ...withWeb]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain("return useText ? text : other;");
  expect(js).toContain("for (const part of parts) {\n    result += String(part);");
  expect(js).toContain("return String(n);");
  const lib = await import(join(dir, "mir.js"));
  expect([lib.message("a", 1, true), lib.message("a", 1, false), lib.message("a", undefined, false)]).toEqual(["a", 1, undefined]);
  expect(lib.joined(["a", 1, undefined, null, { b: 1 }])).toBe("a1undefinednull[object Object]");
  expect(lib.counted(7)).toBe("7");
});

// ADR 0051: an item that may be `None` is boxed by `$some` where it looks
// like one, and taken out by `$someValue`: JS's `null` comes out `null`,
// as `String(null)` tells it from `undefined`, stepped by `next()`, matched
// or the last; and it's still `Some(None)`.
test("an Option's box keeps the null it holds", async () => {
  const withWeb = ["--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("some-null");
  writeFileSync(join(dir, "lib.rs"), `use js::{Unknown, string};
pub fn stepped(parts: Vec<Option<&Unknown>>) -> String {
    let mut it = parts.into_iter();
    let mut result = String::new();
    while let Some(part) = it.next() {
        result.push_str(&string(part));
    }
    result
}
pub fn first(parts: Vec<Option<&Unknown>>) -> String {
    let mut it = parts.into_iter();
    match it.next() {
        Some(part) => string(part),
        None => String::new(),
    }
}
pub fn last(parts: Vec<Option<&Unknown>>) -> String {
    match parts.into_iter().last() {
        Some(part) => string(part),
        None => String::new(),
    }
}
pub fn none_first(parts: Vec<Option<&Unknown>>) -> bool {
    matches!(parts.into_iter().next(), Some(None))
}
pub fn nested(parts: Vec<Option<&Unknown>>) -> String {
    let boxed: Vec<Option<Option<&Unknown>>> = parts.into_iter().map(Some).collect();
    match boxed.into_iter().next() {
        Some(Some(part)) => string(part),
        _ => String::new(),
    }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js"), ...withWeb]);
  const lib = await import(join(dir, "mir.js"));
  expect([lib.stepped([null, 1, undefined]), lib.first([null]), lib.last([1, null])]).toEqual(["null1undefined", "null", "null"]);
  expect([lib.noneFirst([null]), lib.noneFirst([undefined]), lib.noneFirst([1]), lib.noneFirst([])]).toEqual([true, true, false, false]);
  expect([lib.nested([null]), lib.nested([undefined]), lib.nested([1])]).toEqual(["null", "undefined", "1"]);
});

// A rest parameter, `#[rust_js::variadic]`'s last, is `impl js::Rest<T>`
// (ADRs 0221, 0640): a tuple is its arguments, `Math.max(a, b, 1)`, and a
// slice or a `Vec` of them, by reference, is spread, `Math.max(...values)`.
// A slice written out is no tuple, and a slice declared is refused,
// naming the one way.
test("a rest parameter takes a tuple or a slice to spread", async () => {
  const withJs = ["--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("variadic");
  const source = `use js::Rest;
#[cfg_attr(rust_js, rust_js::link_name = "Math.max")]
#[cfg_attr(rust_js, rust_js::variadic)]
#[allow(unused_variables)]
fn max(values: impl Rest<f64>) -> f64 {
    unreachable!()
}
pub fn largest(a: f64, b: f64) -> f64 {
    max((a, b, 1.0))
}
pub fn largest_of(values: Vec<f64>) -> f64 {
    max(&values)
}
pub fn largest_in(values: &[f64]) -> f64 {
    max(values)
}
pub fn smallest(values: &[f64]) -> f64 {
    js::math::min(values)
}
`;
  writeFileSync(join(dir, "lib.rs"), source);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("return Math.max(a, b, 1);");
  expect(js).toContain("return Math.max(...values);");
  expect(js).toContain("return Math.min(...values);");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.largest(-3, -2), lib.largest(4, 2), lib.largestOf([2, 9, 4]), lib.largestIn([3, 1]), lib.smallest([3, 1])]).toEqual([1, 4, 9, 3, 1]);
  for (const [wrong, written, error] of [
    ["max((a, b, 1.0))", "max(&[a, b, 1.0])", "`&[f64; 3]` is not rest arguments"],
    ["fn max(values: impl Rest<f64>)", "fn max(values: &[f64])", "`impl js::Rest<T>`"],
  ]) {
    const declared = written.startsWith("fn ") ? source.replace("max((a, b, 1.0))", "max(&[a, b, 1.0])").replace("use js::Rest;\n", "") : source;
    writeFileSync(join(dir, "lib.rs"), declared.replace(wrong, written));
    const result = Bun.spawnSync([compiler, join(dir, "lib.rs"), "-o", join(dir, "wrong.js"), ...withJs], { cwd: dir });
    expect(result.exitCode).not.toBe(0);
    expect(result.stderr.toString()).toContain(error);
  }
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

// ADR 0221: a binding that skips what's falsy, `#[rust_js::skips_falsy]`, as
// classnames does, is given an argument shown only if a test holds as `test
// && value`, as react.dev writes `cn('a', isExpanded && 'sp-layout-expanded')`.
// So too where a test that branches makes it a temporary of MIR's (ADR 0364).
test("a binding that skips what's falsy is given test && value", async () => {
  const dir = fixture("skips-falsy");
  writeFileSync(join(dir, "cn.js"), 'export default (...classes) => classes.filter(Boolean).join(" ");\n');
  writeFileSync(join(dir, "lib.rs"), `#[cfg_attr(rust_js, rust_js::link_name = "./cn.js#default")] #[rust_js::variadic] #[rust_js::skips_falsy] #[allow(unused_variables)] fn cn(classes: impl js::Rest<dyn js::IntoUnknown>) -> String { unreachable!() } #[cfg_attr(rust_js, rust_js::link_name = "./cn.js#default")] #[rust_js::variadic] #[allow(unused_variables)] fn plain(classes: impl js::Rest<dyn js::IntoUnknown>) -> String { unreachable!() }
pub fn classes(expanded: bool, shown: bool) -> String {
    cn((Some("a"), expanded.then_some("wide"), (!expanded && shown).then_some("tall")))
}
pub fn listed(expanded: bool) -> String {
    plain((Some("a"), expanded.then_some("wide")))
}
pub struct Failure(pub u32);
pub fn flagged(error: Option<&Failure>, ready: bool) -> String {
    cn((Some("a"), (error.is_some() || ready).then_some("b")))
}
#[derive(Clone, Copy)]
pub enum Dir {
    #[rust_js::name = "start"]
    Start,
    #[rust_js::name = "end"]
    End,
    #[rust_js::name = "right"]
    Right,
    #[rust_js::name = "left"]
    Left,
    #[rust_js::name = "up"]
    Up,
    #[rust_js::name = "down"]
    Down,
}
pub struct ChevronProps {
    pub class_name: Option<&'static str>,
    pub d: Option<Dir>,
}
pub fn chevron(props: ChevronProps) -> String {
    let d = props.d;
    let classes = cn((
        matches!(d, Some(Dir::Down)).then_some("r0"),
        matches!(d, Some(Dir::Left)).then_some("r90"),
        matches!(d, Some(Dir::Up)).then_some("r180"),
        matches!(d, Some(Dir::Right)).then_some("r-90"),
        matches!(d, Some(Dir::Start)).then_some("s"),
        matches!(d, Some(Dir::End)).then_some("e"),
        props.class_name,
    ));
    classes
}
pub fn numbered(n: Option<u32>) -> String {
    cn((matches!(n, Some(1)).then_some("one"), matches!(n, Some(2)).then_some("two"), Some("c")))
}
pub fn kept(n: Option<u32>) -> String {
    plain((matches!(n, Some(1)).then_some("one"), matches!(n, Some(2)).then_some("two")))
}
// A test of a string's truth, read as it is, as react.dev's Footer.
pub fn linked(href: Option<&str>) -> String {
    cn((Some("a"), href.filter(|href| !href.is_empty()).map(|_| "hover")))
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain('return cn("a", expanded && "wide", !expanded && shown && "tall");');
  expect(js).toContain('return cn("a", expanded ? "wide" : undefined);');
  expect(js).toContain('return cn("a", (error != null || ready) && "b");');
  expect(js).toContain('  const classes = cn(\n    d === "down" && "r0",\n    d === "left" && "r90",\n    d === "up" && "r180",\n    d === "right" && "r-90",\n    d === "start" && "s",\n    d === "end" && "e",\n    props.className,\n  );\n');
  expect(js).toContain('return cn(n === 1 && "one", n === 2 && "two", "c");');
  expect(js).toContain('return cn(n === 1 ? "one" : undefined, n === 2 ? "two" : undefined);');
  expect(js).toContain('return cn("a", href && "hover");');
  const lib = await import(join(dir, "mir.js"));
  expect([lib.classes(true, true), lib.classes(false, true), lib.classes(false, false), lib.listed(true), lib.flagged([1], false), lib.flagged(undefined, false), lib.numbered(2), lib.kept(1)]).toEqual(["a wide", "a tall", "a", "a wide", "a b", "a", "two c", "one"]);
  expect([lib.linked("x"), lib.linked(""), lib.linked(undefined)]).toEqual(["a hover", "a", "a"]);
});

// ADR 0301: a binding that can't throw, `#[rust_js::cannot_throw]`, as reading
// React's `ref.current` can't, read before a value is moved leaves it nothing
// to drop: a library's function takes no drop for it, and has no `try`, as
// react.dev's `useEvent` reads its ref, then calls with its arguments. So
// does `unwrap_unchecked`, which can't panic. One unmarked keeps both.
test("a binding that can't throw, read before a move, leaves nothing to drop", async () => {
  const dir = fixture("cannot-throw");
  writeFileSync(join(dir, "lib.rs"), `#[cfg_attr(rust_js, rust_js::link_name = "get globalThis.calls")] #[rust_js::cannot_throw] fn calls() -> u32 { unreachable!() } #[cfg_attr(rust_js, rust_js::link_name = "get globalThis.calls")] fn calls_unmarked() -> u32 { unreachable!() }
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
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js"), "--library", "--manifest", join(dir, "mir.js".replace(".js", ".manifest.json")),
    "--", "--crate-name", "counted", `--emit=metadata=${join(dir, "libcounted.rmeta")}`]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain("export function count(value) {\n  const n = globalThis.calls;\n  return [n, value];\n}");
  expect(js).toContain("export function first(value, given) {\n  const n = given;\n  return [n, value];\n}");
  expect(js).toContain("export function call(value, given) {\n  const f = given;\n  return f(value);\n}");
  expect(js).toContain("export function countUnmarked(value, dropT) {");
  (globalThis as { calls?: number }).calls = 3;
  const lib = await import(join(dir, "mir.js"));
  expect([lib.count("a"), lib.countUnmarked("b"), lib.first("c", 4), lib.call(5, (n: number) => n + 1)]).toEqual([[3, "a"], [3, "b"], [4, "c"], 6]);
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
#[cfg_attr(rust_js, rust_js::link_name = "./math.js#twice")]
#[allow(unused_variables)]
fn twice(n: f64) -> f64 {
    unreachable!()
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
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js"), ...withWeb]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain('const { run } = await import("./lint.js");');
  expect(js).toContain('const { twice } = await import("./math.js");');
  expect(js).toContain('const { run: next } = await import("./lint.js");');
  expect(js).toContain('const module = await import("./root.js");');
  expect(js).not.toMatch(/^import /m);
  const lib = await import(join(dir, "mir.js"));
  expect([await lib.linted(1), await lib.doubled(3), await lib.named(2), await lib.rooted()]).toEqual([2, 6, 3, 7]);
});

// ADR 0305: a binding marked `#[rust_js::require]` is read through
// `require(module)`, where it's read, as react.dev's runESLint reads
// `require('eslint-plugin-react-hooks').rules`: nothing imports it.
test("a required binding is read through require", async () => {
  const withWeb = ["--", "--extern", `webapi=${join(target, "libwebapi.rmeta")}`, "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("required");
  writeFileSync(join(dir, "lib.rs"), `#![allow(non_upper_case_globals)]
#[cfg_attr(rust_js, rust_js::link_name = "get node:path#sep")]
#[rust_js::require]
fn sep() -> String {
    unreachable!()
}

#[cfg_attr(rust_js, rust_js::link_name = "get node:path#delimiter")]
fn delimiter() -> String {
    unreachable!()
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

// Of any value, what may throw, of a symbol (ADR 0639).
pub fn text_of(value: Option<&Unknown>) -> Result<String, &'static JsError> {
    js::concat_text(value)
}

pub fn px(n: f64) -> String {
    js::concat_text(n) + "px"
}

pub fn counted(count: f64, noun: &str) -> String {
    js::concat_text(count) + " " + noun
}

pub fn anchor(id: f64) -> String {
    "#".to_string() + &js::concat_text(id)
}

pub fn int_of(value: Option<&Unknown>) -> String {
    number::to_string(js::parse_int(value, Some(10)))
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
  expect(js).toContain('text.replace(pattern, (matched, escaped, ptn, flag) =>\n    escaped ? matched : `<${flag ?? ""}>`,\n  );');
  expect(js).toContain('return text.replace(/%{2,2}/g, "%");');
  expect(js).toContain('return $try(() => "" + value);');
  // Text joined to it already makes it text, as react.dev writes `right - left + 'px'`.
  expect(js).toContain('return n + "px";');
  expect(js).toContain('return count + " " + noun;');
  expect(js).toContain('return "#" + id;');
  expect(js).toContain("return parseInt(value, 10).toString();");
  expect(js).toContain("return parseFloat(value).toString();");
  expect(js).toContain("JSON.stringify(value, null, 2)");
  expect(js).toContain("return typeof value;");
  expect(js).toContain("Object.prototype.toString.call(value)");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.formatted("a %s b %%d"), lib.unescaped("100%% sure")]).toEqual(["a <s> b %%d", "100% sure"]);
  expect([lib.textOf(undefined), lib.textOf(12), lib.textOf({})].map((r) => r._0)).toEqual(["undefined", "12", "[object Object]"]);
  expect(lib.textOf(Symbol("s")).TAG).toBe("Err");
  expect([lib.px(1.5), lib.px(-0), lib.counted(2, "videos"), lib.anchor(2), lib.anchor(1e21)]).toEqual(["1.5px", "0px", "2 videos", "#2", "#1e+21"]);
  expect([lib.intOf("42px"), lib.intOf(undefined), lib.floatOf("1e21"), lib.floatOf("x")]).toEqual(["42", "NaN", "1e+21", "NaN"]);
  const cyclic: any = {};
  cyclic.self = cyclic;
  expect([lib.jsonOf({ a: 1 }), lib.jsonOf(() => 1)]).toEqual([{ TAG: "Ok", _0: '{\n  "a": 1\n}' }, { TAG: "Ok", _0: undefined }]);
  expect(lib.jsonOf(cyclic).TAG).toBe("Err");
  expect([lib.kindOf(undefined), lib.kindOf("s"), lib.kindOf(1n)]).toEqual(["undefined", "string", "bigint"]);
  expect([lib.tagOf([]), lib.tagOf(null)]).toEqual([{ TAG: "Ok", _0: "[object Array]" }, { TAG: "Ok", _0: "[object Null]" }]);
});

// ADR 0639: `value + ""` of a value of any shape may throw, of a symbol or
// of an object whose `valueOf` throws, so its result is a `Result`, which
// React's `willCoercionThrow` catches; of a number or text, it's `String`.
test("concat_text of any value is a Result, which a match catches", async () => {
  const withJs = ["--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("concat-text-throws");
  writeFileSync(join(dir, "lib.rs"), `use js::Unknown;

pub fn will_coercion_throw(value: Option<&Unknown>) -> bool {
    match js::concat_text(value) {
        Ok(_) => false,
        Err(_) => true,
    }
}

pub fn throws(value: &Unknown) -> bool {
    js::concat_text(value).is_err()
}

pub fn coercible(value: &Unknown) -> bool {
    js::concat_text(value).is_ok()
}

pub fn width(n: f64) -> String {
    js::concat_text(n) + "px"
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('function willCoercionThrow(value) {\n  try {\n    "" + value;\n    return false;\n  } catch {\n    return true;\n  }\n}');
  expect(js).toContain('function throws(value) {\n  try {\n    "" + value;\n    return false;\n  } catch {\n    return true;\n  }\n}');
  expect(js).toContain('function coercible(value) {\n  try {\n    "" + value;\n    return true;\n  } catch {\n    return false;\n  }\n}');
  // Of a number, nothing to catch.
  expect(js).toContain('return n + "px";');
  expect(js).not.toContain("$try");
  const lib = await import(join(dir, "lib.js"));
  const throwing = { valueOf() { throw new Error("no"); } };
  const values = [undefined, null, 1, "a", {}, 2n, Symbol("s"), throwing];
  expect(values.map(lib.willCoercionThrow)).toEqual([false, false, false, false, false, false, true, true]);
  expect([lib.throws(Symbol("s")), lib.throws(2n), lib.coercible(Symbol("s")), lib.coercible(2n)]).toEqual([true, false, false, true]);
  expect(lib.width(1.5)).toBe("1.5px");
});

// `localeCompare`, an `Ordering`, as an Intl collator gives -1, 0 or 1: a
// sort's comparator, as react.dev's LanguageList sorts languages by name.
test("a string's localeCompare is a sort's comparator", async () => {
  const withJs = ["--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("locale-compare");
  writeFileSync(join(dir, "lib.rs"), `use js::string;

pub fn sorted(mut names: Vec<String>) -> Vec<String> {
    names.sort_by(|a, b| string::locale_compare(a, b));
    names
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs]);
  expect(readFileSync(join(dir, "lib.js"), "utf8")).toContain("names.sort((a, b) => a.localeCompare(b));");
  expect((await import(join(dir, "lib.js"))).sorted(["Ébène", "zèbre", "abeille", "Zoo"])).toEqual(["abeille", "Ébène", "zèbre", "Zoo"]);
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

// A number as JS shows it, \`1e+21\`, as react.dev's copyright shows a year.
pub fn year(year: f64) -> String {
    format!("© {}", js::shown(year))
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("return `${msg}-${index}`;");
  expect(js).toContain("return `© ${year}`;");
  expect((await import(join(dir, "lib.js"))).year(1e21)).toBe("© 1e+21");
  // `String(arg)` stays, which names a symbol a template throws on.
  expect(js).toContain("return ` ${String(arg)}`;");
  const { key, named } = await import(join(dir, "lib.js"));
  expect(named(Symbol("s"))).toBe(" Symbol(s)");
  expect([key(undefined, 0), key(null, 1), key({}, 2), key("a", 3)]).toEqual(["undefined-0", "null-1", "[object Object]-2", "a-3"]);
});

// ADR 0312: js::shown's text is a string, `${n}`, wherever it goes: alone,
// with another value alone, padded, by `to_string()`, in a map, in JSON.
// It was the value itself, a number in a `Vec<String>`, and `a + b` of two.
test("js::shown's text is text, alone or with values alone", async () => {
  const dir = fixture("shown-text");
  writeFileSync(join(dir, "lib.rs"), `use js::Unknown;
use std::collections::HashMap;
use std::fmt::{self, Display, Write};

pub fn length(n: f64) -> usize {
    format!("{}", js::shown(n)).len()
}

pub fn pushed(n: f64) -> Vec<String> {
    let mut out = Vec::new();
    out.push(format!("{}", js::shown(n)));
    out
}

pub fn both(a: f64, b: f64) -> String {
    format!("{}{}", js::shown(a), js::shown(b))
}

pub fn same(n: f64, s: String) -> bool {
    format!("{}", js::shown(n)) == s
}

pub fn keyed(n: f64) -> usize {
    let mut seen: HashMap<String, usize> = HashMap::new();
    seen.insert(format!("{}", js::shown(n)), 1);
    seen.get("5").copied().unwrap_or(0)
}

pub fn json(n: f64) -> String {
    serde_json::to_string(&format!("{}", js::shown(n))).unwrap()
}

pub fn padded(n: f64) -> String {
    format!("[{:>4}] [{:.1}]", js::shown(n), js::shown(n))
}

pub fn text(n: f64) -> usize {
    js::shown(n).to_string().len()
}

fn shown<T: Display>(value: T) -> String {
    format!("{value}")
}

pub fn generic(n: f64) -> usize {
    shown(js::shown(n)).len()
}

struct Hours(u32);

impl Display for Hours {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.0 > 23 {
            return Err(fmt::Error);
        }
        write!(f, "{}h", self.0)
    }
}

pub fn written(v: Option<&Unknown>, h: u32) -> String {
    let mut s = String::new();
    let _ = write!(s, "{}{}", js::shown(v), Hours(h));
    s
}

pub fn added(v: Option<&Unknown>) -> String {
    format!("{}!", js::concat_text(v).unwrap())
}

pub fn lines(n: f64) -> String {
    format!(
        "<b>
{}</b>",
        js::shown(n)
    )
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", ...buildSerde(), "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("return $byteLen(`${n}`);");
  expect(js).toContain("return `${a}${b}`;");
  expect(js).toContain("s += `${v}`;");
  // Its template spliced into one written across lines, which has no `\n`.
  expect(js).toContain("return `<b>\n${n}</b>`;");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.length(5), lib.length(1e21), lib.length(-0)]).toEqual([1, 5, 1]);
  expect(lib.pushed(5)).toEqual(["5"]);
  expect(lib.both(1, 2)).toBe("12");
  expect([lib.same(5, "5"), lib.same(5, "6")]).toEqual([true, false]);
  expect(lib.keyed(5)).toBe(1);
  expect(lib.json(5)).toBe('"5"');
  expect(lib.padded(5)).toBe("[   5] [5]");
  expect([lib.text(1e21), lib.generic(1e21)]).toEqual([5, 5]);
  // An object as a template shows it, by its `toString`, not its `valueOf`.
  const odd = { toString: () => "t", valueOf: () => 1 };
  expect([lib.written(odd, 2), lib.written(odd, 30)]).toEqual(["t2h", "t"]);
  // `"" + v`, `concat_text`'s, by its `valueOf`, in a template too.
  expect(lib.added(odd)).toBe("1!");
});

// react.dev's Console's `formatStr` (ADR 0364): the string a
// replace is of read where it's replaced, past what its callback captures.
test("a value replaced in place is read where it's replaced", async () => {
  const withJs = ["--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("mir-format-str");
  writeFileSync(join(dir, "lib.rs"), `#![allow(non_snake_case)]
use std::collections::VecDeque;
use js::{Unknown, number, reg_exp};
pub fn formatStr(inputArgs: Vec<Option<&'static Unknown>>) -> Vec<Option<&'static Unknown>> {
    let maybeMessage = inputArgs.first().copied().flatten();
    if js::type_of(maybeMessage) != "string" {
        return inputArgs;
    }
    // If the first argument is a string, check for substitutions.
    let mut args: VecDeque<Option<&'static Unknown>> = inputArgs.iter().skip(1).copied().collect();
    let mut formatted: String = js::string(maybeMessage);
    if !args.is_empty() {
        let REGEXP = reg_exp::new("(%?)(%([jds]))", "g");

        formatted = reg_exp::replace(
            &formatted,
            REGEXP,
            |r#match: &str, escaped: Option<&str>, ptn: Option<&str>, flag: Option<&str>| {
                let mut arg = args.pop_front().flatten();
                match flag {
                    Some("s") => {
                        arg = Some(js::unknown_of(js::concat_text(arg).unwrap()));
                    }
                    Some("d") | Some("i") => {
                        arg = Some(js::unknown_of(number::to_string(js::parse_int(arg, Some(10)))));
                    }
                    Some("f") => {
                        arg = Some(js::unknown_of(number::to_string(js::parse_float(arg))));
                    }
                    _ => {}
                }
                if escaped.is_none_or(str::is_empty) {
                    return arg;
                }
                args.push_front(arg);
                Some(js::unknown_of(r#match.to_string()))
            },
        );
    }

    // Arguments that remain after formatting.
    if !args.is_empty() {
        for i in 0..args.len() {
            formatted += &format!(" {}", js::string(args[i]));
        }
    }

    // Update escaped %% values.
    vec![Some(js::unknown_of(reg_exp::replace(
        &formatted,
        reg_exp::new("%{2,2}", "g"),
        "%",
    )))]
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js"), ...withJs]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain("    formatted = formatted.replace(REGEXP, (match, escaped, ptn, flag) => {\n");
  // react.dev's `arg = arg + ''`, a symbol's throw its own (ADR 0643).
  expect(js).toContain('arg = "" + arg;');
  const lib = await import(join(dir, "mir.js"));
  expect(lib.formatStr(["a %s b %d", "x", "4.2", "y"])).toEqual(["a x b 4 y"]);
});

// ADR 0443: a binding's optional parameter, TypeScript's `x?`, is an
// `Option`, and a trailing `None` of one is no argument; a `None` before a
// `Some` is `undefined`, as JS gives one left out. One of a union, a trait
// of no methods, is taken by reference, `Option<&dyn IntoX>`, and given to
// JS as its value. An `undefined` given a parameter that isn't optional is
// still given (ADR 0330).
test("a binding's trailing None is no argument, and a union's dyn is its value", () => {
  const dir = fixture("binding-optional");
  writeFileSync(join(dir, "lib.rs"), `pub trait IntoNumberOrStr {}
impl IntoNumberOrStr for f64 {}
impl IntoNumberOrStr for &str {}
#[cfg_attr(rust_js, rust_js::link_name = "addRule")]
fn add_rule(selector: Option<&str>, style: Option<&str>, index: Option<u32>) -> i32 { unreachable!() }
#[cfg_attr(rust_js, rust_js::link_name = "toDataURL")]
fn to_data_url(kind: Option<&str>, quality: Option<&dyn IntoNumberOrStr>) -> String { unreachable!() }
#[cfg_attr(rust_js, rust_js::link_name = "Math.max")]
fn max<T>(a: T, b: T) -> f64 { unreachable!() }
pub fn rules() -> i32 {
    add_rule(Some("p"), None, None) + add_rule(None, None, Some(1))
}
pub fn urls(q: f64) -> String {
    to_data_url(None, None) + &to_data_url(Some("image/png"), Some(&q)) + &to_data_url(None, Some(&"x"))
}
pub fn forward(q: Option<&dyn IntoNumberOrStr>) -> String {
    to_data_url(Some("a"), q)
}
pub fn counted() -> f64 {
    max(Some(1), None)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('return (addRule("p") + addRule(undefined, undefined, 1)) | 0;');
  expect(js).toContain('toDataURL() + toDataURL("image/png", q) + toDataURL(undefined, "x")');
  expect(js).toContain('return toDataURL("a", q?.value);');
  expect(js).toContain("return Math.max(1, undefined);");
});

// A binding generator's camelCase, scripts/rust-names.ts's, is rust-js's,
// src/lower.rs's `camel_case` (ADR 0573): each name compiled is the JS one
// the generators write `#[rust_js::name]` where it differs from. And one
// that repeats it is rust-js's error, a second way to write the name.
test("a generator's camelCase is the compiler's, and a name that repeats it is an error", async () => {
  const { camelCase } = await import("../scripts/rust-names.ts");
  const names = ["count", "set_count", "_unused", "type_", "self_", "FOO_BAR", "write_u32", "html_iframe_element",
    "a__b", "x2_y", "__proto", "on_click_", "sort_index", "is_3d", "max_ttl_ms"];
  const dir = fixture("generator-camel-case");
  const fields = names.map((name, i) => `    pub ${name}: u32,`).join("\n");
  const values = names.map((name, i) => `${name}: ${i}`).join(", ");
  writeFileSync(join(dir, "lib.rs"), `#![allow(non_snake_case)]
pub struct Names {
${fields}
}
pub fn names() -> Names {
    Names { ${values} }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const { names: made } = await import(join(dir, "lib.js"));
  expect(Object.keys(made())).toEqual(names.map(camelCase));
  writeFileSync(join(dir, "lib.rs"), `pub struct Point {
    #[rust_js::name = "sortIndex"]
    pub sort_index: u32,
}
`);
  const refused = runSync([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")], root, 60_000);
  expect(refused.code).not.toBe(0);
  expect(refused.stderr).toContain('rust-js: `sort_index` is `sortIndex` in JS without `#[rust_js::name = "sortIndex"]`: remove it');
});

// ADR 0855: a parameter's `#[rust_js::name]` is its JS name, where it isn't
// the camelCase one, React's `useContext(Context)`: its body's, its
// closure's and its declaration's, through `lower::parameter_name`. One of a
// pattern, of `self`, not a string or not a JS name, or the name it has
// anyway, is an error saying what to write.
test("a parameter's rust_js::name is its JS name", async () => {
  const dir = fixture("parameter-name");
  writeFileSync(join(dir, "lib.rs"), `pub struct Context {
    pub value: u32,
}

pub fn use_context(#[cfg_attr(rust_js, rust_js::name = "Context")] context: &Context) -> u32 {
    let read = |#[cfg_attr(rust_js, rust_js::name = "Given")] given: u32| given + context.value;
    read(1)
}
`);
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("export function useContext(Context) {\n  const read = (Given) => (Given + Context.value) >>> 0;");
  expect(readFileSync(join(dir, "lib.d.ts"), "utf8")).toContain("export function useContext(Context: Context): number;");
  expect((await import(join(dir, "lib.js"))).useContext({ value: 2 })).toBe(3);
  writeFileSync(join(dir, "lib.rs"), `pub fn same(#[cfg_attr(rust_js, rust_js::name = "value")] value: u32) -> u32 {
    value
}
pub fn pair(#[cfg_attr(rust_js, rust_js::name = "Pair")] (x, y): (u32, u32)) -> u32 {
    x + y
}
pub fn bare(#[cfg_attr(rust_js, rust_js::name)] given: u32) -> u32 {
    given
}
pub fn dashed(#[cfg_attr(rust_js, rust_js::name = "my-name")] given: u32) -> u32 {
    given
}
`);
  const refused = runSync([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")], root, 60_000);
  expect(refused.code).not.toBe(0);
  expect(refused.stderr).toContain('rust-js: `value` is `value` in JS without `#[rust_js::name = "value"]`: remove it');
  expect(refused.stderr).toContain("rust-js: `#[rust_js::name]` names a parameter that's a name, `context: T`, not a pattern or `self`");
  expect(refused.stderr).toContain('rust-js: write a parameter\'s JS name as a string, `#[cfg_attr(rust_js, rust_js::name = "Context")]`');
  expect(refused.stderr).toContain("rust-js: a parameter's JS name is a name JS lets a variable have, not `my-name`");
});

// A function's `#[rust_js::nullable]`, alone, is of its result: its `None`
// is `null`, as a JS caller tests `!== null`, React's scheduler's
// `peek(taskQueue)`; one made `None` is `null`, `Some(x)` its value, and
// another `Option` `value ?? null` (ADR 0574). And a type parameter a
// binding of its bound reads, `T: Node`'s `get sortIndex`, is never
// nullish: `Some` of one is the value itself, no `$some` (ADR 0574).
test("a nullable function's None is null, and a binding bound's Some is the value", async () => {
  const dir = fixture("nullable-result");
  writeFileSync(join(dir, "lib.rs"), `pub trait Node {
    #[cfg_attr(rust_js, rust_js::link_name = "get sortIndex")]
    fn sort_index(&self) -> f64 {
        unreachable!()
    }
}

#[cfg_attr(rust_js, rust_js::nullable)]
pub fn peek<T: Node>(heap: &[T]) -> Option<&T> {
    heap.first()
}

#[cfg_attr(rust_js, rust_js::nullable)]
pub fn pop<T: Node>(heap: &mut Vec<T>) -> Option<T> {
    if heap.is_empty() {
        return None;
    }
    let first = heap.remove(0);
    Some(first)
}

pub fn smallest<T: Node>(heap: &[T]) -> Option<&T> {
    heap.iter().min_by(|a, b| a.sort_index().total_cmp(&b.sort_index()))
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("return heap[0] ?? null;");
  expect(js).toContain("return null;");
  expect(js).toContain("return first;");
  expect(js).not.toContain("$some");
  const lib = await import(join(dir, "lib.js"));
  const node = { sortIndex: 2, id: 1 };
  expect([lib.peek([]), lib.peek([node]), lib.pop([]), lib.pop([node])]).toEqual([null, node, null, node]);
  // Not nullable, its `None` is `undefined`, as every `None` is (ADR 0030).
  expect(lib.smallest([])).toBeUndefined();
  writeFileSync(join(dir, "lib.rs"), `#[rust_js::nullable]
pub fn count() -> u32 {
    1
}
`);
  const refused = runSync([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")], root, 60_000);
  expect(refused.code).not.toBe(0);
  expect(refused.stderr).toContain("rust-js: `#[rust_js::nullable]` of a function is of its result, which isn't an `Option`");
});
