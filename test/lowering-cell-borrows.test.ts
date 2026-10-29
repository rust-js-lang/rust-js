// What a `RefCell`'s borrow held across a binding's call meets: one that
// runs only what it's given, `console.error(..)` and `Math.random()`,
// given only numbers and text, runs no code of the crate's, so the borrow
// isn't counted and its cell is the array it holds; given an object, whose
// `toString` may be the crate's, it's counted, and panics as Rust's would.
// Each test is a program of its own, compiled and run.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildReact, compiler, fixture, run, target } from "./support";

beforeAll(buildReact, 600_000);

const withBindings = [
  "--",
  "--extern",
  `js=${join(target, "libjs.rmeta")}`,
  "--extern",
  `webapi=${join(target, "libwebapi.rmeta")}`,
  "-L",
  target,
];

// ADR 0898: React's ReactFiberLane computes a lane's expiration time while
// it holds `root.expirationTimes`, and the computation may `console.error`.
test("a borrow held while a binding given only numbers and text runs isn't counted", async () => {
  const dir = fixture("binding-given-borrows");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::RefCell;
use webapi::console;

pub struct Root {
    pub expiration_times: RefCell<Vec<f64>>,
    pub reported: RefCell<Vec<u32>>,
}

pub fn root() -> Root {
    Root {
        expiration_times: RefCell::new(vec![-1.0; 4]),
        reported: RefCell::new(Vec::new()),
    }
}

fn compute_expiration_time(lane: u32, current_time: f64) -> f64 {
    if lane == 0 {
        console::error(("Should have found matching lanes. This is a bug in React.", lane));
        return -1.0;
    }
    current_time + js::math::max((250.0, js::math::random()))
}

pub fn mark(root: &Root, index: usize, lane: u32, now: f64) {
    let mut expiration_times = root.expiration_times.borrow_mut();
    expiration_times[index] = compute_expiration_time(lane, now);
}

pub fn expiration_time(root: &Root, index: usize) -> f64 {
    root.expiration_times.borrow()[index]
}

pub fn report(root: &Root, value: &js::Unknown) {
    let mut reported = root.reported.borrow_mut();
    console::error(("%s", value));
    reported.push(1);
}

pub fn touch(root: &Root) {
    root.reported.borrow_mut().push(2);
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withBindings]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("  const expirationTimes = root.expirationTimes;\n");
  expect(js).toContain("  return $index(root.expirationTimes, index);\n");
  expect(js).not.toContain("$borrowMut(root.expirationTimes");
  expect(js).toContain("$borrowMut(root.reported");
  const lib = await import(join(dir, "lib.js"));
  const error = console.error;
  const logged: unknown[][] = [];
  try {
    console.error = (...args: unknown[]) => {
      logged.push(args);
      args.forEach((arg) => String(arg));
    };
    const r = lib.root();
    lib.mark(r, 1, 0, 10);
    lib.mark(r, 2, 4, 10);
    expect([lib.expirationTime(r, 1), lib.expirationTime(r, 2)]).toEqual([-1, 260]);
    expect(logged).toEqual([["Should have found matching lanes. This is a bug in React.", 0]]);
    lib.report(r, { toString: () => "quiet" });
    expect(() => lib.report(r, { toString: () => (lib.touch(r), "touched") })).toThrow("already borrowed");
  } finally {
    console.error = error;
  }
});

