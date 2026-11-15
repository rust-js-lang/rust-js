// An `Option` a test of it proved there, read where every path took the
// test's edge `None` never takes, is given as it is: no `?? null` at a
// nullable exit, no `$unwrap` of its `unwrap()` (#1933, `proved_present`,
// MIR's one answer of it). The tests: `is_some()`'s true edge,
// `is_none()`'s false one, a `match` or `if let` of it, `Some`'s arm,
// `js::type_of(x) == "function"` and `js::truthy` (test/lowering-nullable-
// truthy.test.ts). Each beside a second condition, `&& flag`, which no
// test of the printed JS reads as saying it. Each compiled and run.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildReact, compiler, fixture, run, target } from "./support";

beforeAll(buildReact, 600_000);

// Compiled counting its decisions: what tidy's tests of the printed JS
// answered of these, `tidy-present-in` and `tidy-nulled-as-they-are`,
// which MIR's answer leaves nothing to (#1933's step 0 counters).
const compile = (name: string, source: string) => {
  const dir = fixture(name);
  writeFileSync(join(dir, "lib.rs"), source);
  const counts = join(dir, "decisions.tsv");
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--edition=2024", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target], 120_000, { RUST_JS_DECISIONS: counts });
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  const fn = (name: string) => js.slice(js.indexOf(`export function ${name}(`), js.indexOf("\n}\n", js.indexOf(`export function ${name}(`)) + 2);
  const tidied = readFileSync(counts, "utf8")
    .split("\n")
    .map((line) => line.split("\t"))
    .filter((cells) => cells[0]!.startsWith("tidy-") && Number(cells[4]) > 0)
    .map((cells) => `${cells[0]} ${cells[1]}`);
  return { dir, fn, tidied };
};

test("an Option is_some, is_none, a match and a typeof of \"function\" proved there is given as it is", async () => {
  const compiled = compile(
    "nullable-tests",
    `use js::Unknown;

#[cfg_attr(rust_js, rust_js::nullable)]
pub fn some(value: Option<&'static Unknown>, flag: bool) -> Option<&'static Unknown> {
    if value.is_some() && flag {
        return value;
    }
    None
}

#[cfg_attr(rust_js, rust_js::nullable)]
pub fn none(value: Option<&'static Unknown>, flag: bool) -> Option<&'static Unknown> {
    if value.is_none() || !flag {
        return None;
    }
    value
}

#[cfg_attr(rust_js, rust_js::nullable)]
pub fn matched(value: Option<&'static Unknown>, flag: bool) -> Option<&'static Unknown> {
    if let Some(_) = value
        && flag
    {
        return value;
    }
    None
}

#[cfg_attr(rust_js, rust_js::nullable)]
pub fn arms(value: Option<&'static Unknown>, flag: bool) -> Option<&'static Unknown> {
    match value {
        None => None,
        _ if flag => value,
        _ => None,
    }
}

#[cfg_attr(rust_js, rust_js::nullable)]
pub fn function(value: Option<&'static Unknown>, flag: bool) -> Option<&'static Unknown> {
    if js::type_of(value) == "function" && flag {
        return value;
    }
    None
}

pub fn unwrapped(value: Option<&'static Unknown>, flag: bool) -> &'static Unknown {
    if js::truthy(value) && flag {
        return value.unwrap();
    }
    js::unknown(&0.0)
}
`,
  );
  const { dir, fn } = compiled;
  const { tidied } = compiled;
  expect(tidied).toEqual([]);
  for (const name of ["some", "none", "matched", "arms", "function"]) expect([name, fn(name).includes("??")]).toEqual([name, false]);
  expect(fn("unwrapped")).not.toContain("$unwrap");
  const lib = await import(join(dir, "lib.js"));
  const one = { id: 1 };
  const f = () => 1;
  for (const name of ["some", "none", "matched", "arms"]) {
    expect([name, lib[name](one, true), lib[name](one, false), lib[name](undefined, true), lib[name](null, true)]).toEqual([name, one, null, null, null]);
  }
  expect([lib.function(f, true), lib.function(f, false), lib.function(one, true), lib.function(undefined, true), lib.function(null, true)]).toEqual([f, null, null, null, null]);
  expect([lib.unwrapped(one, true), lib.unwrapped(one, false), lib.unwrapped(undefined, true)]).toEqual([one, 0, 0]);
});

