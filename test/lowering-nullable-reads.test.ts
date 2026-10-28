// What the compiler writes of an `Option` read from a place that holds its
// `None` as `null` (ADR 0949): a `#[rust_js::nullable]` field's or
// thread-local's, which is `null` or a value already, so a nullable
// function that returns it returns it as it is, no `?? null`. Each test is
// a program of its own, compiled and run.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildReact, compiler, fixture, run, target } from "./support";

beforeAll(buildReact, 600_000);

// React's getSuspenseInstanceFromFiber, `return suspenseState.dehydrated;`,
// and popSearchTarget, `return searchTarget;`.
test("a nullable place's read is returned as it is", async () => {
  const dir = fixture("nullable-reads");
  writeFileSync(
    join(dir, "lib.rs"),
    `use std::cell::Cell;

use js::Unknown;

pub struct SuspenseState {
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub dehydrated: Option<&'static Unknown>,
}

thread_local! {
    #[cfg_attr(rust_js, rust_js::nullable)]
    static CURRENT: Cell<Option<&'static Unknown>> = const { Cell::new(None) };
}

#[cfg_attr(rust_js, rust_js::nullable)]
pub fn dehydrated(memoized_state: Option<&'static Unknown>) -> Option<&'static Unknown> {
    let suspense_state: Option<&SuspenseState> = unsafe { js::cast(memoized_state) };
    if let Some(suspense_state) = suspense_state {
        return suspense_state.dehydrated;
    }
    None
}

#[cfg_attr(rust_js, rust_js::nullable)]
pub fn current() -> Option<&'static Unknown> {
    CURRENT.get()
}

pub fn set_current(value: Option<&'static Unknown>) {
    CURRENT.set(value);
}
`,
  );
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  // The cast's \`let\` is a \`const\`, as ReactFiberHydrationContext's \`const
  // suspenseState: null | SuspenseState = fiber.memoizedState;\` (ADR 0598,
  // amended).
  expect(js).toContain("  const suspenseState = memoizedState;\n  if (suspenseState != null) {\n    return suspenseState.dehydrated;\n  }\n  return null;\n");
  expect(js).toContain("export function current() {\n  return CURRENT;\n}");
  expect(js).toContain("  CURRENT = value ?? null;\n");
  const lib = await import(join(dir, "lib.js"));
  const one = {};
  expect([lib.dehydrated({ dehydrated: one }), lib.dehydrated({ dehydrated: null }), lib.dehydrated(null), lib.current()]).toEqual([one, null, null, null]);
  lib.setCurrent(undefined);
  expect(lib.current()).toBe(null);
  lib.setCurrent(one);
  expect(lib.current()).toBe(one);
});

// ADR 0957: React's popHydrationStateOnInterruptedWork, `if (fiber.tag ===
// HostComponent && fiber.stateNode != null) { nextHydratableInstance =
// fiber.stateNode; }`: a place a test shows isn't `null`, stored right
// after it, is stored as it is, no `?? null`.
test("a place a test shows present is stored as it is", async () => {
  const dir = fixture("nullable-store-after-test");
  writeFileSync(
    join(dir, "lib.rs"),
    `use std::cell::Cell;

use js::Unknown;

pub struct Fiber {
    pub tag: Cell<u8>,
    pub state_node: Cell<Option<&'static Unknown>>,
}

thread_local! {
    #[cfg_attr(rust_js, rust_js::nullable)]
    static NEXT: Cell<Option<&'static Unknown>> = const { Cell::new(None) };
}

pub fn restore(fiber: &Fiber) {
    if fiber.tag.get() == 5 && fiber.state_node.get().is_some() {
        NEXT.set(fiber.state_node.get());
    }
}

pub fn restore_or(fiber: &Fiber) {
    if fiber.tag.get() == 7 || fiber.state_node.get().is_some() {
        NEXT.set(fiber.state_node.get());
    }
}

#[cfg_attr(rust_js, rust_js::nullable)]
pub fn next() -> Option<&'static Unknown> {
    NEXT.get()
}
`,
  );
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("  if (fiber.tag === 5 && fiber.stateNode != null) {\n    NEXT = fiber.stateNode;\n  }\n");
  const lib = await import(join(dir, "lib.js"));
  const node = {};
  lib.restore({ tag: 5, stateNode: undefined });
  expect(lib.next()).toBe(null);
  lib.restore({ tag: 5, stateNode: node });
  expect(lib.next()).toBe(node);
  // Of an `||`, where the place may be `undefined`: `?? null` kept.
  lib.restoreOr({ tag: 7, stateNode: undefined });
  expect(lib.next()).toBe(null);
});