// ADR 0907: React's ReactFiberLane picks a lane's index through `clz32`, a
// thread-local fn pointer of `Math.clz32` or its fallback, while it holds
// `root.entanglements`. What a fn pointer may be is each function the
// crate makes one of, where no value of its type comes from outside.
test("a borrow held while a fn pointer runs what the crate made it of isn't counted", async () => {
  const dir = fixture("fn-pointer-borrows");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::RefCell;

thread_local! {
    pub static CLZ32: fn(u32) -> u32 = if js::is_defined(u32::leading_zeros) {
        u32::leading_zeros
    } else {
        clz32_fallback
    };
}

fn clz32_fallback(x: u32) -> u32 {
    if x == 0 {
        return 32;
    }
    31 - x.ilog2()
}

pub struct Root {
    pub entanglements: RefCell<Vec<i32>>,
    pub given: RefCell<Vec<u32>>,
}

pub fn root() -> Root {
    Root {
        entanglements: RefCell::new(vec![1, 2, 4, 8]),
        given: RefCell::new(vec![0]),
    }
}

fn pick_arbitrary_lane_index(lanes: i32) -> usize {
    31 - CLZ32.with(|clz32| clz32(lanes as u32)) as usize
}

pub fn entangled_lanes(root: &Root, mut lanes: i32) -> i32 {
    let entanglements = root.entanglements.borrow();
    let mut entangled = lanes;
    while lanes > 0 {
        let index = pick_arbitrary_lane_index(lanes);
        entangled |= entanglements[index];
        lanes &= !(1 << index);
    }
    entangled
}

pub fn apply(root: &Root, f: fn(i32) -> i32) -> i32 {
    let given = root.given.borrow();
    f(given[0] as i32)
}

pub fn touch(root: &Root) {
    root.given.borrow_mut().push(1);
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withBindings]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("  const entanglements = root.entanglements;\n");
  expect(js).not.toContain("$borrow(root.entanglements");
  // What JS calls `apply` with may be any function: counted.
  expect(js).toContain("$borrow(root.given, true)");
  const lib = await import(join(dir, "lib.js"));
  const r = lib.root();
  expect(lib.entangledLanes(r, 0b101)).toBe(0b101);
  expect(lib.apply(r, (n: number) => n + 1)).toBe(1);
  expect(() => lib.apply(r, () => (lib.touch(r), 0))).toThrow("already borrowed");
});

// React's ReactFiberCallUserSpace binds each frame's function in DEV,
// `fn(&dyn Fn() -> u32) -> u32`, which `function::bind` gives from outside:
// a type of no key, which no `fn(u32) -> u32` can be, so ReactFiberLane's
// `clz32` under a borrow still runs only what the crate made it of. What
// such a type's signature holds may come from outside with it (ADR 0907,
// amended).
test("a fn pointer of a dyn given from outside leaves other fn pointers what the crate made them of", async () => {
  const dir = fixture("fn-pointer-dyn-outside");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::RefCell;

use js::function;

pub struct Root {
    pub times: RefCell<Vec<u32>>,
    pub given: RefCell<Vec<u32>>,
}

pub fn root() -> Root {
    Root {
        times: RefCell::new(vec![1; 32]),
        given: RefCell::new(vec![2]),
    }
}

thread_local! {
    pub static CLZ32: fn(u32) -> u32 = u32::leading_zeros;
}

pub fn expire(root: &Root, lanes: u32) -> u32 {
    let mut times = root.times.borrow_mut();
    let index = CLZ32.with(|clz32| clz32(lanes)) as usize;
    times[index] = 0;
    times.iter().sum()
}

pub type CallAdd = fn(&dyn Fn() -> u32) -> u32;

struct CallFrame {
    react_stack_bottom_frame: CallAdd,
}

const CALL_ADD: CallFrame = CallFrame {
    react_stack_bottom_frame: |component| component(),
};

thread_local! {
    pub static CALL_ADD_IN_DEV: CallAdd =
        function::bind(CALL_ADD.react_stack_bottom_frame, Some(&CALL_ADD), ());
}

pub fn call_add(root: &Root) -> u32 {
    let given = root.given.borrow();
    CALL_ADD_IN_DEV.with(|call| call(&|| given[0] + 1))
}

pub fn apply_made(root: &Root, make: fn() -> fn(u8) -> u8) -> u8 {
    let f = make();
    let given = root.given.borrow();
    f(given[0] as u8)
}

pub fn touch(root: &Root) {
    root.given.borrow_mut().push(1);
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withBindings]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("export function expire(root, lanes) {\n  const times = root.times;\n  const index = clz32(lanes);\n");
  // What a fn pointer JS gives returns may be any function: counted.
  expect(js).toContain("$borrow(root.given, true)");
  const lib = await import(join(dir, "lib.js"));
  const r = lib.root();
  expect(lib.expire(r, 1)).toBe(31);
  expect(lib.callAdd(r)).toBe(3);
  expect(lib.applyMade(r, () => (n: number) => n + 1)).toBe(3);
  expect(() => lib.applyMade(r, () => () => (lib.touch(r), 0))).toThrow("already borrowed");
});

