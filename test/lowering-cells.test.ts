// What the compiler writes of `Cell`, `RefCell`, `Rc` and `thread_local!`: a
// cell that's only its value, a thread-local that's its module's variable, and
// `Rc::ptr_eq` as `===`. Each test is a program of its own, compiled and run.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildReact, compiler, fixture, root, run, target } from "./support";

beforeAll(buildReact, 600_000);

// ADR 0362: a `RefCell` in a field that's only ever borrowed for a moment,
// where nothing can ask, is never counted, so it's the value it holds, as
// react.dev's nested table of contents pushes onto a node's `children`.
test("a field's RefCell only borrowed for a moment is its value", async () => {
  const dir = fixture("plain-refcell-field");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

pub struct Item {
    pub depth: u32,
    pub url: String,
}

pub struct Node {
    pub item: Option<Rc<Item>>,
    pub children: RefCell<Vec<Rc<Node>>>,
}

pub fn nested(toc: Vec<Rc<Item>>) -> Rc<Node> {
    let mut ancestors: HashMap<u32, Rc<Node>> = HashMap::new();
    let root = Rc::new(Node { item: None, children: RefCell::new(vec![]) });
    for item in toc {
        let parent = ancestors.get(&(item.depth - 1)).cloned().unwrap_or_else(|| root.clone());
        let node = Rc::new(Node { item: Some(item.clone()), children: RefCell::new(vec![]) });
        parent.children.borrow_mut().push(node.clone());
        ancestors.insert(item.depth, node);
    }
    root
}

pub fn width(node: &Node) -> usize {
    node.children.borrow().len()
}

// A local of the same type is a cell still, its borrows checked.
pub fn both(node: Rc<Node>) -> usize {
    let spare: RefCell<Vec<Rc<Node>>> = RefCell::new(vec![]);
    let reading = spare.borrow();
    spare.borrow_mut().push(node);
    reading.len()
}

// One lent whole stays a cell: what it's lent counts its borrows.
pub struct Held {
    pub values: RefCell<Vec<u32>>,
}

fn first(values: &RefCell<Vec<u32>>) -> Option<u32> {
    values.borrow().first().copied()
}

pub fn held(n: u32) -> Option<u32> {
    let held = Held { values: RefCell::new(vec![n]) };
    first(&held.values)
}

// One bound by a pattern is a cell of its own.
pub struct Bound {
    pub values: RefCell<Vec<u16>>,
}

pub fn bound(n: u16) -> usize {
    let Bound { values } = Bound { values: RefCell::new(vec![n]) };
    values.borrow().len()
}

pub fn peek(bound: &Bound) -> usize {
    bound.values.borrow().len()
}

// One held while something may ask is counted.
pub struct Asked {
    pub values: RefCell<Vec<u8>>,
}

fn ask(asked: &Asked) -> usize {
    asked.values.borrow().len()
}

pub fn asked(n: u8) -> usize {
    let asked = Asked { values: RefCell::new(vec![n]) };
    let guard = asked.values.borrow();
    let len = guard.len() + ask(&asked);
    drop(guard);
    len
}

// One held while its statement calls what may ask is counted.
pub struct Pushed {
    pub values: RefCell<Vec<i16>>,
}

fn count(pushed: &Pushed) -> i16 {
    pushed.values.borrow().len() as i16
}

pub fn pushed(n: i16) {
    let pushed = Pushed { values: RefCell::new(vec![n]) };
    pushed.values.borrow_mut().push(count(&pushed));
}

// One held while its statement drops what may ask is counted.
pub struct Ticked {
    pub values: RefCell<Vec<i8>>,
}

pub struct Tick<'a>(&'a Cell<u32>, i8);

impl Drop for Tick<'_> {
    fn drop(&mut self) {
        self.0.set(self.0.get() + 1);
    }
}

pub fn ticked(n: i8) -> u32 {
    let ticks = Cell::new(0);
    let ticked = Ticked { values: RefCell::new(vec![n]) };
    ticked.values.borrow_mut().push(Tick(&ticks, n).1);
    ticks.get() + ticked.values.borrow().len() as u32
}

// Two shared borrows in one statement ask nothing of each other.
pub struct Shared {
    pub values: RefCell<Vec<f64>>,
}

pub fn shared(n: f64) -> usize {
    let shared = Shared { values: RefCell::new(vec![n]) };
    let len = shared.values.borrow().len() + shared.values.borrow().len();
    len
}

// A derive's reads its fields itself.
#[derive(Clone)]
pub struct Derived {
    pub values: RefCell<Vec<i32>>,
}