// ADR 0957, amended: React's getParentFragmentInstances, `let
// parentFragmentInstances = null; .. return parentFragmentInstances;`: a
// `let` made and set only to what's never `undefined` is returned as it is;
// one a closure may set keeps its `?? null`.
test("a let set only to values is returned as it is", async () => {
  const dir = fixture("nullable-let-returned");
  writeFileSync(
    join(dir, "lib.rs"),
    `#[cfg_attr(rust_js, rust_js::nullable)]
pub fn collect(items: &[&'static js::Unknown]) -> Option<Vec<&'static js::Unknown>> {
    let mut found = js::null();
    for &item in items {
        if js::truthy(js::get(item, "keep")) {
            match &mut found {
                None => found = Some(vec![item]),
                Some(found) => found.push(item),
            }
        }
    }
    found
}

#[cfg_attr(rust_js, rust_js::nullable)]
pub fn first(items: &[u32]) -> Option<u32> {
    let mut seen = js::null();
    items.iter().for_each(|&n| {
        if seen.is_none() {
            seen = items.get(n as usize).copied();
        }
    });
    seen
}
`,
  );
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("  return found;\n");
  expect(js).toContain("  return seen ?? null;\n");
  const lib = await import(join(dir, "lib.js"));
  const a = { keep: true };
  expect([lib.collect([]), lib.collect([{}, a]), lib.first([5]), lib.first([0])]).toEqual([null, [a], null, 0]);
});

// The user's decision (2026-10-27), ADR 0861, amended: a
// `#[rust_js::nullable(..)]` parameter is `T | null` as declared, given on
// as it is, React's `captureCommitPhaseError(current,
// nearestMountedAncestor)` and `searchTarget = target`; a nullable
// thread-local's whole value cloned, `with_borrow(Clone::clone)`, is
// returned as it is, popSearchTarget's `return searchTarget;`; and a `let`
// of `None` a nullable function returns, set only to values, starts `null`
// and is returned as it is, findStrictRoot's `let maybeStrictRoot = null;`.
test("a nullable parameter, a nullable thread-local's clone and a let of None returned are as they are", async () => {
  const dir = fixture("nullable-as-declared");
  writeFileSync(
    join(dir, "lib.rs"),
    `use std::cell::RefCell;
use std::rc::Rc;

pub struct Fiber {
    pub tag: u8,
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub r#return: RefCell<Option<Rc<Fiber>>>,
}

#[cfg_attr(rust_js, rust_js::link_name = "rustJsCapture")]
#[cfg_attr(rust_js, rust_js::nullable(nearest))]
fn capture(source: &Fiber, nearest: Option<&Rc<Fiber>>) {
    let _ = (source, nearest);
    unreachable!()
}

thread_local! {
    #[cfg_attr(rust_js, rust_js::nullable)]
    static SEARCH_TARGET: RefCell<Option<Rc<Fiber>>> = const { RefCell::new(None) };
}

#[cfg_attr(rust_js, rust_js::nullable(nearest))]
pub fn pass_on(current: &Fiber, nearest: Option<&Rc<Fiber>>) {
    capture(current, nearest);
}

#[cfg_attr(rust_js, rust_js::nullable(target))]
pub fn push_search_target(target: Option<Rc<Fiber>>) {
    SEARCH_TARGET.set(target);
}

#[cfg_attr(rust_js, rust_js::nullable)]
pub fn pop_search_target() -> Option<Rc<Fiber>> {
    SEARCH_TARGET.with_borrow(Clone::clone)
}

#[cfg_attr(rust_js, rust_js::nullable)]
pub fn find_strict_root(fiber: &Rc<Fiber>) -> Option<Rc<Fiber>> {
    let mut maybe_strict_root = None;
    let mut node = Some(Rc::clone(fiber));
    while let Some(current) = node {
        if current.tag == 8 {
            maybe_strict_root = Some(Rc::clone(&current));
        }
        node = current.r#return.borrow().clone();
    }
    maybe_strict_root
}
`,
  );
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("export function passOn(current, nearest) {\n  rustJsCapture(current, nearest);\n}");
  expect(js).toContain("export function pushSearchTarget(target) {\n  searchTarget = target;\n}");
  expect(js).toContain("export function popSearchTarget() {\n  return searchTarget;\n}");
  expect(js).toContain("  let maybeStrictRoot = null;\n");
  expect(js).toContain("  return maybeStrictRoot;\n}");
  const global = globalThis as Record<string, unknown>;
  const given: unknown[] = [];
  global.rustJsCapture = (_: unknown, nearest: unknown) => given.push(nearest);
  const lib = await import(join(dir, "lib.js"));
  const root = { tag: 8, return: null };
  const child = { tag: 1, return: root };
  lib.passOn(child as never, null as never);
  lib.passOn(child as never, root as never);
  expect(given).toEqual([null, root]);
  expect(lib.popSearchTarget()).toBe(null);
  lib.pushSearchTarget(child as never);
  expect(lib.popSearchTarget()).toBe(child);
  expect([lib.findStrictRoot(child as never), lib.findStrictRoot({ tag: 1, return: null } as never)]).toEqual([root, null]);
});

