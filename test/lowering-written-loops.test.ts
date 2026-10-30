// What the compiler writes of a loop the program wrote, as it wrote it: a
// `while`'s test as its test, its body's `break` where it was written.
// Each test is a program of its own, compiled and run.

import { expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildNode, compiler, fixture, root, run } from "./support";

// ADR 0702: a `while` whose test calls, `!lines[s].contains(..)` of a
// `Vec`'s index, is `while (test) { .. }`, its test as written (ADR 0683),
// not `while (true) { if (s >= lines.length || ..) { break; } .. }`; and
// one whose body leaves by a `break` in an `if`, the rest stepping on, is
// `while (test) { if (t) { ..; break; } s--; }`, as React's
// describeNativeComponentFrame matches two stacks.
test("a while is its test, its break where it was written", async () => {
  const dir = fixture("written-whiles");
  writeFileSync(
    join(dir, "lib.rs"),
    `pub fn root(lines: Vec<&str>) -> usize {
    let mut s = 0;
    while s < lines.len() && !lines[s].contains("Root") {
        s += 1;
    }
    s
}

pub fn differ(a: &[i32], b: &[i32]) -> usize {
    let mut s = a.len().min(b.len());
    let mut found = 0;
    while s >= 1 {
        if a[s - 1] != b[s - 1] {
            if s > 1 {
                found = s;
            }
            break;
        }
        s -= 1;
    }
    found
}

pub fn first_diff(a: &[i32], b: &[i32]) -> usize {
    let mut s = a.len().min(b.len());
    let mut found = 0;
    while s >= 1 {
        if a[s - 1] == b[s - 1] {
            s -= 1;
        } else {
            found = s;
            break;
        }
    }
    found
}

pub fn scan(rows: &[Vec<i32>]) -> i32 {
    let mut total = 0;
    'rows: for row in rows {
        let mut i = 0;
        while i < row.len() {
            if row[i] < 0 {
                i += 1;
                continue;
            }
            total += row[i];
            i += 1;
        }
        if total > 0 {
            break 'rows;
        }
    }
    total
}
`,
  );
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('  while (s < lines.length && !lines[s].includes("Root")) {\n    s++;\n  }\n');
  expect(js).toContain(
    "  while (s >= 1) {\n    if (a[s - 1] !== $index(b, s - 1)) {\n      if (s > 1) {\n        found = s;\n      }\n      break;\n    }\n    s--;\n  }\n",
  );
  // The program's `if` and its `else`, the `break` the `else`'s.
  expect(js).toContain(
    "  while (s >= 1) {\n    if (a[s - 1] === $index(b, s - 1)) {\n      s--;\n    } else {\n      found = s;\n      break;\n    }\n  }\n",
  );
  const lib = await import(join(dir, "lib.js"));
  expect([lib.root(["a", "Root"]), lib.root(["a"]), lib.root([])]).toEqual([1, 1, 0]);
  expect([lib.differ([1, 2, 3], [1, 5, 3]), lib.differ([4, 2], [1, 2]), lib.differ([1], [1])]).toEqual([2, 0, 0]);
  expect([lib.firstDiff([1, 2, 3], [1, 5, 3]), lib.firstDiff([4, 2], [1, 2]), lib.firstDiff([1], [1])]).toEqual([2, 1, 0]);
  expect([lib.scan([[-1, 2], [3]]), lib.scan([[-1], [-2], [4, 1]]), lib.scan([])]).toEqual([2, 5, 0]);
});

