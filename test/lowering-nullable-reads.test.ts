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

#[cfg_attr(rust_js, rust_js::nullable)]
pub fn current_of(fiber: Option<u32>) -> Option<&'static Unknown> {
    fiber.and_then(|_| CURRENT.get())
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
  // A closure's read of it is `null` or a value too, so its `and_then`'s
  // `None` is `null`, as the read's is.
  expect(js).toContain("export function currentOf(fiber) {\n  return fiber != null ? CURRENT : null;\n}");
  expect(js).toContain("  CURRENT = value ?? null;\n");
  const lib = await import(join(dir, "lib.js"));
  const one = {};
  expect([lib.dehydrated({ dehydrated: one }), lib.dehydrated({ dehydrated: null }), lib.dehydrated(null), lib.current()]).toEqual([one, null, null, null]);
  lib.setCurrent(undefined);
  expect(lib.current()).toBe(null);
  lib.setCurrent(one);
  expect(lib.current()).toBe(one);
  expect([lib.currentOf(1), lib.currentOf(undefined)]).toEqual([one, null]);
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

#[cfg_attr(rust_js, rust_js::nullable(nearest))]
pub fn pass_on_held(current: &Fiber, nearest: Option<&Rc<Fiber>>) {
    capture(current, nearest);
}

pub fn held() -> fn(&Fiber, Option<&Rc<Fiber>>) {
    pass_on_held
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
  // One used as a value, whose callers JS's calls may be: as declared.
  expect(js).toContain("export function passOnHeld(current, nearest) {\n  rustJsCapture(current, nearest);\n}");
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

// rd613 (ADR 0861, amended, the user's decision): a function's own
// `#[rust_js::nullable(..)]` parameter each of the crate's calls gives
// `null` or a value is tested `!== null`, React's webpack server config's
// `if (moduleLoading !== null)`; JS's calls keep its declared `T | null`.
// One a call gives what may be `undefined`, one used as a value, and one
// not marked are tested `!= null`, as JS may give them either.
test("a function's own nullable parameter each call gives null or a value is tested against null alone", async () => {
  const dir = fixture("nullable-param-tested");
  writeFileSync(
    join(dir, "lib.rs"),
    `pub struct ModuleLoading {
    pub prefix: String,
}

#[cfg_attr(rust_js, rust_js::nullable(module_loading))]
pub fn prepare(module_loading: Option<&ModuleLoading>, chunk: &str) -> String {
    if let Some(module_loading) = module_loading {
        return module_loading.prefix.clone() + chunk;
    }
    String::new()
}

#[cfg_attr(rust_js, rust_js::nullable(module_loading))]
pub fn forward(module_loading: Option<&ModuleLoading>) -> String {
    prepare(module_loading, "a.js") + &prepare(None, "b.js")
}

pub fn prefixed(prefix: &str) -> String {
    let loading = ModuleLoading { prefix: prefix.to_string() };
    prepare(Some(&loading), "c.js")
}

#[cfg_attr(rust_js, rust_js::nullable(module_loading))]
pub fn cleared(mut module_loading: Option<&ModuleLoading>, clear: bool) -> bool {
    if clear {
        module_loading = None;
    }
    module_loading.is_none()
}

#[cfg_attr(rust_js, rust_js::nullable(module_loading))]
pub fn given_on(module_loading: Option<&ModuleLoading>) -> bool {
    module_loading.is_some()
}

pub fn given(module_loading: Option<&ModuleLoading>) -> bool {
    given_on(module_loading)
}

#[cfg_attr(rust_js, rust_js::nullable(module_loading))]
pub fn mapped(module_loading: Option<&ModuleLoading>) -> bool {
    module_loading.is_some()
}

pub fn each(loadings: &[Option<&ModuleLoading>]) -> usize {
    loadings.iter().copied().map(mapped).filter(|&set| set).count()
}

pub trait Loads {
    fn loads(&self, module_loading: Option<&ModuleLoading>) -> bool;
}

pub struct Loader;

impl Loads for Loader {
    #[cfg_attr(rust_js, rust_js::nullable(module_loading))]
    fn loads(&self, module_loading: Option<&ModuleLoading>) -> bool {
        module_loading.is_some()
    }
}

pub fn through(loader: &dyn Loads, module_loading: Option<&ModuleLoading>) -> bool {
    loader.loads(module_loading)
}

pub fn loose(module_loading: Option<&ModuleLoading>) -> bool {
    module_loading.is_some()
}
`,
  );
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("  if (moduleLoading !== null) {\n");
  expect(js).toContain("  return moduleLoading === null;\n");
  expect(js).toContain('  return prepare(moduleLoading, "a.js") + prepare(null, "b.js");\n');
  // `given` gives what JS gave it, which may be `undefined`, as it is.
  expect(js).toContain("export function givenOn(moduleLoading) {\n  return moduleLoading != null;\n}");
  expect(js).toContain("  return givenOn(moduleLoading);\n");
  expect(js).toContain("export function mapped(moduleLoading) {\n  return moduleLoading != null;\n}");
  expect(js).toContain("export function loose(moduleLoading) {\n  return moduleLoading != null;\n}");
  // A trait's method, which its dictionary calls with what it's given.
  expect(js).toContain("function loaderLoads_loads(loader, moduleLoading) {\n  return moduleLoading != null;\n}");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.prepare(null, "x"), lib.prepare({ prefix: "p/" }, "x")]).toEqual(["", "p/x"]);
  expect([lib.forward(null), lib.forward({ prefix: "p/" })]).toEqual(["", "p/a.js"]);
  expect(lib.prefixed("q/")).toBe("q/c.js");
  expect([lib.cleared(null, false), lib.cleared({ prefix: "" }, false), lib.cleared({ prefix: "" }, true)]).toEqual([
    true,
    false,
    true,
  ]);
  expect([lib.given(undefined), lib.given(null), lib.given({ prefix: "" })]).toEqual([false, false, true]);
  expect(lib.each([undefined, null, { prefix: "" }])).toBe(1);
  expect([lib.loose(undefined), lib.loose(null), lib.loose({ prefix: "" })]).toEqual([false, false, true]);
});

