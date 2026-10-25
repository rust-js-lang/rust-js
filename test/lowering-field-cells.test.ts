// A struct's `RefCell` field is a place of its own (ADR 0863, amended): the
// value it holds by its declaration, as JS that makes the object writes it,
// whatever else has its type, a generic struct's `RefCell<T>` or a
// thread-local; and where it's lent or a borrow of it held while code that
// may borrow it runs, its one cell, `$fieldCell`, which counts its borrows.
// Each test is a program of its own, compiled and run, given objects JS made.

import { expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { compiler, fixture, run } from "./support";

// React's reconciler: a root JS makes, `FiberRootNode`, whose arrays a port
// indexes, beside the port's generic `StackCursor<T>`; and fibers JS makes
// too, whose `return` the current fiber's thread-local shares a type with.
test("a struct's RefCell field holds its value whatever else has its type", async () => {
  const dir = fixture("refcell-field-places");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::RefCell;
use std::rc::Rc;

pub struct Root {
    pub times: RefCell<Vec<f64>>,
}

pub fn mark(root: &Root, index: usize) {
    let mut times = root.times.borrow_mut();
    times[index] = 1.0;
}

pub fn reset(root: &Root) -> Vec<f64> {
    root.times.replace(Vec::new())
}

pub fn first(root: &Root) -> f64 {
    let times = root.times.borrow();
    let first = times[0];
    drop(times);
    first
}

pub struct Cursor<T> {
    pub current: RefCell<T>,
}

pub fn read<T: Clone>(cursor: &Cursor<T>) -> T {
    cursor.current.borrow().clone()
}

pub struct Fiber {
    pub tag: u8,
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub r#return: RefCell<Option<Rc<Fiber>>>,
}

thread_local! {
    pub static CURRENT: RefCell<Option<Rc<Fiber>>> = const { RefCell::new(None) };
}

pub fn set_current_fiber(fiber: Option<Rc<Fiber>>) {
    CURRENT.set(fiber);
}

pub fn parent_tag(fiber: &Fiber) -> u8 {
    fiber.r#return.borrow().as_ref().map_or(0, |parent| parent.tag)
}

pub fn detach(fiber: &Fiber) {
    fiber.r#return.replace(None);
}
`);
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("  const times = root.times;\n  times[$at(times, index)] = 1;\n");
  expect(js).toContain("  const old = root.times;\n  root.times = [];\n  return old;\n");
  expect(js).toContain('  const borrow = $borrow($fieldCell(cursor, "current"), true);\n');
  expect(js).toContain("export let current;\n");
  expect(js).toContain("  current = fiber;\n");
  expect(js).toContain("  const return$ = fiber.return;\n");
  expect(js).toContain("  fiber.return = null;\n");
  expect(js).not.toContain("$fieldCell(fiber");
  expect(js).not.toContain("$unborrow(times");
  const types = readFileSync(join(dir, "lib.d.ts"), "utf8");
  expect(types).toContain("    times: number[];\n");
  expect(types).toContain("    current: T;\n");
  expect(types).toContain("    return?: Fiber | null | undefined;\n");
  const lib = await import(join(dir, "lib.js"));
  const root = { times: [0, 0, 0] };
  lib.mark(root, 1);
  expect(root.times).toEqual([0, 1, 0]);
  const marked = root.times;
  expect([lib.reset(root), root.times]).toEqual([[0, 1, 0], []]);
  expect(marked).toEqual([0, 1, 0]);
  expect(lib.first({ times: [7] })).toBe(7);
  expect(lib.read({ current: "root" }, { clone: (value: string) => value })).toBe("root");
  const parent = { tag: 3, return: null };
  const child = { tag: 5, return: parent };
  expect([lib.parentTag(child), lib.parentTag(parent)]).toEqual([3, 0]);
  lib.setCurrentFiber(child);
  expect(lib.current).toBe(child);
  lib.detach(child);
  expect([child.return, lib.parentTag(child)]).toEqual([null, 0]);
});

// A field lent, whose borrows may meet: its one cell, which JS's object
// keeps for it, its `value` the property, its nullable `None` `null`.
test("a struct's RefCell field lent is its one cell, of the object JS made", async () => {
  const dir = fixture("refcell-field-lent");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::RefCell;
use std::rc::Rc;

pub struct Fiber {
    pub tag: u8,
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub r#return: RefCell<Option<Rc<Fiber>>>,
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub sibling: RefCell<Option<Rc<Fiber>>>,
}

pub fn lent(fiber: &Fiber) -> &RefCell<Option<Rc<Fiber>>> {
    &fiber.r#return
}

pub fn detach(fiber: &Fiber) {
    fiber.r#return.replace(None);
}

pub fn during(fiber: &Fiber, f: &dyn Fn()) -> u8 {
    let sibling = fiber.sibling.borrow();
    f();
    sibling.as_ref().map_or(0, |sibling| sibling.tag)
}

pub fn unlink(fiber: &Fiber) {
    fiber.sibling.replace(None);
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('  return $fieldCell(fiber, "return", true);\n');
  expect(js).toContain('  const sibling = $borrow($fieldCell(fiber, "sibling", true), true);\n');
  const lib = await import(join(dir, "lib.js"));
  const parent = { tag: 3, return: null, sibling: null };
  const child: { tag: number; return: object | null; sibling: object | null } = { tag: 5, return: parent, sibling: parent };
  const cell = lib.lent(child);
  expect([cell.value, lib.lent(child) === cell, Object.keys(child)]).toEqual([parent, true, ["tag", "return", "sibling"]]);
  lib.detach(child);
  expect([child.return, cell.value]).toEqual([null, null]);
  // One borrowed while code that may borrow it runs, counted on its one
  // cell: the code's `replace` finds it borrowed, as std's does.
  expect(lib.during(child, () => {})).toBe(3);
  expect(() => lib.during(child, () => lib.unlink(child))).toThrow("RefCell already borrowed");
  lib.unlink(child);
  expect(child.sibling).toBe(null);
});

// A transparent struct is its field, the cell (ADR 0584): its `RefCell`
// field holds no value of its own, so a nullable one is refused.
test("a transparent struct's nullable RefCell field is refused", () => {
  const dir = fixture("refcell-field-transparent");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::RefCell;

#[repr(transparent)]
pub struct Slot {
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub held: RefCell<Option<u32>>,
}

pub fn held(slot: &Slot) -> Option<u32> {
    *slot.held.borrow()
}
`);
  expect(() => run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")])).toThrow(
    "this one is a `#[repr(transparent)]` struct's, which is its field, the cell",
  );
});

