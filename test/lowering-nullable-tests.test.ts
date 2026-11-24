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
import { withoutBoundary } from "./without-boundary";

beforeAll(buildReact, 600_000);

// Compiled counting its decisions: what tidy's tests of the printed JS
// answered of these, `tidy-present-in` and `tidy-nulled-as-they-are`,
// which MIR's answer leaves nothing to (#1933's step 0 counters).
const compile = (name: string, source: string) => {
  const dir = fixture(name);
  writeFileSync(join(dir, "lib.rs"), source);
  const counts = join(dir, "decisions.tsv");
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--edition=2024", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target], 120_000, { RUST_JS_DECISIONS: counts });
  const js = withoutBoundary(readFileSync(join(dir, "lib.js"), "utf8"));
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
// (#1942: `if js::truthy(x) { x } else { x }` returned `undefined`), as
// is one a test proved defined, `null` there (#1933 step 4 (D)). Of the
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

#[cfg_attr(rust_js, rust_js::nullable)]
pub fn alike_strict(x: Option<&'static Unknown>) -> Option<&'static Unknown> {
    let kept = if !js::strict_equals(x, None::<&Unknown>) { x } else { x };
    kept
}

#[cfg_attr(rust_js, rust_js::nullable)]
pub fn alike_classify(x: Option<&'static Unknown>) -> Option<&'static Unknown> {
    let kept = if !matches!(js::classify(x), js::Kind::Undefined) { x } else { x };
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
    for (const name of ["matched", "some", "alike", "alikeSome", "alikeStrict", "alikeClassify"]) {
      expect([library, name, lib[name](undefined), lib[name](null), lib[name](one)]).toEqual([library, name, null, null, one]);
    }
  }
});

// A program's own `js::strict_equals`, JS's `x !== null`, is no test of an
// `Option`'s being set (`Op::tests_set`, #2112 C S5): its ternary never
// folds into `?.` or `??`, `x?.a`, which of an `undefined` `x` gives
// `undefined` where the program's `x.a` throws.
test("a program's strict_equals ternary never folds into ?. or ??", async () => {
  const dir = fixture("nullable-tests-strict-equals-unfolded");
  writeFileSync(
    join(dir, "lib.rs"),
    `use js::Unknown;

pub fn read(x: &'static Unknown) -> bool {
    let a = if !js::strict_equals(x, js::null::<&Unknown>()) { js::get(x, "a") } else { None };
    a.is_some()
}
`,
  );
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--crate-name", "read", "--crate-type=lib", "--edition=2024", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = withoutBoundary(readFileSync(join(dir, "lib.js"), "utf8"));
  expect(js).toContain("x !== null ? x.a : undefined");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.read({ a: 1 }), lib.read(null)]).toEqual([true, false]);
  expect(() => lib.read(undefined)).toThrow(TypeError);
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
  const js = withoutBoundary(readFileSync(join(own.dir, "lib.js"), "utf8"));
  const body = (name: string) => js.slice(js.indexOf(`function ${name}(`), js.indexOf("\n}\n", js.indexOf(`function ${name}(`)));
  expect(body("called")).toContain('$unwrap(node.sibling, "lib.rs:');
  expect(body("rebound")).toContain('$unwrap(node.sibling, "lib.rs:');
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
  expect(given.fn("tested")).toContain('$unwrap(node.sibling, "lib.rs:');
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
  expect(built.fn("nextId")).toContain("$unwrap(init.next, \"lib.rs:34:47\")");
  expect(built.tidied).toEqual([]);
  expect(built.fn("tested")).toContain("const sibling = node.sibling;");
  const builtLib = await import(join(built.dir, "lib.js"));
  expect([builtLib.tested(builtLib.fiber(1, builtLib.fiber(5, null))), builtLib.tested(builtLib.fiber(1, null))]).toEqual([5, 0]);
});

// A variable a part of an `&&` shows there, read right after it, as React's
// Flight client's `deferred`, lent as a `&mut` that never takes its `Some`
// away and moved into a temporary under the test: stored as it is, MIR's
// answer, the temporary a copy of what's proved where it's written
// (`proved_present`, #1933's removal); of an `||`'s part, `?? null`. A
// field so shown is MIR's alone (#1950, above).
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
  expect(tidied).toEqual([]);
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
  // Its check leaves an accessor's value to the body (ADR 1194, amended).
  lib.restore({ blocked: true, get pending() { reads += 1; return reads === 1 ? 7 : undefined; } }, to);
  expect(to.pending).toBe(null);
});

