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
`,
  );
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("function create(callbacks) {\n  return callbacks !== null;\n}");
  expect(js).toContain("function untested(callbacks) {\n  return callbacks != null;\n}");
  expect(js).toContain("function negated(callbacks) {\n  return callbacks != null;\n}");
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
