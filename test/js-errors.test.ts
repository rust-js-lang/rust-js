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
// anything, `any`. An `Error` is what may be thrown, by `Deref`.
test("an Error object is declared Error, and what's thrown any", async () => {
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
    error::new(message, None)
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
  expect(declarations).toContain("export function thrown(value: any): string;");
  expect(declarations).toContain("export function made(message: string): Error;");
  expect(declarations).toContain("export function all(): AggregateError;");
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('  const error = made("boom");');
  const lib = await import(join(dir, "lib.js"));
  expect(lib.named()).toBe("Error boom every one");
  expect(lib.made("x")).toBeInstanceOf(Error);
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
  expect(js).toContain("aggregateErrors($unwrap(internals.thrownErrors))");
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
