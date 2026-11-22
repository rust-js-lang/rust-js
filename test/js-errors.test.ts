// The js crate's errors: what JS throws and what it shows of it, an Error
// object made and typed, an AggregateError, and what was caught. Each test
// is a program of its own, compiled and run.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

import { buildReact, compiler, fixture, run, target } from "./support";

beforeAll(buildReact, 600_000);


// ADR 0035: what a JS function threw is shown as JS shows it, `String(error)`,
// `SyntaxError: ..`, by `{:?}` and by an `expect`'s panic.
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
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js"), ...withJs]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toMatch(/return String\((match\._0|error)\);/);
  // A `match` of what it threw, whose `Ok` only keeps the value and
  // whose `Err` reads no error, is JS's `try` (ADR 0035).
  expect(js).toContain('  let value;\n  try {\n    value = JSON.parse(text);\n  } catch {\n    console.error("invalid");\n    value = undefined;\n  }');
  const lib = await import(join(dir, "mir.js"));
  expect(lib.parsed("[1]")).toEqual([1]);
  expect([lib.kept("[1]"), lib.kept("{")]).toEqual([[1], undefined]);
  // One whose `Err` reads the error keeps it, `$try`'s.
  expect([lib.keptOrTold("[1]"), lib.keptOrTold("{")]).toEqual([[1], undefined]);
  expect([lib.told("1"), lib.told("{").startsWith("SyntaxError")]).toEqual(["", true]);
  expect([lib.valid("[1]"), lib.valid("{")]).toEqual([true, false]);
  // Its `unwrap()` is the call, which throws what JS threw (ADR 0643).
  expect(() => lib.parsed("{")).toThrow(SyntaxError);
  expect([lib.shown("{").startsWith("SyntaxError: "), lib.shown("1")]).toEqual([true, "parsed"]);
});


// React's ReactCapturedValue: `createCapturedValueFromError(value: Error,
// ..)` takes an `Error` object, `js::Error`, which declares `Error`, as an
// `AggregateError` declares its own; what's thrown, a `JsError`, is
// anything, `unknown`, as TypeScript's strict `catch` and React's Flow
// `mixed` have it (#1905, Tu's decision). An `Error` is what may be thrown,
// by `Deref`.
test("an Error object is declared Error, and what's thrown unknown", async () => {
  const withJs = ["--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("js-error-objects");
  writeFileSync(join(dir, "lib.rs"), `use js::{AggregateError, Error, JsError, WeakKey, WeakMap, aggregate_error, error};

thread_local! {
    static STACKS: &'static WeakMap<dyn WeakKey, String> = js::weak_map::new();
}

pub fn captured(value: &'static Error, stack: &str) -> String {
    STACKS.with(|stacks| stacks.set(value, stack.to_string()));
    value.message()
}

pub fn thrown(value: &JsError) -> String {
    value.name()
}

pub fn made(message: &str) -> &'static Error {
    error::new(Some(message), None)
}

// React's Flight thenables stash \`new Error()\`, TypeScript's \`message?\`
// left out: no \`message\` of its own, where \`new Error("")\` has one (rd474).
pub fn unnamed() -> &'static Error {
    error::new(None, None)
}

pub fn all() -> &'static AggregateError {
    aggregate_error::new(&[], Some("every one"), None)
}

pub fn named() -> String {
    let error = made("boom");
    format!("{} {} {}", thrown(error), captured(error, "at"), all().message())
}

pub fn restacked(from: &JsError, frames: bool) -> &'static Error {
    let error = made("fatal");
    match from.stack() {
        Some(stack) if frames => error.set_stack(Some(&(error.name() + ": " + &error.message() + &stack))),
        _ => error.set_stack(None),
    }
    error
}
`);
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs]);
  const declarations = readFileSync(join(dir, "lib.d.ts"), "utf8");
  expect(declarations).toContain("export function captured(value: Error, stack: string): string;");
  expect(declarations).toContain("export function thrown(value: unknown): string;");
  expect(declarations).toContain("export function made(message: string): Error;");
  expect(declarations).toContain("export function all(): AggregateError;");
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('  const error = made("boom");');
  const lib = await import(join(dir, "lib.js"));
  expect(lib.named()).toBe("Error boom every one");
  expect(lib.made("x")).toBeInstanceOf(Error);
  expect(js).toContain("return new Error();");
  expect(Object.hasOwn(lib.unnamed(), "message")).toBe(false);
  expect(Object.hasOwn(lib.made(""), "message")).toBe(true);
  expect(declarations).toContain("export function unnamed(): Error;");
  // An error's frames another's, or none: React's cloneRecoverableErrorAsFatal.
  expect(js).toContain("error.stack = undefined;");
  const from = new Error("x");
  expect(lib.restacked(from, true).stack).toBe("Error: fatal" + from.stack);
  expect(lib.restacked(from, false).stack).toBeUndefined();
});


