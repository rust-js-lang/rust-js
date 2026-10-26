// What an exported thread-local's cell, or a `static mut`, is to the modules
// that import it: its module's `export let`, written only there and read
// live by its importers, JS's and Rust's, in its crate and in another; a
// write from another module refused (ADRs 0270, 0096, amended). Each test
// compiles a program and runs it.

import { beforeAll, expect, test } from "bun:test";
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { runSync } from "./child";
import { buildReact, compiler, fixture, root, run, target } from "./support";

beforeAll(buildReact, 600_000);

const js = ["--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];

const flags = `use std::cell::{Cell, RefCell};

thread_local! {
    pub static ENABLED: Cell<bool> = const { Cell::new(false) };
    pub(crate) static COUNT: Cell<u32> = const { Cell::new(0) };
    pub static LIMIT: Cell<u32> = const { Cell::new(4) };
    pub static LABEL: RefCell<String> = const { RefCell::new(String::new()) };
    pub static SEEN: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };
}

pub fn enable(label: &str) {
    ENABLED.set(true);
    COUNT.set(COUNT.get() + 1);
    LABEL.set(label.to_string());
}
`;

// React's BadMapPolyfill exports `let hasBadMapPolyfill`, which its module
// sets as it loads and its importers read as it is now, ES's live binding:
// a `pub` thread-local's cell only its module sets is that `export let`, an
// importer's read of it its name, and one nothing sets `export const`.
test("an exported cell its module alone sets is its export let, which importers read", async () => {
  const dir = fixture("exported-cell-let");
  writeFileSync(join(dir, "lib.rs"), "pub mod flags;\npub mod users;\n");
  writeFileSync(join(dir, "flags.rs"), flags);
  writeFileSync(join(dir, "users.rs"), `use crate::flags::{COUNT, ENABLED, LABEL, LIMIT};

pub fn told() -> (bool, u32, u32, String) {
    (ENABLED.get(), COUNT.get(), LIMIT.get(), LABEL.with_borrow(|label| label.clone()))
}
`);
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", ...js]);
  const flagsJs = readFileSync(join(dir, "flags.js"), "utf8");
  expect(flagsJs).toContain("export let enabled = false;\nexport let count = 0;\nexport const limit = 4;\nexport let label = \"\";\n");
  expect(flagsJs).toContain("  enabled = true;\n  count = (count + 1) >>> 0;\n  label = label$1;\n");
  expect(flagsJs).not.toContain(".value");
  const usersJs = readFileSync(join(dir, "users.js"), "utf8");
  expect(usersJs).toContain("  return [enabled, count, limit, label];\n");
  expect(readFileSync(join(dir, "flags.d.ts"), "utf8")).toContain("export let enabled: boolean;\n\nexport const limit: number;\n\nexport let label: string;\n");
  // A JS module of the program's imports it, as React's JS imports
  // `hasBadMapPolyfill`, and reads what it is now.
  writeFileSync(join(dir, "reader.js"), 'import { enabled, label } from "./flags.js";\nexport const read = () => [enabled, label];\n');
  const reader = await import(join(dir, "reader.js"));
  const flagsModule = await import(join(dir, "flags.js"));
  const users = await import(join(dir, "users.js"));
  expect(reader.read()).toEqual([false, ""]);
  flagsModule.enable("on");
  expect(reader.read()).toEqual([true, "on"]);
  expect(users.told()).toEqual([true, 1, 4, "on"]);
});

// React's `let callbacks = []`, set `callbacks = []` again: a thread-local
// `RefCell` of an array its module sets whole is its `export let`, given
// the new one there; another module's `set` writes the one it reads in
// place, which the live binding shares (ADR 0270, amended).
test("a thread-local of an array its module sets is its export let", async () => {
  const dir = fixture("exported-array-let");
  writeFileSync(join(dir, "lib.rs"), "pub mod callbacks;\npub mod users;\n");
  writeFileSync(join(dir, "callbacks.rs"), `use std::cell::RefCell;

thread_local! {
    pub static CALLBACKS: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };
}

pub fn add(value: u32) {
    CALLBACKS.with_borrow_mut(|callbacks| callbacks.push(value));
}

pub fn reset() {
    CALLBACKS.set(Vec::new());
}

pub fn count() -> usize {
    CALLBACKS.with_borrow(|callbacks| callbacks.len())
}
`);
  writeFileSync(join(dir, "users.rs"), `use crate::callbacks::CALLBACKS;

pub fn only(value: u32) {
    CALLBACKS.set(vec![value]);
}
`);
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const callbacksJs = readFileSync(join(dir, "callbacks.js"), "utf8");
  expect(callbacksJs).toContain("export let callbacks = [];\n");
  expect(callbacksJs).toContain("  callbacks = [];\n");
  expect(readFileSync(join(dir, "users.js"), "utf8")).toContain("  $assign(callbacks, [value]);\n");
  expect(readFileSync(join(dir, "callbacks.d.ts"), "utf8")).toContain("export let callbacks: number[];\n");
  writeFileSync(join(dir, "reader.js"), 'import { callbacks } from "./callbacks.js";\nexport const read = () => callbacks;\n');
  const reader = await import(join(dir, "reader.js"));
  const module = await import(join(dir, "callbacks.js"));
  const users = await import(join(dir, "users.js"));
  module.add(1);
  const before = reader.read();
  module.reset();
  module.add(2);
  expect([before, reader.read(), module.count()]).toEqual([[1], [2], 1]);
  users.only(5);
  expect([reader.read(), module.count()]).toEqual([[5], 1]);
});

// JS can't set another module's `let`: a write of an exported cell from
// another module, `set`, `replace`, `take`, a `with_borrow_mut`, or its cell
// handed out, `with`, is refused, saying to write a function of its module
// that sets it, as React's ReactCurrentFiber exports `setIsRendering`.
test("another module's write of an exported cell is refused", () => {
  const dir = fixture("exported-cell-written");
  const refused = (name: string, body: string) => {
    mkdirSync(join(dir, name), { recursive: true });
    writeFileSync(join(dir, name, "lib.rs"), "pub mod flags;\npub mod users;\n");
    writeFileSync(join(dir, name, "flags.rs"), flags);
    writeFileSync(join(dir, name, "users.rs"), `use crate::flags;

pub fn written() -> u32 {
    ${body}
}
`);
    return runSync([compiler, join(dir, name, "lib.rs"), "-o", join(dir, name, "lib.js"), "--", ...js], root, 120_000);
  };
  for (const [name, body] of [
    ["set", "flags::ENABLED.set(true);\n    0"],
    ["replace", "flags::COUNT.replace(3)"],
    ["take", "flags::COUNT.take()"],
    ["borrow-mut", "flags::LABEL.with_borrow_mut(|label| label.push('!'));\n    0"],
    ["with", "flags::ENABLED.with(|enabled| enabled.set(true));\n    0"],
  ]) {
    const out = refused(name, body);
    expect([name, out.code === 0, out.stderr]).toEqual([
      name,
      false,
      expect.stringContaining("is its module's variable, which JS lets only that module set: write a `pub fn` in that module that sets it"),
    ]);
  }
  // A read is a read of its variable; a `RefCell` of an array is that
  // array, which any module may change, as JS's importer of it may.
  const read = refused("read", "flags::SEEN.with_borrow_mut(|seen| seen.push(1));\n    flags::COUNT.get() + u32::from(flags::ENABLED.get())");
  expect([read.code, read.stderr]).toEqual([0, ""]);
});

// A library's exported cell is its `export let` to its consumers too, whose
// manifest says so: a consumer reads it by its name and can't set it.
test("a library's exported cell is a variable its consumers read and can't set", async () => {
  const dir = fixture("exported-cell-library");
  const manifest = join(dir, "dep", "lib.manifest.json"), metadata = join(dir, "libdep.rmeta");
  writeFileSync(join(dir, "dep.rs"), `use std::cell::{Cell, RefCell};

thread_local! {
    pub static HAS_BAD_MAP_POLYFILL: Cell<bool> = const { Cell::new(true) };
    pub static NAME: RefCell<String> = const { RefCell::new(String::new()) };
}

js::on_load! {
    HAS_BAD_MAP_POLYFILL.set(false);
}

pub fn mark() {
    HAS_BAD_MAP_POLYFILL.set(true);
    NAME.set("marked".to_string());
}
`);
  run([compiler, join(dir, "dep.rs"), "-o", join(dir, "dep", "lib.js"), "--library", "--manifest", manifest,
    "--", "--crate-name", "dep", `--emit=metadata=${metadata}`, ...js]);
  expect(readFileSync(join(dir, "dep", "lib.js"), "utf8")).toContain("export let hasBadMapPolyfill = true;\nexport let name = \"\";\n\nhasBadMapPolyfill = false;\n");
  const app = (name: string, body: string) => {
    mkdirSync(join(dir, name), { recursive: true });
    writeFileSync(join(dir, name, "lib.rs"), `pub fn bad() -> bool {\n    ${body}\n}\n`);
    return runSync([compiler, join(dir, name, "lib.rs"), "-o", join(dir, name, "lib.js"), "--dependency", manifest,
      "--", "--crate-name", name, "--extern", `dep=${metadata}`, "-L", dir, ...js], root, 120_000);
  };
  const reads = app("reads", "dep::HAS_BAD_MAP_POLYFILL.get() && dep::NAME.with_borrow(|name| !name.is_empty())");
  expect([reads.code, reads.stderr]).toEqual([0, ""]);
  const readsJs = readFileSync(join(dir, "reads", "lib.js"), "utf8");
  expect(readsJs).toContain("  return hasBadMapPolyfill && !!name;\n");
  const { bad } = await import(join(dir, "reads", "lib.js"));
  const { mark } = await import(join(dir, "dep", "lib.js"));
  expect(bad()).toBe(false);
  mark();
  expect(bad()).toBe(true);
  // A `with_borrow` that runs what may borrow it, the library's own code,
  // can't be read so: nothing counts its borrows, where Rust's would panic.
  const unquiet = app("unquiet", "dep::NAME.with_borrow(|name| {\n        dep::mark();\n        name.is_empty()\n    })");
  expect([unquiet.code === 0, unquiet.stderr]).toEqual([false, expect.stringContaining("is its module's variable, which JS lets only that module set")]);
  const sets = app("sets", "dep::HAS_BAD_MAP_POLYFILL.set(false);\n    true");
  expect([sets.code === 0, sets.stderr]).toEqual([false, expect.stringContaining("is its module's variable, which JS lets only that module set")]);
});

// React's BadMapPolyfill, as its port writes it: upstream's JS.
test("React's BadMapPolyfill is its export let", () => {
  const dir = fixture("bad-map-polyfill");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::Cell;
use std::panic::catch_unwind;

use js::Kind;

unsafe extern "Rust" {
    pub safe static __DEV__: bool;
}

thread_local! {
    pub static HAS_BAD_MAP_POLYFILL: Cell<Option<bool>> = const { Cell::new(None) };
}

js::on_load! {
    if __DEV__ {
        HAS_BAD_MAP_POLYFILL.set(Some(false));
        if let Err(_e) = catch_unwind(|| {
            let frozen_object = js::object::freeze(js::dict::new::<Kind>());
            /* eslint-disable no-new */
            js::map::new(Some(&[(frozen_object, Kind::Null)]));
            js::set::new(Some(&[frozen_object]));
            /* eslint-enable no-new */
        }) {
            // TODO: Consider warning about bad polyfills
            HAS_BAD_MAP_POLYFILL.set(Some(true));
        }
    }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", ...js]);
  expect(readFileSync(join(dir, "lib.js"), "utf8")).toContain(`export let hasBadMapPolyfill;

if (__DEV__) {
  hasBadMapPolyfill = false;
  try {
    const frozenObject = Object.freeze({});
    /* eslint-disable no-new */
    new Map([[frozenObject, null]]);
    new Set([frozenObject]);
    /* eslint-enable no-new */
  } catch (_e) {
    // TODO: Consider warning about bad polyfills
    hasBadMapPolyfill = true;
  }
}
`);
});

// React's ReactCurrentFiber: `export let current: Fiber | null = null`, set
// `null` again where it's reset. A `#[rust_js::nullable]` thread-local's
// `None` is `null` where it's made and where it's set (ADR 0586, amended).
test("a nullable thread-local's None is null where it's made and set", async () => {
  const dir = fixture("nullable-thread-local");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::{Cell, RefCell};
use std::rc::Rc;

pub struct Fiber {
    pub tag: u8,
}

thread_local! {
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub static CURRENT: RefCell<Option<Rc<Fiber>>> = const { RefCell::new(None) };
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub static OWNER: Cell<Option<&'static str>> = const { Cell::new(None) };
}

#[cfg_attr(rust_js, rust_js::nullable(fiber))]
pub fn set_current_fiber(fiber: Option<Rc<Fiber>>) {
    CURRENT.set(fiber);
}

pub fn own(name: &'static str) {
    OWNER.set(Some(name));
}

pub fn reset() {
    CURRENT.set(None);
    OWNER.set(None);
}

pub fn current_tag() -> Option<u8> {
    CURRENT.with_borrow(|current| current.as_ref().map(|fiber| fiber.tag))
}
`);
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("export let current = null;\n");
  expect(js).toContain("export let owner = null;\n");
  // A nullable parameter's `None` is `null` where a call gives it, and
  // `undefined` from JS too, as its `.d.ts` says (ADR 0861).
  expect(js).toContain("  current = fiber ?? null;\n");
  expect(js).toContain("  current = null;\n  owner = null;\n");
  const types = readFileSync(join(dir, "lib.d.ts"), "utf8");
  expect(types).toContain("export let current: Fiber | null;\n");
  expect(types).toContain("export let owner: string | null;\n");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.current, lib.owner]).toEqual([null, null]);
  lib.setCurrentFiber({ tag: 3 });
  lib.own("App");
  expect([lib.currentTag(), lib.owner]).toEqual([3, "App"]);
  lib.reset();
  expect([lib.current, lib.owner, lib.currentTag()]).toEqual([null, null, undefined]);
});

// A binding's type parameter `#[rust_js::nullable(..)]` names: its `None`
// `null` where it's given an `Option`, React's `createCursor(null)`, and
// any other value as it is, `createCursor(0)` (ADR 0586, amended).
test("a binding's nullable type parameter is given null for None", () => {
  const dir = fixture("nullable-generic-binding");
  writeFileSync(join(dir, "stack.js"), "export function createCursor(defaultValue) {\n  return { current: defaultValue };\n}\n");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::RefCell;
use std::rc::Rc;

pub struct StackCursor<T> {
    pub current: RefCell<T>,
}

pub struct Hidden {
    pub lanes: u32,
}

#[cfg_attr(rust_js, rust_js::link_name = "./stack.js#createCursor")]
#[cfg_attr(rust_js, rust_js::nullable(default_value))]
fn create_cursor<T>(default_value: T) -> StackCursor<T> {
    let _ = default_value;
    unreachable!()
}

thread_local! {
    pub static HIDDEN: StackCursor<Option<Rc<Hidden>>> = create_cursor(None);
    pub static LANES: StackCursor<u32> = create_cursor(0);
}

pub fn hidden() -> bool {
    HIDDEN.with(|cursor| cursor.current.borrow().is_some())
}

pub fn lanes() -> u32 {
    LANES.with(|cursor| *cursor.current.borrow())
}

pub fn hidden_of(context: Option<Rc<Hidden>>) -> StackCursor<Option<Rc<Hidden>>> {
    create_cursor(context)
}

pub fn lanes_of(lanes: u32) -> StackCursor<u32> {
    create_cursor(lanes)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("createCursor(null)");
  expect(js).toContain("createCursor(0)");
  expect(js).toContain("  return createCursor(context ?? null);\n");
  expect(js).toContain("  return createCursor(lanes);\n");
});

// One of what's no `Option`, whose `None` it can't be, is refused.
test("a nullable thread-local of no Option is refused", () => {
  const dir = fixture("nullable-thread-local-number");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::Cell;

thread_local! {
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub static COUNT: Cell<u32> = const { Cell::new(0) };
}

pub fn bump() {
    COUNT.set(COUNT.get() + 1);
}
`);
  expect(() => run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")])).toThrow(
    "`#[rust_js::nullable]` of a thread-local is of one of a `Cell` or a `RefCell` of an `Option`",
  );
});


// React's ReactFiberTransition imports `entangledTransitionTypes`, an
// `export let` that starts `null`, and tests it `!== null`. Its port fills
// the `Option` where it's first needed, in place, in the closure of a
// `with_borrow_mut`, which was given the cell's `{ value }`, the box its
// `&mut` is: the box was exported, never `null` to a JS importer, whose
// `for (const t of entangledTransitionTypes)` then threw. Read and set
// there by `value` alone, the box is the variable (ADR 0270, amended).
test("an exported Option a with_borrow_mut fills in place is its export let", async () => {
  const dir = fixture("exported-option-filled");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::RefCell;
use std::rc::Rc;

thread_local! {
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub static ENTANGLED: RefCell<Option<Vec<String>>> = const { RefCell::new(None) };
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub static SHARED: RefCell<Option<Rc<Vec<u32>>>> = const { RefCell::new(None) };
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub static DEPTH: RefCell<Option<u32>> = const { RefCell::new(None) };
}

pub fn entangle(types: &[String]) {
    ENTANGLED.with_borrow_mut(|entangled| {
        let queued = entangled.get_or_insert_with(Vec::new);
        for t in types {
            if !queued.contains(t) {
                queued.push(t.clone());
            }
        }
    });
}

pub fn first_shared() -> Rc<Vec<u32>> {
    SHARED.with_borrow_mut(|shared| Rc::clone(shared.get_or_insert_with(|| Rc::new(vec![1]))))
}

pub fn deeper() -> u32 {
    DEPTH.with_borrow_mut(|depth| {
        let depth = depth.get_or_insert_default();
        *depth += 1;
        *depth
    })
}

pub fn clear() {
    ENTANGLED.set(None);
    DEPTH.set(None);
}
`);
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", ...js]);
  const libJs = readFileSync(join(dir, "lib.js"), "utf8");
  expect(libJs).toContain("export let entangled = null;\nexport let shared = null;\nexport let depth = null;\n");
  expect(libJs).toContain("  entangled ??= [];\n  const queued = entangled;\n");
  expect(libJs).toContain("  entangled = null;\n  depth = null;\n");
  expect(libJs).not.toContain(".value");
  expect(libJs).not.toContain("$withBorrowMut");
  expect(readFileSync(join(dir, "lib.d.ts"), "utf8")).toContain("export let entangled: string[] | null;\n");
  // A JS module imports it, as ReactFiberTransition.js does, and reads it.
  writeFileSync(join(dir, "reader.js"), `import { depth, entangled } from "./lib.js";
export const read = () => [entangled !== null ? [...entangled] : null, depth];
`);
  const reader = await import(join(dir, "reader.js"));
  const lib = await import(join(dir, "lib.js"));
  expect(reader.read()).toEqual([null, null]);
  lib.entangle(["a", "b"]);
  lib.entangle(["b", "c"]);
  expect([lib.deeper(), lib.deeper()]).toEqual([1, 2]);
  expect(reader.read()).toEqual([["a", "b", "c"], 2]);
  expect(lib.firstShared()).toBe(lib.firstShared());
  lib.clear();
  expect(reader.read()).toEqual([null, null]);
});

// Where its module needs the cell itself, a JS importer of it would read
// the box, not what it holds: an exported one is refused, saying how to
// read and set it; one its module keeps compiles, as before; and one a
// `with` lends its cell, used in place, is its variable (ADR 0270, amended).
test("an exported cell its module needs the cell of is refused", () => {
  const dir = fixture("exported-cell-boxed");
  const compiled = (name: string, visibility: string, body: string) => {
    mkdirSync(join(dir, name), { recursive: true });
    writeFileSync(join(dir, name, "lib.rs"), `use std::cell::{Cell, RefCell};

thread_local! {
    ${visibility}static DEPTH: RefCell<Option<u32>> = const { RefCell::new(None) };
    ${visibility}static COUNT: Cell<u32> = const { Cell::new(0) };
    ${visibility}static LABEL: RefCell<String> = const { RefCell::new(String::new()) };
}

fn fill(depth: &mut Option<u32>) {
    depth.get_or_insert(3);
}

pub fn used() -> u32 {
    ${body}
}
`);
    const out = runSync([compiler, join(dir, name, "lib.rs"), "-o", join(dir, name, "lib.js"), "--", ...js], root, 120_000);
    return [out.code, out.stderr, out.code === 0 ? readFileSync(join(dir, name, "lib.js"), "utf8") : ""];
  };
  for (const [name, body, refused] of [
    ["mut-handed-on", "DEPTH.with_borrow_mut(|depth| fill(depth));\n    0", "DEPTH"],
    ["fn-given", "DEPTH.with_borrow_mut(fill);\n    0", "DEPTH"],
    ["borrowed-in-with", "LABEL.with(|label| label.borrow().len() as u32)", "LABEL"],
  ]) {
    const [code, stderr] = compiled(name, "pub ", body);
    expect([name, code === 0, stderr]).toEqual([
      name,
      false,
      expect.stringContaining(`\`${refused}\` is exported, so a JS module imports it as what its cell holds, its module's \`export let\`, but its module needs the cell itself`),
    ]);
    const [kept, keptErr] = compiled(`${name}-kept`, "", body);
    expect([name, kept, keptErr]).toEqual([name, 0, expect.not.stringContaining("error")]);
  }
  const [code, stderr, withJs] = compiled("with-in-place", "pub ", "COUNT.with(|count| count.set(count.get() + 1));\n    COUNT.get()");
  expect([code, stderr]).toEqual([0, expect.not.stringContaining("error")]);
  expect(withJs).toContain("export let count = 0;\n");
  expect(withJs).toContain("  count = (count + 1) >>> 0;\n  return count;\n");
});