// ADR 0705 (amended): a `while` in a guard's scope, whose test calls, is
// `while (test) { .. }` once its scope is one `finally`. Its first tidy
// made `while (s < lines.length) { let tmp; try { tmp = ..; } catch ..
// if (tmp) { break; } s++; }`, its call apart in a statement's own `try`;
// tidy's stages run again read `tmp` in place, and the rest of the test
// the program wrote joins the loop's, as `while (true)`'s whole test does
// (ADR 0702). An `if` the program wrote first in the body, of a `break`
// alone, stays where it wrote it.
test("a while in a guard's scope is its test", async () => {
  const dir = fixture("guarded-whiles");
  writeFileSync(
    join(dir, "lib.rs"),
    `use std::cell::Cell;

thread_local! {
    static DROPS: Cell<u32> = const { Cell::new(0) };
}

struct Restore;

impl Drop for Restore {
    fn drop(&mut self) {
        DROPS.set(DROPS.get() + 1);
    }
}

pub fn drops() -> u32 {
    DROPS.get()
}

fn is_root(line: &str) -> bool {
    line.contains("Root")
}

pub fn root(lines: &[&str]) -> usize {
    let _restore = Restore;
    let mut s = 0;
    while s < lines.len() && !is_root(lines[s]) {
        s += 1;
    }
    s
}

pub fn written(lines: &[&str]) -> usize {
    let _restore = Restore;
    let mut s = 0;
    while s < lines.len() {
        if is_root(lines[s]) {
            break;
        }
        s += 1;
    }
    s
}
`,
  );
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('    while (s < lines.length && !isRoot(lines[s])) {\n      s++;\n    }\n');
  expect(js).toContain('    while (s < lines.length) {\n      if (isRoot(lines[s])) {\n        break;\n      }\n      s++;\n    }\n');
  const lib = await import(join(dir, "lib.js"));
  expect([lib.root(["a", "Root"]), lib.root(["a"]), lib.root([])]).toEqual([1, 1, 0]);
  expect([lib.written(["a", "Root"]), lib.written(["a"]), lib.written([])]).toEqual([1, 1, 0]);
  expect(lib.drops()).toBe(6);
});

// ADR 0892: what can't throw, a `str`'s `includes` of Rust's types
// (`Known::Read`), needs no `finally`, so a guard's scope round a loop that
// only reads is no `try` at all: the loop reads as it would unguarded, its
// drop after it.
test("a guarded loop that only reads is no try", async () => {
  const dir = fixture("guarded-reads");
  writeFileSync(
    join(dir, "lib.rs"),
    `use std::cell::Cell;

thread_local! {
    static DROPS: Cell<u32> = const { Cell::new(0) };
}

struct Restore;

impl Drop for Restore {
    fn drop(&mut self) {
        DROPS.set(DROPS.get() + 1);
    }
}

pub fn drops() -> u32 {
    DROPS.get()
}

pub fn root(lines: &[&str]) -> usize {
    let _restore = Restore;
    let mut s = 0;
    while s < lines.len() && !lines[s].contains("Root") {
        s += 1;
    }
    s
}
`,
  );
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('  while (s < lines.length && !lines[s].includes("Root")) {\n    s++;\n  }\n');
  expect(js).not.toContain("finally");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.root(["a", "Root"]), lib.root(["a"]), lib.root([])]).toEqual([1, 1, 0]);
  expect(lib.drops()).toBe(3);
});

// ADR 0705 (amended): a loop that ends a labeled block MIR made, `bb9: {
// if (..) { .. while (true) { .. break bb9; } } }`, as the last of loops
// each in the one before's way out is once `exits after` puts each after
// it, breaks as a loop does, and the block goes, as React's
// describeNativeComponentFrame matches its two stacks.
test("a loop that ends a block breaks as a loop", async () => {
  const dir = fixture("loop-ends-block");
  writeFileSync(
    join(dir, "lib.rs"),
    `pub fn common(sample: Option<String>, control: Option<String>) -> String {
    if let Some(sample) = sample
        && let Some(control) = control
    {
        let sample_lines: Vec<&str> = sample.split('\\n').collect();
        let control_lines: Vec<&str> = control.split('\\n').collect();
        let mut s = 0;
        while s < sample_lines.len() && !sample_lines[s].contains("Root") {
            s += 1;
        }
        let mut c = control_lines.len() - 1;
        while s >= 1 {
            if sample_lines[s] != control_lines[c] {
                if s != 1 {
                    loop {
                        s -= 1;
                        if s == 0 || sample_lines[s] != control_lines[c] {
                            return sample_lines[s].to_string();
                        }
                    }
                }
                break;
            }
            s -= 1;
            c = c.saturating_sub(1);
        }
    }
    String::new()
}

fn item(t: Option<&i32>) -> Result<i32, String> {
    t.copied().ok_or_else(|| "end".to_string())
}

pub fn args(tokens: &[i32]) -> Result<Vec<i32>, String> {
    let mut it = tokens.iter().peekable();
    let mut args = Vec::new();
    if it.peek() != Some(&&0) {
        args.push(item(it.next())?);
        while let Some(&&1) = it.peek() {
            it.next();
            args.push(item(it.next())?);
        }
    }
    Ok(args)
}
`,
  );
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("    while (s >= 1) {\n");
  expect(js).not.toMatch(/\bbb\d+/);
  // A calculator's arguments: its `?` an early return, and no `continue`
  // where the loop's body ends anyway.
  expect(js).toContain("      args.push(result._0);\n    }\n  }\n");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.common("a\nb\nRoot\nx\ny", "q\nz\ny"), lib.common("a\nRoot", "Root"), lib.common(undefined, "a")]).toEqual(["b", "", ""]);
  expect([lib.args([2, 1, 3]), lib.args([2, 1]), lib.args([0])]).toEqual([{ TAG: "Ok", _0: [2, 3] }, { TAG: "Err", _0: "end" }, { TAG: "Ok", _0: [] }]);
});

