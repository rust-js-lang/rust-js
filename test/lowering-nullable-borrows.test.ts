// A `#[rust_js::nullable]` place stores `null` for `None` however it's
// written: through what a thread-local's `with` and `with_borrow_mut`
// lend, and through a `&mut` to a field or a parameter, a callee's box
// copied back (ADR 1078, #541). What its reads give JS, a nullable
// function's result, is `null` then, as its `.d.ts` says. Each test is a
// program of its own, compiled and run.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildReact, compiler, fixture, installStd, run, target } from "./support";

beforeAll(buildReact, 600_000);

test("a nullable place written through a borrow stores null, which a nullable function returns", async () => {
  const dir = fixture("nullable-borrows");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::{Cell, RefCell};

thread_local! {
    #[cfg_attr(rust_js, rust_js::nullable)]
    static CURRENT: Cell<Option<u32>> = const { Cell::new(None) };
    #[cfg_attr(rust_js, rust_js::nullable)]
    static TARGET: RefCell<Option<u32>> = const { RefCell::new(None) };
    #[cfg_attr(rust_js, rust_js::nullable)]
    static COUNTED: RefCell<Option<u32>> = const { RefCell::new(None) };
    #[cfg_attr(rust_js, rust_js::nullable)]
    static MADE: RefCell<Option<Vec<u32>>> = const { RefCell::new(None) };
}

fn made() -> Vec<u32> {
    vec![1, 2]
}

pub fn made_len() -> usize {
    MADE.with_borrow_mut(|m| m.get_or_insert_with(made).len())
}

fn none_of(n: u32) -> Option<u32> {
    if n > 3 { Some(n) } else { None }
}

pub struct Fiber {
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub owner: Option<u32>,
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub lanes: Vec<Option<u32>>,
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub target: RefCell<Option<u32>>,
}

fn clear_slot(slot: &mut Option<u32>, n: u32) {
    *slot = none_of(n);
}

fn clear_generic<T>(slot: &mut Option<T>) {
    *slot = None;
}

pub fn set(n: u32) {
    CURRENT.set(Some(n));
    TARGET.set(Some(n));
    COUNTED.set(Some(n));
}

pub fn clear(n: u32) {
    CURRENT.with(|c| c.set(none_of(n)));
    TARGET.with_borrow_mut(|t| *t = none_of(n));
    COUNTED.with(|t| {
        drop(t.borrow());
        *t.borrow_mut() = none_of(n);
    });
}

pub fn clear_through_generic() {
    TARGET.with_borrow_mut(clear_generic);
    COUNTED.with_borrow_mut(|t| std::mem::take(t));
}

#[cfg_attr(rust_js, rust_js::nullable)]
pub fn current() -> Option<u32> {
    CURRENT.get()
}

#[cfg_attr(rust_js, rust_js::nullable)]
pub fn target() -> Option<u32> {
    TARGET.with_borrow(Clone::clone)
}

#[cfg_attr(rust_js, rust_js::nullable)]
pub fn counted() -> Option<u32> {
    COUNTED.with_borrow(Clone::clone)
}

pub fn make() -> Fiber {
    Fiber { owner: Some(3), lanes: vec![Some(1)], target: RefCell::new(Some(2)) }
}

pub fn clear_parts(fiber: &mut Fiber) {
    clear_generic(&mut fiber.lanes[0]);
    clear_generic(fiber.target.get_mut());
}

pub fn clear_owner(fiber: &mut Fiber, n: u32) {
    clear_slot(&mut fiber.owner, n);
}

pub fn clear_owner_generic(fiber: &mut Fiber) {
    clear_generic(&mut fiber.owner);
}

pub fn swap_owner(fiber: &mut Fiber, n: u32) {
    let mut other = none_of(n);
    std::mem::swap(&mut fiber.owner, &mut other);
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--crate-type=lib"]);
  installStd(dir);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  // A module's variable, written through what's lent, stores so; a cell
  // whose borrows are counted stores so itself.
  expect(js).toContain("  CURRENT = noneOf(n) ?? null;\n  TARGET = noneOf(n) ?? null;\n");
  expect(js).toContain("const COUNTED = $nullableCell(null);\n");
  // What a `get_or_insert_with` puts there is a `Some`'s, stored as it is.
  expect(js).toContain("  MADE ??= made();\n");
  // A callee's box copied back, and a swap, as the field stores.
  expect(js).toContain("  clearSlot(slot, n);\n  fiber.owner = slot.value ?? null;\n");
  expect(js).toContain("  fiber.owner = other ?? null;\n");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.madeLen(), lib.madeLen()]).toEqual([2, 2]);
  lib.set(5);
  expect([lib.current(), lib.target(), lib.counted()]).toEqual([5, 5, 5]);
  lib.clear(1);
  expect([lib.current(), lib.target(), lib.counted()]).toStrictEqual([null, null, null]);
  lib.set(5);
  lib.clearThroughGeneric();
  expect([lib.target(), lib.counted()]).toStrictEqual([null, null]);
  const fiber = lib.make();
  lib.clearOwner(fiber, 1);
  expect(fiber.owner).toBe(null);
  fiber.owner = 3;
  lib.clearOwnerGeneric(fiber);
  expect(fiber.owner).toBe(null);
  fiber.owner = 3;
  lib.swapOwner(fiber, 1);
  expect(fiber.owner).toBe(null);
  // An item of a nullable field's items, and a nullable `RefCell` field's
  // `get_mut()`, given on.
  lib.clearParts(fiber);
  expect([fiber.lanes, fiber.target]).toStrictEqual([[null], null]);
});