// ADR 0841, amended: React's claimNextHydratableActivityInstance, `const
// activityInstance = nextInstance ? tryHydrateActivity(fiber, nextInstance)
// : null;`. An `and_then` of a closure whose value is a nullable function's
// call is `null` for `None`, as of the function itself; of a closure of
// another function's, `undefined`.
test("an and_then of a closure of a nullable function's call is null for None", async () => {
  const dir = fixture("and-then-closure-nullable");
  writeFileSync(
    join(dir, "lib.rs"),
    `use js::Unknown;

#[cfg_attr(rust_js, rust_js::nullable)]
fn try_hydrate(id: u32, next: &'static Unknown) -> Option<&'static Unknown> {
    js::set(next, "id", id);
    js::get(next, "instance")
}

fn plain(next: &'static Unknown) -> Option<&'static Unknown> {
    js::get(next, "instance")
}

pub fn claim(id: u32, next: Option<&'static Unknown>) -> Option<&'static Unknown> {
    let instance = next.and_then(|next| try_hydrate(id, next));
    let Some(instance) = instance else {
        return None;
    };
    Some(instance)
}

pub fn other(next: Option<&'static Unknown>) -> bool {
    let instance = next.and_then(|next| plain(next));
    instance.is_some()
}
`,
  );
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("  const instance = next != null ? tryHydrate(id, next) : null;\n");
  expect(js).toContain(": undefined;\n");
  const lib = await import(join(dir, "lib.js"));
  const next = { instance: 3 };
  expect([lib.claim(1, next), next, lib.claim(2, undefined), lib.other({ instance: 0 }), lib.other(undefined)]).toEqual([3, { instance: 3, id: 1 }, undefined, true, false]);
});

