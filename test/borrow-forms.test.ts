// What a borrow used for a moment and `Vec::remove` are, written as a
// person writes them, with no call where nothing fails (#1692, ADR 1141):
// a momentary borrow, held only while nothing can ask (rd218), checked
// first where it's evaluated first in its statement, `if (nodes.borrows)
// $alreadyBorrowed();`, then the cell, else the call as before; a remove's bounds checked where it's called, then JS's own
// `splice`, its item read only where something takes it. A guard held
// while code of the program's runs, a `Drop`'s, is counted as before.
// Each answer is native Rust's.
import { expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { runSync } from "./child";
import { compiler, fixture, run } from "./support";

const program = `use std::cell::RefCell;
use std::panic::{catch_unwind, AssertUnwindSafe};

fn consume(nodes: &RefCell<Vec<u32>>, i: usize) {
    nodes.borrow_mut().remove(i);
}

fn message(payload: Box<dyn std::any::Any + Send>) -> String {
    match payload.downcast_ref::<String>() {
        Some(text) => text.clone(),
        None => payload.downcast_ref::<&str>().map_or(String::new(), |text| text.to_string()),
    }
}

pub fn removed() -> String {
    let nodes = RefCell::new(vec![1, 2, 3, 4]);
    consume(&nodes, 1);
    let first = nodes.borrow_mut().remove(0);
    format!("{first} {:?}", nodes.borrow())
}

pub fn out_of_bounds() -> String {
    let nodes = RefCell::new(vec![1]);
    let result = catch_unwind(AssertUnwindSafe(|| consume(&nodes, 3)));
    result.err().map_or(String::new(), message)
}

pub fn already_borrowed() -> String {
    let nodes = RefCell::new(vec![1]);
    let held = nodes.borrow();
    let result = catch_unwind(AssertUnwindSafe(|| consume(&nodes, 0)));
    drop(held);
    result.err().map_or(String::new(), message)
}

fn summed(cell: &RefCell<Vec<u32>>) -> u32 {
    let mut total = 0;
    let mut i = 0;
    while i < cell.borrow().len() {
        total += cell.borrow()[i];
        i += 1;
    }
    total
}

fn guarded(cell: &RefCell<Vec<u32>>, look: bool) -> bool {
    look && !cell.borrow().is_empty()
}

fn chosen(cell: &RefCell<Vec<u32>>, look: bool) -> usize {
    if look { cell.borrow().len() } else { 0 }
}

pub fn loops_and_short_circuits() -> String {
    let cell = RefCell::new(vec![1, 2, 3]);
    let total = summed(&cell);
    let held = cell.borrow_mut();
    let (look, length) = (guarded(&cell, false), chosen(&cell, false));
    drop(held);
    format!("{total} {look} {length}")
}

struct Pusher<'a>(&'a RefCell<Vec<u32>>);

impl Drop for Pusher<'_> {
    fn drop(&mut self) {
        self.0.borrow_mut().push(9);
    }
}

fn counted_while_dropping(cell: &RefCell<Vec<u32>>) -> usize {
    cell.borrow().iter().map(|_| Pusher(cell)).count()
}

fn total(cell: &RefCell<Vec<u32>>, each: impl Fn(u32) -> u32) -> u32 {
    cell.borrow().iter().map(|&n| each(n)).sum()
}

fn bump(cell: &RefCell<Vec<u32>>, each: impl Fn(u32) -> u32) {
    cell.borrow_mut().iter_mut().for_each(|n| *n = each(*n));
}

pub fn counted_while_mutably_held() -> String {
    let cell = RefCell::new(vec![1, 2]);
    let held = cell.borrow_mut();
    let result = catch_unwind(AssertUnwindSafe(|| total(&cell, |n| n * 2)));
    drop(held);
    format!("{} {}", result.err().map_or(String::new(), message), total(&cell, |n| n))
}

pub fn counted_while_held() -> String {
    let cell = RefCell::new(vec![1, 2]);
    let held = cell.borrow();
    let result = catch_unwind(AssertUnwindSafe(|| bump(&cell, |n| n + 1)));
    drop(held);
    format!("{} {:?}", result.err().map_or(String::new(), message), cell.borrow())
}

pub fn borrowed_across_a_drop() -> String {
    let cell = RefCell::new(vec![1, 2]);
    let result = catch_unwind(AssertUnwindSafe(|| counted_while_dropping(&cell)));
    format!("{} {:?}", result.err().map_or(String::new(), message), cell.borrow())
}
`;

function compiled(name: string): string {
  const dir = fixture(name);
  writeFileSync(join(dir, "lib.rs"), program);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  return dir;
}

function ran(dir: string, name: string): string {
  const call = name.replace(/_(.)/g, (_, c: string) => c.toUpperCase());
  writeFileSync(join(dir, "main.js"), `import { ${call} } from "./lib.js";\nconsole.log(${call}());\n`);
  return runSync(["node", join(dir, "main.js")], dir, 60_000).stdout;
}

test("a momentary borrow and Vec::remove are checked where they're called, then JS's own", () => {
  const dir = compiled("borrow-forms");
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain(
    "function consume(nodes, i) {\n  if (nodes.borrows) {\n    $alreadyBorrowed();\n  }\n  if (i >= nodes.length) {\n    $removeOutOfBounds(nodes, i);\n  }\n  nodes.splice(i, 1);\n}",
  );
  expect(js).toContain("const first = nodes.splice(0, 1)[0];");
  expect(ran(dir, "removed")).toBe("1 [3, 4]\n");
  expect(ran(dir, "out_of_bounds")).toBe("removal index (is 3) should be < len (is 1)\n");
  expect(ran(dir, "already_borrowed")).toBe("RefCell already borrowed\n");
});

// A momentary borrow a loop's test evaluates again, or a short circuit's
// or a conditional's branch evaluates only where it's taken, is the call,
// as before (ADR 0328): the loop's header as Rust's, and no check where
// Rust doesn't borrow, a mutable borrow held then.
test("a momentary borrow in a loop's test or a branch is the call, as before", () => {
  const dir = compiled("borrow-forms-tests");
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("  for (let i = 0; i < $borrow(cell).length; i++) {\n");
  expect(js).toContain("  return look && $borrow(cell).length !== 0;\n");
  expect(js).toContain("  return look ? $borrow(cell).length : 0;\n");
  expect(ran(dir, "loops_and_short_circuits")).toBe("6 false 0\n");
});

// A guard held while a `Drop` of the program's runs, which may borrow its
// cell, is counted, `$borrow(cell, true)` and its `$unborrow`, so the
// drop's borrow panics as Rust's does.
test("a guard held while a Drop runs is counted, as it was", () => {
  const dir = compiled("borrow-forms-counted");
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("const borrow = $borrow(cell, true);");
  expect(js).toContain("$unborrow(borrow);");
  expect(ran(dir, "borrowed_across_a_drop")).toBe("RefCell already borrowed [1, 2]\n");
  // A counted guard taken where another is held panics as Rust's does.
  expect(ran(dir, "counted_while_mutably_held")).toBe("RefCell already mutably borrowed 3\n");
  expect(ran(dir, "counted_while_held")).toBe("RefCell already borrowed [1, 2]\n");
});
