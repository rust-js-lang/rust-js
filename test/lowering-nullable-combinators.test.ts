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

// `by.map(|x| items.iter().map(|n| n + x).collect())`: the map's closure,
// called with what it's given, holds a closure that reads its argument, `x`,
// which is named where the inner one reads it, never left reading a name
// replaced (examples/combinators.rs's `shifted`, #1953).
test("a map's closure whose own closure reads its argument reads it there", async () => {
  const dir = fixture("combinator-closure-reads-argument");
  writeFileSync(join(dir, "lib.rs"), `pub fn shifted(by: Option<u32>, items: Vec<u32>) -> Option<Vec<u32>> {
    by.map(|x| items.iter().map(|n| n + x).collect())
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const lib = await import(join(dir, "lib.js"));
  expect([lib.shifted(2, [1, 3]), lib.shifted(undefined, [1])]).toEqual([[3, 5], undefined]);
});

// `opt.map(Reason::Root)` of an untagged enum's variant, its payload, is
// the option itself, so `null` or a value where `opt` is, stored as it is
// in a nullable field, `reason: root`, MIR's answer, not tidy's (React's
// Flight reply server's `array_root.map(InitializedReason::ArrayRoot)`,
// `constructs_payload`, #1933 step 4 (E)); of an `opt` that may be
// `undefined`, `?? null`; of an enum a value of which may look like `None`,
// `#[rust_js::undefined]`'s, whose `Option` is boxed, the option too, as a
// variant's payload is never `null` nor `undefined` (an untagged enum
// refuses one that may be, ADR 0214), so its box is the payload itself.
// Not a tagged enum's variant, which is no payload.
test("a map by an untagged enum's variant is the option itself", async () => {
  const dir = fixture("nullable-map-untagged-variant");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::RefCell;
use std::rc::Rc;

pub struct Context {
    pub count: u32,
}

#[cfg_attr(rust_js, rust_js::untagged)]
pub enum Reason {
    Root(Rc<Context>),
    Text(String),
}

#[cfg_attr(rust_js, rust_js::untagged)]
pub enum Maybe {
    Root(Rc<Context>),
    #[cfg_attr(rust_js, rust_js::undefined)]
    Missing,
}

pub struct Slot {
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub maybe: Option<Maybe>,
}

#[cfg_attr(rust_js, rust_js::tag = "status")]
pub enum State {
    #[cfg_attr(rust_js, rust_js::name = "pending")]
    Pending,
    #[cfg_attr(rust_js, rust_js::name = "fulfilled")]
    Fulfilled {
        value: u32,
        #[cfg_attr(rust_js, rust_js::nullable)]
        reason: Option<Reason>,
    },
}

pub struct Chunk {
    pub state: RefCell<State>,
}

pub struct Handler {
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub root: RefCell<Option<Rc<Context>>>,
    pub value: u32,
}

pub fn resolved(handler: &Handler, chunk: &Chunk) {
    let root = handler.root.borrow().clone();
    *chunk.state.borrow_mut() = State::Fulfilled {
        value: handler.value,
        reason: root.map(Reason::Root),
    };
}

pub fn given(root: Option<Rc<Context>>, chunk: &Chunk) {
    *chunk.state.borrow_mut() = State::Fulfilled {
        value: 2,
        reason: root.map(Reason::Root),
    };
}

pub fn slotted(handler: &Handler) -> Slot {
    let root = handler.root.borrow().clone();
    Slot {
        maybe: root.map(Maybe::Root),
    }
}

pub fn told(slot: &Slot) -> u32 {
    match &slot.maybe {
        None => 0,
        Some(Maybe::Root(context)) => context.count,
        Some(Maybe::Missing) => 9,
    }
}

pub enum Shape {
    Circle(f64),
    Square(f64),
}

pub fn shaped(r: Option<f64>) -> u32 {
    match r.map(Shape::Circle) {
        Some(Shape::Circle(_)) => 1,
        Some(Shape::Square(_)) => 2,
        None => 0,
    }
}

pub fn pending() -> Chunk {
    Chunk {
        state: RefCell::new(State::Pending),
    }
}
`);
  const counts = join(dir, "decisions.tsv");
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target], 120_000, { RUST_JS_DECISIONS: counts });
  const tidied = readFileSync(counts, "utf8")
    .split("\n")
    .map((line) => line.split("\t"))
    .filter((cells) => cells[0]!.startsWith("tidy-") && Number(cells[4]) > 0);
  expect(tidied).toEqual([]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("    reason: root,\n");
  expect(js).toContain("    reason: root ?? null,\n");
  const lib = await import(join(dir, "lib.js"));
  const context = { count: 1 };
  const reasons = [
    (chunk: { state: { reason?: unknown } }) => lib.resolved({ root: null, value: 1 }, chunk),
    (chunk: { state: { reason?: unknown } }) => lib.resolved({ root: context, value: 1 }, chunk),
    (chunk: { state: { reason?: unknown } }) => lib.given(undefined, chunk),
    (chunk: { state: { reason?: unknown } }) => lib.given(context, chunk),
  ].map((resolve) => {
    const chunk = lib.pending();
    resolve(chunk);
    return chunk.state.reason;
  });
  expect(reasons).toEqual([null, context, null, context]);
  // Of an enum that may be `undefined`, `Maybe::Missing`, an `Option` of
  // it is boxed (ADR 0051), and its box, `$some`, wraps only a value that
  // looks like `None`, which a variant's payload never is (ADR 0214): the
  // map is the option itself there too, read back as each variant.
  expect(js).toContain("  return { maybe: handler.root };\n");
  expect([lib.told(lib.slotted({ root: null, value: 1 })), lib.told(lib.slotted({ root: context, value: 1 }))]).toEqual([0, 1]);
  // A tagged enum's variant is no payload: its map makes the variant.
  expect([lib.shaped(2), lib.shaped(undefined)]).toEqual([1, 0]);
});