// What a test says of its edge alone: `is_none()`'s true edge, a typeof of
// a kind `null` has too, `"object"`, and a value written again after the
// test, keep their `?? null`, as each may be `None` there.
test("an Option's is_none true edge, typeof of \"object\" and a value written after the test aren't proved there", async () => {
  const { dir, fn } = compile(
    "nullable-tests-not",
    `use js::Unknown;

#[cfg_attr(rust_js, rust_js::nullable)]
pub fn none(value: Option<&'static Unknown>, flag: bool) -> Option<&'static Unknown> {
    if value.is_none() && flag {
        return value;
    }
    None
}

#[cfg_attr(rust_js, rust_js::nullable)]
pub fn object(value: Option<&'static Unknown>, flag: bool) -> Option<&'static Unknown> {
    if js::type_of(value) == "object" && flag {
        return value;
    }
    None
}

#[cfg_attr(rust_js, rust_js::nullable)]
pub fn rewritten(first: Option<&'static Unknown>, second: Option<&'static Unknown>, flag: bool) -> Option<&'static Unknown> {
    let mut found = first;
    if found.is_some() && flag {
        found = second;
        return found;
    }
    None
}
`,
  );
  for (const name of ["none", "object", "rewritten"]) expect([name, fn(name).includes("?? null")]).toEqual([name, true]);
  const lib = await import(join(dir, "lib.js"));
  const one = { id: 1 };
  expect([lib.none(undefined, true), lib.none(null, true), lib.none(one, true)]).toEqual([null, null, null]);
  expect([lib.object(null, true), lib.object(one, true), lib.object(undefined, true)]).toEqual([null, one, null]);
  expect([lib.rewritten(one, undefined, true), lib.rewritten(one, one, true)]).toEqual([null, one]);
});

// A read a test proved there is so only there: one tidy moves past the
// test, `x == null ? null : x` made `x ?? null` or two arms alike made one,
// is what it is where it's read now, its `None` `null` at a nullable exit
// (#1942: `if js::truthy(x) { x } else { x }` returned `undefined`). Of the
// program's build and the library's.
test("what a test proved of a read isn't moved past the test", async () => {
  const source = `use js::Unknown;

#[cfg_attr(rust_js, rust_js::nullable)]
pub fn matched(deps: Option<&'static Unknown>) -> Option<&'static Unknown> {
    match deps {
        None => js::null(),
        deps => deps,
    }
}

#[cfg_attr(rust_js, rust_js::nullable)]
pub fn some(deps: Option<&'static Unknown>) -> Option<&'static Unknown> {
    if deps.is_some() { deps } else { js::null() }
}

#[cfg_attr(rust_js, rust_js::nullable)]
pub fn alike(x: Option<&'static Unknown>) -> Option<&'static Unknown> {
    let kept = if js::truthy(x) { x } else { x };
    kept
}

#[cfg_attr(rust_js, rust_js::nullable)]
pub fn alike_some(x: Option<&'static Unknown>) -> Option<&'static Unknown> {
    let kept = if x.is_some() { x } else { x };
    kept
}
`;
  for (const library of [false, true]) {
    const dir = fixture(library ? "nullable-tests-moved-library" : "nullable-tests-moved");
    writeFileSync(join(dir, "lib.rs"), source);
    const mode = library ? ["--library", "--manifest", join(dir, "lib.manifest.json")] : [];
    run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...mode, "--", "--crate-name", "moved", "--crate-type=lib", "--edition=2024", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
    const lib = await import(join(dir, "lib.js"));
    const one = { id: 1 };
    for (const name of ["matched", "some", "alike", "alikeSome"]) {
      expect([library, name, lib[name](undefined), lib[name](null), lib[name](one)]).toEqual([library, name, null, null, one]);
    }
  }
});

