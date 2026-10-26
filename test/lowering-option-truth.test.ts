// How an `Option` is tested, by what its payload's JS value may be (ADR
// 0298, amended): by its truth, `o` and `!o`, where that's never falsy, an
// object, an array, a function, a class's instance or a symbol; against
// `null`, `o != null`, where it may be, a value of any shape, a number,
// text or a bigint, whose `Some(0)`, `Some("")`, `Some(false)`, `Some(NaN)`
// and `Some(0n)` are there. Each test is a program of its own, compiled and
// run with those values given.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { buildReact, compiler, fixture, run, target } from "./support";

beforeAll(buildReact, 600_000);

const externs = (...crates: string[]) => ["--", ...crates.flatMap((c) => ["--extern", `${c}=${join(target, `lib${c}.rmeta`)}`]), "-L", target];

// A JS value of any shape, `js::Unknown` or what `dyn IntoUnknown` is given,
// may be falsy: `Some(0)`, `Some("")`, `Some(false)`, `Some(NaN)` and
// `Some(0n)` are there, so each is tested against `null`, never by its
// truth. A JS object, `&JsObject`, and a symbol never are: by their truth.
test("an Option of a value of any shape is tested against null, of an object by its truth", async () => {
  const dir = fixture("any-shape-truth");
  writeFileSync(join(dir, "lib.rs"), `use js::{IntoUnknown, JsObject, Symbol, Unknown};

pub fn has(value: Option<&Unknown>) -> bool {
    value.is_some()
}

pub fn lacks(value: Option<&Unknown>) -> bool {
    value.is_none()
}

pub fn branch(value: Option<&Unknown>) -> u32 {
    if let Some(_v) = value { 1 } else { 0 }
}

pub fn tried(value: Option<&Unknown>) -> Option<u32> {
    let _v = value?;
    Some(1)
}

pub fn given(value: Option<&dyn IntoUnknown>) -> bool {
    value.is_some()
}

pub fn kept<'a>(values: &[Option<&'a Unknown>]) -> Vec<&'a Unknown> {
    values.iter().filter_map(|v| *v).collect()
}

pub fn object(value: Option<&JsObject>) -> bool {
    value.is_some()
}

pub fn symbol(value: Option<&Symbol>) -> bool {
    value.is_none()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...externs("js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  for (const written of ["return value != null;", "return value == null;", "return value != null ? 1 : 0;", "if (value == null) {\n    return;\n  }",
    ".filter((item) => item != null)", "return !!value;", "return !value;"]) {
    expect(js).toContain(written);
  }
  const lib = await import(join(dir, "lib.js"));
  const falsy = [0, "", false, NaN, 0n];
  expect(falsy.map((v) => [lib.has(v), lib.lacks(v), lib.branch(v), lib.tried(v), lib.given(v)]))
    .toEqual(falsy.map(() => [true, false, 1, 1, true]));
  expect([null, undefined].map((v) => [lib.has(v), lib.lacks(v), lib.branch(v), lib.tried(v), lib.given(v)]))
    .toEqual([null, undefined].map(() => [false, true, 0, undefined, false]));
  expect(lib.kept([0, undefined, "", null, false])).toEqual([0, "", false]);
  expect([lib.object({}), lib.object(undefined), lib.symbol(Symbol("s")), lib.symbol(undefined)]).toEqual([true, false, false, true]);
});

// What `#[rust_js::types]` says may be falsy, a keyword's `number`, a
// union's `string | URL`, the empty string's `""`, is tested against `null` though the type is a JS
// object's struct; an interface's name is an object, tested by its truth.
test("an Option of a binding whose written type may be falsy is tested against null", async () => {
  const dir = fixture("written-falsy");
  writeFileSync(join(dir, "lib.rs"), `use std::marker::PhantomData;
use js::JsObject;

#[cfg_attr(rust_js, rust_js::types = "number")]
pub struct Count(PhantomData<JsObject>);

#[cfg_attr(rust_js, rust_js::types = "string | URL")]
pub struct Place(PhantomData<JsObject>);

#[cfg_attr(rust_js, rust_js::types = "\\"\\" | \\"auto\\"")]
pub struct Mode(PhantomData<JsObject>);

#[cfg_attr(rust_js, rust_js::types = "HTMLElement")]
pub struct Node(PhantomData<JsObject>);

pub fn counted(n: Option<&Count>) -> bool {
    n.is_some()
}

pub fn placed(p: Option<&Place>) -> bool {
    p.is_none()
}

pub fn moded(m: Option<&Mode>) -> bool {
    m.is_some()
}

pub fn noded(n: Option<&Node>) -> bool {
    n.is_some()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...externs("js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("export function counted(n) {\n  return n != null;\n}");
  expect(js).toContain("export function placed(p) {\n  return p == null;\n}");
  expect(js).toContain("export function moded(m) {\n  return m != null;\n}");
  expect(js).toContain("export function noded(n) {\n  return !!n;\n}");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.counted(0), lib.counted(undefined), lib.placed(""), lib.placed(null), lib.moded(""), lib.noded({}), lib.noded(undefined)])
    .toEqual([true, false, false, true, true, true, false]);
});

// JSX shows an `Option` of an object mapped to an element as `error && <b
// />`, which renders nothing where it's missing; one of a value of any
// shape, which may be `0`, is `value != null && <b />`: `value && ..`
// would render `0` where Rust shows the element.
test("JSX shows an Option of a value of any shape where it's not null", async () => {
  const dir = fixture("jsx-any-shape");
  writeFileSync(join(dir, "lib.rs"), `#![allow(non_snake_case)]
use js::Unknown;
use react::{JSX, jsx};
pub struct ShownProps<'a> {
    pub value: Option<&'a Unknown>,
}
pub fn Shown(ShownProps { value }: ShownProps) -> JSX::Element {
    jsx! {
        <div hidden={value.is_none()}>
            {value.map(|_| jsx! { <b>"there"</b> })}
            {value.filter(|v| js::truthy(Some(*v))).map(|_| jsx! { <i>"truthy"</i> })}
        </div>
    }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...externs("react", "js")]);
  const jsx = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(jsx).toContain("hidden={value == null}");
  expect(jsx).toContain("{value != null && <b>there</b>}");
  // Of its truth, `!!value`, a `bool`: `value && ..` would render `0`.
  expect(jsx).toContain("{!!value && <i>truthy</i>}");
  const lib = await import(join(dir, "lib.jsx"));
  const shown = (value: unknown) => renderToStaticMarkup(createElement(lib.Shown, { value }));
  expect([shown(0), shown(""), shown(false), shown(1), shown(undefined)]).toEqual([
    "<div><b>there</b></div>",
    "<div><b>there</b></div>",
    "<div><b>there</b></div>",
    "<div><b>there</b><i>truthy</i></div>",
    '<div hidden=""></div>',
  ]);
});

// A number's `Option` shown in JSX is `height != null && (..)`, which `0`
// passes; that test reads the same after a later child's statement, its
// overlay's `&&`, as `!!error` does, so it's in place, not a `const
// condition` before the element.
test("JSX tests a number's Option against null in place", async () => {
  const dir = fixture("jsx-number-option");
  writeFileSync(join(dir, "lib.rs"), `#![allow(non_snake_case)]
use react::{JSX, jsx, use_state};
fn cn(parts: &[Option<&str>]) -> String {
    parts.iter().flatten().copied().collect::<Vec<_>>().join(" ")
}
pub struct LoadingOverlayProps {
    pub dependencies_loading: bool,
}
pub fn LoadingOverlay(LoadingOverlayProps { dependencies_loading }: LoadingOverlayProps) -> Option<JSX::Element> {
    dependencies_loading.then(|| jsx! { <p /> })
}
pub fn Preview(is_expanded: bool, rawHeight: Option<f64>) -> JSX::Element {
    let (bundlerIsReady, _) = use_state(|| false);
    let (iframeComputedHeight, _) = use_state::<Option<f64>>(|| None);
    let (height, _) = use_state(move || rawHeight);
    let height = *height;
    jsx! {
        <div>
            {height.map(|height| jsx! {
                <b className={cn(&[Some("z-50"), is_expanded.then_some("sticky")])}>{height}</b>
            })}
            <LoadingOverlay dependenciesLoading={!*bundlerIsReady && iframeComputedHeight.is_none()} />
        </div>
    }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...externs("react")]);
  const jsx = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(jsx).toContain("{height != null && (");
  expect(jsx).not.toContain("const condition");
  const lib = await import(join(dir, "lib.jsx"));
  const shown = (height: number | undefined) => renderToStaticMarkup(createElement(() => lib.Preview(false, height)));
  expect([shown(0), shown(1.5), shown(undefined)])
    .toEqual(['<div><b class="z-50">0</b><p></p></div>', '<div><b class="z-50">1.5</b><p></p></div>', "<div><p></p></div>"]);
});

// React's startTransition: `match prev { Some(prev) => prev.types, None =>
// None }`, an object's `Option`, tested by its truth, `prev ? prev.types :
// undefined`, is `prev?.types`, as it was of `prev != null`: a value never
// falsy is set where it's truthy. Text's, which may be `""`, keeps its test,
// `s != null ? $byteLen(s) : undefined`, as `""`'s length is `0`.
test("an Option of an object read in a conditional is an optional chain", async () => {
  const dir = fixture("present-chain");
  writeFileSync(join(dir, "lib.rs"), `pub struct Point {
    pub x: u32,
    pub name: String,
}

pub struct Out {
    pub x: Option<u32>,
    pub name: Option<String>,
}

pub fn read(p: Option<&Point>, out: &mut Out) {
    out.x = match p {
        Some(p) => Some(p.x),
        None => None,
    };
    out.name = if let Some(p) = p { Some(p.name.clone()) } else { None };
}

pub fn text(s: Option<&str>, out: &mut Out) {
    out.x = match s {
        Some(s) => Some(s.len() as u32),
        None => None,
    };
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...externs("js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("out.x = p?.x;");
  expect(js).toContain("out.name = p?.name;");
  expect(js).toContain("out.x = s != null ?");
  const lib = await import(join(dir, "lib.js"));
  const read = (p: unknown) => {
    const out = {};
    lib.read(p, out);
    return out;
  };
  expect([read({ x: 0, name: "a" }), read(undefined)]).toEqual([{ x: 0, name: "a" }, { x: undefined, name: undefined }]);
  const text = (s: unknown) => {
    const out: { x?: number } = {};
    lib.text(s, out);
    return out.x;
  };
  expect([text(""), text("ab"), text(undefined)]).toEqual([0, 2, undefined]);
});

// React's ReactFiberErrorLogger, ported: `match COMPONENT_NAME.with_borrow(
// Clone::clone).filter(|name| !name.is_empty()) { Some(component_name) =>
// format!(..), None => .. }`. A string's truth is exactly a `Some` of one
// not empty, so a module variable's text kept by its truth, then tested,
// is tested by its truth, upstream's `componentName ? `..${componentName}..`
// : '..'`, where the arm that reads it does nothing, so nothing can change
// the variable between; one that runs code, which may, keeps its binding,
// and so does an `if`'s (ADR 0635, amended).
test("a module variable's text filtered by its truth is tested by its truth", async () => {
  const dir = fixture("module-text-truth");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::RefCell;

thread_local! {
    #[cfg_attr(rust_js, rust_js::nullable)]
    static COMPONENT_NAME: RefCell<Option<String>> = const { RefCell::new(None) };
}

pub fn set(name: Option<String>) {
    COMPONENT_NAME.set(name);
}

pub fn message() -> String {
    let component_name_message = match COMPONENT_NAME
        .with_borrow(Clone::clone)
        .filter(|name| !name.is_empty())
    {
        Some(component_name) => {
            format!("An error occurred in the <{component_name}> component.")
        }
        None => "An error occurred in one of your React components.".to_string(),
    };
    component_name_message
}

fn cleared_name(name: &str) -> String {
    set(None);
    name.to_uppercase()
}

/// The arm runs code, which may change the variable.
pub fn shouted() -> String {
    let shout = match COMPONENT_NAME.with_borrow(Clone::clone).filter(|name| !name.is_empty()) {
        Some(name) => cleared_name(&name),
        None => String::new(),
    };
    shout
}

pub fn logged(log: &mut Vec<String>) {
    if let Some(name) = COMPONENT_NAME.with_borrow(Clone::clone).filter(|n| !n.is_empty()) {
        log.push(name);
    }
}

/// Code that changes the variable runs before the binding's read.
pub fn cleared(log: &mut Vec<String>) {
    if let Some(name) = COMPONENT_NAME.with_borrow(Clone::clone).filter(|n| !n.is_empty()) {
        set(None);
        log.push(name);
    }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...externs("js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("  const componentNameMessage = componentName\n    ? `An error occurred in the <${componentName}> component.`\n    : \"An error occurred in one of your React components.\";\n");
  expect(js).toContain("  const name = componentName;\n  const shout = name ? clearedName(name) : \"\";\n");
  const lib = await import(join(dir, "lib.js"));
  const said = [lib.message()];
  lib.set("" as never);
  said.push(lib.message());
  lib.set("App" as never);
  said.push(lib.message());
  const log: string[] = [];
  lib.logged(log as never);
  lib.cleared(log as never);
  lib.logged(log as never);
  lib.set("app" as never);
  said.push(lib.shouted(), lib.shouted());
  expect(said).toEqual(["An error occurred in one of your React components.", "An error occurred in one of your React components.", "An error occurred in the <App> component.", "APP", ""]);
  expect(log).toEqual(["App", "App"]);
});

// React's ReactFiberLegacyContext, ported: `let Some(context_types) =
// js::get(r#type, "contextTypes").filter(..js::truthy..) else { return .. }`.
// What `filter` keeps of a read that's no variable, `type.contextTypes ||
// undefined`, then a `let … else` of it, is the read in its `const`, tested
// by its truth, upstream's `const contextTypes = type.contextTypes; if
// (!contextTypes)`: past the test it's truthy, which `|| undefined` keeps as
// it is (ADR 0635, amended).
test("a let-else of a read kept by its truth is the read tested by its truth", async () => {
  const dir = fixture("let-else-read-truth");
  writeFileSync(join(dir, "lib.rs"), `use js::Unknown;

pub fn masked(r#type: &Unknown) -> u32 {
    let Some(context_types) = js::get(r#type, "contextTypes").filter(|context_types| js::truthy(Some(*context_types)))
    else {
        return 0;
    };
    let mut keys = 0;
    for _key in js::keys_in(context_types) {
        keys += 1;
    }
    keys
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...externs("js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("  const contextTypes = type.contextTypes;\n  if (!contextTypes) {\n    return 0;\n  }\n");
  const lib = await import(join(dir, "lib.js"));
  const masked = [{ contextTypes: { a: 1, b: 2 } }, { contextTypes: 0 }, { contextTypes: "" }, { contextTypes: null }, {}].map(lib.masked);
  expect(masked).toEqual([2, 0, 0, 0, 0]);
});

// ADR 0953: React's shouldConstruct, `return !!(prototype &&
// prototype.isReactComponent);`: the truths of two values joined are the
// truth of the two joined, `||` too.
test("two values' truths joined are the truth of the two", async () => {
  const dir = fixture("truths-joined");
  writeFileSync(
    join(dir, "lib.rs"),
    `use js::Unknown;

pub fn should_construct(component: Option<&Unknown>) -> bool {
    let prototype = js::get(component, "prototype");
    js::truthy(prototype) && js::truthy(js::get(prototype, "isReactComponent"))
}

pub fn either(a: Option<&Unknown>, b: Option<&Unknown>) -> bool {
    js::truthy(a) || js::truthy(b)
}
`,
  );
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("  return !!(prototype && prototype.isReactComponent);\n");
  expect(js).toContain("  return !!(a || b);\n");
  const lib = await import(join(dir, "lib.js"));
  class C {}
  (C.prototype as Record<string, unknown>).isReactComponent = {};
  expect([lib.shouldConstruct(C), lib.shouldConstruct(() => 1), lib.shouldConstruct({ prototype: {} })]).toEqual([true, false, false]);
  expect([lib.either(0, ""), lib.either(0, "x"), lib.either(1, undefined)]).toEqual([false, true, true]);
});