// A `static mut` is its module's variable as an exported cell is (ADR 0096,
// amended): `export let` where it's `pub`, written only in its module and
// read live by its importers, JS's and Rust's.
const counter = `#[derive(Clone, Copy)]
pub struct Stats {
    pub calls: u32,
}

pub static mut LINES: usize = 0;
pub static mut STATS: Stats = Stats { calls: 0 };
pub static mut SLOTS: [u8; 2] = [0; 2];
pub static mut LIMIT: u32 = 4;

pub fn line() -> usize {
    unsafe {
        LINES += 1;
        STATS.calls += 1;
        SLOTS[LINES % 2] += 1;
        LINES
    }
}

pub fn reset() {
    unsafe {
        LINES = 0;
        STATS = Stats { calls: 0 };
    }
}
`;

test("a pub static mut its module alone writes is its export let, which importers read", async () => {
  const dir = fixture("static-mut-let");
  writeFileSync(join(dir, "lib.rs"), "pub mod counter;\npub mod users;\n");
  writeFileSync(join(dir, "counter.rs"), counter);
  writeFileSync(join(dir, "users.rs"), `use crate::counter;

pub fn told() -> (usize, u32, u8, u32) {
    unsafe { (counter::LINES, counter::STATS.calls, counter::SLOTS[1], counter::LIMIT) }
}
`);
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const counterJs = readFileSync(join(dir, "counter.js"), "utf8");
  expect(counterJs).toContain("export let LINES = 0;\nexport let STATS = { calls: 0 };\nexport const SLOTS = [0, 0];\nexport const LIMIT = 4;\n");
  expect(counterJs).toContain("  LINES = (LINES + 1) >>> 0;\n  STATS.calls = (STATS.calls + 1) >>> 0;\n");
  expect(counterJs).toContain("  LINES = 0;\n  STATS = { calls: 0 };\n");
  expect(counterJs).not.toContain(".value");
  const usersJs = readFileSync(join(dir, "users.js"), "utf8");
  expect(usersJs).toContain("  return [LINES, STATS.calls, SLOTS[1], LIMIT];\n");
  expect(usersJs).not.toContain(".value");
  expect(readFileSync(join(dir, "counter.d.ts"), "utf8")).toContain("export let LINES: number;\n\nexport let STATS: Stats;\n\nexport const SLOTS: number[];\n\nexport const LIMIT: number;\n");
  // A JS module of the program's imports it and reads what it is now.
  writeFileSync(join(dir, "reader.js"), 'import { LINES, STATS } from "./counter.js";\nexport const read = () => [LINES, STATS.calls];\n');
  const reader = await import(join(dir, "reader.js"));
  const counterModule = await import(join(dir, "counter.js"));
  const users = await import(join(dir, "users.js"));
  expect(reader.read()).toEqual([0, 0]);
  expect([counterModule.line(), counterModule.line(), counterModule.line()]).toEqual([1, 2, 3]);
  expect(reader.read()).toEqual([3, 3]);
  expect(users.told()).toEqual([3, 3, 2, 4]);
  counterModule.reset();
  expect(reader.read()).toEqual([0, 0]);
});

