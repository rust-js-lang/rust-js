// A `RefCell`'s guard mapped to part of what it guards (#1275): what its
// form can't tell apart is refused as it runs, never a release of the
// wrong cell. test/corpus/ref_cell_guards_mapped.rs holds what runs.

import { expect, test } from "bun:test";
import { writeFileSync } from "node:fs";
import { join } from "node:path";

import { compiler, fixture, run } from "./support";

/** `source`, compiled, its `f` run: what it gave, or what it threw. */
async function ran(name: string, source: string): Promise<{ value?: unknown; error?: Error }> {
  const dir = fixture(name);
  writeFileSync(join(dir, "lib.rs"), source);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const lib = await import(join(dir, "lib.js"));
  try {
    return { value: lib.f() };
  } catch (error) {
    return { error: error as Error };
  }
}

// Two cells reaching one object, each mapped to it: one guard object, two
// cells it would release, which its drop can't tell apart.
test("two cells' guards mapped to one object are refused as they're made", async () => {
  const { error } = await ran(
    "guard-maps-two-cells",
    `use std::cell::{Ref, RefCell};
use std::rc::Rc;

pub struct Point {
    pub x: u32,
}

pub fn f() -> u32 {
    let shared = Rc::new(Point { x: 7 });
    let c1 = RefCell::new(Rc::clone(&shared));
    let c2 = RefCell::new(Rc::clone(&shared));
    let a = Ref::map(c1.borrow(), |r| &**r);
    let b = Ref::map(c2.borrow(), |r| &**r);
    a.x + b.x
}
`,
  );
  expect(error?.message).toContain("two cells' guards mapped to one object");
  expect(error?.message).toContain("#1275");
  // Not a panic a `catch_unwind` would take for the program's.
  expect(error?.constructor).not.toBe(Error);
});

// A guard mapped to a cell, whose own guards' release would meet the
// mapped one's.
test("a guard mapped to a cell is refused as it's made", async () => {
  const { error } = await ran(
    "guard-maps-to-a-cell",
    `use std::cell::{Ref, RefCell};

pub struct Outer {
    pub inner: RefCell<Vec<u32>>,
}

pub fn f() -> usize {
    let outer = RefCell::new(Outer { inner: RefCell::new(vec![1, 2]) });
    let inner = Ref::map(outer.borrow(), |o| &o.inner);
    let v = inner.borrow();
    v.len()
}
`,
  );
  expect(error?.message).toContain("a guard mapped to a cell");
  expect(error?.message).toContain("#1275");
});