// rd613: a call lowering never sees, a namespace's method that only passes
// its parameter on, which is the function it forwards to (rd292b), leaves
// that function's nullable parameter `!= null`: its callers give it what
// they're given, `checked()` of `checked(None)`, `undefined`.
test("a nullable parameter a forwarding namespace method gives on is tested against undefined too", async () => {
  const dir = fixture("nullable-param-forwarded");
  writeFileSync(
    join(dir, "lib.rs"),
    `pub struct ModuleLoading {
    pub prefix: String,
}

pub mod work_loop {
    use super::ModuleLoading;

    #[cfg_attr(rust_js, rust_js::nullable(module_loading))]
    pub fn checked(module_loading: Option<&ModuleLoading>) -> bool {
        module_loading.is_some()
    }
}

pub mod forwarding {
    use super::ModuleLoading;
    use crate::work_loop;

    #[cfg_attr(rust_js, rust_js::namespace)]
    pub struct Reconciler;

    unsafe impl js::Defined for Reconciler {}

    #[cfg_attr(rust_js, rust_js::namespace)]
    impl Reconciler {
        pub fn checked(&self, module_loading: Option<&ModuleLoading>) -> bool {
            work_loop::checked(module_loading)
        }
    }
}

pub fn uses() -> bool {
    forwarding::Reconciler.checked(None)
}
`,
  );
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  expect(readFileSync(join(dir, "work_loop.js"), "utf8")).toContain("  return moduleLoading != null;\n");
  const lib = await import(join(dir, "lib.js"));
  expect(lib.uses()).toBe(false);
});