// A variable a closure captures by reference, tested and read inside it:
// MIR's to prove (`capture_read_proved`, #1933's removal), `x ?? d` under a
// test that `x` is there its `x`, as what a shared reference refers to
// can't change while it's lent. A capture its maker gives `null` or a
// value, React's Fizz's `prevTask` restored in a `catch_unwind`'s closure
// (`captures_nulled`); and not a field of a variable written between two
// tests of it.
test("a variable a closure captures and tests is MIR's to prove", async () => {
  const { dir, fn, tidied } = compile(
    "nullable-tests-lent-variables",
    `use std::cell::RefCell;
use std::rc::Rc;

pub struct Key {
    pub id: u32,
}

pub struct Task {
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub key: RefCell<Option<Rc<Key>>>,
}

pub fn task() -> Task {
    Task { key: RefCell::new(None) }
}

pub fn restored(task: &Task, key: &Rc<Key>, run: &dyn Fn(&dyn Fn())) {
    let prev = task.key.replace(Some(Rc::clone(key)));
    run(&|| {
        task.key.replace(prev.clone());
    });
    task.key.replace(prev);
}

pub fn first_or(value: Option<&'static js::Unknown>, fallback: &'static js::Unknown, run: &dyn Fn(&dyn Fn() -> bool) -> bool) -> bool {
    run(&|| {
        if value.is_some() {
            return js::strict_equals(value.unwrap_or(fallback), fallback);
        }
        false
    })
}

pub fn after_none(value: Option<&'static js::Unknown>, fallback: &'static js::Unknown, run: &dyn Fn(&dyn Fn() -> bool) -> bool) -> bool {
    run(&|| {
        if value.is_none() {
            return false;
        }
        let kept = value.unwrap_or(fallback);
        js::strict_equals(kept, fallback)
    })
}

// Not a capture lent \`&mut\`, set between, nor a test of another.
pub fn reset_or(mut value: Option<&'static js::Unknown>, fallback: &'static js::Unknown, run: &mut dyn FnMut(&mut dyn FnMut() -> bool) -> bool) -> bool {
    run(&mut || {
        if value.is_some() {
            value = None;
            return js::strict_equals(value.unwrap_or(fallback), fallback);
        }
        false
    })
}

pub fn else_read(value: Option<&'static js::Unknown>, fallback: &'static js::Unknown, run: &dyn Fn(&dyn Fn() -> bool) -> bool) -> bool {
    run(&|| if value.is_some() { false } else { js::strict_equals(value.unwrap_or(fallback), fallback) })
}

pub fn other_tested(value: Option<&'static js::Unknown>, other: Option<&'static js::Unknown>, fallback: &'static js::Unknown, run: &dyn Fn(&dyn Fn() -> bool) -> bool) -> bool {
    run(&|| other.is_some() && js::strict_equals(value.unwrap_or(fallback), fallback))
}

// Nor a copy made before its test, of what's written between.
#[cfg_attr(rust_js, rust_js::nullable)]
pub fn copied_before(mut x: Option<&'static js::Unknown>, given: &'static js::Unknown) -> Option<&'static js::Unknown> {
    let y = x;
    x = Some(given);
    if x.is_some() {
        return y;
    }
    x
}

// A test of a field of a variable written between two of them decides
// nothing of the second.
pub struct Node {
    pub next: Option<&'static Node>,
    pub id: u32,
}

pub fn walked(mut node: &'static Node) -> u32 {
    if node.next.is_none() {
        return node.id;
    }
    node = node.next.unwrap_or(node);
    if node.next.is_none() {
        return 100 + node.id;
    }
    200 + node.id
}

`,
  );
  expect(tidied).toEqual([]);
  expect(fn("restored")).not.toContain("??");
  expect(fn("firstOr")).not.toContain("??");
  expect(fn("afterNone")).not.toContain("??");
  for (const name of ["resetOr", "otherTested", "elseRead"]) expect([name, fn(name).includes("value ?? fallback")]).toEqual([name, true]);
  expect(fn("copiedBefore")).toContain("?? null");
  const lib = await import(join(dir, "lib.js"));
  const [task, key] = [lib.task(), { id: 1 }];
  let inside: unknown;
  lib.restored(task, key, (restore: () => void) => {
    inside = task.key;
    restore();
  });
  expect([inside, task.key]).toEqual([key, null]);
  const [value, fallback] = [{}, {}];
  const run = (f: () => boolean) => f();
  expect([lib.firstOr(value, fallback, run), lib.firstOr(undefined, fallback, run), lib.afterNone(value, fallback, run)]).toEqual([false, false, false]);
  expect([lib.resetOr(value, fallback, run), lib.otherTested(undefined, value, fallback, run), lib.copiedBefore(undefined, value), lib.elseRead(undefined, fallback, run)]).toEqual([true, true, null, true]);
  const last = { next: undefined, id: 3 };
  expect([lib.walked(last), lib.walked({ next: last, id: 1 }), lib.walked({ next: { next: last, id: 2 }, id: 1 })]).toEqual([3, 103, 202]);
});

