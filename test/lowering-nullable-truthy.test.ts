// An `Option` a test of `js::truthy` proved there, JS's `if (found)`,
// which neither `null` nor `undefined` passes: read inside the test's true
// branch, it's given as it is, no `?? null` at a nullable exit and no `??
// undefined` at a plain one (#1902, `proved_present`). Each test is a
// program of its own, compiled and run.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildReact, compiler, fixture, run, target } from "./support";

beforeAll(buildReact, 600_000);

const compile = (name: string, source: string) => {
  const dir = fixture(name);
  writeFileSync(join(dir, "lib.rs"), source);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "--extern", `webapi=${join(target, "libwebapi.rmeta")}`, "-L", target]);
  return { dir, js: readFileSync(join(dir, "lib.js"), "utf8") };
};

// A faithful SDK port's composedClosest: the optional call's result, tested
// truthy, is returned as it is; the miss is `null`.
test("an Option a truthy test proved there is returned as it is, its miss null", async () => {
  const { dir, js } = compile(
    "nullable-truthy",
    `use webapi::{Element, IsA, Node};

/// # Safety
/// Present methods and results satisfy their asserted interfaces.
#[cfg_attr(rust_js, rust_js::nullable)]
pub unsafe fn closest<T: IsA<Element>>(hop: &Node, selector: &str) -> Option<&'static T> {
    let found = unsafe { js::optional_call_unchecked((hop, Element::closest), (selector,)) }.flatten();
    if js::truthy(found) {
        return unsafe { js::cast(found.map(js::unknown)) };
    }
    None
}

#[cfg_attr(rust_js, rust_js::nullable)]
pub fn given(value: Option<&'static js::Unknown>) -> Option<&'static js::Unknown> {
    if js::truthy(value) {
        return value;
    }
    None
}
`,
  );
  expect(js).toContain("  if (found) {\n    return found;\n  }\n  return null;\n");
  // A parameter nothing writes, the same where it's tested and read.
  expect(js).toContain("export function given(value) {\n  if (value) {\n    return value;\n  }\n  return null;\n}");
  const lib = await import(join(dir, "lib.js"));
  const hit = { id: 1 };
  expect(lib.closest({ closest: () => hit }, "a")).toBe(hit);
  expect(lib.closest({ closest: () => undefined }, "a")).toBe(null);
  expect(lib.closest({ closest: () => null }, "a")).toBe(null);
  expect(lib.closest({}, "a")).toBe(null);
  expect([lib.given(hit), lib.given(undefined), lib.given(null), lib.given(0)]).toEqual([hit, null, null, null]);
});

// What the test proves is of the value it tested: one written again, one
// read where the test failed or where another value's test held, or in a
// negated test's branch, keeps its normalization.
test("a truthy test proves nothing of a value written again, of its else, of another's test, or of a negated test", async () => {
  const { dir, js } = compile(
    "nullable-truthy-not",
    `use js::Unknown;

#[cfg_attr(rust_js, rust_js::nullable)]
pub fn rewritten(first: Option<&'static Unknown>, second: Option<&'static Unknown>) -> Option<&'static Unknown> {
    let mut found = first;
    if js::truthy(found) {
        found = second;
        return found;
    }
    None
}

#[cfg_attr(rust_js, rust_js::nullable)]
pub fn otherwise(found: Option<&'static Unknown>, other: Option<&'static Unknown>) -> Option<&'static Unknown> {
    let found = found.or(other);
    if js::truthy(found) {
        return None;
    }
    found
}

#[cfg_attr(rust_js, rust_js::nullable)]
pub fn another(found: Option<&'static Unknown>, other: Option<&'static Unknown>) -> Option<&'static Unknown> {
    let found = found.or(None);
    if js::truthy(other) {
        return found;
    }
    None
}

#[cfg_attr(rust_js, rust_js::nullable)]
pub fn negated(found: Option<&'static Unknown>, other: Option<&'static Unknown>) -> Option<&'static Unknown> {
    let found = found.or(other);
    if !js::truthy(found) {
        return found;
    }
    None
}
`,
  );
  const fn = (name: string) => js.slice(js.indexOf(`export function ${name}(`), js.indexOf("\n}\n", js.indexOf(`export function ${name}(`)) + 2);
  expect(fn("rewritten")).toContain("?? null");
  expect(fn("otherwise")).toContain("?? null");
  expect(fn("another")).toContain("?? null");
  expect(fn("negated")).toContain("?? null");
  const lib = await import(join(dir, "lib.js"));
  const one = { id: 1 };
  expect([lib.rewritten(one, undefined), lib.rewritten(one, one), lib.rewritten(undefined, one)]).toEqual([null, one, null]);
  expect([lib.otherwise(undefined, undefined), lib.otherwise(one, undefined)]).toEqual([null, null]);
  expect([lib.another(undefined, one), lib.another(one, one), lib.another(one, undefined)]).toEqual([null, one, null]);
  expect([lib.negated(undefined, undefined), lib.negated(one, undefined)]).toEqual([null, null]);
});