// #374, ADR 0960: React's test selectors call their commit hooks, which may
// run anything a JS caller gave, while their thread-local is borrowed, so
// that cell is checked. The hydration diffs' `children`, a `RefCell` of a
// `Vec<Rc<Node>>`, isn't one that may be it: a `Vec`'s `Rc<dyn Fn()>` is
// no `Rc<Node>`, as only a cell's own value, or a struct's last field, is
// made a `dyn` in place, a `RefCell<Circle>` a `RefCell<dyn Shape>`.
test("a borrow held while hooks run checks only cells that may be its own", async () => {
  const dir = fixture("hooks-cells-apart");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::RefCell;
use std::rc::Rc;

thread_local! {
    static COMMIT_HOOKS: RefCell<Vec<Rc<dyn Fn()>>> = const { RefCell::new(Vec::new()) };
}

pub struct Node {
    pub name: String,
    pub children: RefCell<Vec<Rc<Node>>>,
}

pub fn on_commit_root() {
    COMMIT_HOOKS.with_borrow(|commit_hooks| commit_hooks.iter().for_each(|commit_hook| commit_hook()));
}

pub fn observe(callback: Box<dyn Fn()>) {
    let commit_hook: Rc<dyn Fn()> = Rc::new(move || callback());
    COMMIT_HOOKS.with_borrow_mut(|commit_hooks| commit_hooks.push(commit_hook));
}

pub fn node(name: &str) -> Rc<Node> {
    Rc::new(Node {
        name: name.to_string(),
        children: RefCell::new(Vec::new()),
    })
}

fn push(siblings: &RefCell<Vec<Rc<Node>>>, child: Rc<Node>) {
    siblings.borrow_mut().push(child);
}

pub fn adopt(parent: &Node, child: Rc<Node>) {
    push(&parent.children, child);
}

pub fn first(parent: &Node) -> String {
    parent.children.borrow()[0].name.clone()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withBindings]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("function push(siblings, child) {\n  siblings.push(child);\n}");
  expect(js).toContain("$withBorrow(\n    commitHooks,");
  const lib = await import(join(dir, "lib.js"));
  const root = lib.node("root");
  lib.observe(() => lib.adopt(root, lib.node("child")));
  lib.onCommitRoot();
  expect(lib.first(root)).toBe("child");
  // A hook that observes again while the hooks are borrowed panics, as Rust's.
  lib.observe(() => lib.observe(() => {}));
  expect(() => lib.onCommitRoot()).toThrow("already borrowed");
});

