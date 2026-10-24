// What JS is given of a `()` (ADR 0775): `undefined`, what `()` is
// everywhere, where it's a value of any shape, `js::strict_equals(x, ())`'s
// `x === undefined`; and `[]` only where it's a list of none, a rest
// parameter's arguments or a hook's dependencies. Each program is
// compiled and run.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";

import { buildReact, compiler, fixture, run, target } from "./support";

beforeAll(buildReact, 600_000);

const withJs = ["--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];

// The wrong answer ADR 0775 fixed: a `()` given a binding as a value of
// any shape was its tuple's array, `x === []`, never true.
test("a () given to JS as any value is undefined", async () => {
  const dir = fixture("unit-given-any");
  writeFileSync(join(dir, "lib.rs"), `use js::{IntoUnknown, Unknown};

#[cfg_attr(rust_js, rust_js::link_name = "globalThis.kinds")]
#[allow(unused_variables)]
fn kinds(a: Option<&dyn IntoUnknown>, b: impl IntoUnknown) -> String {
    unreachable!()
}

#[cfg_attr(rust_js, rust_js::link_name = "globalThis.kinds")]
#[allow(unused_variables)]
fn unit_kinds(a: (), b: u8) -> String {
    unreachable!()
}

pub fn is_unit(x: Option<&Unknown>) -> bool {
    js::strict_equals(x, ())
}

pub fn unit_is(x: Option<&Unknown>) -> bool {
    js::strict_equals((), x)
}

pub fn held_is(x: Option<&Unknown>) -> bool {
    let nothing = ();
    js::strict_equals(x, nothing)
}

pub fn set_unit(o: &Unknown) {
    js::set(o, "k", ());
}

pub fn given() -> String {
    kinds(Some(&()), ())
}

pub fn given_typed() -> String {
    unit_kinds((), 1)
}

pub fn given_as_value() -> String {
    let typed: fn((), u8) -> String = unit_kinds;
    typed((), 2)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).not.toContain("[]");
  expect(js).toContain("return x === undefined;");
  expect(js).toContain("return undefined === x;");
  expect(js).toContain("o.k = undefined;");
  expect(js).toContain("return globalThis.kinds(undefined, undefined);");
  const lib = await import(join(dir, "lib.js"));
  (globalThis as any).kinds = (...args: unknown[]) => args.map((arg) => typeof arg).join(" ");
  expect([lib.isUnit(undefined), lib.isUnit(null), lib.isUnit({}), lib.unitIs(undefined), lib.heldIs(undefined)])
    .toEqual([true, false, false, true, true]);
  const o: { k?: number } = { k: 1 };
  lib.setUnit(o);
  expect(o.k).toBeUndefined();
  expect("k" in o).toBe(true);
  expect([lib.given(), lib.givenTyped(), lib.givenAsValue()])
    .toEqual(["undefined undefined", "undefined number", "undefined number"]);
});

// A rest parameter's `()` is its arguments, none: `counted("a")`, of
// `impl js::Rest<T>` and of a tuple, and as rest arguments held as a
// value, spread, none too.
test("a () of rest arguments is none of them", async () => {
  const dir = fixture("unit-given-rest");
  writeFileSync(join(dir, "lib.rs"), `#[cfg_attr(rust_js, rust_js::link_name = "globalThis.counted")]
#[cfg_attr(rust_js, rust_js::variadic)]
#[allow(unused_variables)]
fn counted(first: &str, rest: impl js::Rest<dyn js::IntoUnknown>) -> u32 {
    unreachable!()
}

#[cfg_attr(rust_js, rust_js::link_name = "globalThis.counted")]
#[cfg_attr(rust_js, rust_js::variadic)]
#[allow(unused_variables)]
fn counted_held(first: &str, rest: &dyn js::Rest<dyn js::IntoUnknown>) -> u32 {
    unreachable!()
}

#[cfg_attr(rust_js, rust_js::link_name = "globalThis.counted")]
#[cfg_attr(rust_js, rust_js::variadic)]
#[allow(unused_variables)]
fn counted_tuple(first: &str, rest: ()) -> u32 {
    unreachable!()
}

pub fn none() -> u32 {
    counted("a", ())
}

pub fn none_of_a_tuple() -> u32 {
    counted_tuple("b", ())
}

pub fn one_unit() -> u32 {
    counted("a", ((),))
}

pub fn held() -> u32 {
    counted_held("a", &())
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('return globalThis.counted("a");');
  expect(js).toContain('return globalThis.counted("a", undefined);');
  expect(js).toContain('return globalThis.counted("b");');
  const lib = await import(join(dir, "lib.js"));
  (globalThis as any).counted = (...args: unknown[]) => args.length;
  expect([lib.none(), lib.noneOfATuple(), lib.oneUnit(), lib.held()]).toEqual([1, 1, 2, 1]);
});

// A hook's dependencies are a list, `(a, b)`'s `[a, b]`: `()`, none, is
// `[]`, which runs an effect once, where `undefined` would run it after
// every render (ADR 0443).
test("a () of a hook's dependencies is []", async () => {
  const dir = fixture("unit-given-deps");
  writeFileSync(join(dir, "lib.rs"), `#![allow(non_snake_case)]
use react::{JSX, jsx, use_callback, use_effect, use_memo};

pub fn Once() -> JSX::Element {
    use_effect(|| {}, Some(&()));
    let doubled = use_memo(|| 2, ());
    let clicked = use_callback(|| {}, ());
    clicked();
    jsx! { <b>{*doubled}</b> }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `react=${join(target, "libreact.rmeta")}`, "-L", target]);
  const jsx = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(jsx).toMatch(/useEffect\([^;]*, \[\]\);/);
  expect(jsx).toMatch(/useMemo\([^;]*, \[\]\);/);
  expect(jsx).toMatch(/useCallback\([^;]*, \[\]\);/);
  const lib = await import(join(dir, "lib.jsx"));
  expect(renderToStaticMarkup(createElement(lib.Once))).toBe("<b>2</b>");
});
