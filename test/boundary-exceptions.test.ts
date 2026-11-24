// ADR 0262's exceptions at the JS boundary (#1212): what only a JS value, a
// binding or a JS caller can tell from native Rust, each kept as the user
// decided (2026-11-22), each listed in ADR 0262's Consequences and in the
// guide's one list of differences (docs/guide/semantics.md), by its number
// there. Each test holds today's behaviour, so a change to it is a change
// to a decided exception, made with the user, never by accident. And the
// guide's difference 7, gone since ADR 1173: held as Rust's.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

import { buildReact, compiler, fixture, run, target } from "./support";
import { withoutBoundary } from "./without-boundary";

beforeAll(buildReact, 600_000);

const withJs = ["--", "--crate-type", "lib", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];

/** `source` compiled as a library given the `js` crate: its JS and module. */
async function compiled(name: string, source: string): Promise<{ js: string; lib: Record<string, any> }> {
  const dir = fixture(`boundary-${name}`);
  writeFileSync(join(dir, "lib.rs"), source);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs]);
  // Its JS as a test of how its calls throw reads it, without the checks of what JS gives it (#2403).
  return { js: withoutBoundary(readFileSync(join(dir, "lib.js"), "utf8")), lib: await import(join(dir, "lib.js")) };
}

const g = globalThis as Record<string, unknown>;

// The guide's difference 7 is gone: a JS getter that calls back into Rust
// sees what Rust holds, as native Rust would (ADR 1173, 9776c84a0). Its
// borrow of a cell held across the read panics as Rust's does; a `Cell`
// read after the getter sees what its callback set; a `&mut` held across
// it sees what the object's setter wrote, the same place.
test("difference 7 is gone: a getter's callback sees what Rust holds, as Rust's", async () => {
  const { lib } = await compiled("getter-calls-back", `use std::cell::{Cell, RefCell};
use std::marker::PhantomData;

use js::JsObject;

/// A meter whose \`level\` is a JS getter.
pub struct Meter(PhantomData<JsObject>);

unsafe impl js::Defined for Meter {}

impl Meter {
    #[cfg_attr(rust_js, rust_js::link_name = "get level")]
    pub fn level(&self) -> f64 {
        unreachable!()
    }
}

#[cfg_attr(rust_js, rust_js::class)]
pub struct Log {
    seen: RefCell<Vec<f64>>,
    marked: Cell<u32>,
    n: u32,
}

#[cfg_attr(rust_js, rust_js::class)]
impl Log {
    pub fn new() -> Log {
        Log { seen: RefCell::new(Vec::new()), marked: Cell::new(0), n: 0 }
    }

    pub fn record(&self, x: f64) -> usize {
        self.seen.borrow_mut().push(x);
        self.seen.borrow().len()
    }

    pub fn read(&self, meter: &Meter) -> f64 {
        let held = self.seen.borrow();
        let level = meter.level();
        level + held.len() as f64
    }

    pub fn mark(&self, x: u32) {
        self.marked.set(x);
    }

    pub fn around(&self, meter: &Meter) -> u32 {
        let before = self.marked.get();
        let level = meter.level() as u32;
        before * 10000 + level * 100 + self.marked.get()
    }

    pub fn set_n(&mut self, n: u32) {
        self.n = n;
    }

    pub fn count(&self) -> u32 {
        self.n
    }

    pub fn held(&mut self, meter: &Meter) -> u32 {
        let r = &mut self.n;
        *r += 1;
        let level = meter.level() as u32;
        *r += level;
        *r
    }
}
`);
  const log = new lib.Log();
  expect(() => log.read({ get level() { return 2 + log.record(5); } })).toThrow("RefCell already borrowed");
  expect(log.record(1)).toBe(1);
  log.mark(3);
  expect(log.around({ get level() { log.mark(7); return 5; } })).toBe(30507);
  log.setN(10);
  expect(log.held({ get level() { log.setN(100); return 2; } })).toBe(102);
  expect(log.count()).toBe(102);
});