// A plain `Option`'s exit gives Rust's one `None`, `undefined`, of what JS
// may give `null`, a binding's result declared `T | null` (#1840): of one a
// truthy test proved there, it's neither, and given as it is.
test("a plain Option's exit gives a value a truthy test proved there as it is", async () => {
  const { dir, js } = compile(
    "plain-truthy",
    `use webapi::{Element, Node};

pub fn pick(hop: &Element, selector: &str) -> Option<&'static Element> {
    let found = hop.closest(selector);
    if js::truthy(found) {
        return found;
    }
    None
}

pub fn keep(hop: &Element, selector: &str) -> Option<&'static Element> {
    hop.closest(selector)
}

pub fn node_of(element: &Element) -> &Node {
    element
}
`,
  );
  const fn = (name: string) => js.slice(js.indexOf(`export function ${name}(`), js.indexOf("\n}\n", js.indexOf(`export function ${name}(`)) + 2);
  expect(fn("pick")).toContain("  if (found) {\n    return found;\n  }\n");
  expect(fn("keep")).toContain("?? undefined");
  const lib = await import(join(dir, "lib.js"));
  const one = { id: 1 };
  const hop = (found: unknown) => ({ closest: () => found });
  expect([lib.pick(hop(one), "a"), lib.pick(hop(null), "a")]).toEqual([one, undefined]);
  expect([lib.keep(hop(one), "a"), lib.keep(hop(null), "a")]).toEqual([one, undefined]);
});

test("an option filled on its missing branch is present at the join", async () => {
  const { dir, js } = compile("option-filled-on-missing-branch", `
pub fn filled(mut value: Option<f64>, initialize: fn() -> f64) -> f64 {
    if value.is_none() {
        value = Some(initialize());
    }
    value.unwrap()
}

pub fn filled_after_read(read: fn() -> Option<f64>, initialize: fn() -> f64) -> f64 {
    let mut value = read();
    if value.is_none() {
        value = Some(initialize());
    }
    value.unwrap()
}

pub fn maybe_filled(mut value: Option<f64>, fill: bool) -> f64 {
    if value.is_none() && fill {
        value = Some(42.0);
    }
    value.unwrap()
}

pub fn reset(mut value: Option<f64>, change: fn(&mut Option<f64>)) -> f64 {
    if value.is_none() {
        value = Some(42.0);
    }
    change(&mut value);
    value.unwrap()
}
`);
  const filled = js.slice(js.indexOf("export function filled"), js.indexOf("export function maybeFilled"));
  expect(filled).toContain("return value;");
  expect(filled).not.toContain("$unwrap");
  const lib = await import(join(dir, "lib.js"));
  expect(lib.filled(12, () => { throw new Error("must not initialize"); })).toBe(12);
  expect(lib.filled(undefined, () => 42)).toBe(42);
  expect(lib.filled(null, () => 42)).toBe(42);
  expect(lib.filledAfterRead(() => undefined, () => 42)).toBe(42);
  expect(lib.filledAfterRead(() => 12, () => { throw new Error("must not initialize"); })).toBe(12);
  expect(() => lib.maybeFilled(undefined, false)).toThrow("called `Option::unwrap()` on a `None` value");
  expect(() => lib.reset(12, value => { value.value = undefined; })).toThrow("called `Option::unwrap()` on a `None` value");
});