// React's hooks store what's never `undefined` where `None` is `null`:
// `lastRenderedReducer: reducer`, a `Cell::new(Some(reducer))` of a
// nullable field, and `ReactSharedInternals.H = HooksDispatcherOnUpdateInDEV`,
// a nullable thread-local's value, `with`'s, given a nullable parameter.
// Each is `null` or a value already, stored as it is, no `?? null` (ADR
// 0841, amended).
test("a Some made into a nullable field's Cell, and a nullable thread-local's lent value, are stored as they are", async () => {
  const dir = fixture("nullable-stored-as-is");
  writeFileSync(
    join(dir, "lib.rs"),
    `use std::cell::Cell;

pub struct Queue {
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub last_rendered_reducer: Cell<Option<&'static dyn Fn(u32) -> u32>>,
}

pub struct Dispatcher {
    pub use_id: fn() -> u32,
}

thread_local! {
    #[cfg_attr(rust_js, rust_js::nullable)]
    static ON_UPDATE_IN_DEV: Option<Dispatcher> = Some(Dispatcher { use_id: || 1 });
}

/// JS's, \`globalThis.setH\`, which keeps what it's given.
#[cfg_attr(rust_js, rust_js::link_name = "setH")]
#[cfg_attr(rust_js, rust_js::nullable(dispatcher))]
#[allow(unused_variables)]
fn set_h(dispatcher: Option<&Dispatcher>) {
    unreachable!()
}

pub fn make(reducer: &'static dyn Fn(u32) -> u32) -> Queue {
    Queue { last_rendered_reducer: Cell::new(Some(reducer)) }
}

pub fn dispatch() {
    ON_UPDATE_IN_DEV.with(|dispatcher| set_h(dispatcher.as_ref()));
}
`,
  );
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("return { lastRenderedReducer: reducer };");
  expect(js).toContain("  setH(onUpdateInDev);\n");
  const lib = await import(join(dir, "lib.js"));
  expect(lib.make((n: number) => n + 1).lastRenderedReducer(1)).toBe(2);
  const kept: any[] = [];
  (globalThis as any).setH = (dispatcher: any) => kept.push(dispatcher);
  lib.dispatch();
  expect(kept.map((dispatcher) => dispatcher.useId())).toEqual([1]);
});

// A call's `Option`, which may be `undefined`, kept in a variable a nullable
// field alone reads: `?? null` where it's made, the variable read as it is.
test("a call's Option a nullable field alone reads is null where it's made", async () => {
  const dir = fixture("nullable-call-kept");
  writeFileSync(
    join(dir, "lib.rs"),
    `use std::cell::Cell;

pub struct Lazy {
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub label: Option<f64>,
    pub seen: Cell<u32>,
}

fn halved(n: Option<f64>) -> Option<f64> {
    match n {
        Some(n) if n > 0.0 => Some(n / 2.0),
        _ => None,
    }
}

pub fn halved_lazy(count: &Cell<u32>, n: Option<f64>) -> Lazy {
    let label = halved(n);
    count.set(count.get() + 1);
    Lazy {
        label,
        seen: Cell::new(count.get()),
    }
}
`,
  );
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("  const label = halved(n) ?? null;\n");
  expect(js).not.toContain("label: label ?? null");
  const lib = await import(join(dir, "lib.js"));
  const count = { value: 0 };
  expect([4, -4, undefined].map((n) => lib.halvedLazy(count, n).label)).toEqual([2, null, null]);
});

// React's unsafe_markUpdateLaneFromFiberToRoot ends `const root = ..; return
// root;` of what a nullable function gave, a variable's, held while
// more runs: `null` for `None` already, so returned and stored as it is,
// not `root ?? null` (ADR 0620, amended).
test("a variable of what a nullable function gave is returned and stored as it is", async () => {
  const dir = fixture("nullable-call-kept");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::RefCell;
use std::rc::Rc;

pub struct Root {
    pub tag: u32,
}

pub struct Holder {
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub root: RefCell<Option<Rc<Root>>>,
}

#[cfg_attr(rust_js, rust_js::nullable)]
fn lookup(roots: &[Rc<Root>], tag: u32) -> Option<Rc<Root>> {
    roots.iter().find(|r| r.tag == tag).cloned()
}

#[cfg_attr(rust_js, rust_js::nullable)]
pub fn found(roots: &[Rc<Root>], tag: u32, seen: &mut u32) -> Option<Rc<Root>> {
    let root = lookup(roots, tag);
    *seen += 1;
    root
}

pub fn held(holder: &Holder, roots: &[Rc<Root>], tag: u32) {
    let root = lookup(roots, tag);
    holder.root.replace(root);
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("  seen.value = (seen.value + 1) >>> 0;\n  return root;\n");
  expect(js).toContain("  const root = lookup(roots, tag);\n  holder.root = root;\n");
  const lib = await import(join(dir, "lib.js"));
  const roots = [{ tag: 1 }, { tag: 2 }];
  const seen = { value: 0 };
  const holder = { root: roots[0] as unknown };
  lib.held(holder, roots, 3);
  expect([lib.found(roots, 2, seen), lib.found(roots, 5, seen), holder.root]).toEqual([roots[1], null, null]);
});
