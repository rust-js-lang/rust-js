// What an exported thread-local's cell is to the modules that import it: its
// module's `export let`, written only there and read live by its importers,
// JS's and Rust's, in its crate and in another; a write from another module
// refused (ADR 0270, amended). Each test compiles a program and runs it.

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