// React's shared/ReactComponentStackFrame, as react-in-rust ports it
// (`test/ports/component-stack-frame`, copied with the modules it uses):
// describeNativeComponentFrame's loops, in its guard's `finally`, are
// upstream's, `while (s < sampleLines.length &&
// !sampleLines[s].includes('DetermineComponentFrameRoot')) { s++; }`, with
// no label round them.
test("React's describeNativeComponentFrame's loops are upstream's", () => {
  const node = buildNode();
  const out = fixture("component-stack-frame");
  const crates = ["js", "node", "webapi"].flatMap((lib) => ["--extern", `${lib}=${join(node, `lib${lib}.rmeta`)}`]);
  run([compiler, join(root, "test/ports/component-stack-frame/lib.rs"), "-o", join(out, "lib.js"), "--", ...crates, "-L", node]);
  const js = readFileSync(join(out, "ReactComponentStackFrame.js"), "utf8");
  expect(js).toContain(
    '      while (s < sampleLines.length && !sampleLines[s].includes("DetermineComponentFrameRoot")) {\n        s++;\n      }\n',
  );
  expect(js).toContain(
    '      while (c < controlLines.length && !controlLines[c].includes("DetermineComponentFrameRoot")) {\n        c++;\n      }\n',
  );
  expect(js).toContain("      while (s >= 1 && c >= 0) {\n");
  expect(js).not.toContain("$index");
  expect(js).not.toMatch(/\bbb\d+/);
});

