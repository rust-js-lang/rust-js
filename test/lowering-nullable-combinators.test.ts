// What the compiler writes of an `Option` given to the crate's function's
// own `#[rust_js::nullable(..)]` parameter by a combinator (ADR 0861,
// amended): `a.or(b)`, `a.or_else(f)`, a `take()` or `replace(v)` of a
// nullable field, `null` or a value where what it gives is, so the callee
// stores it as it is. Each test is a program of its own, compiled and run.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildReact, compiler, fixture, run, target } from "./support";

beforeAll(buildReact, 600_000);

// #689, React's createCapturedValueFromError, `stack: stack`, whose one
// call, ReactFiberBeginWork's, gives `componentStack ?? null`, of
// `component_stack.or(js::null())`. `a.or(b)` is `a` where it's `Some`, a
// value, else `b`: `null` or a value where `b` is, whatever `a`'s `None`
// is; `a.or_else(f)` where `f` returns so. And an `Option`'s `take()` or
// `replace(v)` gives what its place held, `null` or a value where that's a
// nullable field. Where `b` may be `undefined` the callee keeps `?? null`,
// as it does of `and`, `xor` and `filter`, whose `None` is `undefined`.
test("an or of null, and a take of a nullable field, are null or a value at a call", async () => {
  const dir = fixture("nullable-or-null-call");
  writeFileSync(join(dir, "lib.rs"), `use js::Unknown;

pub struct CapturedValue {
    pub value: u32,
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub stack: Option<&'static Unknown>,
}

pub struct Details {
    pub component_stack: Option<&'static Unknown>,
}

pub struct Root {
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub pending: Option<&'static Unknown>,
}

#[cfg_attr(rust_js, rust_js::link_name = "./given.js#maybe")]
fn maybe() -> Option<&'static Unknown> {
    unreachable!()
}

#[cfg_attr(rust_js, rust_js::nullable(stack))]
fn create_captured_value_from_error(value: u32, stack: Option<&'static Unknown>) -> CapturedValue {
    CapturedValue { value, stack }
}

#[cfg_attr(rust_js, rust_js::nullable(stack))]
fn create_captured_else(value: u32, stack: Option<&'static Unknown>) -> CapturedValue {
    CapturedValue { value, stack }
}

#[cfg_attr(rust_js, rust_js::nullable(stack))]
fn create_captured_taken(value: u32, stack: Option<&'static Unknown>) -> CapturedValue {
    CapturedValue { value, stack }
}

#[cfg_attr(rust_js, rust_js::nullable(stack))]
fn create_captured_maybe(value: u32, stack: Option<&'static Unknown>) -> CapturedValue {
    CapturedValue { value, stack }
}

#[cfg_attr(rust_js, rust_js::nullable(stack))]
fn create_captured_and(value: u32, stack: Option<&'static Unknown>) -> CapturedValue {
    CapturedValue { value, stack }
}

pub fn begin(details: Details) -> CapturedValue {
    create_captured_value_from_error(1, details.component_stack.or(js::null()))
}

pub fn begin_else(details: Details) -> CapturedValue {
    create_captured_else(2, details.component_stack.or_else(js::null))
}

pub fn begin_taken(root: &mut Root) -> CapturedValue {
    let replaced = root.pending.replace(js::unknown(&5));
    create_captured_taken(3, root.pending.take().or(replaced))
}

pub fn begin_maybe(details: Details) -> CapturedValue {
    create_captured_maybe(4, js::null().or(maybe()))
}

pub fn begin_and(details: Details) -> CapturedValue {
    create_captured_and(5, details.component_stack.and(js::null()))
}

#[cfg_attr(rust_js, rust_js::nullable(stack))]
fn create_captured_param(value: u32, stack: Option<&'static Unknown>) -> CapturedValue {
    CapturedValue { value, stack }
}

#[cfg_attr(rust_js, rust_js::nullable(stack))]
pub fn begin_param(mut stack: Option<&'static Unknown>) -> Vec<CapturedValue> {
    let first = create_captured_param(6, stack.take());
    vec![first, create_captured_param(7, stack.take())]
}

#[cfg_attr(rust_js, rust_js::nullable(stack))]
fn create_captured_plain(value: u32, stack: Option<&'static Unknown>) -> CapturedValue {
    CapturedValue { value, stack }
}

pub fn begin_plain(details: &mut Details) -> CapturedValue {
    create_captured_plain(8, details.component_stack.take())
}
`);
  writeFileSync(join(dir, "given.js"), "export const maybe = () => undefined;\n");
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  for (const f of ["createCapturedValueFromError", "createCapturedElse", "createCapturedTaken"]) {
    expect(js).toContain(`function ${f}(value, stack) {\n  return { value, stack };\n}`);
  }
  for (const f of ["createCapturedMaybe", "createCapturedAnd", "createCapturedParam", "createCapturedPlain"]) {
    expect(js).toContain(`function ${f}(value, stack) {\n  return { value, stack: stack ?? null };\n}`);
  }
  expect(js).toContain("  return createCapturedValueFromError(1, details.componentStack ?? null);\n");
  expect(js).toContain("  return createCapturedTaken(3, old ?? replaced);\n");
  const lib = await import(join(dir, "lib.js"));
  const stack = {};
  expect([lib.begin({ componentStack: undefined }), lib.begin({ componentStack: stack })]).toEqual([
    { value: 1, stack: null },
    { value: 1, stack },
  ]);
  expect([lib.beginElse({}), lib.beginElse({ componentStack: stack })]).toEqual([
    { value: 2, stack: null },
    { value: 2, stack },
  ]);
  expect(lib.beginTaken({ pending: null })).toEqual({ value: 3, stack: 5 });
  expect(lib.beginTaken({ pending: stack })).toEqual({ value: 3, stack: 5 });
  expect([lib.beginMaybe({}), lib.beginAnd({}), lib.beginAnd({ componentStack: stack })]).toEqual([
    { value: 4, stack: null },
    { value: 5, stack: null },
    { value: 5, stack: null },
  ]);
  expect(lib.beginParam(stack)).toEqual([
    { value: 6, stack },
    { value: 7, stack: null },
  ]);
  expect([lib.beginPlain({}), lib.beginPlain({ componentStack: stack })]).toEqual([
    { value: 8, stack: null },
    { value: 8, stack },
  ]);
});