// The guide's difference 12 (ADR 0873): a closure boxed as it's handed to
// a binding is JS's, and carries no drop. Where JS gives it back, through
// another binding, and Rust drops it, what it holds isn't dropped, where
// native Rust, its binding giving back the box it was handed, would.
test("difference 12: a closure handed to a binding drops nothing of what it holds, given back", async () => {
  const dir = fixture("boundary-handed-to-js");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::Cell;
thread_local! {
    static DROPS: Cell<u32> = const { Cell::new(0) };
}
pub struct Noisy(u32);
impl Noisy {
    fn id(&self) -> u32 {
        self.0
    }
}
impl Drop for Noisy {
    fn drop(&mut self) {
        DROPS.set(DROPS.get() + 1);
    }
}
#[cfg_attr(rust_js, rust_js::link_name = "globalThis.hold")]
fn hold(task: Box<dyn Fn() -> u32>) {
    unreachable!()
}
#[cfg_attr(rust_js, rust_js::link_name = "globalThis.take")]
fn take() -> Box<dyn Fn() -> u32> {
    unreachable!()
}
pub fn handed() -> (u32, u32) {
    let noisy = Noisy(7);
    hold(Box::new(move || noisy.id()));
    let task = take();
    let id = task();
    drop(task);
    (id, DROPS.get())
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const out = readFileSync(join(dir, "lib.js"), "utf8");
  expect(out).toContain("  globalThis.hold(() => Noisy.id(noisy));\n");
  expect(out).not.toContain("$dropping");
  let held: unknown;
  Object.assign(globalThis, { hold: (task: unknown) => (held = task), take: () => held });
  const lib = await import(join(dir, "lib.js"));
  expect(lib.handed()).toEqual([7, 0]);
  delete g.hold;
  delete g.take;
});

// The guide's difference 14 (ADR 0170, amended): generic code's
// `size_hint()` of an iterator JS gives, a binding's, is `(0, None)`, of
// a JS iterator and of an array JS gives alike, where Rust's is what its
// own impl gives, which a JS value has none of; of an array or a range
// Rust code made, its length.
test("difference 14: generic code's size_hint() of an iterator JS gives is (0, None)", async () => {
  const { js, lib } = await compiled("js-iterator-hint", `#[cfg_attr(rust_js, rust_js::link_name = "globalThis.numbers")]
fn numbers() -> Box<dyn Iterator<Item = u32>> {
    unreachable!()
}

fn hint<I: Iterator<Item = u32>>(it: I) -> (usize, Option<usize>) {
    it.size_hint()
}

pub fn given() -> u32 {
    let (lo, hi) = hint(numbers());
    lo as u32 * 1000 + hi.map_or(999, |hi| hi as u32)
}
`);
  expect(js).toContain("return $sizeHint(it);");
  g.numbers = () => [1, 2, 3].values();
  expect(lib.given()).toBe(999);
  g.numbers = () => [1, 2, 3];
  expect(lib.given()).toBe(999);
  delete g.numbers;
});

// The guide's difference 19 (ADR 0644): `unwrap()` of a throwing JS call's
// `Result` throws what JS threw, a `SyntaxError` of `JSON.parse`, not
// Rust's ``called `Result::unwrap()` on an `Err` value`` panic.
test("difference 19: unwrap() of a throwing JS call's Result throws what JS threw", async () => {
  const { js, lib } = await compiled("unwrap-throwing", `pub fn parsed(text: &str) -> bool {
    js::json::parse(text).unwrap().is_some()
}
`);
  expect(js).toContain("return JSON.parse(text) != null;");
  expect(lib.parsed("[1]")).toBe(true);
  let thrown: unknown;
  try {
    lib.parsed("{");
  } catch (error) {
    thrown = error;
  }
  expect(thrown).toBeInstanceOf(SyntaxError);
  expect(String((thrown as Error).message)).not.toContain("unwrap");
});

// The guide's difference 21 (ADR 0743): a test of a chain that an earlier
// one decides is left out, so `Array.isArray` is asked once; of a revoked
// `Proxy`, which throws a `TypeError` whenever it's asked, that one ask
// throws.
test("difference 21: Array.isArray is asked once of a chain, and a revoked Proxy throws at it", async () => {
  const { js, lib } = await compiled("revoked-proxy", `use js::{Kind, Unknown};

pub fn kind(value: Option<&Unknown>) -> u32 {
    match js::classify(value) {
        Kind::Array(&[first, _]) if matches!(js::classify(first), Kind::String(_)) => 1,
        Kind::Object(_) | Kind::Array(_) => 2,
        _ => 3,
    }
}
`);
  expect(js.match(/Array\.isArray\(value\)/g)?.length).toBe(1);
  expect([lib.kind(["a", 1]), lib.kind([1, 2]), lib.kind({}), lib.kind(5)]).toEqual([1, 2, 2, 3]);
  const { proxy, revoke } = Proxy.revocable([], {});
  revoke();
  expect(() => lib.kind(proxy)).toThrow(TypeError);
});

// The guide's difference 22 (ADR 0750, amended): a declared JS type's
// method is its class's, looked up before its arguments run, where Rust
// runs them first: an argument that replaces the method calls the old one.
test("difference 22: a declared type's method is looked up before its arguments run", async () => {
  const { js, lib } = await compiled("declared-method", `use std::marker::PhantomData;

use js::JsObject;

/// A declared JS type, its methods its class's.
pub struct App(PhantomData<JsObject>);

unsafe impl js::Defined for App {}

impl App {
    #[cfg_attr(rust_js, rust_js::link_name = "append")]
    pub fn append(&self, x: f64) {
        unreachable!()
    }
}

#[cfg_attr(rust_js, rust_js::link_name = "globalThis.stepper")]
fn stepper(app: &App) -> f64 {
    unreachable!()
}

pub fn run(app: &App) {
    app.append(stepper(app));
}
`);
  expect(js).toContain("app.append(globalThis.stepper(app));");
  const called: string[] = [];
  const app: Record<string, (x: number) => void> = { append: (x) => called.push(`old ${x}`) };
  g.stepper = (of: typeof app) => {
    of.append = (x) => called.push(`new ${x}`);
    return 4;
  };
  lib.run(app);
  expect(called).toEqual(["old 4"]);
  delete g.stepper;
});

// The guide's difference 23 (ADR 0774, amended): room for items whose size
// no caller can give, an impl's associated type its dictionary doesn't
// carry (#934), is checked by their count alone, as if they had none, so
// `with_capacity` past `isize::MAX` bytes of 8-byte items runs, where
// wasm32 Rust panics `capacity overflow`.
test("difference 23: room for items whose size no caller can give is checked by their count alone", async () => {
  const { js, lib } = await compiled("room-by-count", `pub struct Buf<I: Iterator> {
    pub it: I,
}

pub trait Room {
    fn fits(&self, n: usize) -> bool;
}

impl<I: Iterator> Room for Buf<I> {
    fn fits(&self, n: usize) -> bool {
        let v: Vec<I::Item> = Vec::with_capacity(n);
        v.is_empty()
    }
}

pub fn wide(n: usize) -> bool {
    Buf { it: [1u64, 2].into_iter() }.fits(n)
}
`);
  expect(js).toContain("  $roomFor(n);\n");
  expect([lib.wide(1), lib.wide(600_000_000), lib.wide(2 ** 31 - 1)]).toEqual([true, true, true]);
});