// ADR 0960: the same hooks as a struct's field, not a thread-local only its
// key reaches (ADR 0960, amended), so the `dyn` in their `Vec` alone keeps
// `children` apart: a borrow of it, while the hooks run, is no borrow of
// them.
test("a field's hooks held while they run meet no cell of a Vec's other items", async () => {
  const dir = fixture("hooks-field-cells-apart");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::RefCell;
use std::rc::Rc;

pub struct Root {
    hooks: RefCell<Vec<Rc<dyn Fn()>>>,
}

pub struct Node {
    pub name: String,
    pub children: RefCell<Vec<Rc<Node>>>,
}

pub fn root() -> Root {
    Root {
        hooks: RefCell::new(Vec::new()),
    }
}

pub fn commit(root: &Root) {
    root.hooks.borrow().iter().for_each(|hook| hook());
}

pub fn observe(root: &Root, callback: Box<dyn Fn()>) {
    let hook: Rc<dyn Fn()> = Rc::new(move || callback());
    root.hooks.borrow_mut().push(hook);
}

pub fn node(name: &str) -> Rc<Node> {
    Rc::new(Node {
        name: name.to_string(),
        children: RefCell::new(Vec::new()),
    })
}

fn push(siblings: &RefCell<Vec<Rc<Node>>>, child: Rc<Node>) {
    siblings.borrow_mut().push(child);
}

pub fn adopt(parent: &Node, child: Rc<Node>) {
    push(&parent.children, child);
}

pub fn first(parent: &Node) -> String {
    parent.children.borrow()[0].name.clone()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("function push(siblings, child) {\n  siblings.push(child);\n}");
  expect(js).toContain("export function first(parent) {\n  return $index(parent.children, 0).name;\n}");
  expect(js).toContain("const borrow = $borrow(root.hooks, true);");
  const lib = await import(join(dir, "lib.js"));
  const r = lib.root();
  const parent = lib.node("root");
  lib.observe(r, () => lib.adopt(parent, lib.node("child")));
  lib.commit(r);
  expect(lib.first(parent)).toBe("child");
  // A hook that observes again panics, as Rust's.
  lib.observe(r, () => lib.observe(r, () => {}));
  expect(() => lib.commit(r)).toThrow("already borrowed");
});

// ADR 0962: React's view transitions store the array they then fill,
// `(current.memoizedState = [])`, given to the call that fills it. A
// `RefCell` of an array is the array (ADR 0863), so it's `js::Defined` and
// given to JS as it; one that's a `{ value }` box, of a number, isn't
// what JS should see, and is refused where a binding is given it.
test("a RefCell of an array given to JS is the array, and a boxed one is refused", async () => {
  const dir = fixture("refcell-given-to-js");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::{Cell, RefCell};
use std::rc::Rc;

use js::Unknown;

pub struct Fiber {
    pub memoized_state: Cell<Option<&'static Unknown>>,
}

pub fn fiber() -> Fiber {
    Fiber {
        memoized_state: Cell::new(None),
    }
}

fn collect(into: &RefCell<Vec<u32>>, n: u32) {
    into.borrow_mut().push(n);
    into.borrow_mut().push(n + 1);
}

pub fn before_update(current: &Fiber) {
    let measurements = Rc::new(RefCell::new(Vec::new()));
    current
        .memoized_state
        .set(Some(js::unknown_of(Rc::clone(&measurements))));
    collect(&measurements, 1);
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withBindings]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("export function beforeUpdate(current) {\n  const measurements = [];\n  current.memoizedState = measurements;\n  collect(measurements, 1);\n}");
  const lib = await import(join(dir, "lib.js"));
  const f = lib.fiber();
  lib.beforeUpdate(f);
  expect(f.memoizedState).toEqual([1, 2]);
  writeFileSync(join(dir, "boxed.rs"), `use std::cell::RefCell;
use std::rc::Rc;

pub fn count() -> &'static js::Unknown {
    let n = Rc::new(RefCell::new(1u32));
    let given = js::unknown_of(Rc::clone(&n));
    *n.borrow_mut() += 1;
    given
}
`);
  expect(() => run([compiler, join(dir, "boxed.rs"), "-o", join(dir, "boxed.js"), ...withBindings]))
    .toThrow("a `std::cell::RefCell<u32>` given to JS or taken from it");
});

// React's DEV dispatchers, as upstream writes them: `let
// HooksDispatcherOnMountInDEV = null; if (__DEV__) { HooksDispatcherOnMountInDEV
// = { .. }; }`, and each render sets the dispatcher to one,
// `ReactSharedInternals.H = HooksDispatcherOnMountInDEV`, a binding's call,
// which may run anything. Ported, the thread-local is set where its module
// loads, `js::on_load!`, which runs once, before JS can call into it: what
// JS runs while a borrow is held may borrow only what the crate's other
// bodies do, here only `with_borrow`s, which ask nothing of a shared one.
// So the cell is the module's `let`, upstream's (ADR 0580, amended). A
// borrow held mutably across JS, and a thread-local another crate may
// borrow, `pub`, or one whose cell is lent, `with`, stay checked.
test("a thread-local set only where its module loads is a variable JS can't meet a borrow of", async () => {
  const dir = fixture("set-where-loaded");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::RefCell;
use std::rc::Rc;

unsafe extern "Rust" {
    safe static __DEV__: bool;
}

pub struct Dispatcher {
    pub use_a: fn(u32) -> u32,
}

thread_local! {
    static ON_MOUNT_IN_DEV: RefCell<Option<Rc<Dispatcher>>> = RefCell::new(js::null());
    static SEEN: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };
    pub static SHARED: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    static LENT: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };
}

js::on_load! {
    if __DEV__ {
        ON_MOUNT_IN_DEV.set(Some(Rc::new(Dispatcher { use_a: |n| n + 1 })));
    }
}

pub fn render(set_h: &dyn Fn(Option<&Dispatcher>)) {
    ON_MOUNT_IN_DEV.with_borrow(|dispatcher| set_h(dispatcher.as_deref()));
}

// Held mutably while JS runs, which may call \`seen\`: checked.
pub fn see(f: &dyn Fn()) {
    SEEN.with_borrow_mut(|seen| {
        f();
        seen.push(1);
    });
}

pub fn seen() -> usize {
    SEEN.with_borrow(|seen| seen.len())
}

pub fn shared(f: &dyn Fn()) -> usize {
    SHARED.with_borrow(|shared| {
        f();
        shared.len()
    })
}

pub fn lent(f: &dyn Fn()) -> usize {
    LENT.with(|lent| lent.borrow_mut().push(1));
    LENT.with_borrow(|lent| {
        f();
        lent.len()
    })
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withBindings]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("let onMountInDev = null;\n");
  expect(js).toContain("if (__DEV__) {\n  onMountInDev = { useA: (n) => (n + 1) >>> 0 };\n}\n");
  expect(js).toContain("export function render(setH) {\n  setH(onMountInDev);\n}");
  expect(js).toContain("$withBorrowMut(\n    SEEN,");
  expect(js).toContain("$withBorrow(\n    SHARED,");
  expect(js).toContain("$withBorrow(\n    LENT,");
  (globalThis as any).__DEV__ = true;
  const lib = await import(join(dir, "lib.js"));
  const used: number[] = [];
  lib.render((dispatcher: any) => used.push(dispatcher.useA(1)));
  expect(used).toEqual([2]);
  expect(() => lib.see(() => lib.seen())).toThrow("already mutably borrowed");
});

