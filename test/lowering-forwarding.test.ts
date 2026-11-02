// What the compiler writes of a function that only calls a module
// variable's function with its parameters (ADR 0610): that variable, by
// the function's name; and of one that only passes them on to another
// module's function, that function re-exported (ADR 1064). Each test is a program of its own, compiled and
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

// A function that only passes its parameters on to another module's
// function is that function, re-exported by its own name or its
// `rust_js::name`, which another module imports from it as before, and
// declared as written (rd575, ADR 1064). One its own module calls, which
// has no name there for a re-export, stays a function; so do a module's
// default, one its module re-exports by another name, one kept out of
// line, one with a comment in it, one of its own module's function, a
// method's or a binding's, and one whose function's JS would take more
// than its callers give it: what JS gives past its parameters, a drop
// left out or JS's `arguments`, or a copy that boxes what it's given.
test("a function that only passes its parameters on to another module's is its re-export", async () => {
  const dir = fixture("reexported-forwarders");
  writeFileSync(join(dir, "lib.rs"), `mod bound;
mod caller;
mod inner;

use js::Unknown;

pub fn helper(n: f64) -> f64 {
    inner::helper(n)
}

#[cfg_attr(rust_js, rust_js::name = "triple")]
pub fn tripled(n: f64) -> f64 {
    inner::helper(n)
}

pub fn called_here(n: f64) -> f64 {
    inner::helper(n)
}

pub fn twice(n: f64) -> f64 {
    called_here(n) * 2.0
}

pub fn renamed(n: f64) -> f64 {
    inner::helper(n)
}

pub use self::renamed as other_name;

#[inline(never)]
pub fn apart(n: f64) -> f64 {
    inner::helper(n)
}

pub fn commented(n: f64) -> f64 {
    // Passed on.
    inner::helper(n)
}

pub fn defaulted(n: f64) -> f64 {
    inner::helper(n)
}

js::export_default!(defaulted);

pub fn plus_one(n: f64) -> f64 {
    caller::plus_one(n)
}

fn own(n: f64) -> f64 {
    n + 2.0
}

pub fn twin(n: f64) -> f64 {
    own(n)
}

pub fn count(counter: &inner::Counter) -> f64 {
    counter.get()
}

pub fn halved(n: f64) -> f64 {
    bound::half(n)
}

pub fn held(n: u32) -> u32 {
    inner::hold(n)
}

pub struct Guard(pub u32);

impl Drop for Guard {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

pub fn guarded(guard: Guard) -> u32 {
    inner::hold(guard)
}

pub fn counted(first: Option<&Unknown>) -> Option<&'static Unknown> {
    inner::given(first)
}

pub fn nested(x: Option<Option<u32>>) -> bool {
    inner::some(x)
}
`);
  writeFileSync(join(dir, "inner.rs"), `use js::Unknown;

pub fn helper(n: f64) -> f64 {
    n * 3.0
}

pub struct Counter {
    pub n: f64,
}

impl Counter {
    pub fn get(&self) -> f64 {
        self.n
    }
}

pub fn hold<T>(value: T) -> u32 {
    drop(value);
    1
}

pub fn given(_first: Option<&Unknown>) -> Option<&'static Unknown> {
    js::get(js::arguments(), "length")
}

pub fn some<T>(x: Option<T>) -> bool {
    x.is_some()
}
`);
  writeFileSync(join(dir, "caller.rs"), "pub fn plus_one(n: f64) -> f64 {\n    crate::helper(n) + 1.0\n}\n");
  writeFileSync(join(dir, "bound.rs"), '#[cfg_attr(rust_js, rust_js::link_name = "./math.js#half")]\npub fn half(n: f64) -> f64 {\n    let _ = n;\n    unreachable!()\n}\n');
  writeFileSync(join(dir, "math.js"), "export function half(n, by = 2) {\n  return n / by;\n}\n");
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('export { helper, helper as triple } from "./inner.js";\n');
  expect(js).toContain('export { plusOne } from "./caller.js";\n');
  for (const kept of ["calledHere(n)", "twice(n)", "apart(n)", "commented(n)", "twin(n)", "count(counter)", "halved(n)", "held(n)", "counted(first)", "nested(x)"]) {
    expect(js).toContain(`export function ${kept} {`);
  }
  expect(js).toContain("function renamed(n) {");
  expect(js).toContain("function defaulted(n) {");
  expect(js).not.toMatch(/function (helper|tripled|triple|plusOne)\b/);
  expect(readFileSync(join(dir, "caller.js"), "utf8")).toContain('import { helper } from "./lib.js";\n');
  const dts = readFileSync(join(dir, "lib.d.ts"), "utf8");
  expect(dts).toContain("export function helper(n: number): number;");
  expect(dts).toContain("export function triple(n: number): number;");
  expect(dts).not.toContain('export { helper');
  const lib = await import(join(dir, "lib.js"));
  const inner = await import(join(dir, "inner.js"));
  expect([lib.helper, lib.triple, lib.plusOne(2)]).toEqual([inner.helper, inner.helper, 7]);
  expect([lib.calledHere(1), lib.twice(1), lib.otherName(1), lib.apart(1), lib.commented(1), lib.default(1)]).toEqual([3, 6, 3, 3, 3, 3]);
  // What a JS caller gives past their parameters, none of theirs.
  expect([lib.twin(1), lib.count({ n: 4 }), lib.halved(8, 4), lib.held(5, 6), lib.counted(), lib.nested({ value: undefined })]).toEqual([3, 4, 4, 1, 1, true]);
});