// A private one is its module's `let`; one a module beneath it reads is
// exported to it, its `export let`, which that module reads by its name.
test("a private static mut is its module's let", async () => {
  const dir = fixture("static-mut-private");
  writeFileSync(join(dir, "lib.rs"), `static mut HITS: u32 = 0;
static mut SEEN: u32 = 0;

pub mod child;

pub fn hit() -> u32 {
    unsafe {
        HITS += 1;
        SEEN += 2;
        HITS
    }
}
`);
  writeFileSync(join(dir, "child.rs"), "pub fn seen() -> u32 {\n    unsafe { super::SEEN }\n}\n");
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const libJs = readFileSync(join(dir, "lib.js"), "utf8");
  expect(libJs).toContain("let HITS = 0;\nexport let SEEN = 0;\n");
  expect(libJs).toContain("  HITS = (HITS + 1) >>> 0;\n  SEEN = (SEEN + 2) >>> 0;\n  return HITS;\n");
  expect(readFileSync(join(dir, "child.js"), "utf8")).toContain("  return SEEN;\n");
  const lib = await import(join(dir, "lib.js"));
  const child = await import(join(dir, "child.js"));
  expect([lib.hit(), lib.hit(), child.seen()]).toEqual([1, 2, 4]);
});

// JS can't set another module's `let`: a write of a `static mut` from
// another module, whole, a field's, an item's, or a pointer taken to it,
// is refused, saying to write a function of its module that sets it.
test("another module's write of a static mut is refused", () => {
  const dir = fixture("static-mut-written");
  const compiled = (name: string, body: string) => {
    mkdirSync(join(dir, name), { recursive: true });
    writeFileSync(join(dir, name, "lib.rs"), "pub mod counter;\npub mod users;\n");
    writeFileSync(join(dir, name, "counter.rs"), counter);
    writeFileSync(join(dir, name, "users.rs"), `use crate::counter;

pub fn written() -> usize {
    unsafe {
        ${body}
    }
}
`);
    return runSync([compiler, join(dir, name, "lib.rs"), "-o", join(dir, name, "lib.js")], root, 120_000);
  };
  for (const [name, body] of [
    ["assigned", "counter::LINES = 3;\n        0"],
    ["added-to", "counter::LINES += 1;\n        0"],
    ["field", "counter::STATS.calls = 1;\n        0"],
    ["item", "counter::SLOTS[0] = 1;\n        0"],
    ["raw-mut", "let lines = &raw mut counter::LINES;\n        *lines = 2;\n        0"],
    ["raw-const", "*std::ptr::addr_of!(counter::LINES)"],
  ]) {
    const out = compiled(name, body);
    expect([name, out.code === 0, out.stderr]).toEqual([
      name,
      false,
      expect.stringContaining("is its module's variable, which JS lets only that module set: write a `pub fn` in that module that sets it"),
    ]);
  }
  const read = compiled("read", "counter::LINES + counter::STATS.calls as usize + usize::from(counter::SLOTS[0])");
  expect([read.code, read.stderr]).toEqual([0, ""]);
});