// React's test selectors call their commit hooks while COMMIT_HOOKS, a
// `RefCell<Vec<Rc<dyn Fn()>>>`, is borrowed; ReactFiberCacheComponent's
// AbortControllerShim keeps its listeners in a cell of the same type. A
// thread-local's own cell only its key reaches, never lent by `with`, is no
// other cell: its borrow held while the hooks run checks its own borrows,
// not every cell of its type (ADR 0960, amended). One its `with` lends may
// be any `&RefCell` of its type, so those stay checked.
test("a thread-local's borrow held while hooks run checks no other cell of its type", async () => {
  const dir = fixture("hooks-key-apart");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::RefCell;
use std::rc::Rc;

pub struct Controller {
    pub add: Box<dyn Fn(Rc<dyn Fn()>)>,
    pub abort: Box<dyn Fn()>,
}

pub fn controller() -> Controller {
    let listeners: Rc<RefCell<Vec<Rc<dyn Fn()>>>> = Rc::new(RefCell::new(Vec::new()));
    Controller {
        add: {
            let listeners = Rc::clone(&listeners);
            Box::new(move |listener| listeners.borrow_mut().push(listener))
        },
        abort: Box::new(move || {
            let count = listeners.borrow().len();
            for index in 0..count {
                let listener = Rc::clone(&listeners.borrow()[index]);
                listener();
            }
        }),
    }
}

thread_local! {
    static HOOKS: RefCell<Vec<Rc<dyn Fn()>>> = const { RefCell::new(Vec::new()) };
}

pub fn add_hook(hook: Rc<dyn Fn()>) {
    HOOKS.with_borrow_mut(|hooks| hooks.push(hook));
}

pub fn run_hooks() {
    HOOKS.with_borrow(|hooks| hooks.iter().for_each(|hook| hook()));
}

pub fn clear_hooks() {
    HOOKS.set(Vec::new());
}

pub fn take_hooks() -> usize {
    HOOKS.take().len()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withBindings]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("      listeners.push(listener);\n");
  expect(js).toContain("      const listener = listeners[index];\n");
  expect(js).toContain("$withBorrow(\n    hooks,");
  const lib = await import(join(dir, "lib.js"));
  const controller = lib.controller();
  const log: string[] = [];
  controller.add(() => {
    log.push("first");
    controller.add(() => log.push("added"));
  });
  lib.addHook(() => controller.abort());
  lib.runHooks();
  controller.abort();
  expect(log).toEqual(["first", "first", "added"]);
  // A hook that adds a hook while the hooks are borrowed panics, as Rust's,
  // and so does one that sets them, the key's own write.
  lib.addHook(() => lib.addHook(() => {}));
  expect(() => lib.runHooks()).toThrow("already borrowed");
  lib.clearHooks();
  lib.addHook(() => lib.clearHooks());
  expect(() => lib.runHooks()).toThrow("already borrowed");
  expect(lib.takeHooks()).toBe(1);
  lib.addHook(() => lib.takeHooks());
  expect(() => lib.runHooks()).toThrow("already borrowed");

  const lent = fixture("hooks-key-lent");
  writeFileSync(join(lent, "lib.rs"), `use std::cell::RefCell;
use std::rc::Rc;

thread_local! {
    static HOOKS: RefCell<Vec<Rc<dyn Fn()>>> = const { RefCell::new(Vec::new()) };
}

pub fn add_hook(hook: Rc<dyn Fn()>) {
    HOOKS.with_borrow_mut(|hooks| hooks.push(hook));
}

pub fn run_hooks() {
    HOOKS.with_borrow(|hooks| hooks.iter().for_each(|hook| hook()));
}

fn clear(hooks: &RefCell<Vec<Rc<dyn Fn()>>>) {
    hooks.borrow_mut().clear();
}

pub fn clear_hooks() {
    HOOKS.with(clear);
}

pub fn listened(listener: Rc<dyn Fn()>) -> usize {
    let listeners: RefCell<Vec<Rc<dyn Fn()>>> = RefCell::new(Vec::new());
    listeners.borrow_mut().push(listener);
    let count = listeners.borrow().len();
    count
}
`);
  run([compiler, join(lent, "lib.rs"), "-o", join(lent, "lib.js"), ...withBindings]);
  const borrowed = await import(join(lent, "lib.js"));
  borrowed.addHook(() => borrowed.clearHooks());
  expect(() => borrowed.runHooks()).toThrow("already borrowed");
  expect(readFileSync(join(lent, "lib.js"), "utf8")).toContain("$borrowMut(listeners");
});

