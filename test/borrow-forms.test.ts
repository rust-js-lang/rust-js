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

fn only_child_of(chunks: &RefCell<Vec<u32>>, children: &RefCell<Vec<u32>>) -> Option<u32> {
    let children = children.borrow();
    (chunks.borrow().is_empty() && children.len() == 1).then(|| children[0])
}

pub fn only_child() -> String {
    let (chunks, children) = (RefCell::new(Vec::new()), RefCell::new(vec![7]));
    let held = chunks.borrow_mut();
    let result = catch_unwind(AssertUnwindSafe(|| only_child_of(&chunks, &children)));
    drop(held);
    format!("{:?} {}", only_child_of(&chunks, &children), result.err().map_or(String::new(), message))
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
    "function consume(nodes, i) {\n  if (nodes.borrows) {\n    $alreadyBorrowed(\"lib.rs:5:11\");\n  }\n  if (i >= nodes.length) {\n    $removeOutOfBounds(nodes, i, \"lib.rs:5:24\");\n  }\n  nodes.splice(i, 1);\n}",
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
  expect(js).toContain("  for (let i = 0; i < $borrow(cell, \"lib.rs:52:20\").length; i++) {\n");
  expect(js).toContain("  return look && $borrow(cell, \"lib.rs:60:19\").length !== 0;\n");
  expect(js).toContain("  return look ? $borrow(cell, \"lib.rs:64:20\").length : 0;\n");
  expect(ran(dir, "loops_and_short_circuits")).toBe("6 false 0\n");
});

// A momentary borrow in the test of a `Some` made where it holds, React
// Fizz's `(segment.chunks.borrow().is_empty() && ..).then(..)`, is checked
// first, before the `const`, as in any test (`SomeWhere`, #2112 C S3c).
test("a momentary borrow in a then's test is checked first", () => {
  const dir = compiled("borrow-forms-then");
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("  if (chunks.borrows < 0) {\n    $alreadyMutablyBorrowed(\"lib.rs:24:13\");\n  }\n  return chunks.length === 0 && children.length === 1\n    ? $index(children, 0, \"lib.rs:24:73\")\n    : undefined;\n");
  expect(ran(dir, "only_child")).toBe("Some(7) RefCell already mutably borrowed\n");
});

// A guard held while a `Drop` of the program's runs, which may borrow its
// cell, is counted, `$borrow(cell, true)` and its `$unborrow`, so the
// drop's borrow panics as Rust's does.
test("a guard held while a Drop runs is counted, as it was", () => {
  const dir = compiled("borrow-forms-counted");
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("const borrow = $borrow(cell, \"lib.rs:85:10\", true);");
  expect(js).toContain("$unborrow(borrow);");
  expect(ran(dir, "borrowed_across_a_drop")).toBe("RefCell already borrowed [1, 2]\n");
  // A counted guard taken where another is held panics as Rust's does.
  expect(ran(dir, "counted_while_mutably_held")).toBe("RefCell already mutably borrowed 3\n");
  expect(ran(dir, "counted_while_held")).toBe("RefCell already borrowed [1, 2]\n");
});

/** A corpus case's JS, as a program. */
function caseJs(name: string): string {
  const dir = fixture(`borrow-forms-${name}`);
  run([compiler, join(import.meta.dir, "corpus", `${name}.rs`), "-o", join(dir, "case.js"), "--", "--edition=2024", "-Awarnings"]);
  return readFileSync(join(dir, "case.js"), "utf8");
}

// An undo stack's guards, each held while nothing that may borrow its cell
// runs (#1815): an `Rc`'s count, `entries[i] = state` or `truncate` of an
// `Rc<State>` whose `State` drops nothing, asks no cell, nor does a guard's
// deref in a closure of generic code. No guard of it is counted or checked.
test("an undo stack's guards, held while only Rc counts and reads run, are neither counted nor checked", () => {
  const js = caseJs("guard_held_in_an_undo_stack");
  expect(js).not.toContain("$borrowMut(");
  expect(js).not.toContain("$unborrow(");
});

// A guard a `let` holds quietly beside another of its cell counted while a
// callback runs: checked where it's made, not counted (#1815); and one
// named as the parameter it borrows is its check alone, no `state = state;`.
test("a quiet let guard of a counted cell is checked, not counted, and no self-assignment is left", () => {
  const one = caseJs("guard_let_checked_beside_a_counted_one");
  expect(one).toContain("if (history.borrows) {\n      $alreadyBorrowed(\"guard_let_checked_beside_a_counted_one.rs:23:26\");\n    }");
  expect(one.match(/\$borrowMut\(/g) ?? []).toEqual([]);
  const advance = caseJs("refcell_eq_through_guard");
  expect(advance).toContain("if (state.borrows) {\n    $alreadyBorrowed(\"refcell_eq_through_guard.rs:15:27\");\n  }");
  expect(advance).not.toContain("state = state;");
  // One dropped unread is its check alone, no `COUNTED;` after it (#2490).
  const unread = caseJs("nullable_written_through_borrows");
  expect(unread).toContain("if (COUNTED.borrows < 0) {\n    $alreadyMutablyBorrowed(\"nullable_written_through_borrows.rs:119:23\");\n  }\n  const value$1 = noneOf(1);");
});
