// A `#[rust_js::nullable]` place stores `null` for `None` however it's
// written: through what a thread-local's `with` and `with_borrow_mut`
// lend, and through a `&mut` to a field or a parameter, a callee's box
// copied back (ADR 1078, #541). What its reads give JS, a nullable
// function's result, is `null` then, as its `.d.ts` says. Each test is a
// program of its own, compiled and run.

import { expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { compiler, fixture, installRuntime, run } from "./support";

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
  installRuntime(dir);
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
