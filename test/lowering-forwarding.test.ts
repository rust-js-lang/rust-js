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

// ADR 0610, amended (#533): alien-signals builds its system where its
// module loads and takes it apart, `const { link, unlink, .. } =
// createReactiveSystem(..)`. Its port's thread-local of the system and the
// private functions that each only call one of its fields are that
// destructuring, each field by its function's name, a call that leaves a
// `None` out, `unlinkDep(1)`, too (ADR 0313). Not where what the
// call reads its `this`, a binding's method, nor where the field's JS name
// is no identifier, nor where the thread-local is set, nor where the
// function's JS converts what it's given.
test("a private function only called that only calls a thread-local's field is that field", async () => {
  const dir = fixture("forwarding-local-fields");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::Cell;
use std::rc::Rc;

pub struct System {
    link: Rc<dyn Fn(u32, u32) -> u32>,
    unlink: Box<dyn Fn(u32, Option<u32>)>,
    #[cfg_attr(rust_js, rust_js::name = "check-dirty")]
    check_dirty: Rc<dyn Fn(u32) -> bool>,
}

fn create_reactive_system(base: u32) -> System {
    System {
        link: Rc::new(move |dep, sub| base + dep * 10 + sub),
        unlink: Box::new(|_, _| {}),
        check_dirty: Rc::new(|n| n > 2),
    }
}

thread_local! {
    static SYSTEM: System = create_reactive_system(1000);
    static EPOCH: &'static js::Date = js::date::new(Some(&0.0));
    static NAMED: System = create_reactive_system(2000);
    static SWAPPED: Cell<&'static System> = Cell::new(Box::leak(Box::new(create_reactive_system(3000))));
    static GIVEN: System = create_reactive_system(4000);
}

fn link(dep: u32, sub: u32) -> u32 {
    SYSTEM.with(|system| (system.link)(dep, sub))
}

fn unlink_dep(dep: u32, sub: Option<u32>) {
    SYSTEM.with(|system| (system.unlink)(dep, sub));
}

fn epoch_time() -> f64 {
    EPOCH.with(|epoch| epoch.get_time())
}

fn check_dirty(n: u32) -> bool {
    NAMED.with(|named| (named.check_dirty)(n))
}

fn swapped_link(dep: u32, sub: u32) -> u32 {
    (SWAPPED.get().link)(dep, sub)
}

fn given_link(dep: Option<u32>, sub: u32) -> u32 {
    GIVEN.with(|given| (given.link)(dep.unwrap_or(5), sub))
}

pub fn run() -> Vec<f64> {
    unlink_dep(1, None);
    unlink_dep(1, Some(2));
    let before = swapped_link(1, 2);
    SWAPPED.set(Box::leak(Box::new(create_reactive_system(5000))));
    vec![
        link(1, 2) as f64,
        epoch_time(),
        f64::from(u8::from(check_dirty(3))),
        before as f64,
        swapped_link(1, 2) as f64,
        given_link(None, 1) as f64,
    ]
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...flags]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("const { link, unlink: unlinkDep } = createReactiveSystem(1000);\n");
  expect(js).not.toContain("function link(");
  expect(js).not.toContain("function unlinkDep(");
  expect(js).toContain("  unlinkDep(1);\n");
  // A binding's method, which reads its \`this\`; a field of no identifier;
  // a thread-local set; and a call that converts what it's given: each a
  // function still.
  expect(js).toContain("function epochTime() {\n  return epoch.getTime();\n}");
  expect(js).toContain("function checkDirty(n) {\n");
  expect(js).toContain("function swappedLink(dep, sub) {\n");
  expect(js).toContain("function givenLink(dep, sub) {\n");
  const lib = await import(join(dir, "lib.js"));
  expect(lib.run()).toEqual([1012, 0, 1, 3012, 5012, 4051]);
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

// ADR 0610, amended (#561): React's noop Flight client makes one instance
// of the Flight client's module and takes it apart once, `const {
// createResponse, .. } = ReactFlightClient({..})`. Its port's private
// thread-local of the namespace value, read only to call its methods, is
// that destructuring, each call the function alone. Not one also read as a
// value, nor one whose method's name the module has already, nor a JS
// object's method, which may read its `this`.
test("a private thread-local of a namespace read only for its methods is taken apart once", async () => {
  const dir = fixture("forwarding-namespace-members");
  writeFileSync(join(dir, "reconciler.rs"), `#[cfg_attr(rust_js, rust_js::namespace)]
pub struct Reconciler;

unsafe impl js::Defined for Reconciler {}

#[cfg_attr(rust_js, rust_js::namespace)]
impl Reconciler {
    pub fn create_container(&self, tag: u32) -> u32 {
        self.update_container(tag) + 1
    }

    pub fn update_container(&self, n: u32) -> u32 {
        n * 2
    }
}
`);
  writeFileSync(join(dir, "make.js"), "export default (config) => ({ createContainer: (n) => n + config.base, updateContainer: (n) => n * 3 });\n");
  const factory = `use crate::reconciler::Reconciler;

pub struct Config {
    pub base: u32,
}

unsafe impl js::Defined for Config {}

#[cfg_attr(rust_js, rust_js::link_name = "./make.js#default")]
fn create_reconciler(_config: &Config) -> &'static Reconciler {
    unreachable!()
}
`;
  writeFileSync(join(dir, "lib.rs"), `pub mod other;
pub mod reconciler;

${factory}
thread_local! {
    static NOOP: &'static Reconciler = create_reconciler(&Config { base: 100 });
    static SECOND: &'static Reconciler = create_reconciler(&Config { base: 400 });
    static EPOCH: &'static js::Date = js::date::new(Some(&0.0));
}

pub fn noop() -> u32 {
    NOOP.with(|noop| noop.create_container(4) + noop.update_container(1))
}

pub fn again(n: u32) -> u32 {
    NOOP.with(|noop| noop.update_container(n))
}

pub fn epoch() -> f64 {
    EPOCH.with(|epoch| epoch.get_time())
}

pub fn second() -> u32 {
    SECOND.with(|second| second.create_container(5))
}
`);
  writeFileSync(join(dir, "other.rs"), `${factory}
thread_local! {
    static KEPT: &'static Reconciler = create_reconciler(&Config { base: 200 });
    static NAMED: &'static Reconciler = create_reconciler(&Config { base: 300 });
}

pub fn kept() -> (u32, &'static Reconciler) {
    KEPT.with(|kept| (kept.create_container(1), *kept))
}

pub fn named() -> u32 {
    NAMED.with(|named| named.create_container(2)) + create_container()
}

fn create_container() -> u32 {
    1
}

thread_local! {
    static SWAPPED: std::cell::Cell<&'static Reconciler> = std::cell::Cell::new(create_reconciler(&Config { base: 500 }));
}

pub fn swapped() -> u32 {
    let before = SWAPPED.get().update_container(1);
    SWAPPED.set(create_reconciler(&Config { base: 600 }));
    before + SWAPPED.get().update_container(2)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...flags]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("const { createContainer, updateContainer } = make({ base: 100 });\n");
  expect(js).toContain("  return (createContainer(4) + updateContainer(1)) >>> 0;\n");
  expect(js).toContain("  return updateContainer(n);\n");
  // A JS object's method, which may read its `this`.
  expect(js).toContain("  return EPOCH.getTime();\n");
  // Another's of a name the first took.
  expect(js).toContain("const SECOND = make({ base: 400 });\n");
  expect(js).toContain("  return SECOND.createContainer(5);\n");
  const other = readFileSync(join(dir, "other.js"), "utf8");
  expect(other).toContain("const KEPT = make({ base: 200 });\n");
  expect(other).toContain("  return [KEPT.createContainer(1), KEPT];\n");
  expect(other).toContain("const NAMED = make({ base: 300 });\n");
  expect(other).toContain("  return (NAMED.createContainer(2) + createContainer()) >>> 0;\n");
  // One the module sets.
  expect(other).toContain("let SWAPPED = make({ base: 500 });\n");
  expect(other).toContain("  const before = SWAPPED.updateContainer(1);\n");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.noop(), lib.again(2), lib.epoch(), lib.second()]).toEqual([107, 6, 0, 405]);
  const other$ = await import(join(dir, "other.js"));
  expect(other$.kept()[0]).toBe(201);
  expect(other$.named()).toBe(303);
  expect(other$.swapped()).toBe(9);
});