// React's update clone, `{ .., gesture: update.gesture, .., next: null }`: a
// `js::null()` made while a borrow of a field is held, as a struct literal's
// temporaries are, runs no code, so it can't borrow the field's cell, which
// stays the value it holds, unchecked (ADR 0863, amended).
test("js::null() made while a borrow is held runs nothing", async () => {
  const dir = fixture("null-under-borrow");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::{Cell, RefCell};
use std::rc::Rc;

pub struct G {
    pub count: Cell<i32>,
}

pub struct Update {
    pub lane: Cell<u32>,
    pub gesture: RefCell<Option<Rc<G>>>,
    pub next: RefCell<Option<Rc<Update>>>,
}

pub fn clone_update(update: &Update) -> Rc<Update> {
    Rc::new(Update {
        lane: Cell::new(update.lane.get()),
        gesture: RefCell::new(update.gesture.borrow().clone()),
        next: RefCell::new(js::null()),
    })
}

pub fn write(update: &Update, gesture: &Rc<G>) {
    update.gesture.replace(Some(Rc::clone(gesture)));
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withBindings]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).not.toContain("$fieldCell");
  expect(js).toContain("  update.gesture = gesture;\n");
  const lib = await import(join(dir, "lib.js"));
  const update = { lane: 2, gesture: { count: 1 }, next: null };
  expect(lib.cloneUpdate(update)).toEqual({ lane: 2, gesture: { count: 1 }, next: null });
});

// React's DEV dispatchers: `X.with_borrow(|d| set_h(d.as_deref()))`, a binding's
// call, which may run anything, made while a thread-local only its key
// reaches is borrowed. What may run then borrows a cell through a reference,
// `cursor.current.borrow_mut()` of any `RefCell<T>`, never that one's own,
// which no reference reaches: it stays its module's variable, `let nested;`
// (ADR 0960, amended).
test("a thread-local only its key reaches is no cell a borrow by reference is of", async () => {
  const dir = fixture("closed-key-under-binding");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::RefCell;
use std::rc::Rc;

pub struct D {
    pub n: u32,
}

#[cfg_attr(rust_js, rust_js::link_name = "globalThis.setH")]
fn set_h(_d: Option<&D>) {
    unreachable!()
}

pub struct Cursor<T> {
    pub current: RefCell<T>,
}

pub fn push<T>(cursor: &Cursor<T>, value: T) {
    *cursor.current.borrow_mut() = value;
}

thread_local! {
    static NESTED: RefCell<Option<Rc<D>>> = const { RefCell::new(None) };
}

js::on_load! {
    NESTED.set(Some(Rc::new(D { n: 1 })));
}

pub fn render() {
    NESTED.with_borrow(|d| set_h(d.as_deref()));
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withBindings]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("export function render() {\n  globalThis.setH(nested);\n}");
  const lib = await import(join(dir, "lib.js"));
  const seen: unknown[] = [];
  (globalThis as any).setH = (d: unknown) => seen.push(d);
  lib.render();
  expect(seen).toEqual([{ n: 1 }]);
});