pub fn derived(n: i32) -> usize {
    let derived = Derived { values: RefCell::new(vec![n]) };
    let copy = derived.clone();
    copy.values.borrow_mut().push(1);
    copy.values.borrow().len() + derived.values.borrow().len()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("{ children: [] }");
  expect(js).toContain("parent.children.push(node);");
  expect(js.slice(js.indexOf("export function nested"), js.indexOf("export function both"))).not.toContain("$borrow");
  expect(js).toContain("const shared = { values: [n] };");
  // Each of the others is a cell: lent, bound, counted, held while asked, derived.
  expect(js.match(/\{ values: \{ value: \[n\] \} \}/g)?.length).toBe(6);
  const lib = await import(join(dir, "lib.js"));
  const items = [1, 2, 2, 3, 2].map((depth, i) => ({ depth, url: `#${i}` }));
  const root = lib.nested(items);
  expect([lib.width(root), root.children[0].children.length, root.children[0].children[1].children[0].item.url]).toEqual([1, 3, "#3"]);
  expect([lib.held(4), lib.bound(1), lib.asked(1), lib.derived(1), lib.shared(1), lib.ticked(1)]).toEqual([4, 1, 2, 3, 2, 3]);
  expect(() => lib.both(root)).toThrow("already borrowed");
  expect(() => lib.pushed(1)).toThrow("already mutably borrowed");
});

