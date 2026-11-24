// A nullable thread-local nothing reads apart: it stores what it's given
// as it is, and a test tidy makes of it. Split from
// lowering-nullable-reads.test.ts by concept (#1334).

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildReact, compiler, fixture, run, target } from "./support";

beforeAll(buildReact, 600_000);

// #665, React's ReactFiberHotReloading: `resolveFamily = handler;`, where
// `setRefreshHandler`'s callers may give `undefined`. A nullable
// thread-local private to its module, every read of which only tests it,
// stores what it's given as it is, its `None` `null`; one whose read may
// be told apart keeps `?? null`: exported, read by another module or by
// generic code, lent, given to a binding or to a function of the crate's.
test("a nullable thread-local nothing reads apart stores what it's given as it is", async () => {
  const dir = fixture("nullable-local-alike");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::{Cell, RefCell};
use std::thread::LocalKey;

use js::Unknown;

mod other;

#[cfg_attr(rust_js, rust_js::link_name = "./seen.js#seen")]
fn seen(value: Option<&'static Unknown>) {
    let _ = value;
    unreachable!()
}

fn count(value: Option<&'static Unknown>) -> u32 {
    u32::from(value.is_some())
}

thread_local! {
    #[cfg_attr(rust_js, rust_js::nullable)]
    static RESOLVE_FAMILY: Cell<Option<&'static Unknown>> = const { Cell::new(None) };
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub static CURRENT: Cell<Option<&'static Unknown>> = const { Cell::new(None) };
    #[cfg_attr(rust_js, rust_js::nullable)]
    static SHARED: Cell<Option<&'static Unknown>> = const { Cell::new(None) };
    #[cfg_attr(rust_js, rust_js::nullable)]
    static GENERIC: Cell<Option<&'static Unknown>> = const { Cell::new(None) };
    #[cfg_attr(rust_js, rust_js::nullable)]
    static LENT: RefCell<Option<&'static Unknown>> = const { RefCell::new(None) };
    #[cfg_attr(rust_js, rust_js::nullable)]
    static SEEN: Cell<Option<&'static Unknown>> = const { Cell::new(None) };
    #[cfg_attr(rust_js, rust_js::nullable)]
    static COUNTED: Cell<Option<&'static Unknown>> = const { Cell::new(None) };
    #[cfg_attr(rust_js, rust_js::nullable)]
    static BORROWED: RefCell<Option<&'static Unknown>> = const { RefCell::new(None) };
    #[cfg_attr(rust_js, rust_js::nullable)]
    static HANDED: Cell<Option<&'static Unknown>> = const { Cell::new(None) };
    #[cfg_attr(rust_js, rust_js::nullable)]
    static INITIAL: Cell<Option<&'static Unknown>> = const { Cell::new(None) };
    #[cfg_attr(rust_js, rust_js::nullable)]
    static COPIED: Cell<Option<&'static Unknown>> = Cell::new(INITIAL.get());
    #[cfg_attr(rust_js, rust_js::nullable)]
    static MOVED: Cell<Option<&'static Unknown>> = const { Cell::new(None) };
    #[cfg_attr(rust_js, rust_js::nullable)]
    static MOVED_TO: Cell<Option<&'static Unknown>> = const { Cell::new(None) };
}

#[cfg_attr(rust_js, rust_js::nullable(handler))]
pub fn set_refresh_handler(handler: Option<&'static Unknown>) {
    RESOLVE_FAMILY.set(handler);
}

pub fn reset_refresh_handler() {
    RESOLVE_FAMILY.set(None);
}

#[cfg_attr(rust_js, rust_js::nullable(handler))]
pub fn set_unless_debug(debug: bool, handler: Option<&'static Unknown>) {
    let previous = RESOLVE_FAMILY.get();
    RESOLVE_FAMILY.set(handler.filter(|_| !debug));
    RESOLVE_FAMILY.set(previous);
}

pub fn resolve(tag: u32) -> u32 {
    let Some(_) = RESOLVE_FAMILY.get() else {
        return tag;
    };
    tag + 1
}

pub fn disabled() -> bool {
    RESOLVE_FAMILY.get().is_none()
}

#[cfg_attr(rust_js, rust_js::nullable(value))]
pub fn set_all(value: Option<&'static Unknown>) {
    CURRENT.set(value);
    SHARED.set(value);
    GENERIC.set(value);
    LENT.set(value);
    SEEN.set(value);
    COUNTED.set(value);
    BORROWED.set(value);
    HANDED.set(value);
    INITIAL.set(value);
    MOVED.set(value);
}

pub fn current() -> bool {
    CURRENT.get().is_some()
}

fn generic<T: Default>() -> (T, bool) {
    (T::default(), GENERIC.get().is_some())
}

pub fn generic_now() -> bool {
    generic::<u32>().1
}

pub fn lent() -> bool {
    LENT.with_borrow_mut(|value| value.is_some())
}

pub fn seen_now() {
    seen(SEEN.get());
}

pub fn counted() -> u32 {
    count(COUNTED.get())
}

pub fn shared() -> bool {
    other::shared()
}

pub fn borrowed() {
    BORROWED.with_borrow(|value| seen(*value));
}

fn is_set(key: &'static LocalKey<Cell<Option<&'static Unknown>>>) -> bool {
    key.get().is_some()
}

pub fn handed() -> bool {
    is_set(&HANDED)
}

pub fn copied() -> bool {
    COPIED.get().is_some()
}

#[cfg_attr(rust_js, rust_js::nullable)]
pub fn moved() -> Option<&'static Unknown> {
    MOVED_TO.set(MOVED.get());
    MOVED_TO.get()
}

// What may be \`undefined\`, which a call of the crate's gives as it is.
pub fn give(value: Option<&'static Unknown>) {
    set_refresh_handler(value);
    set_unless_debug(false, value);
    set_all(value);
}
`);
  writeFileSync(join(dir, "other.rs"), `pub fn shared() -> bool {
    super::SHARED.get().is_some()
}
`);
  writeFileSync(join(dir, "seen.js"), "export const given = [];\nexport const seen = (value) => given.push(value);\n");
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("export function setRefreshHandler(handler) {\n  resolveFamily = handler;\n}");
  expect(js).toContain("export function resetRefreshHandler() {\n  resolveFamily = null;\n}");
  expect(js).toContain("  resolveFamily = handler != null && !debug ? handler : null;\n  resolveFamily = previous;\n");
  const kept = ["CURRENT", "SHARED", "GENERIC", "LENT", "SEEN", "COUNTED"].map((key) => `  ${key} = value ?? null;\n`);
  expect(js).toContain(
    `export function setAll(value) {\n${kept.join("")}  $cellReplace($borrowMut(BORROWED, $stdSites.localKeySet[0]), value ?? null);\n` +
      "  HANDED.value = value ?? null;\n  initial = value ?? null;\n  MOVED = value ?? null;\n}",
  );
  const lib = await import(join(dir, "lib.js"));
  lib.give(undefined);
  expect([lib.disabled(), lib.resolve(1)]).toEqual([true, 1]);
  lib.setRefreshHandler({} as never);
  expect([lib.disabled(), lib.resolve(1)]).toEqual([false, 2]);
  lib.resetRefreshHandler();
  expect([lib.disabled(), lib.resolve(1)]).toEqual([true, 1]);
  lib.seenNow();
  lib.borrowed();
  const { given } = await import(join(dir, "seen.js"));
  expect(given).toEqual([null, null]);
  expect([lib.current(), lib.genericNow(), lib.lent(), lib.counted(), lib.shared()]).toEqual([false, false, false, 0, false]);
  expect([lib.handed(), lib.copied(), lib.moved()]).toEqual([false, false, null]);
});

// #683: React's ReactFlightServer, `importStringRequest = __DEV__ && debug ?
// null : request;`. MIR sets a temporary in each branch, which tidy makes
// the test after the store is lowered: its `None` is `null` there too.
test("a nullable thread-local nothing reads apart stores a test tidy makes of its value with its None null", async () => {
  const dir = fixture("nullable-local-alike-test");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::RefCell;
use std::rc::Rc;

pub struct Request {
    pub id: u32,
}

thread_local! {
    #[cfg_attr(rust_js, rust_js::nullable)]
    static IMPORT_STRING_REQUEST: RefCell<Option<Rc<Request>>> = const { RefCell::new(None) };
    static PLAIN: RefCell<Option<Rc<Request>>> = const { RefCell::new(None) };
}

// One not nullable holds \`undefined\` for \`None\`, as its \`None\` is made.
pub fn plain(request: &Rc<Request>, debug: bool) -> bool {
    PLAIN.set(if debug { None } else { Some(Rc::clone(request)) });
    PLAIN.with_borrow(Option::is_some)
}

fn replacer() -> u32 {
    let request = IMPORT_STRING_REQUEST.with_borrow(Clone::clone);
    let Some(request) = request else {
        return 0;
    };
    request.id
}

pub fn stringify(request: &Rc<Request>, dev: bool, debug: bool) -> u32 {
    let prev_request = IMPORT_STRING_REQUEST.with_borrow(Clone::clone);
    IMPORT_STRING_REQUEST.set(if dev && debug {
        None
    } else {
        Some(Rc::clone(request))
    });
    let result = replacer();
    IMPORT_STRING_REQUEST.set(prev_request);
    result
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("  importStringRequest = dev && debug ? null : request;\n");
  expect(js).toContain("debug ? undefined : request");
  const lib = await import(join(dir, "lib.js"));
  const request = { id: 7 };
  expect([lib.stringify(request, true, true), lib.stringify(request, true, false), lib.stringify(request, false, true)]).toEqual([0, 7, 7]);
  expect([lib.plain(request, true), lib.plain(request, false)]).toEqual([false, true]);
});