// A library's `static mut` is its `export let` to its consumers too, whose
// manifest says so: a consumer reads it by its name and can't set it.
test("a library's static mut is a variable its consumers read and can't set", async () => {
  const dir = fixture("static-mut-library");
  const manifest = join(dir, "dep", "lib.manifest.json"), metadata = join(dir, "libdep.rmeta");
  writeFileSync(join(dir, "dep.rs"), `pub static mut LINES: usize = 0;

pub fn line() -> usize {
    unsafe {
        LINES += 1;
        LINES
    }
}
`);
  run([compiler, join(dir, "dep.rs"), "-o", join(dir, "dep", "lib.js"), "--library", "--manifest", manifest,
    "--", "--crate-name", "dep", `--emit=metadata=${metadata}`]);
  expect(readFileSync(join(dir, "dep", "lib.js"), "utf8")).toContain("export let LINES = 0;\n");
  const app = (name: string, body: string) => {
    mkdirSync(join(dir, name), { recursive: true });
    writeFileSync(join(dir, name, "lib.rs"), `pub fn lines() -> usize {\n    unsafe {\n        ${body}\n    }\n}\n`);
    return runSync([compiler, join(dir, name, "lib.rs"), "-o", join(dir, name, "lib.js"), "--dependency", manifest,
      "--", "--crate-name", name, "--extern", `dep=${metadata}`, "-L", dir], root, 120_000);
  };
  const reads = app("reads", "dep::LINES");
  expect([reads.code, reads.stderr]).toEqual([0, ""]);
  expect(readFileSync(join(dir, "reads", "lib.js"), "utf8")).toContain("  return LINES;\n");
  const { lines } = await import(join(dir, "reads", "lib.js"));
  const { line } = await import(join(dir, "dep", "lib.js"));
  expect(lines()).toBe(0);
  line();
  expect(lines()).toBe(1);
  for (const [name, body] of [["sets", "dep::LINES = 5;\n        0"], ["points", "*&raw const dep::LINES"]]) {
    const out = app(name, body);
    expect([name, out.code === 0, out.stderr]).toEqual([name, false, expect.stringContaining("is its module's variable, which JS lets only that module set")]);
  }
});