// rd623: a `let` that starts `js::null()` and is set only to a place just
// tested `is_some()` is `null` or a value, so the nullable parameter it's
// given is tested `!== null` and another crate's call gives it as it is,
// React DOM's createRoot's `transitionCallbacks`. Set to a place tested
// `is_none()`, or to one another place's test guards, it may be what JS
// gave, `undefined`.
test("a let of null set only to a place tested is_some is null or a value", async () => {
  const dir = fixture("nullable-param-tested-some");
  writeFileSync(
    join(dir, "lib.rs"),
    `use js::Unknown;

pub struct Options {
    pub callbacks: Option<&'static Unknown>,
    pub other: Option<&'static Unknown>,
}

#[cfg_attr(rust_js, rust_js::nullable(callbacks))]
fn create(callbacks: Option<&'static Unknown>) -> bool {
    callbacks.is_some()
}

pub fn root(options: Option<&Options>) -> bool {
    let mut callbacks = js::null();
    if let Some(options) = options {
        if options.callbacks.is_some() {
            callbacks = options.callbacks;
        }
    }
    create(callbacks)
}

#[cfg_attr(rust_js, rust_js::nullable(callbacks))]
fn untested(callbacks: Option<&'static Unknown>) -> bool {
    callbacks.is_some()
}

pub fn other(options: &Options) -> bool {
    let mut callbacks = js::null();
    if options.other.is_some() {
        callbacks = options.callbacks;
    }
    untested(callbacks)
}

#[cfg_attr(rust_js, rust_js::nullable(callbacks))]
fn negated(callbacks: Option<&'static Unknown>) -> bool {
    callbacks.is_some()
}

pub fn none(options: &Options) -> bool {
    let mut callbacks = js::null();
    if options.callbacks.is_none() {
        callbacks = options.callbacks;
    }
    negated(callbacks)
}

#[cfg_attr(rust_js, rust_js::nullable(callbacks))]
fn either(callbacks: Option<&'static Unknown>) -> bool {
    callbacks.is_some()
}

pub fn chosen(c: bool, d: bool) -> bool {
    either(if c { js::null() } else if d { None } else { js::null() })
}
`,
  );
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("function create(callbacks) {\n  return callbacks !== null;\n}");
  expect(js).toContain("function untested(callbacks) {\n  return callbacks != null;\n}");
  expect(js).toContain("function negated(callbacks) {\n  return callbacks != null;\n}");
  // A temporary `js::null()` writes is `null` already, so its `None` is too.
  expect(js).toContain("return either(null);");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.root(undefined), lib.root({}), lib.root({ callbacks: 1 })]).toEqual([false, false, true]);
  expect([lib.other({ other: 1 }), lib.other({ other: 1, callbacks: 1 }), lib.other({ callbacks: 1 })]).toEqual([
    false,
    true,
    false,
  ]);
  expect([lib.none({}), lib.none({ callbacks: 1 })]).toEqual([false, false]);
});

// rd624 (P1, ADR 0861, amended): a nullable parameter given on is `null`
// or a value only where the one it's given is, each passing on followed:
// React's public `updateContainer(element, container)`, called from JS
// with its optional arguments left out, reaches `getContextForSubtree`'s
// `parentComponent` as `undefined`, which is tested `!= null`.
test("a nullable parameter given on what JS may leave out is tested against undefined too", async () => {
  const dir = fixture("nullable-param-given-on");
  writeFileSync(join(dir, "lib.rs"), readFileSync(join(import.meta.dir, "corpus/nullable_param_passed_on.rs"), "utf8"));
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("function getContextForSubtree(parentComponent) {\n  if (parentComponent == null) {");
  expect(js).toContain("function strictContext(parentComponent) {\n  if (parentComponent === null) {");
  const lib = await import(join(dir, "lib.js"));
  const seen: number[] = [];
  expect([lib.updateContainer(1), lib.updateContainer(1, undefined, (n: number) => seen.push(n)), lib.updateContainer(1, { context: 10 })]).toEqual([1, 1, 11]);
  expect(seen).toEqual([1]);
});

// rd632 (ADR 0861, amended): a function of the crate's whose every return
// is `null` or a value, React DOM's `getEventTarget`, gives a nullable
// parameter so, which its body then tests `!== null`, as upstream's
// ScrollEndEventPlugin does; so does a temporary each of whose writes is
// so, and a shared `as_deref()` of such. One that may return a `None` it
// makes or its own nullable parameter, given what JS may leave
// `undefined`, keeps `!= null`. `event_target` comes after `run` and
// `direct`, so each is lowered again once it's found, what it saw of its
// calls before forgotten.
test("a function whose every return is null or a value returns null or a value", async () => {
  const dir = fixture("nulled-returns");
  writeFileSync(join(dir, "lib.rs"), readFileSync(join(import.meta.dir, "corpus/nulled_returns_given_on.rs"), "utf8"));
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("function eventTarget(node) {\n  return node.text ? parentOf(node) : node;\n}");
  // Its call is written as it is, no `?? null`.
  expect(js).toContain("  const target = eventTarget(node);\n");
  expect(js).toContain("function dispatch(target) {\n  return target !== null ? target.id : 0;\n}");
  expect(js).toContain("function dispatchLoose(target) {\n  return target != null ? target.id : 0;\n}");
  // Given the call straight, which `direct`'s first lowering saw before
  // `event_target` was found, and forgot.
  expect(js).toContain("function dispatchDirect(target) {\n  return target !== null ? target.id : 0;\n}");
  expect(js).toContain("function dispatchGivenBack(target) {\n  return target != null ? target.id : 0;\n}");
  expect(js).toContain("function commit(reason) {\n  return reason !== null ?");
  expect(js).toContain("function commitLoose(reason) {\n  return reason != null ?");
  expect(js).toContain("function commitReassigned(reason) {\n  return reason != null ?");
  expect(js).toContain("function placed(instances) {\n  if (instances === null) {");
  const lib = await import(join(dir, "lib.js"));
  // A node JS made leaves out its `parent`, which `parent_of` gives `null`.
  expect(lib.run({ id: 5, text: true }, true)).toEqual([0, 0, 0, 9, 9, 0]);
  expect(lib.run({ id: 1, text: true, parent: { id: 2, text: false } }, false)).toEqual([2, 0, 0, 0, 0, 2]);
  expect([lib.reassigned(undefined, false), lib.reassigned(undefined, true)]).toEqual([0, 3]);
  expect([lib.direct({ id: 5, text: true }), lib.direct({ id: 4, text: false })]).toEqual([0, 4]);
});

