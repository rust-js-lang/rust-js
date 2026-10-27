// What the compiler writes of an arrow that only passes on what it's given,
// by where it's given (ADR 0605, amended): a binding's `FnOnce` callback, an
// array method's callback, and what JS would give the function that the
// arrow leaves out. Each test is a program of its own, compiled and run.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildReact, compiler, fixture, run, target } from "./support";

beforeAll(buildReact, 600_000);

const externs = ["--", "--extern", `webapi=${join(target, "libwebapi.rmeta")}`, "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];

// ADR 0262's exception, the user's (2026-10-26): a closure given to a
// binding's `FnOnce` callback that only passes on its call to a function
// of the crate's is that function, `p.then(ping, ping)`. A callback JS
// calls once and never compares can't tell it from a fresh closure. An
// `Fn` parameter's, a listener's, keeps its arrow: `addEventListener`
// de-duplicates by identity.
test("a closure given to a binding's FnOnce callback that only passes its call on is the function", async () => {
  const dir = fixture("once-callbacks");
  writeFileSync(join(dir, "lib.rs"), `use js::Promise;

fn ping() {
    println!("ping");
}

fn count() -> u32 {
    7
}

fn shown(x: u32) -> u32 {
    x + 1
}

/// JS calls \`f\` once, with 5.
#[cfg_attr(rust_js, rust_js::link_name = "listenOnce")]
#[allow(unused_variables)]
fn listen_once(f: impl FnOnce(u32) -> u32 + 'static) {
    unreachable!()
}

/// JS keeps \`f\`, and may call it again.
#[cfg_attr(rust_js, rust_js::link_name = "listenEvery")]
#[allow(unused_variables)]
fn listen_every(f: impl Fn(u32) -> u32 + 'static) {
    unreachable!()
}

pub fn pinged(p: Promise<u32>) -> Promise<()> {
    p.then_with(|_| ping(), |_| ping())
}

pub fn counted(p: Promise<u32>) -> Promise<u32> {
    p.then_with(|_| count(), |_| count())
}

pub fn shown_of(p: Promise<u32>) -> Promise<u32> {
    p.then_with(|x| shown(x), |_| 0)
}

pub fn later() {
    js::set_timeout(Box::new(|| ping()), None);
    webapi::global::queue_microtask(Box::new(|| ping()));
}

pub fn listened() {
    listen_once(|_| count());
    listen_every(|_| count());
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...externs]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("return p.then(ping, ping);");
  expect(js).toContain("return p.then(count, count);");
  expect(js).toContain("return p.then(shown, () => 0);");
  expect(js).toContain("  setTimeout(ping);\n  queueMicrotask(ping);\n");
  expect(js).toContain("  listenOnce(count);\n  listenEvery(() => count());\n");
  const given: unknown[] = [];
  Object.assign(globalThis, {
    listenOnce: (f: (x: number) => number) => given.push(f(5)),
    listenEvery: (f: (x: number) => number) => given.push(f(5)),
  });
  const lib = await import(join(dir, "lib.js"));
  lib.listened();
  expect(given).toEqual([7, 7]);
  expect(await Promise.all([lib.counted(Promise.resolve(1)), lib.counted(Promise.reject(new Error("no"))), lib.shownOf(Promise.resolve(1))])).toEqual([7, 7, 2]);
  expect(await lib.pinged(Promise.resolve(1))).toBeUndefined();
});

// What a once-callback is the function of, and what not: a function whose
// value the closure gives, or one that gives `undefined` anyway. Not one
// whose value it drops, `|_| { count(); }`, as `then` would fulfil with
// it; nor a binding's JS function, a generic `impl Fn` or a `dyn Fn`,
// which may be a JS function that reads what the callback is given; nor a
// function of rest arguments, which reads them all. An array method's
// callback of a `dyn Fn` keeps its arrow too, as the method gives the
// index and the array (ADR 0279).
test("a once-callback is the function only where the function would see and give what the closure does", async () => {
  const dir = fixture("once-callbacks-kept");
  writeFileSync(join(dir, "lib.rs"), `use js::Promise;

fn count() -> u32 {
    7
}

fn many(#[cfg_attr(rust_js, rust_js::rest)] xs: &[u32]) -> u32 {
    xs.len() as u32
}

#[cfg_attr(rust_js, rust_js::link_name = "Number")]
#[allow(unused_variables)]
fn number() -> u32 {
    unreachable!()
}

pub fn dropped(p: Promise<u32>) -> Promise<()> {
    p.then_with(
        |_| {
            count();
        },
        |_| {},
    )
}

pub fn bound(p: Promise<u32>) -> Promise<u32> {
    p.then_with(|_| number(), |_| 0)
}

pub fn generic(p: Promise<u32>, f: impl Fn() -> u32 + 'static) -> Promise<u32> {
    p.then_with(move |_| f(), |_| 0)
}

pub fn dynamic(p: Promise<u32>, f: Box<dyn Fn() -> u32>) -> Promise<u32> {
    p.then_with(move |_| f(), |_| 0)
}

pub fn rest(p: Promise<u32>) -> Promise<u32> {
    p.then_with(|x| many(&[x]), |_| 0)
}

pub fn rest_mapped(xs: &[u32]) -> Vec<u32> {
    xs.iter().map(|x| many(&[*x])).collect()
}

pub fn dyn_mapped(xs: &[u32], f: &dyn Fn(&u32) -> u32) -> Vec<u32> {
    xs.iter().map(|x| f(x)).collect()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...externs]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("() => {\n      count();\n    },");
  expect(js).toContain("() => Number(),");
  expect(js).toContain("() => f(),");
  expect(js).toContain("(x) => many(x),");
  expect(js).toContain("return xs.map((x) => many(x));");
  expect(js).toContain("return xs.map((x) => f(x));");
  const lib = await import(join(dir, "lib.js"));
  const reads = (...args: unknown[]) => args.length;
  expect(await Promise.all([
    lib.dropped(Promise.resolve(1)),
    lib.bound(Promise.resolve(1)),
    lib.generic(Promise.resolve(1), reads),
    lib.dynamic(Promise.resolve(1), reads),
    lib.rest(Promise.resolve(1)),
  ])).toEqual([undefined, 0, 0, 0, 1]);
  expect([lib.restMapped([4, 5]), lib.dynMapped([4, 5], reads)]).toEqual([[1, 1], [1, 1]]);
});

// ADR 0605, amended: React's runWithFiberInDEV(fiber, commitCallbacks,
// updateQueue, instance), the crate's own function of an `impl FnOnce(A0,
// .., A4)` it calls with the arguments after it. A closure given to it that
// only passes its call on, its `()`s left out, is the function; one given to
// an `impl Fn`'s, which the function may call again and keep, stays.
test("a closure given to the crate's FnOnce callback that only passes its call on is the function", async () => {
  const dir = fixture("crate-once-callbacks");
  writeFileSync(join(dir, "lib.rs"), `pub fn run_with<A0, A1, A2, T>(callback: impl FnOnce(A0, A1, A2) -> T, arg0: A0, arg1: A1, arg2: A2) -> T {
    callback(arg0, arg1, arg2)
}

pub fn twice<T>(callback: impl Fn(T) -> T, value: T) -> T {
    callback(callback(value))
}

fn commit(queue: &js::Unknown, value: u32) {
    js::set(queue, "value", value);
}

fn doubled(value: u32) -> u32 {
    value * 2
}

pub fn callbacks(queue: &js::Unknown, value: u32) -> u32 {
    run_with(|queue: &js::Unknown, value: u32, _: ()| commit(queue, value), queue, value, ());
    twice(|value| doubled(value), value)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...externs]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("  runWith(commit, queue, value);\n");
  expect(js).toContain("  return twice((value) => doubled(value), value);\n");
  const lib = await import(join(dir, "lib.js"));
  const queue: Record<string, number> = {};
  expect([lib.callbacks(queue, 3), queue]).toEqual([12, { value: 3 }]);
});

// React's ContextOnlyDispatcher gives each hook one function, `useCallback:
// throwInvalidHookError, useState: throwInvalidHookError, ..`, of whatever
// arity; ported, each is a closure, `use_callback: |_, _|
// throw_invalid_hook_error()`, as Rust can't make a `fn()` a `fn(A, B)`. A
// closure that captures nothing as a `fn`, that only calls a function of
// the crate's with what it's given, or none of it, is that function: Rust
// compares no two `fn`s reliably, and what JS gives it more, the function
// never reads (ADR 0605, amended). One of a function Rust could name there,
// of the closure's own signature, is a function of its own as written; one
// that keeps its call's result back, or calls a binding, whose JS function
// may read more, stays an arrow.
test("a closure as a fn that only calls the crate's function with what it's given is the function", async () => {
  const dir = fixture("fn-pointer-passed-on");
  writeFileSync(join(dir, "lib.rs"), `pub struct Dispatcher {
    pub use_callback: fn(u32, u32) -> u32,
    pub use_id: fn() -> u32,
    pub use_state: fn(u32) -> u32,
    pub use_log: fn(u32),
}

fn throw_invalid_hook_error() -> ! {
    panic!("Invalid hook call.")
}

fn first(a: u32) -> u32 {
    a
}

pub static CONTEXT_ONLY: Dispatcher = Dispatcher {
    use_callback: |_, _| throw_invalid_hook_error(),
    use_id: || throw_invalid_hook_error(),
    use_state: |_| throw_invalid_hook_error(),
    use_log: |_| throw_invalid_hook_error(),
};

pub static RUNNING: Dispatcher = Dispatcher {
    use_callback: |a, _| first(a),
    use_id: || 3,
    use_state: |a| first(a) + 1,
    use_log: |a| {
        first(a);
    },
};

// A function Rust could name here, wrapped as upstream's DEV dispatchers
// wrap theirs, \`readContext(context) { return readContext(context); }\`:
// a function of its own, as written.
pub static WRAPPED: Dispatcher = Dispatcher {
    use_callback: |a, _| first(a),
    use_id: || 3,
    use_state: |a| first(a),
    use_log: |_| throw_invalid_hook_error(),
};

pub fn call(dispatcher: &Dispatcher) -> (u32, u32, u32) {
    (dispatcher.use_callback)(1, 2);
    (dispatcher.use_log)(9);
    ((dispatcher.use_callback)(4, 5), (dispatcher.use_id)(), (dispatcher.use_state)(6))
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...externs]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain(`export const CONTEXT_ONLY = {
  useCallback: throwInvalidHookError,
  useId: throwInvalidHookError,
  useState: throwInvalidHookError,
  useLog: throwInvalidHookError,
};`);
  expect(js).toContain("useCallback: first,");
  expect(js).toContain("useState: (a) => (first(a) + 1) >>> 0,");
  expect(js).toContain("useState: (a) => first(a),");
  const lib = await import(join(dir, "lib.js"));
  expect(lib.call(lib.RUNNING)).toEqual([4, 3, 7]);
  expect(lib.RUNNING.useLog(9)).toBe(undefined);
  expect(() => lib.call(lib.CONTEXT_ONLY)).toThrow("Invalid hook call.");
});

// React renders TransitionAwareHostComponent as a component, a function of
// no parameters where a component of two goes; ported, `render(&|_, _|
// transition_aware_host_component())`. A closure that captures nothing,
// lent as a `&dyn Fn`, that only adapts a function of the crate's, is the
// function, as one as a `fn` is: the same component each render, as
// upstream's. One an `Rc` owns has a pointer of its own, which
// `Rc::ptr_eq` compares, and stays its arrow (ADR 0605, amended).
test("a closure that captures nothing lent as a dyn Fn that only adapts the crate's function is the function", async () => {
  const dir = fixture("dyn-fn-passed-on");
  writeFileSync(join(dir, "lib.rs"), `use std::rc::Rc;

pub type Component = dyn Fn(u32, u32) -> u32;

fn render(component: &Component) -> u32 {
    component(1, 2)
}

pub fn transition_aware_host_component() -> u32 {
    7
}

pub fn render_transition_aware() -> u32 {
    render(&|_, _| transition_aware_host_component())
}

pub fn owned_apart() -> bool {
    let a: Rc<Component> = Rc::new(|_, _| transition_aware_host_component());
    let b: Rc<Component> = Rc::new(|_, _| transition_aware_host_component());
    Rc::ptr_eq(&a, &b) || a(1, 2) != b(1, 2)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...externs]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("return render(transitionAwareHostComponent);");
  const lib = await import(join(dir, "lib.js"));
  expect(lib.renderTransitionAware()).toBe(7);
  expect(lib.ownedApart()).toBe(false);
});