// What a test proved of a read is no other knowledge of it: an `Option`
// made of it where it's tested, `kind.object()`'s `t ? v : undefined`, is
// there where `t` holds, its test `t` (ADR 0910), as of any read (React
// DOM's hydration of an object `src`).
test("a proved read's object, made where it's tested, is tested by that test", async () => {
  const { dir, fn } = compile(
    "nullable-tests-object",
    `use js::Unknown;

fn used(_: &Unknown) {}

pub fn shown(value: Option<&'static Unknown>, flag: bool) -> &'static str {
    if value.is_some() {
        if flag && let Some(object) = js::classify(value).object() {
            used(js::unknown(object));
            return "object";
        }
    }
    "other"
}
`,
  );
  expect(fn("shown")).toContain(`if (flag && typeof value === "object" && value !== null) {`);
  const lib = await import(join(dir, "lib.js"));
  expect([lib.shown({}, true), lib.shown(null, true), lib.shown(1, true), lib.shown({}, false)]).toEqual(["object", "other", "other", "other"]);
});

// What a nullable `RefCell` field holds, read through its guard and
// cloned, `let child = current.child.borrow().clone();`, is `null` or a
// value: stored in another nullable place as it is, no `?? null`, which
// MIR's nulled owner says, not tidy's reading of the JS (#1933's step
// 1(c)). Of a field that isn't nullable, whose `None` is `undefined`, the
// store keeps its `?? null`.
test("a nullable RefCell field's clone is stored in a nullable place as it is", async () => {
  const { dir, fn, tidied } = compile(
    "nullable-tests-cloned",
    `use std::cell::RefCell;
use std::rc::Rc;

pub struct Fiber {
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub child: RefCell<Option<Rc<Fiber>>>,
    pub sibling: RefCell<Option<Rc<Fiber>>>,
    pub id: u32,
}

pub fn fiber(id: u32) -> Fiber {
    Fiber { child: RefCell::new(None), sibling: RefCell::new(None), id }
}

pub fn swap_children(base: &Fiber, pending: &Fiber) {
    let base_first = base.child.borrow().clone();
    let pending_first = pending.child.borrow().clone();
    base.child.replace(pending_first);
    pending.child.replace(base_first);
}

pub fn sibling_as_child(to: &Fiber, from: &Fiber) {
    let sibling = from.sibling.borrow().clone();
    to.child.replace(sibling);
}
`,
  );
  expect(tidied).toEqual([]);
  expect(fn("swapChildren")).not.toContain("??");
  expect(fn("siblingAsChild")).toContain("?? null");
  const lib = await import(join(dir, "lib.js"));
  const [a, b] = [lib.fiber(1), lib.fiber(2)];
  lib.swapChildren(a, b);
  expect([a.child, b.child]).toEqual([null, null]);
  a.child = b;
  lib.swapChildren(a, b);
  expect([a.child, b.child === b]).toEqual([null, true]);
  lib.siblingAsChild(a, b);
  expect(a.child).toBe(null);
});