// React's `pendingViewTransitionEvents ??= []; ..push(event)`, a thread-local
// `RefCell<Option<Vec<F>>>` of functions: std's function given to std's,
// `get_or_insert_with(Vec::new)`, runs what its own bounds say, as a call
// of it does, not each type it's of: `Vec::<F>::new` calls no `F`. One the
// crate makes, given so, still runs its body (ADR 0868, amended).
test("std's function given to std's runs what its bounds say, not what it's of", async () => {
  const dir = fixture("std-function-given");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::RefCell;

thread_local! {
    #[cfg_attr(rust_js, rust_js::nullable)]
    static PENDING: RefCell<Option<Vec<&'static dyn Fn(&[String])>>> = const { RefCell::new(js::null()) };
    static HOOKS: RefCell<Option<Vec<fn() -> u32>>> = const { RefCell::new(None) };
}

pub fn push_event(event: &'static dyn Fn(&[String])) {
    PENDING.with_borrow_mut(|events| events.get_or_insert_with(Vec::new).push(event));
}

pub fn pending_count() -> usize {
    PENDING.with_borrow(|events| events.as_ref().map_or(0, Vec::len))
}

fn first_hooks() -> Vec<fn() -> u32> {
    let pushed = HOOKS.with_borrow(|hooks| hooks.as_ref().map_or(0, Vec::len));
    vec![|| 1, if pushed == 0 { || 2 } else { || 3 }]
}

pub fn push_hook(hook: fn() -> u32) -> u32 {
    HOOKS.with_borrow_mut(|hooks| hooks.get_or_insert_with(first_hooks).push(hook));
    HOOKS.with_borrow(|hooks| hooks.iter().flatten().map(|hook| hook()).sum())
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withBindings]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("let pending = null;");
  expect(js).toContain("  pending ??= [];\n  pending.push(event);\n");
  // \`first_hooks\` borrows HOOKS while \`get_or_insert_with\` holds it.
  expect(js).toContain("$withBorrowMut(");
  const lib = await import(join(dir, "lib.js"));
  lib.pushEvent(() => {});
  lib.pushEvent(() => {});
  expect(lib.pendingCount()).toBe(2);
  expect(() => lib.pushHook(() => 4)).toThrow("RefCell already mutably borrowed");
});

// React Float's hydrateHoistable: in a loop over a `RefCell`'s borrow, one
// `try` around it, a condition written with `||` of comparisons each with
// a value `if`, its one `continue`, is one `if` of it all, `if (a !== (..)
// || b !== (..)) { continue; }`, each value the conditional it is (#413,
// rd225, ADR 0683, amended).
test("a condition of value ifs written with || in a borrow's loop is one if", async () => {
  const dir = fixture("borrow-loop-ors");
  writeFileSync(join(dir, "lib.rs"), `use js::Unknown;
use std::cell::RefCell;
use webapi::Element;

pub fn find(cache: &RefCell<Vec<&'static Element>>, props: &Unknown) -> Option<usize> {
    let mut nodes = cache.borrow_mut();
    for i in 0..nodes.len() {
        let node = nodes[i];
        if !js::strict_equals(
            node.get_attribute("href"),
            if js::get(props, "href").is_none() || js::strict_equals(js::get(props, "href"), "") {
                js::null()
            } else {
                js::get(props, "href")
            },
        ) || !js::strict_equals(
            node.get_attribute("title"),
            if js::get(props, "title").is_none() { js::null() } else { js::get(props, "title") },
        ) {
            // mismatch, try the next node;
            continue;
        }
        nodes.remove(i);
        return Some(i);
    }
    None
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withBindings]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain(`      if (
        node.getAttribute("href") !==
          (props.href == null || props.href === "" ? null : props.href) ||
        node.getAttribute("title") !== (props.title == null ? null : props.title)
      ) {
        // mismatch, try the next node;
        continue;
      }
      nodes.splice(i, 1);`);
  expect(js).not.toContain("let unknown");
  const lib = await import(join(dir, "lib.js"));
  const node = (href: string | null, title: string | null) => ({
    getAttribute: (name: string) => (name === "href" ? href : title),
  });
  // The cell is the array it holds (ADR 0898).
  const nodes = [node("a", null), node(null, "t"), node("b", "u")];
  expect(lib.find(nodes, { title: "t" })).toBe(1);
  expect(lib.find(nodes, { href: "b", title: "u" })).toBe(1);
  expect(lib.find(nodes, { href: "", title: "x" })).toBeUndefined();
  expect(nodes.length).toBe(1);
});