// ADR 0782's amendment: what was thrown, a `&JsError`, as a value of any
// shape, `error.as_unknown()`, the value itself, which a field of any
// value stores and `js::throw` throws again: of an `Error`, an object, a
// string, and a falsy one, `0`, `""`, `undefined` and `null`.
test("what was caught is a value of any shape", async () => {
  const dir = fixture("js-error-as-unknown");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::Cell;
use std::panic::{self, AssertUnwindSafe};

use js::{JsError, Unknown};

pub struct Node {
    pub v: Cell<Option<&'static Unknown>>,
}

pub fn caught(f: &dyn Fn(), node: &Node) -> bool {
    match panic::catch_unwind(AssertUnwindSafe(f)) {
        Ok(()) => false,
        Err(error) => {
            node.v.set(error.downcast_ref::<&'static JsError>().and_then(|error| error.as_unknown()));
            true
        }
    }
}

pub fn thrown_again(node: &Node) {
    js::throw(node.v.get())
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("node.v = error;");
  const lib = await import(join(dir, "lib.js"));
  for (const thrown of [new Error("e"), { at: 1 }, "x", 0, "", undefined, null]) {
    const node: { v: unknown } = { v: "unset" };
    expect(lib.caught(() => {
      throw thrown;
    }, node)).toBe(true);
    expect(node.v).toBe(thrown);
    let again: unknown = "none";
    try {
      lib.thrownAgain(node);
    } catch (error) {
      again = error;
    }
    expect(again).toBe(thrown);
  }
});


// ADR 0867: an `AggregateError` is TypeScript's of ES2021, `new
// AggregateError(errors: Iterable<any>, message?: string)` and its `errors`,
// an `Error` by `Deref`, as React's `act` makes one of what was thrown; and
// its constructor runs nothing of the crate's, so a `RefCell` borrowed for
// the call is the array it holds, `ReactSharedInternals.thrownErrors`.
test("an AggregateError is ES2021's, and making one borrows nothing", async () => {
  const dir = fixture("js-aggregate-error");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::RefCell;

use js::{Unknown, aggregate_error};

pub struct Internals {
    pub thrown_errors: RefCell<Option<Vec<Option<&'static Unknown>>>>,
}

thread_local! {
    pub static INTERNALS: Internals = Internals { thrown_errors: RefCell::new(Some(Vec::new())) };
}

fn aggregate_errors(errors: &[Option<&'static Unknown>]) -> Option<&'static Unknown> {
    if errors.len() > 1 && js::is_defined(aggregate_error::new) {
        return aggregate_error::new(errors, None, None).as_unknown();
    }
    errors.first().copied().flatten()
}

pub fn record(error: Option<&'static Unknown>) {
    INTERNALS.with(|i| i.thrown_errors.borrow_mut().as_mut().unwrap().push(error));
}

pub fn thrown() -> Option<&'static Unknown> {
    let thrown = INTERNALS.with(|i| aggregate_errors(i.thrown_errors.borrow().as_ref().unwrap()));
    INTERNALS.with(|i| i.thrown_errors.borrow_mut().as_mut().unwrap().clear());
    thrown
}

pub fn named(errors: Vec<Option<&'static Unknown>>) -> (String, String, usize, bool) {
    let e = aggregate_error::new(&errors, Some("several"), None);
    (e.name(), e.message(), e.errors().len(), e.is_error())
}

js::export_default!(INTERNALS);
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("return new AggregateError(errors);");
  expect(js).toContain('const e = new AggregateError(errors, "several");');
  expect(js).toContain("e.errors.length");
  // The cell is the array, borrowed unchecked: no borrow meets another.
  expect(js).toContain("const internals = { thrownErrors: [] };");
  expect(js).toContain("aggregateErrors($unwrap(internals.thrownErrors, \"lib.rs:25:88\"))");
  expect(js).not.toContain("$borrow");
  const lib = await import(join(dir, "lib.js"));
  const [a, b] = [new Error("a"), "b"];
  lib.record(a);
  expect(lib.thrown()).toBe(a);
  lib.record(a);
  lib.record(b);
  const both = lib.thrown();
  expect(both).toBeInstanceOf(AggregateError);
  expect(both.errors).toEqual([a, b]);
  expect(lib.default.thrownErrors).toEqual([]);
  expect(lib.named([a, undefined])).toEqual(["AggregateError", "several", 2, true]);
});

// #1773: JS's standard errors of a kind, ECMAScript's six NativeErrors,
// each made by its module's `new` as `error::new` makes an `Error`
// (builtins/errors.ts): the object is that class's, its prototype and
// constructor JS's own, so `instanceof RangeError` holds where JS reads it,
// and a function that gives one is declared that class.
test("a RangeError and each standard error of a kind is its own class", async () => {
  const withJs = ["--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("js-native-errors");
  writeFileSync(join(dir, "lib.rs"), `use js::{ErrorOptions, RangeError, Unknown};
use js::{eval_error, range_error, reference_error, syntax_error, type_error, uri_error};

pub fn make(message: Option<&str>) -> &'static RangeError {
    range_error::new(message, None)
}

pub fn caused(message: &str, cause: &Unknown) -> &'static RangeError {
    range_error::new(Some(message), Some(&ErrorOptions { cause: Some(cause) }))
}

pub fn shown(error: &RangeError) -> String {
    format!("{}: {}", error.name(), error.message())
}

pub fn target_length(length: usize, expected: usize) -> usize {
    if length != expected {
        js::throw(range_error::new(Some("invalid target buffer length"), None).as_unknown());
    }
    length
}

pub fn each() -> Vec<Option<&'static Unknown>> {
    vec![
        eval_error::new(Some("eval"), None).as_unknown(),
        range_error::new(Some("range"), None).as_unknown(),
        reference_error::new(Some("reference"), None).as_unknown(),
        syntax_error::new(Some("syntax"), None).as_unknown(),
        type_error::new(Some("type"), None).as_unknown(),
        uri_error::new(Some("uri"), None).as_unknown(),
    ]
}
`);
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('throw new RangeError("invalid target buffer length");');
  const declarations = readFileSync(join(dir, "lib.d.ts"), "utf8");
  expect(declarations).toContain("export function make(message?: string | null | undefined): RangeError;");
  expect(declarations).toContain("export function shown(error: RangeError): string;");
  const lib = await import(join(dir, "lib.js"));
  const made = lib.make("boom");
  expect([made instanceof RangeError, made instanceof Error, Object.getPrototypeOf(made) === RangeError.prototype, made.constructor === RangeError]).toEqual([true, true, true, true]);
  expect([made.name, made.message, lib.shown(made)]).toEqual(["RangeError", "boom", "RangeError: boom"]);
  expect(Object.hasOwn(lib.make(undefined), "message")).toBe(false);
  expect(Object.hasOwn(lib.make(""), "message")).toBe(true);
  const cause = { why: 1 };
  expect(lib.caused("bad", cause).cause).toBe(cause);
  expect(lib.targetLength(4, 4)).toBe(4);
  expect(() => lib.targetLength(3, 4)).toThrow(new RangeError("invalid target buffer length"));
  try {
    lib.targetLength(3, 4);
  } catch (thrown) {
    expect(thrown).toBeInstanceOf(RangeError);
  }
  const classes = [EvalError, RangeError, ReferenceError, SyntaxError, TypeError, URIError];
  expect(lib.each().map((e: Error, i: number) => [Object.getPrototypeOf(e) === classes[i].prototype, e.name, e.message])).toEqual([
    [true, "EvalError", "eval"],
    [true, "RangeError", "range"],
    [true, "ReferenceError", "reference"],
    [true, "SyntaxError", "syntax"],
    [true, "TypeError", "type"],
    [true, "URIError", "uri"],
  ]);
});

// A caught `null` is `None` (ADR 0030): a binding given a `JsError`'s `None`
// is given no argument, as Rust says, so its default parameter is its, not
// `null`, though `JsError` is declared `unknown`, a value of any shape, as
// `js::Unknown` is, whose `None` is `undefined` alone (#1905).
test("a JsError's None caught from throw null is no argument to a binding", async () => {
  const withJs = ["--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("js-error-none-default");
  writeFileSync(join(dir, "lib.rs"), `use js::JsError;

#[cfg_attr(rust_js, rust_js::link_name = "globalThis.fallback")]
fn fallback(error: Option<&JsError>) -> String {
    unreachable!()
}

pub fn caught(source: &str) -> String {
    fallback(js::eval(source).err())
}

pub fn thrown(source: &str) -> Option<&'static JsError> {
    js::eval(source).err()
}
`);
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs]);
  expect(readFileSync(join(dir, "lib.d.ts"), "utf8")).toContain("export function thrown(source: string): unknown;");
  const g = globalThis as Record<string, unknown>;
  g.fallback = (error: unknown = "default") => String(error);
  const lib = await import(join(dir, "lib.js"));
  expect([lib.caught("throw null"), lib.caught("throw undefined"), lib.caught("throw new Error('x')")]).toEqual(["default", "default", "Error: x"]);
  expect([lib.thrown("throw null"), lib.thrown("throw undefined"), lib.thrown("1")]).toEqual([undefined, undefined, undefined]);
  delete g.fallback;
});

// A local each write of which is a `Some(..)` or a literal `None`, where a
// `Some`'s payload is a caught JS `null`, a `None` JS gave (ADR 0030): one
// value to each reader, `None`, as `.err()` gives it, never a `Some` to
// `is_some()` that `unwrap_or` takes for a `None`. Its test is `!= null`,
// not `!== undefined` (`mir_nones` asks `payload_may_be_null`, #2184).
test("a Some of a caught null is None to every reader of one local", async () => {
  const withJs = ["--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("js-error-some-of-caught-null");
  const held = "let held = match js::eval(source) {\n        Ok(_) => None,\n        Err(e) => Some(e),\n    };";
  writeFileSync(join(dir, "lib.rs"), `use js::{JsError, Unknown};

pub fn by_err(source: &str) -> bool {
    js::eval(source).err().is_some()
}

pub fn by_if_let(source: &str) -> bool {
    ${held}
    if let Some(_) = held { true } else { false }
}

pub fn by_is_some(source: &str) -> bool {
    ${held}
    held.is_some()
}

pub fn by_unwrap_or(source: &str, d: &'static JsError) -> Option<&'static Unknown> {
    ${held}
    held.unwrap_or(d).as_unknown()
}

pub fn by_match(source: &str, d: &'static JsError) -> Option<&'static Unknown> {
    ${held}
    let picked = match held {
        Some(e) => e,
        None => d,
    };
    picked.as_unknown()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs]);
  expect(readFileSync(join(dir, "lib.js"), "utf8")).not.toContain("held !== undefined");
  const lib = await import(join(dir, "lib.js"));
  const d = new Error("d");
  for (const source of ["throw null", "throw undefined", "1"]) {
    expect([source, lib.byErr(source), lib.byIfLet(source), lib.byIsSome(source), lib.byUnwrapOr(source, d), lib.byMatch(source, d)]).toEqual([source, false, false, false, d, d]);
  }
  const x = lib.byUnwrapOr("throw new Error('x')", d);
  expect([lib.byIfLet("throw new Error('x')"), lib.byIsSome("throw new Error('x')"), x instanceof Error && x.message]).toEqual([true, true, "x"]);
});

// #1939: a caught `null`, or `undefined`, is a `JsError`'s `None` (ADR
// 0030), and an export's `None` is `undefined` alone (#1840): of `.err()`,
// `.ok()`, `Option::from`, a match arm's `Some(e)`, a field written and a
// struct made, each `=== undefined` to a typed caller, as Rust's `None`
// is; an `Error` thrown is itself. Of the crate as a program and as a
// library alike.
test("a JsError's None JS gave leaves for JS as undefined, each way it's made", async () => {
  const withJs = ["--", "--crate-type=lib", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const source = `use js::JsError;

pub fn err(source: &str) -> Option<&'static JsError> {
    js::eval(source).err()
}

pub fn ok(source: &str) -> Option<&'static JsError> {
    let flipped: Result<&'static JsError, u8> = match js::eval(source) {
        Ok(_) => Err(0),
        Err(e) => Ok(e),
    };
    flipped.ok()
}

pub fn from(source: &str) -> Option<&'static JsError> {
    match js::eval(source) {
        Ok(_) => None,
        Err(e) => Option::from(e),
    }
}

pub fn arm(source: &str) -> Option<&'static JsError> {
    match js::eval(source) {
        Ok(_) => None,
        Err(e) => Some(e),
    }
}

pub struct Held {
    pub error: Option<&'static JsError>,
}

pub fn field(source: &str) -> Held {
    let mut held = Held { error: None };
    if let Err(e) = js::eval(source) {
        held.error = Some(e);
    }
    held
}

pub fn made(source: &str) -> Held {
    match js::eval(source) {
        Ok(_) => Held { error: None },
        Err(e) => Held { error: Some(e) },
    }
}
`;
  for (const library of [false, true]) {
    const dir = fixture(library ? "js-error-none-exits-library" : "js-error-none-exits");
    writeFileSync(join(dir, "lib.rs"), source);
    const flags = library ? ["--library", "--manifest", join(dir, "lib.manifest.json")] : [];
    run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...flags, ...withJs]);
    const lib = await import(join(dir, "lib.js"));
    const kinds = (f: (s: string) => unknown) =>
      ["throw null", "throw undefined", "throw new Error('x')"].map((s) => {
        const v = f(s);
        return v === undefined ? "undefined" : v === null ? "null" : String(v);
      });
    const given = [lib.err, lib.ok, lib.from, lib.arm, (s: string) => lib.field(s).error, (s: string) => lib.made(s).error];
    expect(given.map(kinds)).toEqual(Array(6).fill(["undefined", "undefined", "Error: x"]));
  }
});