// An `Option<&T>`'s `cloned()` is a clone of the `Option<T>`, which gives
// its `None` as it is (`clone_keeps_none`, the clone owner's): of what's
// `null` or a value, `null` or a value. So a nullable parameter each call
// gives one, a `bind` of a `cloned()` of a nullable thread-local's value,
// is tested `!== null` alone, React's completeRoot, `recoverableErrors !==
// null`, upstream's, where `!= null` was; and the scheduler's `currentTask
// = peek(taskQueue)` of `with_borrow(|q| peek(q).cloned())` is set as it
// is (ADR 0861, amended).
test("a cloned() of what's null or a value is null or a value", async () => {
  const dir = fixture("nullable-with-borrow-cloned");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::RefCell;
use std::rc::Rc;

pub struct Task {
    pub id: u32,
}

thread_local! {
    static TASK_QUEUE: RefCell<Vec<&'static Task>> = const { RefCell::new(Vec::new()) };
    #[cfg_attr(rust_js, rust_js::nullable)]
    static CURRENT_TASK: RefCell<Option<&'static Task>> = const { RefCell::new(None) };
    #[cfg_attr(rust_js, rust_js::nullable)]
    static PENDING_TASK: RefCell<Option<&'static Task>> = const { RefCell::new(None) };
}

#[cfg_attr(rust_js, rust_js::nullable)]
fn peek<'a>(heap: &'a [&'static Task]) -> Option<&'a &'static Task> {
    heap.first()
}

pub fn push(id: u32) {
    TASK_QUEUE.with_borrow_mut(|queue| queue.push(Box::leak(Box::new(Task { id }))));
}

pub fn advance() -> Option<u32> {
    CURRENT_TASK.set(TASK_QUEUE.with_borrow(|queue| peek(queue).cloned()));
    CURRENT_TASK.with_borrow(|task| task.map(|task| task.id))
}

#[cfg_attr(rust_js, rust_js::nullable(task))]
fn complete(root: u32, task: Option<&'static Task>) -> u32 {
    PENDING_TASK.set(task);
    match task {
        Some(task) => root + task.id,
        None => root,
    }
}

pub fn scheduled(root: u32) -> Rc<dyn Fn() -> u32> {
    let current = CURRENT_TASK.with_borrow(Clone::clone);
    Rc::from(js::function::bind(complete, None, (root, current.as_ref().cloned())))
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--crate-type=lib", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  installStd(dir);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("  currentTask = peek(taskQueue);\n");
  expect(js).toContain("  pendingTask = task;\n  return task !== null ?");
  expect(js).toContain("  return complete.bind(undefined, root, current);\n");
  const lib = await import(join(dir, "lib.js"));
  expect(lib.advance()).toBe(undefined);
  expect(lib.scheduled(1)()).toBe(1);
  lib.push(7);
  expect(lib.advance()).toBe(7);
  expect(lib.scheduled(1)()).toBe(8);
});

// React's Flight `ResponseInstance`: `this._debugRootStack = rootOwner !=
// null ? new Error(..) : null`, a nullable field's `set(if owner.is_some()
// { Some(..) } else { None })`, whose branches tidy joins to `t ? v :
// undefined`: stored by its form, `t ? v : null`, not `(t ? v : undefined)
// ?? null` (`store_again`, ADR 0841, amended, #2112).
test("a nullable place's write of an if's Some and None is stored as one conditional, its None null", async () => {
  const dir = fixture("nullable-joined-write");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::Cell;

pub struct Response {
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub root_owner: Cell<Option<&'static str>>,
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub root_stack: Cell<Option<&'static str>>,
}

pub fn start(response: &Response, owner: Option<&'static str>) {
    response.root_owner.set(owner);
    response.root_stack.set(if owner.is_some() { Some("react-stack-top-frame") } else { None });
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--crate-type=lib", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('  response.rootStack = owner != null ? "react-stack-top-frame" : null;\n');
  const lib = await import(join(dir, "lib.js"));
  const response: { rootOwner: string | null; rootStack: string | null } = { rootOwner: null, rootStack: null };
  lib.start(response, undefined);
  expect(response).toEqual({ rootOwner: null, rootStack: null });
  lib.start(response, "App");
  expect(response).toEqual({ rootOwner: "App", rootStack: "react-stack-top-frame" });
});