test("a thread-local's with(f) gives what f returns", async () => {
  const { dir, fn, tidied } = compile(
    "nullable-tests-local-with",
    `use std::cell::{Cell, RefCell};

use js::Unknown;

pub struct StackCursor<T> {
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub current: RefCell<T>,
}

thread_local! {
    #[cfg_attr(rust_js, rust_js::nullable)]
    static CURSOR: StackCursor<Option<&'static Unknown>> = const { StackCursor { current: RefCell::new(None) } };
}

pub struct Context {
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub current_renderer: Cell<Option<&'static Unknown>>,
}

pub fn context() -> Context {
    Context { current_renderer: Cell::new(None) }
}

pub fn push(value: Option<&'static Unknown>) {
    CURSOR.with(|cursor| cursor.current.replace(value));
}

pub fn pop(context: &Context) {
    let current = CURSOR.with(|cursor| *cursor.current.borrow());
    CURSOR.with(|cursor| cursor.current.replace(None));
    context.current_renderer.set(current);
}
`,
  );
  expect(tidied).toEqual([]);
  expect(fn("pop")).not.toContain("??");
  const lib = await import(join(dir, "lib.js"));
  const context = lib.context();
  lib.pop(context);
  expect(context.currentRenderer).toBe(null);
  const renderer = {};
  lib.push(renderer);
  lib.pop(context);
  expect(context.currentRenderer).toBe(renderer);
});

// A variable written more than once, React's getIteratorFn's
// `maybeIterator`, a `match`'s in each arm, proved by its test where
// nothing writes it on any way from the test to the read: given as it is,
// MIR's answer (`unwritten_since`, #1933 step 4 (A)); and `js::is_function`'s
// true edge, React's scheduler's `continuationCallback`. Not where it's
// written between, nor where a closure that captures it by `&mut` is
// called between.
test("a variable written more than once is proved where nothing writes it after its test", async () => {
  const { dir, fn, tidied } = compile(
    "nullable-tests-written-twice",
    `#[cfg_attr(rust_js, rust_js::nullable)]
pub fn iterator_fn(value: &'static js::Unknown, faux: &'static js::Unknown) -> Option<&'static js::Unknown> {
    let found = match js::get(Some(value), "iterator") {
        found if js::truthy(found) => found,
        _ => js::get(Some(faux), "iterator"),
    };
    if js::type_of(found) == "function" {
        return found;
    }
    None
}

#[repr(transparent)]
pub struct Callback(pub Box<dyn FnMut(bool) -> Option<Callback>>);

pub struct Task {
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub callback: std::cell::RefCell<Option<Callback>>,
}

pub fn run(task: &Task) -> bool {
    if let Some(Callback(mut callback)) = task.callback.take() {
        let continuation = callback(true);
        if js::is_function(&continuation) {
            task.callback.replace(continuation);
            return true;
        }
    }
    false
}

#[cfg_attr(rust_js, rust_js::nullable)]
pub fn rewritten(value: &'static js::Unknown, faux: &'static js::Unknown, again: bool) -> Option<&'static js::Unknown> {
    let mut found = js::get(Some(value), "iterator");
    if js::truthy(found) {
        if again {
            found = js::get(Some(faux), "iterator");
        }
        return found;
    }
    None
}

#[cfg_attr(rust_js, rust_js::nullable)]
pub fn captured(value: &'static js::Unknown, faux: &'static js::Unknown) -> Option<&'static js::Unknown> {
    let mut found = js::get(Some(value), "iterator");
    let mut again = || found = js::get(Some(faux), "iterator");
    if js::truthy(js::get(Some(value), "iterator")) {
        again();
    }
    if js::truthy(found) {
        return found;
    }
    None
}
`,
  );
  expect(tidied).toEqual([]);
  expect(fn("iteratorFn")).toContain("return found;");
  expect(fn("run")).toContain("task.callback = continuation;");
  expect(fn("rewritten")).toContain("return found ?? null;");
  expect(fn("captured")).toContain("return found ?? null;");
  const lib = await import(join(dir, "lib.js"));
  const f = () => 1;
  expect([lib.iteratorFn({ iterator: f }, {}), lib.iteratorFn({}, { iterator: f }), lib.iteratorFn({}, {})]).toEqual([f, f, null]);
  const task = { callback: () => f };
  expect([lib.run(task), task.callback, lib.run(task), task.callback]).toEqual([true, f, false, null]);
  expect([lib.rewritten({ iterator: f }, {}, true), lib.rewritten({ iterator: f }, {}, false)]).toEqual([null, f]);
  expect(lib.captured({ iterator: f }, {})).toBe(null);
});

