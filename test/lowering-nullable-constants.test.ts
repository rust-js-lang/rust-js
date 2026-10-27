// A `#[rust_js::nullable]` field's `None` is `null` in a value rustc
// computed too, a `const`'s or a `const { .. }` thread-local's, as where
// the struct is made at run time (ADRs 0275, 0586, amended). Each test is a
// program of its own, compiled and run.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { runSync } from "./child";
import { buildReact, compiler, fixture, root, run, target } from "./support";

beforeAll(buildReact, 600_000);

// React's reconciler's `CacheContext`, a `ReactContext<Cache | null>` whose
// `_currentValue: Cell<T>` upstream makes `null`: a `const { .. }`
// thread-local's was `{ count: 0 }`, its `None` left out.
test("a constant's nullable field is null for None", async () => {
  const dir = fixture("nullable-constants");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::{Cell, RefCell};

pub struct Context<T: 'static> {
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub value: Cell<T>,
    pub count: Cell<f64>,
}

pub struct Cache {
    pub ref_count: Cell<i32>,
}

pub struct Plain {
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub label: Option<&'static str>,
    #[cfg_attr(rust_js, rust_js::nullable, rust_js::optional)]
    pub renderer: Cell<Option<u8>>,
    pub other: Option<u8>,
}

pub struct Slot {
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub items: RefCell<Option<u8>>,
}

thread_local! {
    pub static CACHE_CONTEXT: Context<Option<&'static Cache>> = const {
        Context {
            value: Cell::new(None),
            count: Cell::new(0.0),
        }
    };
    pub static NUMBERS: Context<f64> = const {
        Context {
            value: Cell::new(1.0),
            count: Cell::new(0.0),
        }
    };
}

pub const PLAIN: Plain = Plain {
    label: None,
    renderer: Cell::new(None),
    other: None,
};

pub const SLOT: Slot = Slot {
    items: RefCell::new(None),
};

pub fn cached() -> bool {
    CACHE_CONTEXT.with(|context| context.value.get().is_some())
}

pub fn labelled() -> bool {
    PLAIN.label.is_some() || SLOT.items.borrow().is_some()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("export const cacheContext = { value: null, count: 0 };\n");
  expect(js).toContain("export const numbers = { value: 1, count: 0 };\n");
  expect(js).toContain("export const PLAIN = {\n  label: null,\n};\n");
  expect(js).toContain("export const SLOT = {\n  items: null,\n};\n");
  const lib = await import(join(dir, "lib.js"));
  expect(lib.cacheContext).toEqual({ value: null, count: 0 });
  expect(lib.PLAIN.label).toBeNull();
  expect(Object.keys(lib.PLAIN)).not.toContain("renderer");
  expect(Object.keys(lib.PLAIN)).not.toContain("other");
  expect([lib.cached(), lib.labelled()]).toEqual([false, false]);
});

