// Cells lowered from MIR: one only its function reads and sets, a
// thread-local only its module reads and sets, a `RefCell` field only ever
// borrowed for a moment, each its value or variable (ADRs 0270, 0362); a
// cell field a `let` pattern binds; a handle used in place. Split from
// lowering-cells.test.ts by concept, at its cap.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildReact, compiler, fixture, root, run, target } from "./support";

beforeAll(buildReact, 600_000);

test("a cell from MIR only its function reads and sets is its variable", async () => {
  const dir = fixture("mir-plain-cells");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::Cell;
use std::rc::Rc;
pub fn counter(f: &dyn Fn(&dyn Fn())) -> u32 {
    let n = Rc::new(Cell::new(0));
    let m = n.clone();
    f(&move || m.set(m.get() + 1));
    n.get()
}
pub fn local(f: &dyn Fn(&dyn Fn())) -> bool {
    let active = Cell::new(true);
    f(&|| active.set(false));
    active.get()
}
fn bump(c: &Cell<u32>) {
    c.set(c.get() + 1);
}
pub fn lent() -> u32 {
    let c = Cell::new(1);
    bump(&c);
    c.get()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain(`export function counter(f) {
  let n = 0;
  f(() => {
    n = (n + 1) >>> 0;
  });
  return n;
}`);
  expect(js).toContain(`export function local(f) {
  let active = true;
  f(() => {
    active = false;
  });
  return active;
}`);
  // One given whole is a cell.
  expect(js).toContain("  const c = { value: 1 };\n  bump(c);\n");
  const lib = await import(join(dir, "mir.js"));
  expect([lib.counter((g: () => void) => { g(); g(); }), lib.local((g: () => void) => g()), lib.lent()]).toEqual([2, false, 2]);
});

test("a thread-local only its module reads and sets, from MIR, is its variable (ADR 0270)", async () => {
  const dir = fixture("mir-plain-statics");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::Cell;
thread_local! {
    static COUNT: Cell<u32> = const { Cell::new(0) };
    static SEEN: Cell<bool> = const { Cell::new(false) };
}
pub fn bump() -> u32 {
    COUNT.set(COUNT.get() + 1);
    COUNT.get()
}
pub fn seen() -> bool {
    SEEN.get()
}
thread_local! {
    static LAST: Cell<u32> = const { Cell::new(0) };
}
pub fn remember(f: &dyn Fn(&dyn Fn(u32))) -> u32 {
    f(&|n| LAST.set(n));
    LAST.get()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain("let count = 0;\nconst SEEN = false;\n");
  expect(js).toContain("let last = 0;\n");
  expect(js).toContain("  count = (count + 1) >>> 0;\n  return count;\n");
  const lib = await import(join(dir, "mir.js"));
  expect([lib.bump(), lib.bump(), lib.seen(), lib.remember((g: (n: number) => void) => g(7))]).toEqual([1, 2, false, 7]);
});

test("a `RefCell` field only ever borrowed for a moment, from MIR, is its value (ADR 0362)", async () => {
  const dir = fixture("mir-plain-ref-cells");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::RefCell;
use std::rc::Rc;
pub struct Node {
    pub item: u32,
    pub children: RefCell<Vec<Rc<Node>>>,
}
pub fn tree(items: &[u32]) -> Rc<Node> {
    let root = Rc::new(Node { item: 0, children: RefCell::new(vec![]) });
    for &i in items {
        let node = Rc::new(Node { item: i, children: RefCell::new(vec![]) });
        root.children.borrow_mut().push(node);
    }
    root
}
pub fn count(node: &Node) -> usize {
    if node.children.borrow().len() < 2 {
        return 0;
    }
    node.children.borrow().len()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain("    const node = { item: i, children: [] };\n    root.children.push(node);\n");
  expect(js).toContain("  if (node.children.length < 2) {\n    return 0;\n  }\n  return node.children.length;\n");
  const lib = await import(join(dir, "mir.js"));
  const root = lib.tree([1, 2, 3]);
  expect([lib.count(root), lib.count(root.children[0])]).toEqual([3, 0]);
});

// A `Cell` field a `let` pattern binds by reference, only read, is its
// value where it's bound (ADR 0287), as react.dev's ErrorMessage reads its
// error's title: `const { message, title } = error`.
test("a cell field a let pattern binds, only read, is its value", async () => {
  const dir = fixture("pattern-cell-field");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::Cell;
pub struct E {
    pub title: Cell<Option<&'static str>>,
    pub message: String,
}
pub fn show(error: &E) -> String {
    let E { message, title } = error;
    format!("{}: {message}", title.get().filter(|t| !t.is_empty()).unwrap_or("Error"))
}
pub fn retitle(error: &E) -> String {
    let E { message, title } = error;
    title.set(Some("New"));
    format!("{}: {message}", title.get().unwrap_or("Error"))
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain("export function show(error) {\n  const { message, title } = error;\n");
  const lib = await import(join(dir, "mir.js"));
  const error = { title: "T", message: "m" };
  expect([lib.show(error), lib.show({ message: "m" }), lib.retitle(error), error.title]).toEqual(["T: m", "Error: m", "New: m", "New"]);
});

// A handle's value read or written in place, `Cell::set` of `&self.n`, is
// its place's, `t.n = ..`: no object of a getter and a setter made to be
// used once.
test("a handle used in place from MIR is its place", async () => {
  const dir = fixture("handle-in-place");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::Cell;
pub struct T { n: Cell<u32> }
impl T {
    pub fn bump(&self) -> u32 { self.n.set(self.n.get() + 1); self.n.get() }
}
pub fn run() -> u32 {
    let t = T { n: Cell::new(0) };
    t.bump();
    t.bump()
}
pub fn pair() -> String {
    match &mut (1, 2) {
        (a, b) => {
            *a += 10;
            *b += 20;
            format!("{a} {b}")
        }
    }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  expect(readFileSync(join(dir, "mir.js"), "utf8")).toContain("  bump(t) {\n    t.n = (t.n + 1) >>> 0;\n    return t.n;\n  },");
  expect(readFileSync(join(dir, "mir.js"), "utf8")).toContain("  return `${value[0]} ${value[1]}`;");
  expect((await import(join(dir, "mir.js"))).run()).toBe(2);
  expect((await import(join(dir, "mir.js"))).pair()).toBe("11 22");
});