// What a nullable cell field held, read whole, `RefCell::replace`'s and
// `take()`'s old value and a `Cell`'s `get()`, is `null` or a value, so
// it's restored as it is, `task.keyPath = prevKeyPath`, as React Fizz's
// tasks are; and a `let mut` written `js::null()` and then a nullable
// function's result is returned as it is: the nulled owner's answers,
// none of tidy's (#1933's step 1(c)). A cell field of a struct only JS
// makes, a getter's, and a `let mut` written `None`, `undefined`, keep their
// `?? null`.
// A closure's read of such a field, a stack cursor's, is the
// representation's answer: never `undefined`, its return as it is.
test("a nullable cell field's old value and get() are restored as they are", async () => {
  const { dir, fn, tidied } = compile(
    "nullable-tests-restored",
    `use std::cell::{Cell, RefCell};
use std::rc::Rc;

pub struct Key {
    pub id: u32,
}

pub struct Task {
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub key_path: RefCell<Option<Rc<Key>>>,
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub row: RefCell<Option<Rc<Key>>>,
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub preamble: Cell<Option<&'static Key>>,
}

pub fn task() -> Rc<Task> {
    Rc::new(Task { key_path: RefCell::new(None), row: RefCell::new(None), preamble: Cell::new(None) })
}

pub fn render(task: &Rc<Task>, key: &Rc<Key>, preamble: &'static Key, work: &dyn Fn()) {
    let prev_key_path = task.key_path.replace(Some(Rc::clone(key)));
    let prev_row = task.row.take();
    let prev_preamble = task.preamble.get();
    task.preamble.set(Some(preamble));
    work();
    task.key_path.replace(prev_key_path);
    task.row.replace(prev_row);
    task.preamble.set(prev_preamble);
}

#[cfg_attr(rust_js, rust_js::nullable)]
pub fn key_of(task: &Task) -> Option<Rc<Key>> {
    task.key_path.borrow().clone()
}

#[cfg_attr(rust_js, rust_js::nullable)]
pub fn active(task: &Task, look: bool) -> Option<Rc<Key>> {
    let mut active = js::null();
    if look {
        active = key_of(task);
    }
    active
}

#[cfg_attr(rust_js, rust_js::nullable)]
pub fn unset(task: &Task, look: bool) -> Option<Rc<Key>> {
    let mut active = None;
    if look {
        active = key_of(task);
    }
    active
}

pub struct Given {
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub row: RefCell<Option<Rc<Key>>>,
}

pub struct StackCursor<T> {
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub current: RefCell<T>,
}

thread_local! {
    static CURSOR: StackCursor<Option<Rc<Key>>> = const { StackCursor { current: RefCell::new(None) } };
}

pub fn push(key: &Rc<Key>) {
    CURSOR.with(|cursor| cursor.current.replace(Some(Rc::clone(key))));
}

#[cfg_attr(rust_js, rust_js::nullable)]
pub fn current() -> Option<Rc<Key>> {
    CURSOR.with(|cursor| cursor.current.borrow().clone())
}

pub fn given(value: Option<&'static js::Unknown>, to: &Task) {
    let from: &Given = unsafe { js::cast(value) };
    let row = from.row.replace(None);
    to.row.replace(row);
}
`,
  );
  expect(tidied).toEqual([]);
  expect(fn("render")).not.toContain("??");
  expect(fn("keyOf")).toContain("return task.keyPath;");
  expect(fn("active")).not.toContain("??");
  expect(fn("unset")).toContain("return active ?? null;");
  expect(fn("given")).toContain("?? null");
  // A closure's read, which the representation says is never `undefined`.
  expect(fn("current")).not.toContain("??");
  const lib = await import(join(dir, "lib.js"));
  const [task, key, preamble, other] = [lib.task(), { id: 1 }, { id: 2 }, { id: 3 }];
  task.row = other;
  const seen: unknown[][] = [];
  lib.render(task, key, preamble, () => seen.push([task.keyPath, task.row, task.preamble]));
  expect(seen).toEqual([[key, null, preamble]]);
  expect([task.keyPath, task.row, task.preamble]).toEqual([null, other, null]);
  expect([lib.active(task, true), lib.active(task, false), lib.unset(task, false)]).toEqual([null, null, null]);
  task.keyPath = key;
  expect([lib.active(task, true), lib.unset(task, true)]).toEqual([key, key]);
  const to = lib.task();
  lib.given({ row: undefined }, to);
  expect(to.row).toBe(null);
  expect(lib.current()).toBe(null);
  lib.push(key);
  expect(lib.current()).toBe(key);
});

