// What the compiler writes of an arrow that only passes on what it's given,
// by where it's given (ADR 0605, amended): a binding's `FnOnce` callback, an
// array method's callback, and what JS would give the function that the
// arrow leaves out. Each test is a program of its own, compiled and run.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { chromium } from "playwright";
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

/// What a trait of the program's takes, which says nothing of how JS calls
/// it: a union's \`Function\` member is one of a binding's
/// \`#[rust_js::types]\` alone (ADR 1020).
pub trait Kept {}
impl<F: FnOnce() -> u32 + 'static> Kept for F {}

#[cfg_attr(rust_js, rust_js::link_name = "keepAny")]
#[allow(unused_variables)]
fn keep_any<K: Kept>(k: K) {
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
    js::set_timeout(|| ping(), None);
    js::set_timeout(Box::new(|| ping()), None);
    webapi::global::queue_microtask(Box::new(|| ping()));
}

pub fn listened() {
    listen_once(|_| count());
    listen_every(|_| count());
    keep_any(|| count());
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...externs]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("return p.then(ping, ping);");
  expect(js).toContain("return p.then(count, count);");
  expect(js).toContain("return p.then(shown, () => 0);");
  expect(js).toContain("  setTimeout(ping);\n  setTimeout(ping);\n  queueMicrotask(ping);\n");
  expect(js).toContain("  listenOnce(count);\n  listenEvery(() => count());\n  keepAny(() => count());\n");
  const given: unknown[] = [];
  Object.assign(globalThis, {
    listenOnce: (f: (x: number) => number) => given.push(f(5)),
    listenEvery: (f: (x: number) => number) => given.push(f(5)),
    keepAny: (f: () => number) => given.push(f()),
  });
  const lib = await import(join(dir, "lib.js"));
  lib.listened();
  expect(given).toEqual([7, 7, 7]);
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
// only passes its call on, its `()`s left out, is the function, in a
// closure too, React's update.callback (gap ce4); one given to an `impl
// Fn`'s, which the function may call again and keep, stays.
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

pub fn later(queue: std::rc::Rc<&'static js::Unknown>, value: u32) -> impl Fn() {
    move || {
        run_with(|queue: &js::Unknown, value: u32, _: ()| commit(queue, value), &**queue, value, ());
    }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...externs]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("  runWith(commit, queue, value);\n");
  expect(js).toContain("  return twice((value) => doubled(value), value);\n");
  // Inside a closure, of what it captured: as it is.
  expect(js).toContain("    runWith(commit, queue, value);\n  };\n");
  const lib = await import(join(dir, "lib.js"));
  const queue: Record<string, number> = {};
  expect([lib.callbacks(queue, 3), queue]).toEqual([12, { value: 3 }]);
  lib.later(queue, 5)();
  expect(queue).toEqual({ value: 5 });
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
  expect(js).toContain("  useState(a) {\n    return (first(a) + 1) >>> 0;\n  },");
  expect(js).toContain("useState: (a) => first(a),");
  const lib = await import(join(dir, "lib.js"));
  expect(lib.call(lib.RUNNING)).toEqual([4, 3, 7]);
  expect(lib.RUNNING.useLog(9)).toBe(undefined);
  expect(() => lib.call(lib.CONTEXT_ONLY)).toThrow("Invalid hook call.");
});