// A pointer to a `static mut` given on, `&raw mut TOTAL`, is a handle on
// its place, which needs its box, `{ value }`: one its module keeps is its
// box; one it exports, which a JS importer would read as the box, is
// refused at its declaration. One only written through where it's taken,
// `*addr_of_mut!(COUNT) += 2`, is the variable written.
test("a static mut a pointer to is given on is its box, refused where it's exported", async () => {
  const dir = fixture("static-mut-pointer");
  const compiled = (name: string, visibility: string) => {
    mkdirSync(join(dir, name), { recursive: true });
    writeFileSync(join(dir, name, "lib.rs"), `use std::ptr::addr_of_mut;

${visibility}static mut COUNT: u32 = 0;
${visibility}static mut TOTAL: u32 = 0;

fn add(total: *mut u32, n: u32) {
    unsafe { *total += n }
}

pub fn counted() -> (u32, u32) {
    unsafe {
        *addr_of_mut!(COUNT) += 2;
        add(&raw mut TOTAL, 3);
        add(&raw mut TOTAL, 4);
        (COUNT, TOTAL)
    }
}
`);
    const out = runSync([compiler, join(dir, name, "lib.rs"), "-o", join(dir, name, "lib.js")], root, 120_000);
    return [out.code, out.stderr, out.code === 0 ? readFileSync(join(dir, name, "lib.js"), "utf8") : ""];
  };
  const [code, stderr, libJs] = compiled("kept", "");
  expect([code, stderr]).toEqual([0, ""]);
  expect(libJs).toContain("let COUNT = 0;\nconst TOTAL = { value: 0 };\n");
  expect(libJs).toContain("  COUNT = (COUNT + 2) >>> 0;\n");
  const { counted } = await import(join(dir, "kept", "lib.js"));
  expect([counted(), counted()]).toEqual([[2, 7], [4, 14]]);
  const [exportedCode, exportedErr] = compiled("exported", "pub ");
  expect([exportedCode === 0, exportedErr]).toEqual([
    false,
    expect.stringContaining("`TOTAL` is exported, so a JS module imports it as its module's `export let`, but its module gives on a pointer to it"),
  ]);
  expect(exportedErr).not.toContain("`COUNT`");
});