// A `let` a test shows isn't `null`, written again in the test's branch
// before it's read: its `None` given `null` by a nullable function, `return
// found ?? null`, where tidy's if-shown took it as the `const` it reads and
// gave `undefined` (#1952, P0).
test("a let written again after its test is given as a nullable function gives it", async () => {
  const { dir, fn } = compile(
    "nullable-tests-let-written-again",
    `pub struct Unit {
    pub id: u32,
}

static A: Unit = Unit { id: 1 };

fn begin(n: u32) -> Option<&'static Unit> {
    if n == 1 { Some(&A) } else { None }
}

#[cfg_attr(rust_js, rust_js::nullable)]
pub fn rewritten(n: u32, again: bool) -> Option<&'static Unit> {
    let mut found = begin(n);
    if found.is_some() {
        if again {
            found = begin(0);
        }
        return found;
    }
    None
}
`,
  );
  expect(fn("rewritten")).toContain("return found ?? null;");
  const lib = await import(join(dir, "lib.js"));
  expect(lib.rewritten(1, true)).toBe(null);
  expect(lib.rewritten(1, false).id).toBe(1);
  expect(lib.rewritten(0, false)).toBe(null);
});

// An `Option` local lent as a `&mut` that never takes its `Some` away, a
// `match &mut found`'s, getParentFragmentInstances', `get_or_insert_with`'s,
// React's `deferred`, or `as_mut()`'s: its own writes still say what it is,
// `null` or a value, and its test proves it there (`lend_keeps_some`, #1933
// step 4 (B)); one that writes `None` through the lend, `*r = None`, is
// `?? null` again.
test("a &mut lend that never takes a Some away leaves a variable's writes to say it", async () => {
  const { dir, fn, tidied } = compile(
    "nullable-tests-kept-lends",
    `#[cfg_attr(rust_js, rust_js::nullable)]
pub fn found(items: Vec<u32>) -> Option<Vec<u32>> {
    let mut found = js::null();
    for item in items {
        match &mut found {
            None => found = Some(vec![item]),
            Some(found) => found.push(item),
        }
    }
    found
}

#[cfg_attr(rust_js, rust_js::nullable)]
pub fn cleared(items: Vec<u32>, clear: bool) -> Option<Vec<u32>> {
    let mut found = js::null();
    for item in items {
        match &mut found {
            None => found = Some(vec![item]),
            Some(found) => found.push(item),
        }
    }
    if clear {
        let lent = &mut found;
        *lent = None;
    }
    found
}

#[cfg_attr(rust_js, rust_js::nullable)]
pub fn emptied(value: &'static js::Unknown) -> Option<&'static js::Unknown> {
    let mut found = js::null();
    if let None = &mut found {
        found = Some(value);
    }
    if found.is_some() {
        let lent = &mut found;
        *lent = None;
        return found;
    }
    None
}

#[cfg_attr(rust_js, rust_js::nullable)]
pub fn taken(items: Vec<u32>, clear: bool) -> Option<Vec<u32>> {
    let mut found = js::null();
    for item in items {
        found.get_or_insert_with(Vec::new).push(item);
    }
    if clear {
        found.take();
    }
    found
}

pub fn pushed_or(mut list: Option<Vec<u32>>, fallback: Vec<u32>) -> usize {
    list.get_or_insert_with(Vec::new).push(1);
    if list.is_some() {
        return list.unwrap_or(fallback).len();
    }
    0
}
`,
  );
  expect(tidied).toEqual([]);
  expect(fn("found")).toContain("return found;");
  expect(fn("cleared")).toContain("?? null");
  expect(fn("taken")).toContain("?? null");
  expect(fn("pushedOr")).toContain("return list.length;");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.found([4, 5]), lib.found([])]).toEqual([[4, 5], null]);
  expect([lib.cleared([4], false), lib.cleared([4], true), lib.cleared([], false)]).toEqual([[4], null, null]);
  expect([lib.taken([4], false), lib.taken([4], true)]).toEqual([[4], null]);
  expect(lib.emptied({})).toBe(null);
  expect([lib.pushedOr([4], [5, 6, 7]), lib.pushedOr(undefined, [5, 6, 7])]).toEqual([2, 1]);
});