// A struct's `RefCell` field tested, then read again and unwrapped where
// nothing runs between but that field's own reads, `while
// node.sibling.borrow().is_none() { .. }` then
// `node.sibling.borrow().clone().unwrap()`, is the field as it is: MIR's
// answer (`field_read_proved`, #1933's step 1(b)), none of tidy's. Not
// where a call runs between, nor of a struct JS gives, whose getter may
// answer each read anew: its `$unwrap` stays, which tidy's reading of the
// printed JS dropped (#1950).
test("a RefCell field tested and read again with nothing between is unwrapped as it is", async () => {
  const own = compile(
    "nullable-tests-field-paths",
    `use std::cell::RefCell;
use std::rc::Rc;

struct Fiber {
    id: u32,
    sibling: RefCell<Option<Rc<Fiber>>>,
    parent: RefCell<Option<Rc<Fiber>>>,
}

fn fiber(id: u32) -> Rc<Fiber> {
    Rc::new(Fiber { id, sibling: RefCell::new(None), parent: RefCell::new(None) })
}

fn next_sibling(fiber: &Rc<Fiber>) -> u32 {
    let mut node = Rc::clone(fiber);
    while node.sibling.borrow().is_none() {
        let parent = node.parent.borrow().clone();
        match parent {
            Some(parent) => node = parent,
            None => return 0,
        }
    }
    let sibling = node.sibling.borrow().clone().unwrap();
    sibling.id
}

fn tested(node: &Fiber) -> u32 {
    if node.sibling.borrow().is_some() {
        let sibling = node.sibling.borrow().clone().unwrap();
        return sibling.id;
    }
    0
}

fn called(node: &Fiber, between: &dyn Fn()) -> u32 {
    if node.sibling.borrow().is_some() {
        between();
        let sibling = node.sibling.borrow().clone().unwrap();
        return sibling.id;
    }
    0
}

fn rebound(a: &Fiber, b: &Fiber) -> u32 {
    let mut node = a;
    if node.sibling.borrow().is_some() {
        node = b;
        let sibling = node.sibling.borrow().clone().unwrap();
        return sibling.id;
    }
    0
}

#[derive(Clone)]
struct Holder(u32);

struct Holding {
    held: RefCell<Option<Holder>>,
}

fn held(holding: &Holding) -> u32 {
    if holding.held.borrow().is_some() {
        return holding.held.borrow().clone().unwrap().0;
    }
    0
}

pub fn run() -> Vec<u32> {
    let (root, a, b) = (fiber(1), fiber(2), fiber(3));
    a.parent.replace(Some(Rc::clone(&root)));
    root.sibling.replace(Some(Rc::clone(&b)));
    let holding = Holding { held: RefCell::new(Some(Holder(5))) };
    vec![next_sibling(&a), next_sibling(&b), tested(&root), tested(&a), called(&root, &|| {}), rebound(&root, &root), held(&holding)]
}
`,
  );
  expect(own.tidied).toEqual([]);
  const js = readFileSync(join(own.dir, "lib.js"), "utf8");
  const body = (name: string) => js.slice(js.indexOf(`function ${name}(`), js.indexOf("\n}\n", js.indexOf(`function ${name}(`)));
  expect(body("called")).toContain("$unwrap(node.sibling)");
  expect(body("rebound")).toContain("$unwrap(node.sibling)");
  // A clone that may run the crate's code: not one this proves.
  expect(body("held")).toContain("$unwrap(");
  expect(body("nextSibling")).not.toContain("$unwrap");
  expect(body("tested")).not.toContain("$unwrap");
  const ownLib = await import(join(own.dir, "lib.js"));
  expect(ownLib.run()).toEqual([3, 0, 3, 0, 3, 3, 5]);
  // A struct JS gives an exported function: not MIR's to prove.
  const given = compile(
    "nullable-tests-field-paths-given",
    `use std::cell::RefCell;
use std::rc::Rc;

pub struct Fiber {
    pub id: u32,
    pub sibling: RefCell<Option<Rc<Fiber>>>,
}

pub fn tested(node: &Fiber) -> u32 {
    if node.sibling.borrow().is_some() {
        let sibling = node.sibling.borrow().clone().unwrap();
        return sibling.id;
    }
    0
}
`,
  );
  expect(given.tidied).toEqual([]);
  expect(given.fn("tested")).toContain("$unwrap(node.sibling)");
  // Its getter answers `Some`, then `None`: Rust's `unwrap()` panics, as
  // `$unwrap` throws, where a read as it is threw a `TypeError` (#1950).
  const givenLib = await import(join(given.dir, "lib.js"));
  let reads = 0;
  const node = {
    id: 1,
    get sibling() {
      reads += 1;
      return reads === 1 ? { id: 3, sibling: undefined } : undefined;
    },
  };
  expect(() => givenLib.tested(node)).toThrow("called `Option::unwrap()` on a `None` value");
  expect(givenLib.tested({ id: 1, sibling: { id: 4, sibling: undefined } })).toBe(4);
  // One only the crate constructs, which exported functions take, is plain
  // data, no getter's: MIR's to prove (Tu's decision, 2026-11-15, ADR
  // 0262's difference 60).
  const built = compile(
    "nullable-tests-field-paths-built",
    `use std::cell::RefCell;
use std::rc::Rc;

pub struct Fiber {
    pub id: u32,
    pub sibling: RefCell<Option<Rc<Fiber>>>,
}

pub fn fiber(id: u32, sibling: Option<Rc<Fiber>>) -> Rc<Fiber> {
    Rc::new(Fiber { id, sibling: RefCell::new(sibling) })
}

pub fn tested(node: &Fiber) -> u32 {
    if node.sibling.borrow().is_some() {
        let sibling = node.sibling.borrow().clone().unwrap();
        return sibling.id;
    }
    0
}

// A WebIDL dictionary, which JS builds too, the crate's or not: read anew.
#[cfg_attr(rust_js, rust_js::dictionary)]
pub struct Init {
    pub id: u32,
    pub next: RefCell<Option<Rc<Init>>>,
}

pub fn init(id: u32) -> Rc<Init> {
    Rc::new(Init { id, next: RefCell::new(None) })
}

pub fn next_id(init: &Init) -> u32 {
    if init.next.borrow().is_some() {
        let next = init.next.borrow().clone().unwrap();
        return next.id;
    }
    0
}
`,
  );
  expect(built.fn("nextId")).toContain("$unwrap(init.next)");
  expect(built.tidied).toEqual([]);
  expect(built.fn("tested")).toContain("const sibling = node.sibling;");
  const builtLib = await import(join(built.dir, "lib.js"));
  expect([builtLib.tested(builtLib.fiber(1, builtLib.fiber(5, null))), builtLib.tested(builtLib.fiber(1, null))]).toEqual([5, 0]);
});

