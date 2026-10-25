// What the compiler writes of a function that only calls a module
// variable's function with its parameters (ADR 0610): that variable, by
// the function's name. Each test is a program of its own, compiled and
// run.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildReact, compiler, fixture, run, target } from "./support";

beforeAll(buildReact, 600_000);

const flags = ["--", "--extern", `js=${join(target, "libjs.rmeta")}`, "--extern", `webapi=${join(target, "libwebapi.rmeta")}`, "-L", target];

// ADR 0610, amended: React's ReactAct has `const queueSeveralMicrotasks =
// typeof queueMicrotask === 'function' ? (callback) => { .. } :
// queueMacrotask;`, a value made where the module loads, another module's
// function one way. Its port's thread-local `fn` and the function that
// calls it are that `const`: the function's only called, by its
// parameters, so none gives it more. Not one passed as a value, nor a
// `pub` one, which JS may call otherwise.
test("a private function of no value that only calls a thread-local fn is the variable", async () => {
  const dir = fixture("forwarding-unit-functions");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::Cell;

mod tasks {
    pub fn enqueue_task(task: Box<dyn Fn()>) {
        task();
    }
}

use tasks::enqueue_task as queue_macrotask;

thread_local! {
    static QUEUE_SEVERAL_MICROTASKS: fn(Box<dyn Fn()>) =
        if js::is_defined(webapi::global::queue_microtask) {
            |callback| {
                webapi::global::queue_microtask(Box::new(move || {
                    webapi::global::queue_microtask(callback);
                }));
            }
        } else {
            queue_macrotask
        };
    static RUN: fn(Box<dyn Fn()>) = if js::math::random() < 2.0 { queue_macrotask } else { |_| {} };
    static COUNT: Cell<u32> = const { Cell::new(0) };
}

fn queue_several_microtasks(callback: Box<dyn Fn()>) {
    QUEUE_SEVERAL_MICROTASKS.with(|f| f(callback));
}

fn run_now(callback: Box<dyn Fn()>) {
    RUN.with(|f| f(callback));
}

pub fn run_given(callback: Box<dyn Fn()>) {
    RUN.with(|f| f(callback));
}

fn run_read(callback: Box<dyn Fn()>) {
    RUN.with(|f| f(callback));
}

#[cfg_attr(rust_js, rust_js::name = "runLater")]
pub use self::run_aliased as run_later;

pub fn run_aliased(callback: Box<dyn Fn()>) {
    RUN.with(|f| f(callback));
}

pub fn later() -> Box<dyn Fn()> {
    Box::new(|| run_read(Box::new(|| COUNT.set(COUNT.get() + 1000))))
}

pub fn count() -> u32 {
    queue_several_microtasks(Box::new(|| COUNT.set(COUNT.get() + 1)));
    run_now(Box::new(|| COUNT.set(COUNT.get() + 10)));
    let given: fn(Box<dyn Fn()>) = run_now;
    given(Box::new(|| COUNT.set(COUNT.get() + 100)));
    COUNT.get()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...flags]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain(`const queueSeveralMicrotasks =
  typeof queueMicrotask !== "undefined"
    ? (callback) => {`);
  expect(js).toContain("    : queueMacrotask;\n");
  expect(js).not.toContain("function queueSeveralMicrotasks");
  expect(js).not.toContain("QUEUE_SEVERAL_MICROTASKS");
  // One a closure calls, the variable too; passed as a value, exported,
  // and exported by another name, functions still.
  expect(js).toContain("    runRead(() => {\n");
  expect(js).not.toContain("function runRead");
  expect(js).toContain("\nfunction runNow(callback) {\n  runRead(callback);\n}");
  expect(js).toContain("\nexport function runGiven(callback) {\n  runRead(callback);\n}");
  expect(js).toContain("\nfunction runAliased(callback) {\n  runRead(callback);\n}");
  const lib = await import(join(dir, "lib.js"));
  lib.later()();
  expect(lib.count()).toBe(1110);
  await new Promise((resolve) => setTimeout(resolve, 0));
  expect(lib.count()).toBe(1221);
});