// What a test proves defined there, never `undefined` though maybe `null`:
// `!matches!(js::classify(x), Kind::Undefined)`'s true edge, React's
// getTaskName's `cachedEntry` and its ref's `refProp`, and
// `js::strict_equals(x, None)`'s false one, given as they are by a
// nullable function, `return entry;`, MIR's answer (`proved_defined`,
// `Presence::Defined`, #1933 step 4 (D)), not tidy's. Through the
// `bool` `matches!` sets in each arm, a variable of it or its negation
// too, and so of `matches!(x, Some(_))`'s; not where it's written
// between, nor on `Kind::Undefined`'s own edge, nor past the test, nor of
// a `bool` a call or a caller writes as well, nor of a test of another
// value.
test("an Option a test proved defined there is given as it is, its null as null", async () => {
  const { dir, fn, tidied } = compile(
    "nullable-tests-defined",
    `use js::Kind;

#[cfg_attr(rust_js, rust_js::nullable)]
pub fn cached(info: &'static js::Unknown) -> Option<&'static js::Unknown> {
    let entry = js::get(Some(info), "debugTask");
    if !matches!(js::classify(entry), Kind::Undefined) {
        return entry;
    }
    None
}

#[cfg_attr(rust_js, rust_js::nullable)]
pub fn strict(info: &'static js::Unknown) -> Option<&'static js::Unknown> {
    let entry = js::get(Some(info), "debugTask");
    if !js::strict_equals(entry, None::<&js::Unknown>) {
        return entry;
    }
    None
}

#[cfg_attr(rust_js, rust_js::nullable)]
pub fn rewritten(info: &'static js::Unknown, again: bool) -> Option<&'static js::Unknown> {
    let mut entry = js::get(Some(info), "debugTask");
    if !matches!(js::classify(entry), Kind::Undefined) {
        if again {
            entry = js::get(Some(info), "other");
        }
        return entry;
    }
    None
}

#[cfg_attr(rust_js, rust_js::nullable)]
pub fn undefined_edge(info: &'static js::Unknown) -> Option<&'static js::Unknown> {
    let entry = js::get(Some(info), "debugTask");
    if matches!(js::classify(entry), Kind::Undefined) {
        return entry;
    }
    None
}

#[cfg_attr(rust_js, rust_js::nullable)]
pub fn named(info: &'static js::Unknown) -> Option<&'static js::Unknown> {
    let entry = js::get(Some(info), "debugTask");
    let defined = !matches!(js::classify(entry), Kind::Undefined);
    if defined {
        return entry;
    }
    None
}

#[cfg_attr(rust_js, rust_js::nullable)]
pub fn past(info: &'static js::Unknown, flag: bool) -> Option<&'static js::Unknown> {
    let entry = js::get(Some(info), "debugTask");
    if flag && !matches!(js::classify(entry), Kind::Undefined) {
        return js::get(Some(info), "other");
    }
    entry
}

#[cfg_attr(rust_js, rust_js::nullable)]
pub fn also_called(info: &'static js::Unknown, other: &'static js::Unknown) -> Option<&'static js::Unknown> {
    let entry = js::get(Some(info), "debugTask");
    let mut defined = js::truthy(Some(other));
    match js::classify(entry) {
        Kind::Undefined => {}
        _ => defined = true,
    }
    if defined {
        return entry;
    }
    None
}

#[cfg_attr(rust_js, rust_js::nullable)]
pub fn nullish(info: &'static js::Unknown) -> Option<&'static js::Unknown> {
    let entry = js::get(Some(info), "debugTask");
    if !matches!(js::classify(entry), Kind::Undefined | Kind::Null) {
        return entry;
    }
    None
}

#[cfg_attr(rust_js, rust_js::nullable)]
pub fn null_only(info: &'static js::Unknown) -> Option<&'static js::Unknown> {
    let entry = js::get(Some(info), "debugTask");
    if !matches!(js::classify(entry), Kind::Null) {
        return entry;
    }
    None
}

#[cfg_attr(rust_js, rust_js::nullable)]
pub fn given_flag(info: &'static js::Unknown, mut defined: bool) -> Option<&'static js::Unknown> {
    let entry = js::get(Some(info), "debugTask");
    match js::classify(entry) {
        Kind::Undefined => {}
        _ => defined = true,
    }
    if defined {
        return entry;
    }
    None
}

#[cfg_attr(rust_js, rust_js::nullable)]
pub fn strict_other(info: &'static js::Unknown, other: Option<&'static js::Unknown>) -> Option<&'static js::Unknown> {
    let entry = js::get(Some(info), "debugTask");
    if !js::strict_equals(entry, other) {
        return entry;
    }
    None
}

#[cfg_attr(rust_js, rust_js::nullable)]
pub fn classify_other(info: &'static js::Unknown, other: Option<&'static js::Unknown>) -> Option<&'static js::Unknown> {
    let entry = js::get(Some(info), "debugTask");
    if !matches!(js::classify(other), Kind::Undefined) {
        return entry;
    }
    None
}

#[cfg_attr(rust_js, rust_js::nullable)]
pub fn some_matched(x: Option<&'static js::Unknown>) -> Option<&'static js::Unknown> {
    if matches!(x, Some(_)) {
        return x;
    }
    None
}
`,
  );
  expect(tidied).toEqual([]);
  expect(fn("cached")).toContain("return entry;");
  expect(fn("strict")).toContain("return entry;");
  expect(fn("named")).toContain("return entry;");
  expect(fn("nullish")).toContain("return entry;");
  expect(fn("someMatched")).toContain("return x;");
  expect(fn("rewritten")).toContain("return entry ?? null;");
  expect(fn("undefinedEdge")).toContain("return entry ?? null;");
  expect(fn("past")).toContain("return entry ?? null;");
  expect(fn("alsoCalled")).toContain("return entry ?? null;");
  for (const name of ["nullOnly", "givenFlag", "strictOther", "classifyOther"]) expect([name, fn(name).includes("return entry ?? null;")]).toEqual([name, true]);
  const lib = await import(join(dir, "lib.js"));
  const f = () => 1;
  for (const given of [lib.cached, lib.strict, lib.named, lib.nullish]) {
    expect([given({ debugTask: f }), given({ debugTask: null }), given({})]).toEqual([f, null, null]);
  }
  expect([lib.past({}, false), lib.past({ debugTask: f }, false), lib.past({ debugTask: f, other: f }, true)]).toEqual([null, f, f]);
  expect([lib.alsoCalled({}, 1), lib.alsoCalled({}, 0), lib.alsoCalled({ debugTask: f }, 0)]).toEqual([null, null, f]);
  expect([lib.someMatched(undefined), lib.someMatched(null), lib.someMatched(f)]).toEqual([null, null, f]);
  expect([lib.givenFlag({}, true), lib.givenFlag({ debugTask: f }, false), lib.strictOther({}, f), lib.classifyOther({}, f), lib.nullOnly({})]).toEqual([null, f, null, null, null]);
  expect([lib.rewritten({ debugTask: f }, true), lib.rewritten({ debugTask: f }, false), lib.rewritten({}, false)]).toEqual([null, f, null]);
  expect([lib.undefinedEdge({}), lib.undefinedEdge({ debugTask: f })]).toEqual([null, null]);
});