// ADR 0702, amended again: React's findFirstSuspended
// (react-reconciler/src/ReactFiberSuspenseComponent.js), as react-in-rust
// ports it. Its inner `while` walks up while a fiber has no sibling, the
// `return None` of its `match` a `return`, and what follows the loop, a
// `continue` of the outer one, after it: upstream's nested loops, with no
// label round either.
test("React's findFirstSuspended's loops are upstream's, with no label", async () => {
  const node = buildNode();
  const dir = fixture("find-first-suspended");
  writeFileSync(
    join(dir, "lib.rs"),
    `use std::cell::{Cell, RefCell};
use std::rc::Rc;

use js::Unknown;

pub struct Fiber {
    pub tag: Cell<u32>,
    pub flags: Cell<u32>,
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub r#return: RefCell<Option<Rc<Fiber>>>,
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub child: RefCell<Option<Rc<Fiber>>>,
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub sibling: RefCell<Option<Rc<Fiber>>>,
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub memoized_props: Cell<Option<&'static Unknown>>,
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub memoized_state: Cell<Option<&'static Unknown>>,
}

pub struct SuspenseState {
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub dehydrated: Option<&'static Unknown>,
}

const SUSPENSE_COMPONENT: u32 = 13;
const SUSPENSE_LIST_COMPONENT: u32 = 19;
const NO_FLAGS: u32 = 0;
const DID_CAPTURE: u32 = 128;

fn is_pending(instance: &Unknown) -> bool {
    js::truthy(js::get(instance, "pending"))
}

#[cfg_attr(rust_js, rust_js::nullable)]
pub fn find_first_suspended(row: &Rc<Fiber>) -> Option<Rc<Fiber>> {
    let mut node = Rc::clone(row);
    loop {
        if node.tag.get() == SUSPENSE_COMPONENT {
            let state: Option<&SuspenseState> = unsafe { js::cast(node.memoized_state.get()) };
            if let Some(state) = state {
                let dehydrated = state.dehydrated;
                if dehydrated.is_none_or(is_pending) {
                    return Some(node);
                }
            }
        } else if node.tag.get() == SUSPENSE_LIST_COMPONENT
            && !js::strict_equals(js::get(node.memoized_props.get(), "revealOrder"), "independent")
        {
            let did_suspend = (node.flags.get() & DID_CAPTURE) != NO_FLAGS;
            if did_suspend {
                return Some(node);
            }
        } else {
            let child = node.child.borrow().clone();
            if let Some(child) = child {
                child.r#return.replace(Some(Rc::clone(&node)));
                node = child;
                continue;
            }
        }
        if Rc::ptr_eq(&node, row) {
            return None;
        }
        while node.sibling.borrow().is_none() {
            let return_fiber = node.r#return.borrow().clone();
            match return_fiber {
                Some(return_fiber) if !Rc::ptr_eq(&return_fiber, row) => node = return_fiber,
                _ => return None,
            }
        }
        let sibling = node.sibling.borrow().clone().unwrap();
        let return_fiber = node.r#return.borrow().clone();
        sibling.r#return.replace(return_fiber);
        node = sibling;
    }
}

// Its first arm a return of its own, the tree walked as before: the
// function's value made in the inner loop's way out, before the loop's
// shape was, the label once left round the outer one.
#[cfg_attr(rust_js, rust_js::nullable)]
pub fn find_first_list(row: &Rc<Fiber>) -> Option<Rc<Fiber>> {
    let mut node = Rc::clone(row);
    loop {
        if node.tag.get() == SUSPENSE_LIST_COMPONENT {
            return Some(node);
        }
        let child = node.child.borrow().clone();
        if let Some(child) = child {
            child.r#return.replace(Some(Rc::clone(&node)));
            node = child;
            continue;
        }
        if Rc::ptr_eq(&node, row) {
            return None;
        }
        while node.sibling.borrow().is_none() {
            let return_fiber = node.r#return.borrow().clone();
            match return_fiber {
                Some(return_fiber) if !Rc::ptr_eq(&return_fiber, row) => node = return_fiber,
                _ => return None,
            }
        }
        let sibling = node.sibling.borrow().clone().unwrap();
        let return_fiber = node.r#return.borrow().clone();
        sibling.r#return.replace(return_fiber);
        node = sibling;
    }
}
`,
  );
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(node, "libjs.rmeta")}`, "-L", node]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).not.toMatch(/\bbb\d+\b/);
  expect(js.match(/\n    while \(node\.sibling == null\) \{\n/g)?.length).toBe(2);
  expect(js).toContain(
    "    while (node.sibling == null) {\n" +
      "      const returnFiber = node.return;\n" +
      "      if (returnFiber == null || returnFiber === row) {\n" +
      "        return null;\n" +
      "      }\n" +
      "      node = returnFiber;\n" +
      "    }\n" +
      "    const sibling = node.sibling;\n",
  );
  const lib = await import(join(dir, "lib.js"));
  const fiber = (tag: number): Record<string, unknown> => ({ tag, flags: 0, return: null, child: null, sibling: null, memoizedProps: null, memoizedState: null });
  // row -> a -> b, b with no sibling, a's sibling c a list that suspended.
  const [row, a, b, c] = [fiber(0), fiber(5), fiber(0), fiber(19)];
  Object.assign(row, { child: a });
  Object.assign(a, { child: b, sibling: c });
  Object.assign(c, { flags: 128, memoizedProps: { revealOrder: "forwards" } });
  expect(lib.findFirstSuspended(row)).toBe(c);
  expect(b.return).toBe(a);
  expect(c.return).toBe(row);
  c.flags = 0;
  expect(lib.findFirstSuspended(row)).toBeNull();
  // A dehydrated boundary, pending: the one found.
  Object.assign(b, { tag: 13, memoizedState: { dehydrated: { pending: true } } });
  expect(lib.findFirstSuspended(row)).toBe(b);
  [b.tag, c.tag] = [0, 0];
  expect(lib.findFirstList(row)).toBeNull();
  c.tag = 19;
  expect(lib.findFirstList(row)).toBe(c);
});

// ADR 0702, amended: a `while`'s test is the loop's own. An `if` the
// program wrote first in its body of a `break` alone stays its `if`, not
// joined to the test, `while (i < n && !c && !d)`, whether it tests an
// `&&`, an `||`, a `!`, or is one of two `if`s one after the other, each
// where it's written (#298).
test("a while's test is its own, an if of a break the program wrote its if", async () => {
  const dir = fixture("written-while-breaks");
  writeFileSync(
    join(dir, "lib.rs"),
    `pub fn both(n: u32, c: bool, d: bool) -> u32 {
    let mut i = 0;
    while i < n {
        if c && d {
            break;
        }
        i += 1;
    }
    i
}

pub fn either(n: u32, c: bool, d: bool) -> u32 {
    let mut i = 0;
    while i < n {
        if c || d {
            break;
        }
        i += 1;
    }
    i
}

pub fn twice(n: u32, c: bool, d: bool) -> u32 {
    let mut i = 0;
    while i < n {
        if c {
            break;
        }
        if d {
            break;
        }
        i += 1;
    }
    i
}

pub fn blanks(words: &[&str]) -> usize {
    let mut i = 0;
    while i < words.len() {
        if !words[i].is_empty() {
            break;
        }
        i += 1;
    }
    i
}
`,
  );
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("  while (i < n) {\n    if (c && d) {\n      break;\n    }\n    i++;\n  }\n");
  expect(js).toContain("  while (i < n) {\n    if (c || d) {\n      break;\n    }\n    i++;\n  }\n");
  expect(js).toContain("  while (i < n) {\n    if (c) {\n      break;\n    }\n    if (d) {\n      break;\n    }\n    i++;\n  }\n");
  expect(js).toContain("  while (i < words.length) {\n    if (words[i]) {\n      break;\n    }\n    i++;\n  }\n");
  const lib = await import(join(dir, "lib.js"));
  const flags = [
    [false, false],
    [true, false],
    [false, true],
    [true, true],
  ];
  expect(flags.map(([c, d]) => [lib.both(4, c, d), lib.either(4, c, d), lib.twice(4, c, d)])).toEqual([
    [4, 4, 4],
    [4, 0, 0],
    [4, 0, 0],
    [0, 0, 0],
  ]);
  expect([lib.blanks(["", "", "a"]), lib.blanks(["", ""]), lib.blanks([])]).toEqual([2, 2, 0]);
});

// A range from arithmetic, `for j in i + 1..m.len()`, counts as one from a
// variable does (#368): `for (let j = i + 1; j < m.length; j++)`, as
// upstream's pairwise loops are written. Its end held first where the body
// changes what it reads, read before its start, of an end of no effects;
// one of effects is read after its start, as Rust reads it.
test("a range from arithmetic is a counting for", async () => {
  const dir = fixture("arithmetic-start");
  writeFileSync(join(dir, "lib.rs"), `pub fn pairs(m: &[u32]) -> u32 {
    let mut s = 0;
    for i in 0..m.len() {
        for j in i + 1..m.len() {
            s += m[i] * m[j];
        }
    }
    s
}

pub fn swapped(m: &mut [u32]) {
    for i in 0..m.len() {
        for j in i * 2..m.len() {
            m.swap(i, j);
        }
    }
}

fn bump(k: &mut usize) -> usize {
    *k += 10;
    *k
}

pub fn ordered(mut k: usize) -> usize {
    let mut seen = 0;
    for j in k + 1..bump(&mut k) {
        seen += j;
    }
    seen
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("    for (let j = i + 1; j < m.length; j++) {\n");
  expect(js).toContain("    const end = m.length;\n    for (let j = i * 2; j < end; j++) {\n");
  expect(js).not.toContain("$rangeNext");
  const lib = await import(join(dir, "lib.js"));
  const m = [1, 2, 3];
  lib.swapped(m);
  expect([lib.pairs([1, 2, 3]), m, lib.ordered(0)]).toEqual([11, [3, 2, 1], 45]);
});