// A variable a part of an `&&` shows there, read right after it, as React's
// Flight client's `deferred`, lent as a `&mut` so MIR proves nothing of it:
// stored as it is, tidy's reading of the JS (ADR 0957); of an `||`'s part,
// `?? null`. A field so shown is MIR's alone (#1950, above).
test("a variable a part of an && shows there is stored as it is", async () => {
  const { dir, fn, tidied } = compile(
    "nullable-tests-and-shown",
    `use std::cell::RefCell;

pub struct Chunk {
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub value: RefCell<Option<Vec<u32>>>,
    pub blocked: bool,
}

pub fn defer(items: Vec<u32>, chunk: &Chunk) {
    let mut deferred: Option<Vec<u32>> = None;
    for item in items {
        deferred.get_or_insert_with(Vec::new).push(item);
    }
    if deferred.is_some() && chunk.blocked {
        *chunk.value.borrow_mut() = deferred;
    }
}

// A field so shown, of a struct JS gives: \`?? null\` (#1950).
pub struct Given {
    pub pending: Option<u32>,
    pub blocked: bool,
}

pub struct Target {
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub pending: Option<u32>,
}

pub fn restore(given: &Given, to: &mut Target) {
    if given.blocked && given.pending.is_some() {
        to.pending = given.pending;
    }
}

pub fn defer_or(items: Vec<u32>, chunk: &Chunk) {
    let mut deferred: Option<Vec<u32>> = None;
    for item in items {
        deferred.get_or_insert_with(Vec::new).push(item);
    }
    if chunk.blocked || deferred.is_some() {
        *chunk.value.borrow_mut() = deferred;
    }
}
`,
  );
  expect(tidied).toEqual(["tidy-nulled-as-they-are and-shown"]);
  expect(fn("defer")).toContain("    chunk.value = deferred;\n");
  expect(fn("deferOr")).toContain("    chunk.value = deferred ?? null;\n");
  const lib = await import(join(dir, "lib.js"));
  const chunk: { value: unknown; blocked: boolean } = { value: null, blocked: true };
  lib.defer([4, 5], chunk);
  expect(chunk.value).toEqual([4, 5]);
  lib.deferOr([], chunk);
  expect(chunk.value).toBe(null);
  // Its getter answers a value, then `undefined`: Rust's second read is a
  // `None`, stored `null` (#1950).
  expect(fn("restore")).toContain("?? null");
  let reads = 0;
  const to: { pending: unknown } = { pending: 1 };
  lib.restore({ blocked: true, get pending() { reads += 1; return reads === 1 ? 7 : undefined; } }, to);
  expect(to.pending).toBe(null);
});