// The reconciler's `CacheContext` is a `ReactContext` of the react crate's:
// a library's struct's nullable field is `null` in a consumer's constant.
test("a constant of a library's struct has its nullable field's None null", async () => {
  const dir = fixture("nullable-constants-library");
  const js = ["--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  writeFileSync(join(dir, "dep.rs"), `use std::cell::Cell;

pub struct Context<T: 'static> {
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub value: Cell<T>,
    pub count: Cell<f64>,
}

pub fn count<T>(context: &Context<T>) -> f64 {
    context.count.get()
}
`);
  run([compiler, join(dir, "dep.rs"), "-o", join(dir, "dep", "lib.js"), "--library", "--manifest", join(dir, "dep", "lib.manifest.json"),
    "--", "--crate-name", "dep", `--emit=metadata=${join(dir, "libdep.rmeta")}`, ...js]);
  writeFileSync(join(dir, "app.rs"), `use std::cell::Cell;

use dep::Context;

thread_local! {
    pub static CACHE_CONTEXT: Context<Option<&'static str>> = const {
        Context {
            value: Cell::new(None),
            count: Cell::new(0.0),
        }
    };
}

pub fn counted() -> f64 {
    CACHE_CONTEXT.with(dep::count)
}
`);
  run([compiler, join(dir, "app.rs"), "-o", join(dir, "app", "lib.js"), "--dependency", join(dir, "dep", "lib.manifest.json"),
    "--", "--crate-name", "app", "--extern", `dep=${join(dir, "libdep.rmeta")}`, "-L", dir, ...js]);
  expect(readFileSync(join(dir, "app", "lib.js"), "utf8")).toContain("export const cacheContext = { value: null, count: 0 };\n");
  const lib = await import(join(dir, "app", "lib.js"));
  expect([lib.cacheContext.value, lib.counted()]).toEqual([null, 0]);
});

// React's Fiber has fields of `any` upstream, `memoizedState`,
// `updateQueue`, `stateNode`, and typed ones, `_debugStack: Error | null`,
// `_debugHookTypes: Array<HookType> | null`, a marker's `name: string |
// null`, made `null` and then holding what JS writes, `undefined` too. A
// plain `Option` field made `None` was left out, which React's dev mode,
// whose objects keep their shape, can't add later; a nullable one wrote
// `?? null` on every store, a stored `undefined` read back `null`.
// `js::null()` is `None` to Rust and `null` where it's written, of any
// `Option` (ADR 0935, amended).
test("js::null() is null where it's written, of any Option, and what's stored after is as it is", async () => {
  const dir = fixture("js-null");
  const js = ["--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  writeFileSync(join(dir, "lib.rs"), `use std::cell::{Cell, RefCell};
use std::rc::Rc;

use js::{JsError, Unknown};

pub struct Fiber {
    pub memoized_state: Cell<Option<&'static Unknown>>,
    pub memoized_props: Option<&'static Unknown>,
    pub update_queue: Cell<Option<&'static Unknown>>,
    pub debug_stack: Cell<Option<&'static JsError>>,
    pub debug_hook_types: RefCell<Option<Rc<RefCell<Vec<String>>>>>,
    pub name: Option<String>,
}

pub const EMPTY: Fiber = Fiber {
    memoized_state: Cell::new(js::null()),
    memoized_props: js::null(),
    update_queue: Cell::new(None),
    debug_stack: Cell::new(js::null()),
    debug_hook_types: RefCell::new(None),
    name: js::null(),
};

thread_local! {
    static CURRENT: Cell<Option<&'static Unknown>> = const { Cell::new(js::null()) };
}

pub fn create_fiber() -> Fiber {
    Fiber {
        memoized_state: Cell::new(js::null()),
        memoized_props: js::null(),
        update_queue: Cell::new(None),
        debug_stack: Cell::new(js::null()),
        debug_hook_types: RefCell::new(js::null()),
        name: js::null(),
    }
}

pub fn set_state(fiber: &Fiber, state: Option<&'static Unknown>) {
    fiber.memoized_state.set(state);
}

pub fn copy(work_in_progress: &Fiber, current: &Fiber) {
    work_in_progress.debug_stack.set(current.debug_stack.get());
    let debug_hook_types = current.debug_hook_types.borrow().clone();
    work_in_progress.debug_hook_types.replace(debug_hook_types);
}

pub fn reset(fiber: &Fiber) {
    fiber.memoized_state.set(js::null());
    fiber.debug_hook_types.replace(js::null());
}

pub fn has_state(fiber: &Fiber) -> bool {
    fiber.memoized_state.get().is_some()
}

pub fn same_state(fiber: &Fiber, other: &Fiber) -> bool {
    fiber.memoized_state.get() == other.memoized_state.get()
}

pub fn named(fiber: &Fiber) -> bool {
    fiber.name.is_some()
}

pub fn same_name(fiber: &Fiber, other: &Fiber) -> bool {
    fiber.name == other.name
}

pub fn hook_types(fiber: &Fiber) -> usize {
    fiber.debug_hook_types.borrow().as_ref().map_or(0, |types| types.borrow().len())
}

pub fn current_state() -> Option<&'static Unknown> {
    CURRENT.get()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", ...js]);
  const out = readFileSync(join(dir, "lib.js"), "utf8");
  expect(out).toContain("const current = null;\n");
  expect(out).toContain("  fiber.memoizedState = state;\n");
  expect(out).toContain("  fiber.memoizedState = null;\n");
  expect(out).toContain("  workInProgress.debugStack = current.debugStack;\n");
  expect(out).toContain("  workInProgress.debugHookTypes = current.debugHookTypes;\n");
  expect(out).toContain("  fiber.debugHookTypes = null;\n");
  expect(out).not.toContain("?? null;\n");
  const lib = await import(join(dir, "lib.js"));
  const keys = ["memoizedState", "memoizedProps", "debugStack", "debugHookTypes", "name"];
  expect(Object.entries(lib.EMPTY)).toEqual(keys.filter((k) => k !== "debugHookTypes").map((k) => [k, null]));
  const fiber = lib.createFiber();
  expect(Object.entries(fiber)).toEqual(keys.map((k) => [k, null]));
  expect([lib.hasState(fiber), lib.named(fiber), lib.hookTypes(fiber), lib.currentState()]).toEqual([false, false, 0, null]);
  // What JS stores is kept as it is: `undefined` isn't made `null`.
  lib.setState(fiber, undefined);
  expect("memoizedState" in fiber && fiber.memoizedState === undefined).toBe(true);
  const other = { debugStack: undefined, debugHookTypes: undefined };
  lib.copy(fiber, other);
  expect([fiber.debugStack, fiber.debugHookTypes]).toEqual([undefined, undefined]);
  // Either is `None` to the program, as to Rust.
  const made = lib.createFiber();
  expect([lib.hasState(fiber), lib.sameState(fiber, made), lib.hookTypes(fiber)]).toEqual([false, true, 0]);
  fiber.name = undefined;
  expect([lib.named(fiber), lib.sameName(fiber, made)]).toEqual([false, true]);
  fiber.name = "a";
  expect([lib.named(fiber), lib.sameName(fiber, made)]).toEqual([true, false]);
  lib.setState(fiber, 0);
  expect([lib.hasState(fiber), fiber.memoizedState, lib.sameState(fiber, made)]).toEqual([true, 0, false]);
  lib.reset(fiber);
  expect([fiber.memoizedState, fiber.debugHookTypes]).toEqual([null, null]);
});

// React's getFragmentInstanceOrTextInstanceSiblings: `const result: [Fiber
// | null, Fiber | null] = [null, null];`, which ReactFiberConfigDOM reads
// `hostSiblings[0] || hostSiblings[1]` of and tests `=== null`. An array's
// or a tuple's `js::null()` is `null` where it's written, as a field's is,
// and what's stored in it after is as it is; a `#[rust_js::nullable]`
// function returning one is refused, its error naming `js::null()` (ADR
// 0935, amended).
test("an array's and a tuple's js::null() are null, and a nullable function of one is refused", async () => {
  const dir = fixture("js-null-arrays");
  const js = ["--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  writeFileSync(join(dir, "lib.rs"), `use std::rc::Rc;

pub struct Fiber {
    pub tag: u8,
}

pub fn siblings(fiber: &Rc<Fiber>) -> [Option<Rc<Fiber>>; 2] {
    let mut result = [js::null(), js::null()];
    if fiber.tag == 0 {
        return result;
    }
    fill(&mut result, fiber);
    result
}

fn fill(result: &mut [Option<Rc<Fiber>>; 2], fiber: &Rc<Fiber>) {
    result[1] = Some(Rc::clone(fiber));
}

pub fn pair(fiber: &Rc<Fiber>) -> (Option<Rc<Fiber>>, Option<u8>) {
    if fiber.tag == 0 {
        return (js::null(), js::null());
    }
    (Some(Rc::clone(fiber)), None)
}

pub fn found(fiber: &Rc<Fiber>) -> bool {
    let [before, after] = siblings(fiber);
    before.or(after).is_some()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", ...js]);
  const out = readFileSync(join(dir, "lib.js"), "utf8");
  expect(out).toContain("  const result = [null, null];\n");
  expect(out).toContain("    return [null, null];\n");
  const lib = await import(join(dir, "lib.js"));
  const empty = { tag: 0 };
  const fiber = { tag: 1 };
  expect(lib.siblings(empty)).toEqual([null, null]);
  // ReactFiberConfigDOM's read of it.
  const hostSiblings = lib.siblings(empty);
  expect((hostSiblings[0] || hostSiblings[1]) === null).toBe(true);
  expect(lib.siblings(fiber)).toEqual([null, fiber]);
  expect([lib.pair(empty), lib.pair(fiber)]).toEqual([
    [null, null],
    [fiber, undefined],
  ]);
  expect([lib.found(empty), lib.found(fiber)]).toEqual([false, true]);
  writeFileSync(join(dir, "lib.rs"), `#[rust_js::nullable]
pub fn siblings() -> [Option<u8>; 2] {
    [None, None]
}
`);
  const refused = runSync([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")], root, 60_000);
  expect(refused.code).not.toBe(0);
  expect(refused.stderr).toContain(
    "`#[rust_js::nullable]` of a function is of its result, which isn't an `Option`; a `None` in it that's `null` is written `js::null()`",
  );
});

// A plain `Option` field a `js::null()` is written to may hold `null`, so
// its `.d.ts` says so, React's `_debugStack: Error | null`; every other
// plain `Option` field is `?: T | undefined`, as @types/react's props are
// (ADR 0935, amended). Written where it's made, by an assignment, through
// a `&mut`, by a call it's lent to with one, and in a constant. A cell's
// is its `Option`'s, `T | null | undefined`, as it was.
test("a plain Option field js::null() is written to is declared | null, and no other", () => {
  const dir = fixture("js-null-declared");
  const js = ["--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  writeFileSync(join(dir, "lib.rs"), `use std::cell::Cell;

pub struct Fiber {
    pub name: Option<String>,
    pub key: Option<String>,
    pub debug_stack: Cell<Option<u32>>,
    pub index: Option<u32>,
    pub title: Option<String>,
    pub ref_: Option<u32>,
}

pub struct Props {
    pub on_click: Option<fn()>,
    pub label: Option<String>,
}

pub const EMPTY: Props = Props {
    on_click: None,
    label: js::null(),
};

pub fn create_fiber(key: Option<String>) -> Fiber {
    Fiber {
        name: js::null(),
        key,
        debug_stack: Cell::new(js::null()),
        index: None,
        title: None,
        ref_: None,
    }
}

pub fn reset(fiber: &mut Fiber) -> Option<String> {
    fiber.index = js::null();
    let r#ref = &mut fiber.ref_;
    *r#ref = js::null();
    std::mem::replace(&mut fiber.title, js::null())
}
`);
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", ...js]);
  const ts = readFileSync(join(dir, "lib.d.ts"), "utf8");
  expect(ts).toContain("    name?: string | null | undefined;\n");
  expect(ts).toContain("    key?: string | undefined;\n");
  expect(ts).toContain("    debugStack: number | null | undefined;\n");
  expect(ts).toContain("    index?: number | null | undefined;\n");
  expect(ts).toContain("    title?: string | null | undefined;\n");
  expect(ts).toContain("    ref_?: number | null | undefined;\n");
  expect(ts).toContain("    onClick?: (() => void) | undefined;\n");
  expect(ts).toContain("    label?: string | null | undefined;\n");
});