// rd632: a `function::bind` of the crate's function with each of its
// parameters bound is a call of it, given what's bound, React's
// `scheduleTimeout(commitRoot.bind(null, root, .., suspendedCommitReason))`:
// its nullable parameter is tested `!== null`. A bind of some of them
// leaves the rest to whatever calls it, and one bound what JS gave may be
// `undefined`: `!= null`.
test("a bind of each parameter is a call of the function", async () => {
  const dir = fixture("nulled-param-bound");
  writeFileSync(join(dir, "lib.rs"), `use std::rc::Rc;

#[cfg_attr(rust_js, rust_js::nullable(reason))]
fn complete(root: u32, reason: Option<&str>) -> u32 {
    match reason {
        Some(reason) => root + reason.len() as u32,
        None => root,
    }
}

#[cfg_attr(rust_js, rust_js::nullable(reason))]
fn partly(root: u32, reason: Option<&str>) -> u32 {
    match reason {
        Some(reason) => root + reason.len() as u32,
        None => root,
    }
}

#[cfg_attr(rust_js, rust_js::nullable(reason))]
fn given_on(root: u32, reason: Option<&str>) -> u32 {
    match reason {
        Some(reason) => root + reason.len() as u32,
        None => root,
    }
}

#[cfg_attr(rust_js, rust_js::nullable)]
fn reason_of(root: u32) -> Option<&'static str> {
    if root > 1 { Some("Throttled") } else { None }
}

pub fn scheduled(root: u32, profiling: bool) -> Rc<dyn Fn() -> u32> {
    let reason = if profiling { reason_of(root) } else { js::null() };
    Rc::from(js::function::bind(complete, None, (root, reason)))
}

pub fn throttled(root: u32) -> Rc<dyn Fn() -> u32> {
    Rc::from(js::function::bind(complete, None, (root, Some("Throttled"))))
}

pub fn later(root: u32) -> Rc<dyn Fn(Option<&'static str>) -> u32> {
    Rc::from(js::function::bind(partly, None, (root,)))
}

pub fn given(root: u32, reason: Option<&'static str>) -> Rc<dyn Fn() -> u32> {
    Rc::from(js::function::bind(given_on, None, (root, reason)))
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("function complete(root, reason) {\n  return reason !== null ?");
  expect(js).toContain("function partly(root, reason) {\n  return reason != null ?");
  expect(js).toContain("function givenOn(root, reason) {\n  return reason != null ?");
  expect(js).toContain("  return complete.bind(undefined, root, reason);\n");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.scheduled(3, true)(), lib.scheduled(1, true)(), lib.scheduled(3, false)(), lib.throttled(1)()]).toEqual([12, 1, 3, 10]);
  expect([lib.later(2)(undefined), lib.later(2)("ab"), lib.given(2, undefined)(), lib.given(2, "ab")()]).toEqual([2, 4, 2, 4]);
});

// rd632 (the user's decision (B)): a `let` a call writes, React's
// `previousMeasurements`, `state.clones?.map(..)` in one arm and a `let` of
// what's cast in the other, read by the crate's own nullable parameter
// through `as_deref()`, and otherwise by `is_some()` and `map_or` alone,
// which read `null` and `undefined` alike, is `null` for `None` where it's
// written, so the parameter is tested `!== null`. Read by a binding's
// parameter that isn't nullable, returned, kept in a field, captured, or
// lent as a `&mut`, it's what the call gave, `undefined` too, and the
// parameter is tested `!= null`.
test("a let a call writes, read by a nullable parameter and alike, is null for None", async () => {
  const dir = fixture("nullable-let-read-alike");
  writeFileSync(join(dir, "lib.rs"), readFileSync(join(import.meta.dir, "corpus/nullable_let_read_alike.rs"), "utf8"));
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("function measure(previous) {\n  return previous !== null ?");
  expect(js).toContain("    previous = clones != null ? measured(clones) : null;\n");
  expect(js).toContain("    previous = previousOf(state, gesture) ?? null;\n");
  expect(js).toContain("function measureMatched(previous) {\n  return previous !== null ?");
  // The cell's read is the `let`'s write, `null` for `None` there, read
  // before the cell is cleared.
  expect(js).toContain("function measureCleared(previous) {\n  return previous !== null ?");
  expect(js).not.toContain("previous$1 ?? null");
  for (const kept of ["Shown", "Returned", "Stored", "Captured", "Lent"]) {
    expect(js).toContain(`function measure${kept}(previous) {\n  return previous != null ?`);
  }
  expect(js.match(/: undefined;\n/g)?.length).toBe(5);
  const lib = await import(join(dir, "lib.js"));
  expect(lib.update({}, true)).toEqual([0, false, 0]);
  expect(lib.update({ clones: [1, 2] }, true)).toEqual([30, true, 2]);
  expect([lib.updateMatched({}, true), lib.updateMatched({}, false), lib.updateMatched({ clones: [1] }, true)]).toEqual([[0, 0], [0, 0], [10, 1]]);
  expect(lib.updateShown({}, true)).toEqual([0, "undefined"]);
  expect(lib.updateReturned({}, true)[1]).toBe(undefined);
  expect(lib.updateStored({}, true)[1].previous).toBe(undefined);
  expect([lib.updateCaptured({}, true), lib.updateLent({}, true, false), lib.updateLent({ clones: [1] }, true, true)]).toEqual([0, 0, 0]);
});

// rd632: a closure that gives its argument as the same JS value, React
// DOM's `parent_node().map(|parent_node| &**parent_node)`, a `Node`'s
// `Deref` to its base `EventTarget`, maps the option to itself, so a
// nullable binding's `null` is given on as it is, and `getEventTarget`'s
// every return is `null` or a value. One that does anything else, counts
// a call say, is made as before, `undefined` for `None`.
test("a closure that gives its argument maps an option to itself", async () => {
  const dir = fixture("nullable-closure-identity-map");
  writeFileSync(
    join(dir, "lib.rs"),
    `use std::cell::Cell;

use webapi::{EventTarget, Node};

thread_local! {
    static MAPPED: Cell<u32> = const { Cell::new(0) };
}

#[cfg_attr(rust_js, rust_js::nullable(target))]
fn dispatch(target: Option<&EventTarget>) -> bool {
    target.is_some()
}

#[cfg_attr(rust_js, rust_js::nullable(target))]
fn dispatch_counted(target: Option<&EventTarget>) -> bool {
    target.is_some()
}

fn event_target(node: &'static Node, text: bool) -> Option<&'static EventTarget> {
    if text { node.parent_node().map(|parent| &**parent) } else { Some(node) }
}

fn counted_target(node: &'static Node) -> Option<&'static EventTarget> {
    node.parent_node().map(|parent| {
        MAPPED.set(MAPPED.get() + 1);
        &**parent
    })
}

pub fn run(node: &'static Node, text: bool) -> (bool, bool, u32) {
    (dispatch(event_target(node, text)), dispatch_counted(counted_target(node)), MAPPED.get())
}
`,
  );
  const withWeb = ["--", "--extern", `webapi=${join(target, "libwebapi.rmeta")}`, "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withWeb]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("  return text ? node.parentNode : node;\n");
  expect(js).toContain("function dispatch(target) {\n  return target !== null;\n}");
  expect(js).toContain("function dispatchCounted(target) {\n  return target != null;\n}");
  const lib = await import(join(dir, "lib.js"));
  expect(lib.run({ parentNode: null }, true)).toEqual([false, false, 0]);
  expect(lib.run({ parentNode: { id: 1 } }, true)).toEqual([true, true, 1]);
});

// rd632: what a binding that's its argument in JS gives, `js::cast` of an
// `Option`, is what it's given: React DOM's hydrateRoot's `match options {
// Some(options) => js::cast(Some(js::unknown(options))), None => js::null() }`
// is `options ?? null` alone, upstream's `options != null ? options : null`,
// and the parameter it's given is tested `!== null`.
test("a cast of an Option is what it's given", async () => {
  const dir = fixture("nullable-cast-given");
  writeFileSync(join(dir, "lib.rs"), `use js::Unknown;

pub struct Options {
    pub prefix: String,
}

unsafe impl js::Defined for Options {}

#[cfg_attr(rust_js, rust_js::nullable(callbacks))]
fn create(callbacks: Option<&'static Unknown>) -> bool {
    callbacks.is_some()
}

pub fn hydrate(options: Option<&'static Options>) -> bool {
    let callbacks = match options {
        Some(options) => unsafe { js::cast(Some(js::unknown(options))) },
        None => js::null(),
    };
    create(callbacks)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("  const callbacks = options ?? null;\n");
  expect(js).toContain("function create(callbacks) {\n  return callbacks !== null;\n}");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.hydrate(undefined), lib.hydrate(null), lib.hydrate({ prefix: "" })]).toEqual([false, false, true]);
});

// rd647 (B), the user's (1): a nullable field is tested `=== null` only
// where it's `null` or a value wherever it's read, of a struct only the
// crate makes. Of one only JS gives, which may leave the field out, it's
// `== null`, and `{}`'s field is `None`; so is one the crate builds that
// JS may give too, to an exported function or to a function a binding
// calls (#519, P0).
test("a nullable field is tested === null only where only the crate makes its struct", async () => {
  const dir = fixture("nullable-field-strict");
  writeFileSync(
    join(dir, "lib.rs"),
    `use std::cell::RefCell;

use js::Unknown;

pub struct Given {
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub x: Option<&'static Unknown>,
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub cell: RefCell<Option<u32>>,
}

pub struct Own {
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub x: Option<&'static Unknown>,
}

pub fn given(g: &Given) -> bool {
    g.x.is_none() && g.cell.borrow().is_none()
}

pub fn own(x: Option<&'static Unknown>) -> bool {
    let own = Own { x };
    own.x.is_none()
}

pub struct Options {
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub nonce: Option<String>,
}

pub fn has_nonce(options: &Options) -> bool {
    options.nonce.is_some()
}

pub fn made() -> bool {
    has_nonce(&Options { nonce: None })
}

pub struct Reply {
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub body: Option<String>,
}

#[cfg_attr(rust_js, rust_js::link_name = "./given.js#call")]
fn call(f: &dyn Fn(&Reply) -> bool) -> bool {
    let _ = f;
    unreachable!()
}

fn has_body(reply: &Reply) -> bool {
    reply.body.is_some()
}

pub fn called() -> bool {
    call(&has_body) || has_body(&Reply { body: None })
}

pub struct Second {
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub body: Option<String>,
}

#[cfg_attr(rust_js, rust_js::link_name = "./given.js#call")]
fn call_closure(f: impl Fn(&Second) -> bool) -> bool {
    let _ = f;
    unreachable!()
}

pub struct Inner {
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub value: Option<String>,
}

pub struct Outer {
    pub inner: Inner,
}

pub fn inner_has(outer: &Outer) -> bool {
    outer.inner.value.is_some()
}

pub fn inner_made() -> bool {
    inner_has(&Outer { inner: Inner { value: None } })
}

pub fn closure_called() -> bool {
    let made = Second { body: None };
    call_closure(|second| second.body.is_some()) || made.body.is_some()
}
`,
  );
  writeFileSync(join(dir, "given.js"), "export const call = (f) => f({});\n");
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("return g.x == null && g.cell == null;");
  expect(js).toContain("return own.x === null;");
  expect(js).toContain("return options.nonce != null;");
  expect(js).toContain("return reply.body != null;");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.hasNonce({}), lib.hasNonce({ nonce: null }), lib.hasNonce({ nonce: "n" }), lib.made()]).toEqual([
    false,
    false,
    true,
    false,
  ]);
  expect(lib.called()).toBe(false);
  expect(js).toContain("call((second) => second.body != null)");
  expect(lib.closureCalled()).toBe(false);
  expect([lib.innerHas({ inner: {} }), lib.innerMade()]).toEqual([false, false]);
  expect([lib.given({}), lib.given({ x: null, cell: null }), lib.given({ x: 1 })]).toEqual([true, true, false]);
  expect([lib.own(undefined), lib.own(1)]).toEqual([true, false]);
});