// React's ReactFiberConfigWithNoPersistence exports `function shim(...args)`
// as each function the renderer lacks, `export const cloneInstance = shim;`,
// which the reconciler calls with that function's arguments. Ported, a `fn`
// of rest arguments as a value, `fn(&dyn Rest<dyn IntoUnknown>) -> !`,
// whose closure passes them on to the rest function, `|args| shim(args)`:
// `(...args) => shim(...args)`, which gathers what JS gives and spreads all
// of it, is `shim` (ADR 0605, amended). A closure of one array that spreads
// it to a rest function, `(x) => shim(...x)`, takes one value, and a rest
// function given less than all of them, `(...args) => shim(...$slice(args,
// 1))`, reads another: each stays its arrow. And an array method's callback
// that calls one with its value, `xs.map(|x| total(&[x]))`, stays
// `(x) => total(x)`: `map` gives the index too, which a rest reads.
test("a closure as a fn of rest arguments that passes them all on to a rest function is the function", async () => {
  const dir = fixture("fn-pointer-rest-passed-on");
  writeFileSync(join(dir, "lib.rs"), `use js::{IntoUnknown, Rest, Unknown};

fn shim(#[cfg_attr(rust_js, rust_js::rest)] args: &[Option<&Unknown>]) -> ! {
    panic!("not supported: {}", args.len())
}

fn total(#[cfg_attr(rust_js, rust_js::rest)] xs: &[f64]) -> f64 {
    xs.iter().sum()
}

/// A function this renderer hasn't: it throws, whatever it's given.
type Shim = fn(&dyn Rest<dyn IntoUnknown>) -> !;

#[cfg_attr(rust_js, rust_js::name = "cloneInstance")]
pub const CLONE_INSTANCE: Shim = |args| shim(args);
#[cfg_attr(rust_js, rust_js::name = "afterFirst")]
pub const AFTER_FIRST: Shim = |args| shim(&args[1..]);
#[cfg_attr(rust_js, rust_js::name = "ofOne")]
pub const OF_ONE: fn(&[Option<&Unknown>]) -> ! = |args| shim(args);

pub fn call(x: &Unknown) {
    CLONE_INSTANCE(&(Some(x), 1.0));
}

pub fn totals(xs: &[f64]) -> Vec<f64> {
    xs.iter().map(|&x| total(&[x])).collect()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...externs]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("export const cloneInstance = shim;\n");
  expect(js).toContain("export const afterFirst = (...args) => {\n  shim(...$slice(args, 1));\n};");
  expect(js).toContain("export const ofOne = (args) => {\n  shim(...args);\n};");
  expect(js).toContain("xs.map((x) => total(x))");
  const lib = await import(join(dir, "lib.js"));
  expect(() => lib.cloneInstance({}, "div", {})).toThrow("not supported: 3");
  expect(() => lib.afterFirst({}, "div", {})).toThrow("not supported: 2");
  expect(() => lib.ofOne([{}, "div"])).toThrow("not supported: 2");
  expect(() => lib.call({})).toThrow("not supported: 2");
  expect(lib.totals([1, 2])).toEqual([1, 2]);
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

// React's mountTransition, `const start = startTransition.bind(null,
// currentlyRenderingFiber, queue, true, false);`, ported as a `move`
// closure that passes what it captured, then its own parameters, to the
// crate's function: that function bound to what it captured, which a
// `bind` reads once, where it's made, as a `move` closure does (ADR 0605,
// amended). Not of a closure that borrows what it passes, one of a value
// read when it's called, one with no parameters of its own, nor one that
// calls a closure.
test("a move closure that passes on what it captured, then what it's given, is the function bound", async () => {
  const dir = fixture("bound-captures");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::Cell;
use std::rc::Rc;

pub struct Fiber {
    pub name: String,
}

fn start_transition(fiber: &Rc<Fiber>, pending: bool, callback: &dyn Fn(), options: Option<&str>) {
    println!("{} {} {}", fiber.name, pending, options.unwrap_or("-"));
    callback();
}

pub type StartTransition = dyn Fn(&dyn Fn(), Option<&str>);

pub fn mount_transition(fiber: Rc<Fiber>) -> Rc<StartTransition> {
    Rc::new(move |callback, options| start_transition(&fiber, true, callback, options))
}

thread_local! {
    static PENDING: Cell<bool> = const { Cell::new(false) };
}

pub fn read_when_called(fiber: Rc<Fiber>) -> Rc<StartTransition> {
    Rc::new(move |callback, options| start_transition(&fiber, PENDING.get(), callback, options))
}

pub fn borrowed(fiber: Rc<Fiber>) {
    let start = |callback: &dyn Fn(), options| start_transition(&fiber, false, callback, options);
    start(&|| println!("lent"), Some("x"));
}

fn shown(fiber: &Rc<Fiber>) -> String {
    fiber.name.clone()
}

pub fn thunk(fiber: Rc<Fiber>) -> Rc<dyn Fn() -> String> {
    Rc::new(move || shown(&fiber))
}

pub fn of_a_closure(fiber: Rc<Fiber>) -> Rc<StartTransition> {
    let start = |fiber: &Rc<Fiber>, pending: bool, callback: &dyn Fn(), options: Option<&str>| {
        start_transition(fiber, pending, callback, options)
    };
    Rc::new(move |callback, options| start(&fiber, true, callback, options))
}

pub fn run(start: &StartTransition) {
    start(&|| println!("called"), Some("opts"));
    start(&|| println!("again"), None);
}

pub fn set_pending() {
    PENDING.set(true);
}

fn tagged(tag: &str, x: u32) -> String {
    format!("{tag} {x}")
}

pub fn tags(xs: Vec<u32>) -> Vec<String> {
    xs.into_iter().map(|x| tagged("take", x)).collect()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("return startTransition.bind(null, fiber, true);");
  expect(js).toContain("(callback, options) => startTransition(fiber, pending, callback, options)");
  expect(js).toContain("(callback, options) => startTransition(fiber, false, callback, options)");
  expect(js).toContain("return () => shown(fiber);");
  expect(js).toContain('(x) => tagged("take", x)');
  expect(js).toContain("(callback, options) => start(fiber, true, callback, options)");
  const lib = await import(join(dir, "lib.js"));
  const logs: string[] = [];
  const log = console.log;
  console.log = (line: string) => logs.push(line);
  try {
    const fiber = { name: "root" };
    const later = lib.readWhenCalled(fiber);
    lib.run(lib.mountTransition(fiber));
    lib.setPending();
    lib.run(later);
    lib.run(lib.ofAClosure(fiber));
    expect(lib.thunk(fiber)()).toBe("root");
    lib.borrowed(fiber);
    expect(lib.tags([1, 2])).toEqual(["take 1", "take 2"]);
  } finally {
    console.log = log;
  }
  expect(logs).toEqual([
    "root true opts", "called", "root true -", "again",
    "root true opts", "called", "root true -", "again",
    "root true opts", "called", "root true -", "again",
    "root false x", "lent",
  ]);
});

// #427: a host function held as a \`fn\` in a field, called through it, is
// given no \`this\`, which a browser's own refuses of another, "Illegal
// invocation": its callee held first, \`const schedule = CONFIG.schedule;
// schedule(..)\`, as a with_this function's is (#402, \`called_unbound\`).
// One the crate's functions alone may be is called through its field.
// A \`Cell<fn()>\` field the crate sets of its own functions alone is called
// through it, as React's \`ReactStrictModeWarnings.discardPendingWarnings()\`;
// one it sets of a host function is held first (#427).
let hostFieldJs: string | undefined;
const compileHostField = (): string => {
  if (hostFieldJs !== undefined) return hostFieldJs;
  const dir = fixture("fn-pointer-field-host");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::Cell;

pub struct Hooks {
    pub schedule: Cell<fn(Box<dyn FnOnce()>)>,
    pub flush: Cell<fn()>,
    pub ended: Cell<fn()>,
}

thread_local! {
    pub static HOOKS: Hooks = Hooks {
        schedule: Cell::new(|_| {}),
        flush: Cell::new(|| {}),
        ended: Cell::new(|| {}),
    };
}

fn lend(cell: &Cell<fn()>) {
    cell.set(webapi::console::group_end);
}

pub fn hooked(n: u32) {
    HOOKS.with(|hooks| {
        hooks.schedule.set(webapi::global::queue_microtask);
        hooks.flush.set(|| record(0));
        lend(&hooks.ended);
        (hooks.schedule.get())(Box::new(move || record(n)));
        (hooks.flush.get())();
        (hooks.ended.get())();
    });
}

pub struct Config {
    pub schedule: fn(Box<dyn FnOnce()>),
    pub count: fn(u32) -> u32,
}

fn twice(n: u32) -> u32 {
    n * 2
}

pub static CONFIG: Config = Config { schedule: webapi::global::queue_microtask, count: twice };

pub fn run(n: u32) -> u32 {
    (CONFIG.schedule)(Box::new(move || record(n)));
    (CONFIG.count)(n)
}

#[cfg_attr(rust_js, rust_js::link_name = "globalThis.record")]
fn record(n: u32) {
    unreachable!()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...externs]);
  hostFieldJs = readFileSync(join(dir, "lib.js"), "utf8");
  return hostFieldJs;
};

// Its JS, apart from the browser that runs it: what the mutations check,
// whose machines have none (#427).
test("a fn field that may hold a host function is held first, its own called through it", () => {
  const js = compileHostField();
  expect(js).toContain("  const schedule = CONFIG.schedule;\n  schedule(() => {");
  expect(js).toContain("return CONFIG.count(n);");
  expect(js).toContain("  const schedule = hooks.schedule;\n  schedule(() => {");
  expect(js).toContain("  hooks.flush();\n  const ended = hooks.ended;\n  ended();\n");
});

test("a fn field that may hold a host function is called with no this, in Chromium", async () => {
  const js = compileHostField();
  const browser = await chromium.launch({ headless: true });
  try {
    const page = await browser.newPage();
    const result = await page.evaluate(async (js) => {
      const seen: number[] = [];
      Object.assign(globalThis, { record: (n: number) => seen.push(n) });
      const url = URL.createObjectURL(new Blob([js], { type: "text/javascript" }));
      const lib = await import(url);
      const counted = lib.run(3);
      lib.hooked(5);
      await new Promise((resolve) => setTimeout(resolve, 0));
      return [counted, seen];
    }, js);
    expect(result).toEqual([6, [0, 3, 5]]);
  } finally {
    await browser.close();
  }
}, 60_000);

// A \`Cell<fn()>\` field any other method of may write, \`swap\`'s, is held
// first: what it's set to isn't seen (#427).
test("a fn field a cell's other method may write is called with no this", () => {
  const dir = fixture("fn-pointer-field-swapped");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::Cell;

pub struct Hooks {
    pub swapped: Cell<fn()>,
}

pub fn swapped(hooks: &Hooks) {
    hooks.swapped.swap(&Cell::new(webapi::console::group_end));
    (hooks.swapped.get())();
}

pub fn made() -> Hooks {
    Hooks { swapped: Cell::new(|| {}) }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...externs]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("  const swapped = hooks.swapped;\n  swapped();\n");
});

// React DOM's scheduleMicrotask and localRequestAnimationFrame hold a host
// function itself, `queueMicrotask : scheduleTimeout`. Ported, a closure as
// a `fn` that only calls one host function with what it's given, in order,
// its optional ones `None`, `|callback| { js::set_timeout(callback, None);
// }`, is that function, `setTimeout`, as the user decided (rd158): a JS
// caller of an exported value sees what it gives, setTimeout's id, and may
// give it more, its delay (ADR 0262's exception). A method, which reads its
// `this`, and a listener's stay arrows.
test("a closure as a fn that only calls one host function with what it's given is that function", async () => {
  const dir = fixture("fn-pointer-host-passed-on");
  writeFileSync(join(dir, "lib.rs"), `type Schedule = fn(Box<dyn FnOnce()>);

pub static SCHEDULE_TIMEOUT: Schedule = |callback| {
    js::set_timeout(callback, None);
};

pub static SCHEDULE_MICROTASK: Schedule = webapi::global::queue_microtask;

pub static SCHEDULE_LOGGED: Schedule = |callback| {
    record(0);
    js::set_timeout(callback, None);
};

pub fn schedule(schedule: Schedule, n: u32) {
    schedule(Box::new(move || record(n)));
}

#[cfg_attr(rust_js, rust_js::link_name = "globalThis.record")]
fn record(n: u32) {
    unreachable!()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...externs]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("export const SCHEDULE_TIMEOUT = setTimeout;");
  expect(js).toContain("export const SCHEDULE_MICROTASK = queueMicrotask;");
  expect(js).toContain("export const SCHEDULE_LOGGED = (callback) => {");
  const seen: number[] = [];
  (globalThis as Record<string, unknown>).record = (n: number) => void seen.push(n);
  const lib = await import(join(dir, "lib.js"));
  lib.schedule(lib.SCHEDULE_TIMEOUT, 1);
  lib.schedule(lib.SCHEDULE_MICROTASK, 2);
  await new Promise((resolve) => setTimeout(resolve, 5));
  expect(seen).toEqual([2, 1]);
});