// rd260: React's Fizz runtime counts its batch, `for (let i = 0; i <
// batch.length; i += 2)`, whose length grows as it runs; its port, `for i
// in (0..).step_by(2) { let Some(&id) = batch.borrow().get(i) else { break;
// }; .. }`, which reads the length each turn as upstream does, printed
// `for (const i of $stepBy($iterator({ start: 0 }, $rangeFromStep(..)),
// 2))`, a BigInt's range code Closure can't make ES5 of. An item that may
// be `None`, or of no size, keeps the iterator.
test("a stepped index to a slice's item is counted to its length", () => {
  const dir = fixture("counted-to-length");
  writeFileSync(
    join(dir, "lib.rs"),
    `use std::cell::RefCell;

pub fn complete(batch: &RefCell<Vec<u32>>) -> u32 {
    let mut sum = 0;
    for i in (0..).step_by(2) {
        let Some(&id) = batch.borrow().get(i) else {
            break;
        };
        let content = batch.borrow().get(i + 1).copied().unwrap_or(0);
        if id == 7 {
            batch.borrow_mut().extend([1, 1]);
        }
        sum += id * content;
    }
    sum
}

pub fn ones(batch: &[u32]) -> u32 {
    let mut sum = 0;
    for i in (1..).step_by(1) {
        let Some(id) = batch.get(i).copied() else {
            break;
        };
        sum += id;
    }
    sum
}

pub fn slots(batch: &[Option<u32>]) -> u32 {
    let mut sum = 0;
    for i in (0..).step_by(2) {
        let Some(&slot) = batch.get(i) else {
            break;
        };
        sum += slot.unwrap_or(100);
    }
    sum
}

pub fn flat(batch: &[Option<u32>]) -> u32 {
    let mut sum = 0;
    for i in (0..).step_by(2) {
        let Some(id) = batch.get(i).copied().flatten() else {
            break;
        };
        sum += id;
    }
    sum
}

pub fn other(batch: &[u32]) -> usize {
    let mut sum = 0;
    let mut k = 0;
    for i in (0..).step_by(2) {
        let Some(&id) = batch.get(k) else {
            break;
        };
        sum += id as usize * i;
        k += 1;
    }
    sum
}

pub fn wide(batch: &[u32]) -> u32 {
    let mut sum = 0;
    for i in (0..).step_by(2_147_483_649) {
        let Some(&id) = batch.get(i) else {
            break;
        };
        sum += id;
    }
    sum
}
`,
  );
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("  for (let i = 0; i < batch.length; i += 2) {\n    const id = batch[i];\n    const content = batch[i + 1] ?? 0;\n");
  expect(js).toContain("  for (let i = 1; i < batch.length; i++) {\n    const id = batch[i];\n");
  expect(js).toContain("  for (const i of $stepBy($iterator({ start: 0 }, $rangeFromStep(0, 4294967295)), 2)) {\n    const get = $someAt(batch, i);\n");
  // A `None` an item may be ends the loop, of `flatten()`, and another
  // index, or a step that may pass `usize`'s most, isn't the loop's count.
  const stepped = (step: number, read: string) =>
    `  for (const i of $stepBy($iterator({ start: 0 }, $rangeFromStep(0, 4294967295)), ${step})) {\n    const id = batch[${read}];\n`;
  expect(js).toContain(`export function flat(batch) {\n  let sum = 0;\n${stepped(2, "i")}`);
  expect(js).toContain(stepped(2, "k"));
  expect(js).toContain(stepped(2147483649, "i"));
});
