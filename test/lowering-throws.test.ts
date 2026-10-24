// `js::throw(value)` is JS's `throw value` (ADR 0760): the one way a
// program throws a value JS already has, an `Error` it was given, a
// promise, any value; `panic!` stays the way it throws a new `Error` (ADR
// 0706). What it throws unwinds as any JS throw. Each test is a program of
// its own, compiled and run in Node.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { runSync } from "./child";
import { node } from "./programs";
import { buildReact, compiler, fixture, root, run, target } from "./support";

beforeAll(buildReact, 600_000);

/** `rust` compiled in a fixture of `name`, against the js crate: its JS, and where it is. */
function compiled(name: string, rust: string): { js: string; dir: string } {
  const dir = fixture(name);
  writeFileSync(join(dir, "lib.rs"), rust);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  return { js: readFileSync(join(dir, "lib.js"), "utf8"), dir };
}

// React's `lazyInitializer` (packages/react/src/ReactLazy.js) ends `throw
// payload._result;`: what the loader rejected with, or the thenable it's
// waiting on, which Suspense catches. Its port throws the value it holds.
test("js::throw of React's lazy payload is throw payload._result", () => {
  const { js, dir } = compiled("throw-lazy", `use std::cell::Cell;

use js::Unknown;

#[derive(Clone, Copy, PartialEq)]
pub enum Status {
    Pending,
    Resolved,
}

pub struct Payload {
    #[cfg_attr(rust_js, rust_js::name = "_status")]
    pub status: Cell<Status>,
    #[cfg_attr(rust_js, rust_js::name = "_result")]
    pub result: Cell<Option<&'static Unknown>>,
}

pub fn lazy_initializer(payload: &Payload) -> Option<&'static Unknown> {
    if payload.status.get() == Status::Resolved {
        payload.result.get()
    } else {
        js::throw(payload.result.get())
    }
}

/// What's thrown of nothing, \`throw undefined\`.
pub fn throw_nothing() {
    js::throw(None);
}
`);
  expect(js).toContain(`export function lazyInitializer(payload) {
  if (payload._status !== "Resolved") {
    throw payload._result;
  }
  return payload._result;
}`);
  expect(js).toContain(`export function throwNothing() {
  throw undefined;
}`);
  // In Node: a thenable, a string, a number and an Error each thrown as
  // they are, the same value, not an Error made of it.
  writeFileSync(join(dir, "run.mjs"), `import { lazyInitializer, throwNothing } from "./lib.js";
const thenable = { then() {} };
const error = new RangeError("rejected");
for (const result of [thenable, "text", 5, error]) {
  try {
    lazyInitializer({ _status: "Pending", _result: result });
    console.log("returned");
  } catch (x) {
    console.log(x === result, typeof x, x instanceof Error);
  }
}
console.log(lazyInitializer({ _status: "Resolved", _result: 7 }));
try { throwNothing(); } catch (x) { console.log(x === undefined); }
`);
  const ran = runSync([node!, join(dir, "run.mjs")], root, 30_000);
  expect(ran.stderr).toBe("");
  expect(ran.stdout).toBe(
    "true object false\ntrue string false\ntrue number false\ntrue object true\n7\ntrue\n",
  );
});

// What `js::throw` threw unwinds as any JS throw (ADR 0710): `catch_unwind`
// catches it, its payload's `&JsError` downcast the value thrown, a
// non-Error's too, a `String` downcast none; and `resume_unwind` of the
// payload throws the same value again.
test("catch_unwind of js::throw downcasts as the value thrown", () => {
  const { js, dir } = compiled("throw-caught", `use std::panic::{catch_unwind, resume_unwind};

use js::{JsError, Unknown};

fn reject(reason: Option<&Unknown>) -> u32 {
    js::throw(reason)
}

/// What was thrown, as the \`&JsError\` its payload downcasts as.
pub fn caught(reason: Option<&'static Unknown>) -> Option<&'static JsError> {
    match catch_unwind(|| reject(reason)) {
        Ok(_) => None,
        Err(x) => x.downcast_ref::<&'static JsError>().copied(),
    }
}

/// Whether a panic's text is what's caught of it: none of a JS throw.
pub fn caught_text(reason: Option<&'static Unknown>) -> Option<String> {
    match catch_unwind(|| reject(reason)) {
        Ok(_) => None,
        Err(x) => x.downcast_ref::<String>().cloned(),
    }
}

/// Thrown again, as it was caught.
pub fn rethrown(reason: Option<&'static Unknown>) -> u32 {
    match catch_unwind(|| reject(reason)) {
        Ok(n) => n,
        Err(payload) => resume_unwind(payload),
    }
}
`);
  expect(js).toContain(`function reject(reason) {
  throw reason;
}`);
  writeFileSync(join(dir, "run.mjs"), `import { caught, caughtText, rethrown } from "./lib.js";
const thenable = { then() {} };
const error = new TypeError("t");
for (const reason of [thenable, "text", 5, error]) {
  console.log(caught(reason) === reason, caughtText(reason));
  try { rethrown(reason); } catch (x) { console.log(x === reason); }
}
`);
  const ran = runSync([node!, join(dir, "run.mjs")], root, 30_000);
  expect(ran.stderr).toBe("");
  expect(ran.stdout).toBe("true undefined\ntrue\n".repeat(4));
});