// An insert whose answer goes unused is JS's own, `set.add(x)`, in a
// thread-local's closure too, as react.dev's home page keeps its liked
// videos and its caches.
test("an insert in a thread-local's closure, its answer unused, is JS's own", async () => {
  const dir = fixture("local-insert");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
thread_local! {
    static LIKED: RefCell<HashSet<&'static str>> = RefCell::new(HashSet::new());
    static CACHE: RefCell<HashMap<String, u32>> = RefCell::new(HashMap::new());
}
pub fn like(id: &'static str) -> bool {
    LIKED.with_borrow_mut(|liked| liked.insert(id));
    LIKED.with_borrow(|liked| liked.contains(id))
}
pub fn cache(key: &str, n: u32) -> Option<u32> {
    CACHE.with_borrow_mut(|cache| cache.insert(key.to_string(), n));
    CACHE.with_borrow(|cache| cache.get(key).copied())
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain("  liked.add(id);\n");
  expect(js).toContain("  CACHE.set(key, n);\n");
  expect(js).not.toContain("@rust-js/runtime");
  const lib = await import(join(dir, "mir.js"));
  expect([lib.like("a"), lib.cache("k", 2)]).toEqual([true, 2]);
});

// ADR 0287: a `Cell`, or an `Rc` of one, that only its function and its
// closures read and set is a `let`, as react.dev's Preview keeps `let
// timeout` its listener sets and its cleanup clears: a clone of it is the
// same variable. One that leaves, returned or given whole, is `{ value }`.
test("a cell only its function reads and sets is a let", async () => {
  const dir = fixture("plain-cells");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::Cell;
use std::rc::Rc;

pub fn local() -> u32 {
    let n = Cell::new(1);
    n.set(n.get() + 1);
    n.get()
}

pub fn counted(each: impl Fn(&dyn Fn())) -> u32 {
    let n = Cell::new(0);
    each(&|| n.set(n.get() + 1));
    n.get()
}

pub fn shared(listen: impl Fn(Box<dyn Fn(u32)>)) -> Box<dyn Fn() -> Option<u32>> {
    let timeout: Rc<Cell<Option<u32>>> = Rc::new(Cell::new(None));
    let started = timeout.clone();
    listen(Box::new(move |n| started.set(Some(n))));
    Box::new(move || timeout.get())
}

pub fn escaping() -> Rc<Cell<u32>> {
    let n = Rc::new(Cell::new(1));
    n.set(2);
    n
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("  let n = 1;\n  n = (n + 1) >>> 0;\n  return n;");
  expect(js).toContain("  let n = 0;\n  each(() => {\n    n = (n + 1) >>> 0;\n  });\n  return n;");
  expect(js).toContain("  let timeout;\n  listen((n) => {\n    timeout = n;\n  });\n  return () => timeout;");
  expect(js).toContain("  const n = { value: 1 };\n  n.value = 2;\n  return n;");
  const lib = await import(join(dir, "lib.js"));
  let listener: (n: number) => void = () => {};
  const read = lib.shared((f: (n: number) => void) => { listener = f; });
  const before = read();
  listener(7);
  expect([lib.local(), lib.counted((f: () => void) => { f(); f(); }), before, read(), lib.escaping().value]).toEqual([2, 2, undefined, 7, 2]);
});

// ADR 0288: a `Cell` in a struct's or a variant's field is the value it
// holds, the property set in place, as ReScript's `mutable` field is:
// `c.count = c.count + 1`, not `c.count.value`. A `&Cell` of one is a handle
// on the property (ADR 0099), so what's lent it sets the field.
test("a cell in a field is the property it holds", async () => {
  const dir = fixture("cell-fields");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::Cell;

#[derive(Clone, Default, PartialEq, Debug)]
pub struct Counter {
    pub label: &'static str,
    pub count: Cell<u32>,
}

pub fn made() -> Counter {
    Counter { label: "a", count: Cell::new(1) }
}

pub fn counting(n: u32) -> Counter {
    Counter { label: "n", count: Cell::new(n) }
}

pub fn bumped(c: &Counter) -> u32 {
    c.count.set(c.count.get() + 1);
    c.count.get()
}

fn add(cell: &Cell<u32>, n: u32) {
    cell.set(cell.get() + n);
}

pub fn lent(c: &Counter) -> u32 {
    add(&c.count, 5);
    c.count.get()
}

pub fn cloned(c: &Counter) -> Counter {
    let copy = c.clone();
    c.count.set(9);
    copy
}

pub fn same(a: &Counter, b: &Counter) -> bool {
    a == b
}

pub fn shown(c: &Counter) -> String {
    format!("{c:?}")
}

pub fn fresh() -> Counter {
    Counter::default()
}

pub fn taken(c: Counter) -> u32 {
    let Counter { count, .. } = c;
    count.set(count.get() + 10);
    count.get()
}

pub fn matched(c: &Counter) -> u32 {
    match c {
        Counter { count, .. } => {
            count.set(count.get() * 3);
            count.get()
        }
    }
}

pub enum Slot {
    Empty,
    Held { n: Cell<u32> },
}

pub fn held(slot: &Slot) -> u32 {
    if let Slot::Held { n } = slot {
        n.set(n.get() * 2);
        n.get()
    } else {
        0
    }
}

pub fn empty() -> Slot {
    Slot::Empty
}

/// A name equal to another of any case: a hand-written eq.
pub struct Name(pub &'static str);

impl PartialEq for Name {
    fn eq(&self, other: &Name) -> bool {
        self.0.eq_ignore_ascii_case(other.0)
    }
}

#[derive(PartialEq)]
pub struct Named {
    pub name: Name,
    pub hits: Cell<u32>,
}

pub fn named_alike(a: &'static str, b: &'static str, hits: u32) -> bool {
    Named { name: Name(a), hits: Cell::new(1) } == Named { name: Name(b), hits: Cell::new(hits) }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('return { label: "a", count: 1 };');
  expect(js).toContain('return { label: "n", count: n };');
  expect(js).toContain("  c.count = (c.count + 1) >>> 0;\n  return c.count;");
  expect(js).toContain("  const copy = { ...c };");
  const lib = await import(join(dir, "lib.js"));
  expect(lib.made()).toEqual({ label: "a", count: 1 });
  const lentTo = lib.made();
  expect([lib.bumped(lib.made()), lib.lent(lentTo), lentTo.count]).toEqual([2, 6, 6]);
  const original = lib.made();
  expect([lib.cloned(original).count, original.count]).toEqual([1, 9]);
  expect([lib.same(lib.made(), lib.made()), lib.same(lib.made(), original)]).toEqual([true, false]);
  expect(lib.shown(lib.made())).toBe('Counter { label: "a", count: Cell { value: 1 } }');
  expect(lib.fresh()).toEqual({ label: "", count: 0 });
  const matched = lib.made();
  expect([lib.taken(lib.made()), lib.matched(matched), matched.count]).toEqual([11, 3, 3]);
  const slot = { TAG: "Held", n: 2 };
  expect([lib.held(slot), slot.n, lib.held(lib.empty())]).toEqual([4, 4, 0]);
  expect([lib.namedAlike("Ann", "ANN", 1), lib.namedAlike("Ann", "ANN", 2), lib.namedAlike("Ann", "Bob", 1)]).toEqual([true, false, false]);
});

// ADR 0580: a call asks whether a cell is borrowed only of a cell it may
// borrow, by what each of the crate's functions borrows: React's scheduler
// moves a timer from one heap to the other in `TIMER_QUEUE.with_borrow_mut`,
// calling the heap's own `pop` and `push`, which borrow no cell. Each queue
// is its module's array. One whose closure calls what borrows it stays a
// cell, its borrows counted, so the second throws as Rust's panics.
test("a thread-local whose closures call only what can't borrow it is its variable", async () => {
  const dir = fixture("cell-borrows");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::RefCell;

thread_local! {
    static TIMER_QUEUE: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };
    static TASK_QUEUE: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };
    static LOG: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };
}

fn pop(heap: &mut Vec<u32>) -> Option<u32> {
    heap.pop()
}

fn push(heap: &mut Vec<u32>, n: u32) {
    heap.push(n);
}

pub fn schedule(n: u32) {
    TIMER_QUEUE.with_borrow_mut(|timers| push(timers, n));
}

pub fn advance() -> usize {
    TIMER_QUEUE.with_borrow_mut(|timers| {
        while let Some(timer) = pop(timers) {
            TASK_QUEUE.with_borrow_mut(|tasks| push(tasks, timer));
        }
    });
    TASK_QUEUE.with_borrow(|tasks| tasks.len())
}

fn logged() -> usize {
    LOG.with_borrow(|log| log.len())
}

pub fn log_twice() -> usize {
    LOG.with_borrow_mut(|log| {
        log.push(1);
        logged()
    })
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("const timerQueue = [];\nconst taskQueue = [];");
  expect(js).not.toMatch(/\$withBorrow(Mut)?\((timerQueue|taskQueue)/);
  expect(js).toContain("const log = { value: [] };");
  const lib = await import(join(dir, "lib.js"));
  lib.schedule(1);
  lib.schedule(2);
  expect(lib.advance()).toBe(2);
  expect(() => lib.logTwice()).toThrow("already");
});

// ADR 0581: a plain thread-local's `with_borrow_mut(|queue| ..)` is its
// variable given to the closure, `((queue) => { .. })(queue)`, called as a
// statement: its body is in its place, reading the variable, as React's
// scheduler's `advanceTimers` reads `timerQueue`. A `return;` that leaves
// it is the function's where it ends the function, and its loop's `break`
// where its body ends with that loop; the comment before it is before its
// body.
test("a plain thread-local's closure called as a statement is its body", async () => {
  const dir = fixture("applied-blocks");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::RefCell;

thread_local! {
    static TIMER_QUEUE: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };
}

pub fn schedule(n: u32) {
    TIMER_QUEUE.with_borrow_mut(|timer_queue| timer_queue.push(n));
}

pub fn drained(limit: u32) -> u32 {
    let mut seen = 0;
    // Count each timer.
    TIMER_QUEUE.with_borrow_mut(|timers| {
        while let Some(t) = timers.pop() {
            if t > limit {
                return;
            }
            seen += t;
        }
    });
    seen
}

pub fn last() {
    // Drop the rest.
    TIMER_QUEUE.with_borrow_mut(|timer_queue| {
        while let Some(t) = timer_queue.pop() {
            if t == 0 {
                return;
            }
        }
    });
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("export function schedule(n) {\n  timerQueue.push(n);\n}");
  expect(js).toContain("  let seen = 0;\n  // Count each timer.\n  let t;\n  while ((t = timerQueue.pop()) != null) {\n");
  expect(js).toContain("    if (t > limit) {\n      break;\n    }\n");
  expect(js).toContain("export function last() {\n  // Drop the rest.\n  while (true) {\n");
  expect(js).toContain("    if (t === 0) {\n      return;\n    }\n");
  expect(js).not.toContain("=> {");
  const lib = await import(join(dir, "lib.js"));
  for (const n of [1, 9, 2, 3]) lib.schedule(n);
  expect(lib.drained(5)).toBe(5);
  lib.last();
  expect(lib.drained(5)).toBe(0);
});

// ADR 0840: a thread-local's `with(|x| ..)` is its closure's body where
// it's called, its parameter the thread-local: React's
// `ReactSharedInternalsClient`, which sets its internals' fields when it's
// loaded, and `ReactNoopUpdateQueue`'s test of what it warned of. One that
// returns early, or in what the module runs when it's loaded, which no
// `return` may end, stays a closure.
test("a thread-local's with is its closure's body where it's called", async () => {
  const dir = fixture("applied-with");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::Cell;

use js::Dict;

pub struct Internals {
    #[cfg_attr(rust_js, rust_js::name = "G")]
    #[cfg_attr(rust_js, rust_js::nullable, rust_js::optional)]
    pub g: Cell<Option<&'static js::Unknown>>,
    pub count: Cell<f64>,
}

thread_local! {
    static ENABLED: bool = const { true };
    #[cfg_attr(rust_js, rust_js::name = "Internals")]
    static INTERNALS: Internals = Internals {
        g: Cell::new(None),
        count: Cell::new(0.0),
    };
    static DID_WARN: &'static Dict<bool> = js::dict::new();
}

js::on_load! {
    INTERNALS.with(|internals| {
        if ENABLED.with(|&enabled| enabled) {
            internals.g.set(None);
        }
        internals.count.set(1.0);
    });
    INTERNALS.with(|internals| {
        if internals.count.get() > 5.0 {
            return;
        }
        internals.count.set(internals.count.get() + 1.0);
    });
}

pub fn warn(key: &str) -> u32 {
    if DID_WARN.with(|did_warn| js::dict::get(did_warn, key).is_some_and(|&warned| warned)) {
        return 1;
    }
    DID_WARN.with(|did_warn| js::dict::set(did_warn, key, true));
    0
}

pub fn counted() -> f64 {
    let n = INTERNALS.with(|internals| {
        let before = internals.count.get();
        internals.count.set(before + 1.0);
        before
    });
    n * 2.0
}

pub fn capped(limit: f64) -> f64 {
    INTERNALS.with(|internals| {
        if internals.count.get() > limit {
            return limit;
        }
        internals.count.get()
    })
}

pub fn read() -> f64 {
    INTERNALS.with(|internals| internals.count.get())
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("\nif (ENABLED) {\n  Internals.G = null;\n}\n");
  expect(js).toContain("\nInternals.count = 1;\n");
  expect(js).toContain("((internals) => {\n  if (internals.count > 5) {\n    return;\n  }\n");
  expect(js).toContain("export function warn(key) {\n  if (didWarn[key]) {\n    return 1;\n  }\n");
  expect(js).toContain("  const before = Internals.count;\n  Internals.count = before + 1;\n  const n = before;\n  return n * 2;\n");
  expect(js).toContain("export function capped(limit) {\n  return ((internals) => {\n");
  expect(js).toContain("export function read() {\n  return Internals.count;\n}");
  const lib = await import(join(dir, "lib.js"));
  expect(lib.read()).toBe(2);
  expect([lib.counted(), lib.read()]).toEqual([4, 3]);
  expect([lib.capped(1), lib.capped(5)]).toEqual([1, 3]);
  expect([lib.warn("a"), lib.warn("a"), lib.warn("b")]).toEqual([0, 1, 0]);
});

// ADR 0582: `Rc::ptr_eq` of an `Rc` of a struct with fields, a JS object,
// is `===` of the objects, whose identity it is: React's scheduler asks
// whether the task it ran is still the first, `peek(taskQueue) ===
// currentTask`, of tasks that aren't counted. One of a number, which JS
// compares by value, is counted as before.
test("ptr_eq of an Rc of a struct is === of its object, uncounted", async () => {
  const dir = fixture("ptr-eq-objects");
  writeFileSync(join(dir, "lib.rs"), `use std::rc::Rc;

pub struct Task {
    pub id: u32,
}

pub fn same(a: &Rc<Task>, b: &Rc<Task>) -> bool {
    Rc::ptr_eq(a, b)
}

pub fn task(id: u32) -> Rc<Task> {
    Rc::new(Task { id })
}

pub fn numbers() -> bool {
    let a = Rc::new(1);
    let b = Rc::new(1);
    Rc::ptr_eq(&a, &b)
}

#[repr(transparent)]
pub struct Meters(pub f64);

pub fn meters() -> bool {
    let a = Rc::new(Meters(1.0));
    let b = Rc::new(Meters(1.0));
    Rc::ptr_eq(&a, &b)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("export function same(a, b) {\n  return a === b;\n}");
  expect(js).toContain("export function task(id) {\n  return { id };\n}");
  expect(js).toContain("strong: 1");
  const lib = await import(join(dir, "lib.js"));
  const t = lib.task(1);
  // A transparent struct is its field, a number, whose `===` is its value
  // (ADR 0582, amended): counted, two `Rc`s of equal ones apart.
  expect([lib.same(t, t), lib.same(t, lib.task(1)), lib.numbers(), lib.meters()]).toEqual([true, false, false, false]);
});

// ADR 0585: a field's \`RefCell\` that \`replace\` and \`take\` write, each
// holding it only while it swaps the value, is its value, as React's
// scheduler's task's callback is: read, then written. One borrowed across
// a \`replace\` is still counted, and the \`replace\` throws as Rust panics.
test("a field's RefCell written by replace and take is its value", async () => {
  const dir = fixture("refcell-field-replace");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::RefCell;

pub struct Task {
    callback: RefCell<Option<Box<dyn FnMut(u32) -> u32>>>,
}

pub fn task() -> Task {
    Task { callback: RefCell::new(Some(Box::new(|n| n + 1))) }
}

pub fn run(task: &Task, n: u32) -> Option<u32> {
    let callback = task.callback.take();
    if let Some(mut callback) = callback {
        let result = callback(n);
        task.callback.replace(Some(Box::new(move |m| m * result)));
        return Some(result);
    }
    None
}

pub fn cancelled(task: &Task) -> bool {
    task.callback.borrow().is_none()
}

pub fn cancel(task: &Task) {
    task.callback.replace(None);
}

pub struct Log {
    lines: RefCell<Vec<String>>,
}

fn reset(log: &Log) -> usize {
    log.lines.replace(Vec::new());
    0
}

pub fn clash() -> usize {
    let log = Log { lines: RefCell::new(vec![String::from("a")]) };
    log.lines.borrow().len() + reset(&log)
}

#[derive(Default)]
pub struct Items {
    n: u32,
}

pub struct Bag {
    items: RefCell<Items>,
}

pub fn emptied(bag: &Bag) -> u32 {
    bag.items.take().n
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("const callback = task.callback;\n  task.callback = undefined;");
  expect(js).toContain("export function cancel(task) {\n  task.callback = undefined;\n}");
  expect(js).toContain("return task.callback == null;");
  expect(js).toContain("return { callback: (n) => (n + 1) >>> 0 };");
  // \`take\` runs a \`Default\` of the crate's while it holds the cell.
  expect(js).toContain("$cellReplace($borrowMut(bag.items)");
  const lib = await import(join(dir, "lib.js"));
  const t = lib.task();
  expect([lib.run(t, 1), lib.run(t, 3), lib.cancelled(t)]).toEqual([2, 6, false]);
  lib.cancel(t);
  expect([lib.cancelled(t), lib.run(t, 1)]).toEqual([true, undefined]);
  expect(() => lib.clash()).toThrow("already borrowed");
});

// ADR 0587: a function given to a thread-local's \`with_borrow\` is asked
// what it may borrow as a closure is: React's scheduler's
// \`CURRENT_TASK.with_borrow(Clone::clone)\` borrows nothing, so its task is
// its module's variable. One that borrows its own key keeps it a cell.
test("a thread-local given a function to run is its variable where it borrows nothing", async () => {
  const dir = fixture("thread-local-fn-given");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::RefCell;
use std::rc::Rc;

thread_local! {
    static CURRENT: RefCell<Option<Rc<String>>> = const { RefCell::new(None) };
    static LOG: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };
}

pub fn start(name: &str) {
    CURRENT.set(Some(Rc::new(name.to_string())));
}

pub fn active() -> Option<Rc<String>> {
    CURRENT.with_borrow(Clone::clone)
}

fn grow(log: &Vec<u32>) -> usize {
    LOG.with_borrow_mut(|log| log.push(1));
    log.len()
}

pub fn clash() -> usize {
    LOG.with_borrow(grow)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("let current;");
  expect(js).toContain("export function active() {\n  return current;\n}");
  expect(js).toContain("$withBorrow(log, grow)");
  const lib = await import(join(dir, "lib.js"));
  expect(lib.active()).toBeUndefined();
  lib.start("a");
  expect(lib.active()).toBe("a");
  expect(() => lib.clash()).toThrow("already borrowed");
});

// ADR 0595: a thread-local \`Cell\`'s \`take\` and \`replace\` are its cell's, a
// read and a write, so it's its module's variable still: React's scheduler
// takes its timeout's id to clear it.
test("a thread-local Cell's take and replace are its variable's read and write", async () => {
  const dir = fixture("local-cell-take");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::Cell;

thread_local! {
    static TIMEOUT: Cell<Option<u32>> = const { Cell::new(None) };
}

pub fn arm(id: u32) -> Option<u32> {
    TIMEOUT.replace(Some(id))
}

pub fn cancel() -> Option<u32> {
    TIMEOUT.take()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("let timeout;");
  expect(js).toContain("export function cancel() {\n  const old = timeout;\n  timeout = undefined;\n  return old;\n}");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.arm(1), lib.arm(2), lib.cancel(), lib.cancel()]).toEqual([undefined, 1, 2, undefined]);
});

// ADR 0601: a module's variable an `if` makes, a thread-local's, is a
// `let` each branch assigns, as React's scheduler has
// `let schedulePerformWorkUntilDeadline; if (..) { .. = ..; } else if ..`,
// not a closure called where it's made.
test("a thread-local an if makes is a let each branch assigns", async () => {
  const dir = fixture("assigned-thread-local");
  writeFileSync(join(dir, "lib.rs"), `#[cfg_attr(rust_js, rust_js::link_name = "rustJsFast")]
fn fast() -> u32 {
    unreachable!()
}

thread_local! {
    static PICK: Box<dyn Fn() -> u32> = if js::is_defined(fast) {
        Box::new(fast)
    } else if js::math::random() < 2.0 {
        let n = 2;
        Box::new(move || n)
    } else {
        Box::new(|| 3)
    };
    static SEEN: u32 = {
        let at = js::math::random();
        if at < 2.0 { 1 } else { 2 }
    };
    static FOUND: u32 = loop {
        if js::math::random() < 2.0 {
            break 1;
        }
    };
}

pub fn pick() -> u32 {
    PICK.with(|pick| pick()) + SEEN.with(|s| *s) + FOUND.with(|f| *f)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('let PICK;\nif (typeof rustJsFast !== "undefined") {\n  PICK = () => rustJsFast();\n} else if');
  expect(js).toContain("} else if (Math.random() < 2) {\n  const n = 2;\n");
  // What it declares at its top would be the module's, and a loop's
  // `return` no branch's: each stays a closure called where it's made.
  expect(js).toContain("const SEEN = (() => {\n  const at = Math.random();\n");
  expect(js).toContain("const FOUND = (() => {\n  while (true) {\n");
  const lib = await import(join(dir, "lib.js"));
  expect(lib.pick()).toBe(4);
});

// ADR 0293: a `Cell` a `let` takes apart, read by `get()` before anything
// else runs, is its value in JS's destructuring: react.dev's ErrorMessage
// has `const {message, title} = error;`. Set before it's read, it's read
// where it is.
test("a Cell a let takes apart and reads at once is its value", async () => {
  const dir = fixture("early-gets");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::Cell;

pub struct Failure {
    pub title: Cell<Option<&'static str>>,
    pub message: String,
}

pub fn shown(error: &Failure) -> String {
    let Failure { message, title } = error;
    format!("{}: {}", title.get().unwrap_or("Error"), message)
}

pub fn retitled(error: &Failure) -> Option<&'static str> {
    let Failure { title, .. } = error;
    error.title.set(Some("new"));
    title.get()
}

pub fn twice(error: &Failure) -> usize {
    let Failure { title, .. } = error;
    let mut n = 0;
    for _ in 0..2 {
        n += title.get().map_or(0, |t| t.len());
        error.title.set(None);
    }
    n
}

pub fn looped(error: &Failure) -> usize {
    let Failure { title, .. } = error;
    let mut n = 0;
    let mut turns = 0;
    loop {
        n += title.get().map_or(0, |t| t.len());
        turns += 1;
        if turns == 2 {
            break;
        }
        error.title.set(None);
    }
    n
}

pub fn aliased(error: &Failure) -> (Option<&'static str>, Option<&'static str>) {
    let Failure { title, .. } = error;
    let first = title.get();
    let other = title;
    other.set(Some("set"));
    (first, error.title.get())
}

struct Retitle<'a>(&'a Failure);

impl Drop for Retitle<'_> {
    fn drop(&mut self) {
        self.0.title.set(Some("dropped"));
    }
}

pub fn dropped(error: &Failure) -> Option<&'static str> {
    let Failure { title, .. } = error;
    {
        let _retitle = Retitle(error);
    }
    title.get()
}

pub fn rebound(error: &Failure, other: &Failure) -> Option<&'static str> {
    let mut error = error;
    let Failure { title, .. } = error;
    error = other;
    error.title.set(Some("other"));
    title.get()
}

pub fn rebound_set(error: &Failure, other: &Failure) -> Option<&'static str> {
    let mut error = error;
    let Failure { title, .. } = error;
    error = other;
    title.set(Some("first"));
    error.title.get()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("const { message, title } = error;");
  expect(js.match(/const \{ title \} = /g)).toBeNull();
  const lib = await import(join(dir, "lib.js"));
  expect([lib.shown({ title: undefined, message: "m" }), lib.shown({ title: "T", message: "m" }), lib.retitled({ title: "old", message: "" }), lib.twice({ title: "ab", message: "" }), lib.looped({ title: "ab", message: "" }), lib.aliased({ title: "T", message: "" }), lib.dropped({ title: "T", message: "" })])
    .toEqual(["Error: m", "T: m", "new", 2, 2, ["T", "set"], "dropped"]);
  const [first, other] = [{ title: "a", message: "" }, { title: "b", message: "" }];
  expect([lib.rebound({ title: "a", message: "" }, { title: "b", message: "" }), lib.reboundSet(first, other), first.title]).toEqual(["a", "b", "first"]);
});

// A thread-local's `with` of a closure of one statement is the statement,
// the thread-local in its parameter's place, as react.dev's runESLint
// writes `linter.defineRules({..})`: no closure called in place.
test("with of a one-statement closure is its statement", async () => {
  const dir = fixture("with-statement");
  writeFileSync(join(dir, "lib.rs"), `#[cfg_attr(rust_js, rust_js::link_name = "globalThis.seen.push")] #[allow(unused_variables)] fn seen(s: &str) { unreachable!() }
pub struct Logger {
    pub prefix: &'static str,
}
thread_local! {
    static LOGGER: Logger = Logger { prefix: "a" };
}
pub fn logged() {
    LOGGER.with(|it| {
        seen(it.prefix);
    });
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("export function logged() {\n  globalThis.seen.push(logger.prefix);\n}");
  (globalThis as { seen?: string[] }).seen = [];
  const lib = await import(join(dir, "lib.js"));
  lib.logged();
  expect((globalThis as { seen?: string[] }).seen).toEqual(["a"]);
});

// MIR's cell only its function uses is a plain variable (tidy's
// \`plain_cells\`), but not where a callback's parameter of its name reads
// a \`value\` of its own, \`{:?}\`'s of a \`Vec<&mut str>\`.
test("a cell a callback's parameter shadows stays a cell", async () => {
  const dir = fixture("cell-shadowed");
  writeFileSync(join(dir, "lib.rs"), `pub fn show() -> String {
    let item = Box::leak(Box::new(5));
    *item += 1;
    let v: Vec<&'static mut str> = vec![String::from("a").leak()];
    format!("{item} {v:?}")
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const lib = await import(join(dir, "mir.js"));
  expect(lib.show()).toBe('6 ["a"]');
});

// A shared cell, `Rc<Cell<bool>>`, cloned in a closure that sets it: its
// function's `let`, which the closures share (ADR 0287).
test("a cell cloned in a closure is its function's let", async () => {
  const dir = fixture("cell-cloned-in-closure");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::Cell;
use std::rc::Rc;

pub fn toggles() -> (bool, bool) {
    let running = Rc::new(Cell::new(false));
    let start = {
        let running = running.clone();
        move || {
            if running.get() {
                return false;
            }
            running.set(true);
            let running = running.clone();
            let stop = move || running.set(false);
            stop();
            true
        }
    };
    (start(), running.get())
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain("let running = false;");
  expect(js).not.toContain(".value");
  const lib = await import(join(dir, "mir.js"));
  expect(lib.toggles()).toEqual([true, false]);
});

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

// ADR 0704: a thread-local an `if` makes `None` of where it doesn't, as
// React's componentFrameCache is `let componentFrameCache; if (__DEV__) {
// componentFrameCache = new WeakMap(); }`, is assigned only where it's
// something: `let` leaves it `undefined` already, and an `else` of nothing
// is gone. A branch left with nothing to do is the other's test.
test("a thread-local an if makes None of is assigned only where it's something", async () => {
  const dir = fixture("assigned-none");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::RefCell;

thread_local! {
    static CACHE: Option<&'static RefCell<Vec<u32>>> = if js::math::random() < 2.0 {
        Some(Box::leak(Box::new(RefCell::new(Vec::new()))))
    } else {
        None
    };
    static NONE_FIRST: Option<u32> = if js::math::random() > 2.0 { None } else { Some(1) };
}

pub fn remember(n: u32) -> usize {
    if let Some(cache) = CACHE.with(|cache| *cache) {
        cache.borrow_mut().push(n);
        return cache.borrow().len();
    }
    0
}

pub fn first() -> Option<u32> {
    NONE_FIRST.with(|n| *n)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("let cache;\nif (Math.random() < 2) {\n  cache = { value: [] };\n}\n");
  expect(js).toContain("let noneFirst;\nif (!(Math.random() > 2)) {\n  noneFirst = 1;\n}\n");
  expect(js).not.toContain("= undefined;");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.remember(4), lib.remember(5), lib.first()]).toEqual([1, 2, 1]);
});

// ADR 0704, of React's componentFrameCache as its port writes it: `if
// __DEV__ { Some(..) } else { None }` of a build global, its `Some` an `if`
// of its own, is upstream's `if (__DEV__) { componentFrameCache = ..; }`,
// as written, not `if (!__DEV__) { .. = undefined; } else { .. }`.
test("a thread-local of a build global's if is assigned under the if as written", () => {
  const dir = fixture("assigned-dev");
  writeFileSync(join(dir, "lib.rs"), `use js::{JsObject, WeakMap};

unsafe extern "Rust" {
    pub safe static __DEV__: bool;
}

thread_local! {
    static COMPONENT_FRAME_CACHE: Option<&'static WeakMap<JsObject, String>> =
        if __DEV__ {
            Some(if js::is_defined(js::weak_map::new::<JsObject, String>) {
                js::weak_map::new()
            } else {
                js::map::new()
            })
        } else {
            None
        };
}

pub fn cached(key: &JsObject) -> Option<String> {
    COMPONENT_FRAME_CACHE.with(|cache| *cache).and_then(|cache| cache.get(key))
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain(
    'let componentFrameCache;\nif (__DEV__) {\n  componentFrameCache = typeof WeakMap !== "undefined" ? new WeakMap() : new Map();\n}\n',
  );
});

// ADR 0703: a thread-local's `.with(|c| *c)` is its variable (ADR 0270)
// where something's there to drop as a panic unwinds too, React's
// `COMPONENT_FRAME_CACHE.with(|cache| *cache)` under its `finally`'s guard:
// the closure made where it's applied, so nothing's called and nothing's
// tried, not `const value = (cache) => cache; try { tmp = value(cache); }`.
test("a thread-local's with is its variable under a guard", async () => {
  const dir = fixture("with-under-guard");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::{Cell, RefCell};

thread_local! {
    static BUSY: Cell<bool> = const { Cell::new(false) };
    static CACHE: Option<&'static RefCell<Vec<String>>> = Some(Box::leak(Box::new(RefCell::new(Vec::new()))));
}

struct Restore;

impl Drop for Restore {
    fn drop(&mut self) {
        BUSY.set(false);
    }
}

fn checked(name: &str) -> String {
    if name.is_empty() {
        panic!("empty");
    }
    name.to_string()
}

pub fn describe(name: &str) -> usize {
    BUSY.set(true);
    let _restore = Restore;
    let frame = checked(name);
    if let Some(cache) = CACHE.with(|cache| *cache) {
        cache.borrow_mut().push(frame);
        return cache.borrow().len();
    }
    0
}

pub fn busy() -> bool {
    BUSY.get()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("  if (cache != null) {\n");
  expect(js).not.toContain("=> cache");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.describe("a"), lib.describe("b"), lib.busy()]).toEqual([1, 2, false]);
  expect(() => lib.describe("")).toThrow("empty");
  expect(lib.busy()).toBe(false);
});