// An `Option` a test proved there, stored through a `RefCell`'s borrow of a
// nullable field, `*value = deferred` under `deferred.is_some()`, is stored
// as it is, `chunk.state.value = deferred;`, MIR's answer, not tidy's
// (React's Flight client's `deferred`, #1933 step 4 (B)); one a write
// between leaves unproved, `deferred = None`, `?? null`, so JS reads the
// field `null`, the nullable field's `None`. Corpus case
// proved_option_stored_through_a_borrow runs both against native.
test("an Option a test proved there, stored through a borrow, is stored as it is", async () => {
  const { dir, fn, tidied } = compile(
    "nullable-tests-through-a-borrow",
    `use std::cell::RefCell;

#[cfg_attr(rust_js, rust_js::tag = "status")]
pub enum State {
    #[cfg_attr(rust_js, rust_js::name = "blocked")]
    Blocked {
        #[cfg_attr(rust_js, rust_js::nullable)]
        value: Option<Vec<u32>>,
    },
    #[cfg_attr(rust_js, rust_js::name = "done")]
    Done,
}

pub struct Chunk {
    pub state: RefCell<State>,
}

pub fn requeue(chunk: &Chunk, items: &[u32], reset: bool) {
    let mut deferred: Option<Vec<u32>> = None;
    for &item in items {
        if item > 1 {
            deferred.get_or_insert_with(Vec::new).push(item);
        }
    }
    if deferred.is_some()
        && let State::Blocked { value } = &mut *chunk.state.borrow_mut()
    {
        if reset {
            deferred = None;
        }
        *value = deferred;
    }
}

pub fn requeue_kept(chunk: &Chunk, items: &[u32]) {
    let mut deferred: Option<Vec<u32>> = None;
    for &item in items {
        if item > 1 {
            deferred.get_or_insert_with(Vec::new).push(item);
        }
    }
    if deferred.is_some()
        && let State::Blocked { value } = &mut *chunk.state.borrow_mut()
    {
        *value = deferred;
    }
}
`,
  );
  expect(tidied).toEqual([]);
  expect(fn("requeueKept")).toContain("    chunk.state.value = deferred;\n");
  expect(fn("requeue")).toContain("    chunk.state.value = deferred ?? null;\n");
  const lib = await import(join(dir, "lib.js"));
  const blocked = () => ({ state: { status: "blocked", value: [0] } });
  const kept = blocked();
  lib.requeueKept(kept, [1, 2, 3]);
  const reset = blocked();
  lib.requeue(reset, [5], true);
  const again = blocked();
  lib.requeue(again, [4], false);
  expect([kept.state.value, reset.state.value, again.state.value]).toEqual<(number[] | null)[]>([[2, 3], null, [4]]);
});

// A closure's capture its maker gives `null` or a value is that in the
// closure, stored back as it is, MIR's answer, not tidy's (React's Fizz
// retryRenderTask's `prev_task` in its `catch_unwind`, `captures_nulled`,
// #1933 step 4 (F)). Corpus case captured_nulled_local_stored_in_a_closure
// runs it, and one the closure writes, against native.
test("a closure's capture given null or a value is stored back as it is", () => {
  const { fn, tidied } = compile(
    "nullable-tests-captured",
    `use std::cell::RefCell;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::rc::Rc;

pub struct Task {
    pub id: u32,
}

pub struct Request {
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub current_task: RefCell<Option<Rc<Task>>>,
}

pub fn retry(request: &Request, task: &Rc<Task>, fail: bool) -> u32 {
    let prev_task = request.current_task.replace(Some(Rc::clone(task)));
    let result = catch_unwind(AssertUnwindSafe(|| {
        if fail {
            request.current_task.replace(prev_task.clone());
            return 0;
        }
        task.id
    }));
    result.unwrap_or(1)
}
`,
  );
  expect(tidied).toEqual([]);
  expect(fn("retry")).toContain("      request.currentTask = prevTask;\n");
});