// React's getComponentNameFromFiber: a shared borrow of a nullable field
// held across a call that only shares borrows of it meets none, so the
// field is the fiber JS made, read as it is.
test("a nullable RefCell field borrowed across a call that only shares it is its value", async () => {
  const dir = fixture("refcell-field-shared-across");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::RefCell;
use std::rc::Rc;

pub struct Fiber {
    pub tag: u8,
    pub name: String,
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub r#return: RefCell<Option<Rc<Fiber>>>,
}

pub fn get_component_name_from_fiber(fiber: &Fiber) -> Option<String> {
    if fiber.tag == 0 {
        return None;
    }
    match &*fiber.r#return.borrow() {
        Some(parent) if parent.tag == 9 => Some(parent.name.clone()),
        _ => Some(fiber.name.clone()),
    }
}

pub fn get_parent_name(fiber: &Fiber) -> Option<String> {
    if let Some(r) = &*fiber.r#return.borrow() {
        return get_component_name_from_fiber(r);
    }
    None
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("  const r = fiber.return;\n  if (r != null) {\n    return getComponentNameFromFiber(r);\n  }\n");
  expect(js).not.toContain("$borrow");
  const lib = await import(join(dir, "lib.js"));
  const root = { tag: 9, name: "Root", return: null };
  const app = { tag: 1, name: "App", return: root };
  const child = { tag: 1, name: "Child", return: app };
  expect([lib.getParentName(child), lib.getParentName(app), lib.getParentName(root)]).toEqual(["Root", "Root", undefined]);
});

// React's getComponentNameFromFiber: `const {tag, type} = fiber;`. Its
// port reads each cell, `let tag = fiber.tag.get(); let r#type =
// fiber.r#type.get();`, a `Cell` field that's its property (ADR 0863): one
// destructuring, each named as the property it reads, as a plain field's
// are; not one named otherwise, `const returnFiber = node.return;`, nor
// past a statement between (ADR 0598, amended).
test("lets side by side of an object's cells, each by its name, are one destructuring", async () => {
  const dir = fixture("cells-read-together");
  writeFileSync(
    join(dir, "lib.rs"),
    `use std::cell::Cell;

pub struct Fiber {
    pub tag: Cell<u32>,
    pub r#type: Cell<u32>,
    pub sibling: Cell<u32>,
    pub r#return: Cell<u32>,
}

pub fn name(fiber: &Fiber) -> u32 {
    let tag = fiber.tag.get();
    let r#type = fiber.r#type.get();
    tag * 10 + r#type
}

pub fn walked(node: &Fiber) -> u32 {
    let sibling = node.sibling.get();
    let return_fiber = node.r#return.get();
    node.r#return.set(sibling);
    let tag = node.tag.get();
    sibling + return_fiber + tag
}
`,
  );
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("  const { tag, type } = fiber;\n  return (((tag * 10) >>> 0) + type) >>> 0;\n");
  expect(js).toContain("  const sibling = node.sibling;\n  const returnFiber = node.return;\n");
  const lib = await import(join(dir, "lib.js"));
  const node = { tag: 1, type: 2, sibling: 3, return: 4 };
  expect([lib.name(node as never), lib.walked(node as never), node.return]).toEqual([12, 8, 3]);
});
