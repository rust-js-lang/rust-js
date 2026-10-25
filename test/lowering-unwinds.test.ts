// Unwinding (ADR 0660): `std::panic::catch_unwind` catches what JS throws,
// a panic or any JS throw, as JS's `try` where its `Result` is matched;
// its payload a `dyn Any`, a panic's message or what JS threw; and
// `resume_unwind`, which throws it again. Each test is a program of its
// own, compiled and run.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { runSync } from "./child";
import { node } from "./programs";
import { buildReact, compiler, fixture, root, run, target } from "./support";

beforeAll(buildReact, 600_000);

/** `rust` compiled in a fixture of `name`, against the js crate: its JS, and where it is. */
function compiled(name: string, rust: string, flags: string[] = []): { js: string; dir: string } {
  const dir = fixture(name);
  writeFileSync(join(dir, "lib.rs"), rust);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target, ...flags]);
  return { js: readFileSync(join(dir, "lib.js"), "utf8"), dir };
}

// React's `getIODescription` (shared/ReactIODescription.js) describes any
// value by its getters, any of which may throw: `try { switch (typeof
// value) { .. } } catch (x) { return ''; }`. Its port catches the unwind.
test("getIODescription's try is catch_unwind of its body", () => {
  const { js, dir } = compiled("io-description", `use js::{Kind, Unknown};
use std::panic::catch_unwind;

/// \`value[key]\`, where it's text.
fn text_at(value: &Unknown, key: &str) -> Option<String> {
    match js::classify(js::get(value, key)) {
        Kind::String(text) => Some(text.to_string()),
        _ => None,
    }
}

/// What describes \`value\`: its \`url\`, its \`name\`, its text; any
/// getter may throw, which leaves it undescribed.
pub fn get_io_description(value: Option<&Unknown>) -> String {
    catch_unwind(|| {
        let Some(value) = value else {
            return String::new();
        };
        match js::classify(Some(value)) {
            Kind::Function(_) => text_at(value, "name").unwrap_or_default(),
            Kind::Object(object) => {
                if let Some(url) = text_at(object, "url") {
                    url
                } else if let Some(name) = text_at(object, "name") {
                    name
                } else {
                    let text = js::concat_text(object).unwrap();
                    if text.starts_with("[object ") || text.len() < 5 || text.len() > 500 {
                        return String::new();
                    }
                    text
                }
            }
            Kind::String(text) if text.len() < 5 || text.len() > 500 => String::new(),
            Kind::String(text) => text.to_string(),
            Kind::Number(_) | Kind::BigInt(_) => js::string(Some(value)),
            _ => String::new(),
        }
    })
    .unwrap_or_default()
}

/// The same, its body a function of its own.
fn describe(value: Option<&Unknown>) -> String {
    match js::classify(value) {
        Kind::Object(object) => text_at(object, "url").unwrap_or_default(),
        _ => String::new(),
    }
}

pub fn get_url(value: Option<&Unknown>) -> String {
    catch_unwind(|| describe(value)).unwrap_or_default()
}
`);
  // The closure's body is the `try`'s, its `return`s the function's; a
  // throw anywhere in it, the `catch`'s `return ""`.
  expect(js).not.toContain("$catchUnwind");
  expect(js).not.toContain("$unwound");
  expect(js).toContain(`export function getIoDescription(value) {
  try {
    if (value == null) {
      return "";
    }`);
  expect(js).toContain(`        const text = "" + value;`);
  expect(js).toContain(`  } catch {
    return "";
  }
}`);
  expect(js).toContain(`export function getUrl(value) {
  try {
    return describe(value);
  } catch {
    return "";
  }
}`);
  // In Node: a getter that throws, a Proxy whose trap throws, a symbol,
  // whose text throws, each undescribed; the rest described.
  writeFileSync(join(dir, "run.mjs"), `import { getIoDescription, getUrl } from "./lib.js";
const throwing = { get url() { throw new TypeError("no url"); } };
const trapped = new Proxy({}, { get() { throw new Error("trapped"); } });
function fetchUser() {}
const values = [throwing, trapped, Symbol("s"), { url: "https://react.dev" }, { name: "lazy" },
  { toString() { return "a long description"; } }, {}, "hello world", "abc", fetchUser, 42, 7n, null, undefined];
console.log(JSON.stringify(values.map(getIoDescription)));
console.log(JSON.stringify([throwing, trapped, { url: "u" }].map(getUrl)));
`);
  const ran = runSync([node!, join(dir, "run.mjs")], root, 30_000);
  expect(ran.stderr).toBe("");
  expect(ran.stdout).toBe(
    `["","","","https://react.dev","lazy","a long description","","hello world","","fetchUser","42","7","",""]\n["","","u"]\n`,
  );
});

// A crate with destructors whose `dyn Any`s hold none of them catches as
// one without (ADR 0691): what unwound, a panic's message or what JS threw,
// drops nothing, so `unwrap_or_default()` is still `try { return ..; }
// catch { return ""; }`, not `$catchUnwind` and a `kept` that drops it. And
// a `match` whose `Ok` arm is the closure's value and whose `Err` arm reads
// the payload, ReactComponentInfoStack's, is `try { ..; return info; }
// catch (x) { .. }`, its shadow, set to what was thrown, the `catch`'s.
test("a crate's destructors leave catch_unwind JS's try", async () => {
  const { js, dir } = compiled("unwind-destructors", `use js::{JsError, Kind, Unknown};
use std::panic::catch_unwind;

pub struct Restore(pub u32);

impl Drop for Restore {
    fn drop(&mut self) {
        self.0 = 0;
    }
}

pub fn describe(value: &Unknown) -> String {
    catch_unwind(|| match js::classify(js::get(value, "url")) {
        Kind::String(url) => url.to_string(),
        _ => String::new(),
    })
    .unwrap_or_default()
}

pub struct Info {
    pub name: String,
    pub owner: Option<&'static Info>,
    pub debug_stack: Option<&'static JsError>,
}

pub fn owner_stack(info: &Info) -> String {
    match catch_unwind(|| {
        let mut stack = String::new();
        if info.owner.is_none() {
            return info.name.clone();
        }
        let mut owner = info.owner;
        while let Some(current) = owner {
            stack += "\\n";
            if let Some(debug_stack) = current.debug_stack {
                stack += &debug_stack.message();
            }
            owner = current.owner;
        }
        stack
    }) {
        Ok(stack) => stack,
        Err(x) => match x.downcast_ref::<&'static JsError>() {
            Some(x) => format!("\\nError generating stack: {}", x.message()),
            None => String::from("\\nError generating stack"),
        },
    }
}
`);
  expect(js).not.toContain("$catchUnwind");
  expect(js).not.toContain("kept");
  expect(js).toContain(`export function describe(value) {
  try {`);
  expect(js).toContain(`  } catch {
    return "";
  }
}`);
  expect(js).toContain(`export function ownerStack(info) {
  try {
    let stack = "";`);
  expect(js).toContain(`    return stack;
  } catch (x) {
    return \`\\nError generating stack: \${x.message}\`;
  }`);
  const lib = await import(join(dir, "lib.js"));
  const throwing = { get url() { throw new TypeError("no url"); } };
  expect([lib.describe({ url: "u" }), lib.describe(throwing), lib.describe({})]).toEqual(["u", "", ""]);
  const root = { name: "Root", owner: undefined, debugStack: { message: "at Root" } };
  const child = { name: "Child", owner: root, debugStack: { message: "at Child" } };
  const broken = { name: "Broken", owner: { debugStack: { get message() { throw new RangeError("lost"); } } } };
  expect([lib.ownerStack(root), lib.ownerStack(child), lib.ownerStack(broken)]).toEqual([
    "Root",
    "\nat Root",
    "\nError generating stack: lost",
  ]);
});

// A panic's payload is its message, as Rust's is; JS doesn't keep whether
// it was a literal's `&str` or a formatted `String`, so it downcasts as
// either. What JS threw is a `&'static JsError`, itself.
test("a payload is a panic's message or what JS threw", async () => {
  const { js, dir } = compiled("unwind-payloads", `use js::{JsError, Unknown};
use std::panic::{catch_unwind, resume_unwind};

#[cfg_attr(rust_js, rust_js::link_name = "globalThis.risky")]
#[allow(unused_variables)]
fn risky(n: f64) -> f64 {
    unreachable!()
}

fn work(n: u32) -> u32 {
    if n == 1 {
        panic!("one");
    }
    if n == 2 {
        panic!("two: {n}");
    }
    n
}

pub fn as_str(n: u32) -> String {
    match catch_unwind(|| work(n)) {
        Ok(v) => v.to_string(),
        Err(payload) => match payload.downcast_ref::<&str>() {
            Some(text) => format!("str {text}"),
            None => "not a str".to_string(),
        },
    }
}

pub fn as_string(n: u32) -> String {
    match catch_unwind(|| work(n)) {
        Ok(v) => v.to_string(),
        Err(payload) => match payload.downcast::<String>() {
            Ok(text) => format!("String {text}"),
            Err(_) => "not a String".to_string(),
        },
    }
}

pub fn from_js(n: f64) -> String {
    match catch_unwind(|| risky(n)) {
        Ok(v) => v.to_string(),
        Err(payload) => match payload.downcast_ref::<&'static JsError>() {
            Some(error) => format!("{}: {}", error.name(), error.message()),
            None => match payload.downcast_ref::<&str>() {
                Some(text) => format!("a panic, {text}"),
                None => "neither".to_string(),
            },
        },
    }
}

pub fn spelled(n: u32) -> &'static str {
    catch_unwind(|| match n {
        1 => "one",
        2 => "two",
        _ => panic!("many"),
    })
    .unwrap_or_default()
}

pub fn named(n: u32, thrown: u32) -> String {
    match catch_unwind(|| work(n)) {
        Ok(v) => v.to_string(),
        Err(payload) => format!("{} {thrown}", payload.downcast_ref::<&str>().copied().unwrap_or("?")),
    }
}

pub fn rethrown(value: Option<&Unknown>) -> String {
    match catch_unwind(|| js::concat_text(value).unwrap()) {
        Ok(text) => text,
        Err(payload) => resume_unwind(payload),
    }
}
`);
  // The `catch`'s binding is what was thrown, `thrown`, and the `Err`'s
  // what unwound, its payload; the closure's call, `work(n)`.
  expect(js).toContain(`  try {
    const v = work(n);
    return String(v);
  } catch (thrown) {
    const payload = $unwound(thrown);`);
  // Where the `Err` arm reads a `thrown` of its own, its `Result`'s.
  expect(js).toContain("export function named(n, thrown) {\n  const catchUnwind = $catchUnwind(() => work(n));");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.named(3, 7), lib.named(1, 7)]).toEqual(["3", "one 7"]);
  // A closure's `switch` each of whose cases returns is its body's end.
  expect(js).toContain(`export function spelled(n) {
  try {
    switch (n) {`);
  expect([lib.spelled(1), lib.spelled(2), lib.spelled(3)]).toEqual(["one", "two", ""]);
  expect([lib.asStr(5), lib.asStr(1), lib.asStr(2)]).toEqual(["5", "str one", "str two: 2"]);
  expect([lib.asString(5), lib.asString(1), lib.asString(2)]).toEqual(["5", "String one", "String two: 2"]);
  const g = globalThis as Record<string, unknown>;
  g.risky = (n: number) => {
    if (n < 0) throw new RangeError("negative");
    if (n === 0) throw new Error("zero");
    return n * 2;
  };
  // A `RangeError` is what JS threw; an `Error` of JS's own constructor, a
  // panic's form, is a panic, whoever threw it, and what JS threw too
  // (ADR 0692): the `&JsError` downcast, tried first, finds it.
  expect([lib.fromJs(2), lib.fromJs(-1), lib.fromJs(0)]).toEqual(["4", "RangeError: negative", "Error: zero"]);
  delete g.risky;
  // What JS threw goes on as itself.
  const symbol = Symbol("s");
  let caught: unknown;
  try {
    lib.rethrown(symbol);
  } catch (e) {
    caught = e;
  }
  expect(caught).toBeInstanceOf(TypeError);
  expect(lib.rethrown("text")).toBe("text");
});

// A thrown `Error` of JS's own constructor, a panic's form, is a panic and
// what JS threw both (ADR 0692): its payload downcasts as its message, a
// `&str` and a `String`, and as the `&JsError` it is, whose `stack` React's
// `catch (sample) { .. sample.stack .. }` reads; `resume_unwind` of it
// throws the same `Error` again. A panic's JS stays `throw new Error(..)`.
test("a thrown Error answers both downcasts", async () => {
  const { js, dir } = compiled("unwind-error-both", `use js::JsError;
use std::panic::{catch_unwind, resume_unwind};

#[cfg_attr(rust_js, rust_js::link_name = "globalThis.render")]
fn render() -> u32 {
    unreachable!()
}

pub fn sample_stack() -> Option<String> {
    let Err(sample) = catch_unwind(render) else {
        return None;
    };
    let message = sample.downcast_ref::<&str>().copied().unwrap_or("none");
    let owned = sample.downcast_ref::<String>().cloned().unwrap_or_default();
    let error = sample.downcast_ref::<&'static JsError>()?;
    Some(format!("{message}|{owned}|{}|{}", error.message(), error.stack()?.contains(message)))
}

pub fn is_error(n: u32) -> bool {
    catch_unwind(|| if n > 1 { panic!("big {n}") } else { n })
        .err()
        .is_some_and(|payload| payload.is::<&JsError>())
}

pub fn boxed(n: u32) -> String {
    match catch_unwind(|| if n > 1 { panic!("big {n}") } else { n }) {
        Ok(n) => n.to_string(),
        Err(payload) => match payload.downcast::<&JsError>() {
            Ok(error) => error.message(),
            Err(_) => "not an Error".to_string(),
        },
    }
}

pub fn rethrown() -> u32 {
    match catch_unwind(render) {
        Ok(n) => n,
        Err(payload) => resume_unwind(payload),
    }
}
`);
  expect(js).toContain('throw new Error(`big ${n}`);');
  const lib = await import(join(dir, "lib.js"));
  const g = globalThis as Record<string, unknown>;
  const thrown = new Error("no props");
  g.render = () => {
    throw thrown;
  };
  expect(lib.sampleStack()).toBe("no props|no props|no props|true");
  expect([lib.isError(1), lib.isError(2)]).toEqual([false, true]);
  expect([lib.boxed(1), lib.boxed(3)]).toEqual(["1", "big 3"]);
  let caught: unknown;
  try {
    lib.rethrown();
  } catch (e) {
    caught = e;
  }
  expect(caught).toBe(thrown);
  g.render = () => 7;
  expect([lib.sampleStack(), lib.rethrown()]).toEqual([undefined, 7]);
  delete g.render;
});

// `resume_unwind` throws its payload again: a panic's message, a panic of
// it; what JS threw, itself; any other, a `$Unwinding` of it, which a
// `catch_unwind` gives back as it was.
test("resume_unwind throws what unwound as it was", async () => {
  const { js, dir } = compiled("unwind-resume", `use std::panic::{self, AssertUnwindSafe};

pub fn again(n: u32) -> u32 {
    match panic::catch_unwind(|| if n > 3 { panic!("big {n}") } else { n }) {
        Ok(v) => v,
        Err(payload) => panic::resume_unwind(payload),
    }
}

pub fn number() -> i32 {
    let caught = panic::catch_unwind(|| -> u32 { panic::resume_unwind(Box::new(5i32)) });
    match caught {
        Ok(_) => 0,
        Err(payload) => *payload.downcast::<i32>().expect("an i32"),
    }
}

pub fn uncaught() {
    panic::resume_unwind(Box::new(5i32));
}

pub fn text() {
    panic::resume_unwind(Box::new(String::from("as text")));
}

fn made(n: u32) -> impl FnOnce() -> u32 {
    if n > 3 {
        panic!("made {n}");
    }
    move || if n > 2 { panic!("ran {n}") } else { n }
}

pub fn given(n: u32) -> u32 {
    match panic::catch_unwind(made(n)) {
        Ok(v) => v,
        Err(_) => 0,
    }
}

pub fn shadowed(n: u32) -> u32 {
    let v = n + 1;
    let result = panic::catch_unwind(|| {
        let v = n * 2;
        if v > 10 {
            panic!("big");
        }
    });
    match result {
        Ok(()) => v,
        Err(_) => 0,
    }
}

pub fn counted(n: u32) -> u32 {
    let mut count = 0;
    let result = panic::catch_unwind(AssertUnwindSafe(|| {
        count += 1;
        if n > 3 {
            panic!("no");
        }
    }));
    if result.is_err() {
        count += 10;
    }
    count
}
`);
  expect(js).toContain(`  } catch (payload) {
    throw payload;
  }`);
  // An `AssertUnwindSafe` is what it holds: its closure's body is the `try`'s.
  expect(js).toContain(`  let count = 0;
  try {
    count = (count + 1) >>> 0;
    if (n > 3) {
      throw new Error("no");
    }
  } catch {
    count = (count + 10) >>> 0;
  }
  return count;`);
  const lib = await import(join(dir, "lib.js"));
  expect(lib.again(2)).toBe(2);
  expect(() => lib.again(5)).toThrow(new Error("big 5"));
  expect(lib.number()).toBe(5);
  expect(() => lib.uncaught()).toThrow("Box<dyn Any>");
  // Text resumed is the panic of it, an `Error` of JS's own constructor.
  expect(() => lib.text()).toThrow(new Error("as text"));
  expect([lib.counted(1), lib.counted(5)]).toEqual([1, 11]);
  // What makes the closure isn't caught, as it runs before `catch_unwind`.
  expect([lib.given(2), lib.given(3)]).toEqual([2, 0]);
  expect(() => lib.given(4)).toThrow("made 4");
  // A closure's body beside the `Ok` arm, its names apart from the arm's.
  expect(js).toContain(`  try {
    const v$1 = (n * 2) >>> 0;`);
  expect([lib.shadowed(2), lib.shadowed(6)]).toEqual([3, 0]);
});

// A future's panic is its promise's, rejected, which `.await` throws where
// it's awaited, as Rust's panics where it's polled: `catch_unwind` of what
// makes one is `Ok` of it.
test("catch_unwind of an async closure is Ok of its promise", async () => {
  const { dir } = compiled("unwind-async", `use std::panic;

pub async fn late(n: u32) -> u32 {
    let made = panic::catch_unwind(|| async move {
        if n > 3 {
            panic!("late {n}");
        }
        n
    });
    match made {
        Ok(future) => future.await,
        Err(_) => 0,
    }
}
`);
  const lib = await import(join(dir, "lib.js"));
  expect(await lib.late(2)).toBe(2);
  expect(lib.late(5)).rejects.toThrow("late 5");
});

// React's describeElementType resolves a lazy component's type, which may
// throw: `try { return describeElementType(init(payload)); } catch (x) {}`,
// then on to what follows. Its port's `if let Ok(name) = catch_unwind(..)
// { return name; }`, in a block of its own, is that `try`, its `catch`
// nothing, not the `Result` made and its `TAG` tested (ADR 0637, amended).
test("an if let Ok of catch_unwind that returns is a try whose catch does nothing", async () => {
  const { js, dir } = compiled("unwind-if-let-ok", `use std::panic::catch_unwind;

fn init(payload: u32) -> u32 {
    if payload == 0 {
        panic!("not loaded");
    }
    payload
}

pub fn lazy(kind: u32, payload: u32) -> String {
    if kind == 3 {
        if let Ok(name) = catch_unwind(|| format!("<{}>", init(payload))) {
            return name;
        }
    }
    String::new()
}
`);
  expect(js).toContain('  if (kind === 3) {\n    try {\n      return `<${init(payload)}>`;\n    } catch {}\n  }\n  return "";');
  expect(js).not.toContain("TAG");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.lazy(3, 2), lib.lazy(3, 0), lib.lazy(1, 2)]).toEqual(["<2>", "", ""]);
});

// A binding's setter may throw, as any JS may: an `Ok` arm that sets one
// runs after the `try`, where what it throws goes on, as Rust's panic out
// of the arm does, never into the `catch` (ADR 0660, amended). React's
// `startGestureTransition`'s `Ok(Some(finish)) => return finish`, which
// can't throw, is in the `try`.
test("an Ok arm that sets a binding's property runs after catch_unwind's try", async () => {
  const { js, dir } = compiled("unwind-ok-setter", `use std::marker::PhantomData;
use std::panic::{AssertUnwindSafe, catch_unwind};

pub struct Shared(PhantomData<js::JsObject>);

unsafe extern "Rust" {
    #[link_name = "./internals.js#default"]
    safe static INTERNALS: &'static Shared;
}

#[cfg_attr(rust_js, rust_js::link_name = "set T")]
fn set_t(this: &Shared, transition: f64) {
    let _ = (this, transition);
    unreachable!()
}

pub fn run(scope: impl FnOnce() -> f64) -> f64 {
    match catch_unwind(AssertUnwindSafe(scope)) {
        Ok(v) => {
            set_t(INTERNALS, v);
            v
        }
        Err(_) => 0.0,
    }
}

pub fn finish(scope: impl FnOnce() -> Option<f64>) -> f64 {
    match catch_unwind(AssertUnwindSafe(scope)) {
        Ok(Some(done)) => return done,
        Ok(None) => {}
        Err(_) => set_t(INTERNALS, -1.0),
    }
    7.0
}
`);
  writeFileSync(join(dir, "internals.js"), `export default {
  set T(value) {
    if (value > 5) throw new Error("too big");
    this.t = value;
  },
};
`);
  expect(js).toContain("  let v;\n  try {\n    v = scope();\n  } catch {\n    return 0;\n  }\n  INTERNALS.T = v;\n");
  expect(js).toContain("  try {\n    const catchUnwind = scope();\n    if (catchUnwind != null) {\n");
  expect(js).not.toContain("TAG");
  const lib = await import(join(dir, "lib.js"));
  const internals = (await import(join(dir, "internals.js"))).default;
  expect([lib.run(() => 2), internals.t, lib.run(() => { throw new Error("no"); })]).toEqual([2, 2, 0]);
  expect(() => lib.run(() => 9)).toThrow("too big");
  expect([lib.finish(() => 3), lib.finish(() => undefined), lib.finish(() => { throw new Error("no"); }), internals.t]).toEqual([3, 7, 7, -1]);
});

// React's describeNativeComponentFrame restores what it changed in one
// `finally`; its port's guard, whose `Drop` restores it, is that `finally`
// (ADR 0693): its scope's `if`, loop and `return` from inside them in the
// `try`, not a `try` and `catch` of the drop for each statement.
test("a guard's scope is one finally, its returns in it", async () => {
  const { js, dir } = compiled("unwind-guard-scope", `use std::cell::Cell;

thread_local! {
    static REENTRY: Cell<bool> = const { Cell::new(false) };
}

struct Restore {
    previous: u32,
}

impl Drop for Restore {
    fn drop(&mut self) {
        REENTRY.set(false);
        restored(self.previous);
    }
}

#[cfg_attr(rust_js, rust_js::link_name = "globalThis.work")]
fn work(n: u32) -> u32 {
    n
}

#[cfg_attr(rust_js, rust_js::link_name = "globalThis.restored")]
fn restored(_: u32) {}

pub fn guarded(n: u32) -> String {
    if REENTRY.get() {
        return String::new();
    }
    {
        REENTRY.set(true);
        let previous = work(0);
        let _restore = Restore { previous };
        let first = work(n);
        if first > 3 {
            let mut i = 0;
            while i < first {
                if work(i) == 7 {
                    return format!("found {i}");
                }
                i += 1;
            }
        }
    }
    String::from("none")
}
`);
  expect(js).not.toContain("catch (error)");
  expect(js.match(/finally/g)?.length).toBe(1);
  expect(js).toContain(`  } finally {
    reentry = false;
    globalThis.restored(previous);
  }
  return "none";`);
  const lib = await import(join(dir, "lib.js"));
  const g = globalThis as Record<string, unknown>;
  const restores: number[] = [];
  g.restored = (n: number) => restores.push(n);
  g.work = (n: number) => {
    if (n === 30) throw new RangeError("thirty");
    return n === 0 ? 40 : n;
  };
  expect([lib.guarded(2), lib.guarded(8)]).toEqual(["none", "found 7"]);
  expect(() => lib.guarded(30)).toThrow("thirty");
  expect(restores).toEqual([40, 40, 40]);
  // Restored each time, so the next call isn't a reentry.
  expect(lib.guarded(2)).toBe("none");
  delete g.restored;
  delete g.work;
});

// React's describeNativeComponentFrame, and its fallback's `finally`, as
// the port's guards write them (ADR 0706): each guard's fields the
// variables it was made of, read in the `finally`; and what each
// statement's `try` held apart, read in place once the scope is one
// `finally`, `if (lines[s].includes(..))`, `return \`..\`;`.
test("a guard's fields are its variables, its scope read in place", async () => {
  const { js, dir } = compiled("unwind-guard-fields", `use std::cell::Cell;

thread_local! {
    static REENTRY: Cell<bool> = const { Cell::new(false) };
}

struct Restore {
    previous: u32,
    dispatcher: Option<u32>,
}

impl Drop for Restore {
    fn drop(&mut self) {
        REENTRY.set(false);
        restored(self.previous, self.dispatcher);
    }
}

struct RestoreProps {
    prototype: u32,
    prev_props: Option<u32>,
}

impl Drop for RestoreProps {
    fn drop(&mut self) {
        match self.prev_props {
            Some(prev_props) => define(self.prototype, prev_props),
            None => delete(self.prototype),
        }
    }
}

#[cfg_attr(rust_js, rust_js::link_name = "globalThis.work")]
fn work(n: u32) -> u32 {
    n
}

#[cfg_attr(rust_js, rust_js::link_name = "globalThis.lines")]
fn lines() -> Vec<String> {
    Vec::new()
}

#[cfg_attr(rust_js, rust_js::link_name = "globalThis.restored")]
fn restored(_: u32, _: Option<u32>) {}

#[cfg_attr(rust_js, rust_js::link_name = "globalThis.define")]
fn define(_: u32, _: u32) {}

#[cfg_attr(rust_js, rust_js::link_name = "globalThis.del")]
fn delete(_: u32) {}

pub fn frame(dev: bool) -> String {
    if REENTRY.get() {
        return String::new();
    }
    {
        REENTRY.set(true);
        let previous = work(0);
        let mut dispatcher = None;
        if dev {
            dispatcher = Some(work(1));
        }
        let _restore = Restore {
            previous,
            dispatcher,
        };
        let lines = lines();
        let mut s = 0;
        while s < lines.len() {
            if lines[s].contains("Root") {
                let frame = format!("\\n{}", lines[s]);
                return frame;
            }
            s += 1;
        }
    }
    String::new()
}

pub fn props(prototype: u32) -> u32 {
    let prev_props = if prototype > 5 { Some(prototype * 2) } else { None };
    let _restore_props = RestoreProps {
        prototype,
        prev_props,
    };
    work(prototype)
}

struct Doubled {
    previous: u32,
}

impl Drop for Doubled {
    fn drop(&mut self) {
        if self.previous > 0 {
            let previous = self.previous * 2;
            restored(previous, None);
        }
    }
}

pub fn doubled(previous: u32) -> u32 {
    let _doubled = Doubled { previous };
    work(previous)
}

pub fn changed(mut previous: u32) -> u32 {
    let _restore = Restore {
        previous,
        dispatcher: Some(previous),
    };
    previous += 1;
    work(previous)
}
`);
  expect(js).not.toContain("_restoreProps");
  expect(js).toContain(`  try {
    const lines = globalThis.lines();
    let s = 0;
    while (s < lines.length) {
      if (lines[s].includes("Root")) {
        const frame = \`\\n\${lines[s]}\`;
        return frame;
      }
      s++;
    }
  } finally {
    reentry = false;
    globalThis.restored(previous, dispatcher);
  }`);
  expect(js).toContain(`  try {
    return globalThis.work(prototype);
  } finally {
    if (prevProps != null) {`);
  // A variable set once the guard's made isn't what the guard holds.
  expect(js).toContain("const _restore = {\n    previous,\n    dispatcher: previous,\n  };");
  expect(js).toContain("globalThis.restored(_restore.previous, _restore.dispatcher);");
  const lib = await import(join(dir, "lib.js"));
  const g = globalThis as Record<string, unknown>;
  const calls: unknown[][] = [];
  g.work = (n: number) => {
    if (n === 9) throw new RangeError("nine");
    return n + 40;
  };
  g.lines = () => ["Error", "at Root", "at x"];
  g.restored = (...args: unknown[]) => calls.push(["restored", ...args]);
  g.define = (...args: unknown[]) => calls.push(["define", ...args]);
  g.del = (...args: unknown[]) => calls.push(["delete", ...args]);
  expect([lib.frame(true), lib.frame(false)]).toEqual(["\nat Root", "\nat Root"]);
  g.lines = () => ["Error"];
  expect(lib.frame(true)).toBe("");
  expect(lib.props(2)).toBe(42);
  expect(() => lib.props(9)).toThrow("nine");
  expect(lib.changed(3)).toBe(44);
  expect(lib.doubled(5)).toBe(45);
  expect(calls).toEqual([
    ["restored", 40, 41],
    ["restored", 40, undefined],
    ["restored", 40, 41],
    ["delete", 2],
    ["define", 9, 18],
    ["restored", 3, 3],
    ["restored", 10, undefined],
  ]);
  for (const name of ["work", "lines", "restored", "define", "del"]) delete g[name];
});

// React's DetermineComponentFrameRoot throws its control error to catch
// it, `try { throw Error(); } catch (x) { control = x; }`: a port's
// `panic!("")` caught, its `&JsError` the `Error` thrown (ADR 0706); and a
// payload resumed is thrown as it was caught.
test("a port throws an Error by panic! and catches it as thrown", async () => {
  const { js, dir } = compiled("unwind-as-thrown", `use js::JsError;
use std::panic::{catch_unwind, resume_unwind};

pub fn control() -> Option<String> {
    let Err(x) = catch_unwind(|| panic!(""));
    let control = x.downcast_ref::<&'static JsError>().copied();
    control.and_then(|c| c.stack())
}

pub fn assigned(n: u32) -> String {
    let mut control: Option<&'static JsError> = None;
    match catch_unwind(|| panic!("")) {
        Err(x) => control = x.downcast_ref::<&'static JsError>().copied(),
    }
    match control {
        Some(error) if n > 1 => error.message(),
        _ => String::from("none"),
    }
}

#[cfg_attr(rust_js, rust_js::link_name = "globalThis.render")]
fn render() -> u32 {
    unreachable!()
}

pub fn rethrown() -> u32 {
    match catch_unwind(render) {
        Ok(n) => n,
        Err(payload) => resume_unwind(payload),
    }
}

pub fn named() -> u32 {
    match catch_unwind(render) {
        Ok(n) => n,
        Err(payload) => {
            if payload.downcast_ref::<String>().is_some_and(|m| m == "skip") {
                return 0;
            }
            resume_unwind(payload)
        }
    }
}

pub fn of_any() -> Option<String> {
    match catch_unwind(render) {
        Ok(_) => None,
        Err(payload) => payload.downcast_ref::<&'static JsError>().map(|e| e.message()),
    }
}
`);
  // What follows that reads the payload, or what that declares, is the
  // `catch`, as nothing after the `try` runs but once it threw (ADR 0830).
  expect(js).toContain(`export function control() {
  try {
    throw new Error();
  } catch (x) {
    const control = x;
    return control?.stack;
  }
}`);
  expect(js).toContain(`  } catch (x) {
    const control = x;
    if (control && n > 1) {
      const error = control;
      return error.message;
    }
  }
  return "none";
}`);
  expect(js).toContain(`  try {
    return globalThis.render();
  } catch (payload) {
    throw payload;
  }`);
  // Read otherwise, the payload is kept, and thrown again as it was.
  expect(js).toContain("    throw thrown;\n");
  // What a call throws may be any value: its downcast is what was thrown,
  // and `map` gives it to its closure (ADR 0710, amended).
  expect(js).toContain("  } catch (payload) {\n    return payload.message;\n  }");
  const lib = await import(join(dir, "lib.js"));
  const g = globalThis as Record<string, unknown>;
  expect(lib.control()).toContain("Error");
  expect([lib.assigned(1), lib.assigned(2)]).toEqual(["none", ""]);
  const thrown = new TypeError("no props");
  g.render = () => {
    throw thrown;
  };
  for (const f of [lib.rethrown, lib.named]) {
    let caught: unknown;
    try {
      f();
    } catch (e) {
      caught = e;
    }
    expect(caught).toBe(thrown);
  }
  expect(lib.ofAny()).toBe("no props");
  g.render = () => {
    throw new Error("skip");
  };
  expect(lib.named()).toBe(0);
  g.render = () => {
    throw 5;
  };
  expect(lib.ofAny()).toBe(undefined);
  delete g.render;
});

// React's getOwnerStackByComponentInfoInDev reads what it caught, `catch
// (x) { return '..' + x.message + '\n' + x.stack; }`: every unwind's
// payload downcasts as the `&JsError` thrown (ADR 0710), a typed one's its
// `$Unwinding`, a falsy throw's itself, so its `None` arm is dead and
// dropped, and the `catch` is of what was thrown. A typed payload still
// downcasts to its type, and is resumed as the same `$Unwinding`.
test("every unwind's payload downcasts as the JsError thrown", async () => {
  const { js, dir } = compiled("unwind-every-thrown", `use js::JsError;
use std::any::Any;
use std::panic::{catch_unwind, resume_unwind};

#[cfg_attr(rust_js, rust_js::link_name = "globalThis.render")]
fn render() -> String {
    unreachable!()
}

pub fn info() -> String {
    match catch_unwind(render) {
        Ok(info) => info,
        Err(x) => match x.downcast_ref::<&'static JsError>() {
            Some(x) => format!("\\nError generating stack: {}\\n{}", x.message(), x.stack().unwrap_or_default()),
            None => format!("\\nError generating stack: {}\\n", x.downcast_ref::<String>().cloned().unwrap_or_default()),
        },
    }
}

pub fn kind() -> String {
    match catch_unwind(render) {
        Ok(_) => String::from("none"),
        Err(x) => match x.downcast_ref::<&'static JsError>() {
            Some(e) => format!("{e:?}"),
            None => String::from("not thrown"),
        },
    }
}

pub fn control() -> Option<&'static JsError> {
    let mut control = None;
    if let Err(x) = catch_unwind(render) {
        control = x.downcast_ref::<&'static JsError>().copied();
    }
    control
}

pub fn typed(n: i32) -> String {
    match catch_unwind(|| -> String { resume_unwind(Box::new(n)) }) {
        Ok(s) => s,
        Err(x) => match x.downcast_ref::<&'static JsError>() {
            Some(e) => format!("{} {}", e.message(), x.downcast_ref::<i32>().copied().unwrap_or_default()),
            None => String::from("none"),
        },
    }
}

pub fn is_thrown(n: i32) -> bool {
    catch_unwind(|| -> u32 { resume_unwind(Box::new(n)) })
        .err()
        .is_some_and(|payload| payload.is::<&JsError>() && payload.is::<i32>())
}

pub fn boxed(n: i32) -> String {
    match catch_unwind(|| -> u32 { resume_unwind(Box::new(n)) }) {
        Ok(_) => String::new(),
        Err(payload) => match payload.downcast::<&JsError>() {
            Ok(error) => error.message(),
            Err(_) => String::from("not an Error"),
        },
    }
}

pub fn own() -> Option<i32> {
    let payload: Box<dyn Any + Send> = Box::new(7i32);
    payload.downcast_ref::<i32>().copied()
}

pub fn typed_throw(n: i32) -> String {
    resume_unwind(Box::new(n))
}

fn resume(payload: Box<dyn Any + Send>) -> String {
    resume_unwind(payload)
}

pub fn passed() -> String {
    match catch_unwind(render) {
        Ok(s) => s,
        Err(payload) => resume(payload),
    }
}

fn message_of(payload: &(dyn Any + Send)) -> String {
    payload.downcast_ref::<&'static JsError>().map(|e| e.message()).unwrap_or_default()
}

pub fn passed_message() -> String {
    match catch_unwind(render) {
        Ok(s) => s,
        Err(payload) => message_of(&*payload),
    }
}

fn text_of(payload: &(dyn Any + Send)) -> String {
    match payload.downcast_ref::<&str>() {
        Some(text) => format!("a panic, {text}"),
        None => String::from("not a panic"),
    }
}

pub fn passed_text() -> String {
    match catch_unwind(render) {
        Ok(s) => s,
        Err(payload) => text_of(&*payload),
    }
}

pub fn tried() -> Option<String> {
    let Err(x) = catch_unwind(render) else {
        return None;
    };
    let e = x.downcast_ref::<&'static JsError>()?;
    Some(format!("{e:?}"))
}

pub fn reset(n: u32) -> bool {
    let Err(x) = catch_unwind(render) else {
        return false;
    };
    let mut e = x.downcast_ref::<&'static JsError>();
    if n > 1 {
        e = None;
    }
    match e {
        Some(_) => true,
        None => false,
    }
}

fn widened(payload: Box<dyn Any + Send>) -> Box<dyn Any> {
    payload
}

pub fn of_any(own: bool) -> String {
    let any: Box<dyn Any> = if own {
        Box::new(js::js_error::new("own"))
    } else {
        widened(catch_unwind(|| -> u32 { resume_unwind(Box::new(4i32)) }).unwrap_err())
    };
    let found = any.downcast_ref::<&'static JsError>().map(|e| e.message());
    format!("{found:?} {}", any.is::<&JsError>())
}

pub fn of_any_boxed(own: bool) -> String {
    let any: Box<dyn Any> = if own {
        Box::new(js::js_error::new("own"))
    } else {
        widened(catch_unwind(|| -> u32 { resume_unwind(Box::new(4i32)) }).unwrap_err())
    };
    match any.downcast::<&JsError>() {
        Ok(e) => e.message(),
        Err(_) => String::from("not an Error"),
    }
}
`);
  // The `catch` is of what was thrown: no `$unwound`, no test, no `None`.
  expect(js).toContain(`  } catch (x) {
    return \`\\nError generating stack: \${x.message}\\n\${x.stack ?? ""}\`;
  }`);
  expect(js).toContain(`  } catch (x) {
    control = x;
  }`);
  expect(js).not.toContain("not thrown");
  expect(js).toContain("    const e = thrown;\n    return `${e.message} ");
  const lib = await import(join(dir, "lib.js"));
  const g = globalThis as Record<string, unknown>;
  const thrown = new TypeError("no props");
  g.render = () => {
    throw thrown;
  };
  expect(lib.info()).toBe(`\nError generating stack: no props\n${thrown.stack}`);
  expect([lib.control(), lib.passedMessage()]).toEqual([thrown, "no props"]);
  g.render = () => {
    throw new Error("a panic's");
  };
  expect(lib.info()).toStartWith("\nError generating stack: a panic's\nError: a panic's");
  // Passed on, a payload is what was thrown, a panic's message too, and
  // resumed, it's thrown again as itself.
  for (const error of [new TypeError("no props"), new Error("a panic's")]) {
    g.render = () => {
      throw error;
    };
    expect([lib.passedMessage(), lib.passedText()]).toEqual([
      error.message,
      error instanceof TypeError ? "not a panic" : "a panic, a panic's",
    ]);
    let resumed: unknown;
    try {
      lib.passed();
    } catch (e) {
      resumed = e;
    }
    expect(resumed).toBe(error);
  }
  // A falsy throw is what was thrown too: the `Some` arm's.
  const kinds = [];
  for (const value of [undefined, null, 0, "", false, 5]) {
    g.render = () => {
      throw value;
    };
    kinds.push(lib.kind());
  }
  expect(kinds).toEqual(["undefined", "null", "0", "", "false", "5"]);
  // `?` of it goes on; a variable set again is what it's set to.
  expect([lib.tried(), lib.reset(1), lib.reset(2)]).toEqual(["5", true, false]);
  g.render = () => {
    throw undefined;
  };
  expect(lib.tried()).toBe("undefined");
  // A `dyn Any` may hold a `&JsError` of its own: an unwind's is what was
  // thrown, and one of its own itself.
  expect([lib.ofAny(true), lib.ofAny(false)]).toEqual(['Some("own") true', 'Some("Box<dyn Any>") true']);
  expect([lib.ofAnyBoxed(true), lib.ofAnyBoxed(false)]).toEqual(["own", "Box<dyn Any>"]);
  // A typed payload is its `$Unwinding`, and its type's still.
  expect(lib.typed(5)).toBe("Box<dyn Any> 5");
  expect([lib.isThrown(5), lib.boxed(5), lib.own()]).toEqual([true, "Box<dyn Any>", 7]);
  // Resumed, it's the same `$Unwinding` again.
  let first: unknown;
  g.render = () => {
    try {
      lib.typedThrow(3);
    } catch (e) {
      first = e;
    }
    throw first;
  };
  let caught: unknown;
  try {
    lib.passed();
  } catch (e) {
    caught = e;
  }
  expect(caught).toBeInstanceOf(Error);
  expect(caught).toBe(first);
  delete g.render;
});

// React's describeBuiltInComponentFrame reads the stack of what it threw,
// `catch (x) { const match = x.stack.trim().match(..); }`. The downcast
// of what's caught is always `Some` (ADR 0710), so what's given its value,
// `and_then(|x| x.stack())`'s closure, `map`'s and `is_some_and`'s, is
// given it, as Rust gives a `Some`'s: `x.stack`, never `x?.stack` (ADR
// 0710, amended).
test("what's given the thrown downcast is given it as it is", async () => {
  const { js, dir } = compiled("unwind-given-thrown", `use js::JsError;
use std::panic::catch_unwind;

#[cfg_attr(rust_js, rust_js::link_name = "globalThis.render")]
fn render() -> u32 {
    unreachable!()
}

pub fn prefix() -> String {
    let Err(x) = catch_unwind(|| panic!(""));
    let stack = x
        .downcast_ref::<&'static JsError>()
        .and_then(|x| x.stack())
        .unwrap_or_default();
    stack.trim().to_string()
}

pub fn control() -> Option<&'static JsError> {
    let Err(x) = catch_unwind(|| panic!(""));
    x.downcast_ref::<&'static JsError>().copied()
}

pub fn message() -> Option<String> {
    match catch_unwind(render) {
        Ok(_) => None,
        Err(payload) => payload.downcast_ref::<&'static JsError>().map(|e| e.message()),
    }
}

pub fn nested() -> Option<Option<u32>> {
    match catch_unwind(render) {
        Ok(_) => None,
        Err(payload) => payload.downcast_ref::<&'static JsError>().map(|_| None),
    }
}

pub fn named(name: &str) -> bool {
    match catch_unwind(render) {
        Ok(_) => false,
        Err(payload) => payload
            .downcast_ref::<&'static JsError>()
            .is_some_and(|e| e.message() == name),
    }
}
`);
  expect(js).toContain(`  } catch (x) {
    const stack = x.stack ?? "";
    return $trim(stack);
  }`);
  expect(js).toContain("  } catch (x) {\n    return x;\n  }");
  expect(js).toContain("  } catch (payload) {\n    return payload.message;\n  }");
  expect(js).toContain("  } catch (payload) {\n    return payload.message === name;\n  }");
  expect(js).not.toContain("?.");
  const lib = await import(join(dir, "lib.js"));
  const g = globalThis as Record<string, unknown>;
  expect(lib.prefix()).toStartWith("Error");
  expect(lib.control()).toBeInstanceOf(Error);
  g.render = () => {
    throw new TypeError("no props");
  };
  expect([lib.message(), lib.named("no props"), lib.named("other")]).toEqual(["no props", true, false]);
  // `map`'s `Some` of a `None` is boxed, as an `Option` of what looks
  // like `None` is (ADR 0051).
  expect(lib.nested()).not.toBe(undefined);
  g.render = () => {
    throw 5;
  };
  expect([lib.message(), lib.named("5")]).toEqual([undefined, false]);
  delete g.render;
});

// React's describeBuiltInComponentFrame reads the prefix and suffix off
// the stack of what it threw, `try { throw Error(); } catch (x) { const
// stack = ..; prefix = ..; suffix = ..; }`. Its port writes rustc's
// irrefutable `let Err(x) = catch_unwind(|| panic!(""))`, and what follows
// it that reads `x`, or what that declares, is the `catch`: nothing after
// the `try` runs but once it threw (ADR 0830).
test("what follows an irrefutable let Err of catch_unwind is its catch", async () => {
  const { js, dir } = compiled("unwind-irrefutable", `use js::JsError;
use std::cell::RefCell;
use std::panic::catch_unwind;

thread_local! {
    static PREFIX: RefCell<Option<String>> = const { RefCell::new(None) };
    static SUFFIX: RefCell<String> = const { RefCell::new(String::new()) };
}

pub fn describe_built_in_component_frame(name: &str) -> String {
    if PREFIX.with_borrow(Option::is_none) {
        // Extract the VM specific prefix used by each line.
        let Err(x) = catch_unwind(|| panic!(""));
        let stack = x
            .downcast_ref::<&'static JsError>()
            .and_then(|x| x.stack())
            .unwrap_or_default();
        let r#match = js::reg_exp::new(r"\\n( *(at )?)", "").exec(&js::string::trim(&stack));
        PREFIX.set(Some(r#match.and_then(|m| m.get(1)).unwrap_or_default()));
        SUFFIX.set(if stack.contains("\\n    at") { " (<anonymous>)" } else { "" }.to_string());
    }
    let prefix = PREFIX.with_borrow(|prefix| prefix.clone().unwrap_or_default());
    let suffix = SUFFIX.with_borrow(String::clone);
    format!("\\n{prefix}{name}{suffix}")
}
`);
  expect(js).toContain(`  if (prefix == null) {
    // Extract the VM specific prefix used by each line.
    try {
      throw new Error();
    } catch (x) {
      const stack = x.stack ?? "";
      const match = /\\n( *(at )?)/.exec(stack.trim());
      prefix = match?.[1] ?? "";
      suffix = stack.includes("\\n    at") ? " (<anonymous>)" : "";
    }
  }
  const prefix$1 = prefix ?? "";`);
  const lib = await import(join(dir, "lib.js"));
  expect(lib.describeBuiltInComponentFrame("App")).toBe("\n    at App (<anonymous>)");
  expect(lib.describeBuiltInComponentFrame("Div")).toBe("\n    at Div (<anonymous>)");
});

// React's DetermineComponentFrameRoot catches its sample, then tests the
// control its `try` set: `try { .. } catch (sample) { if (sample &&
// control && ..) { return [..]; } }`. Its port's `if let Err(sample) =
// catch_unwind(..) && let Some(control) = control && ..` is that `try`,
// the rest of its chain the `catch`'s `if` (ADR 0706, amended); an
// `else` where its arm returns is what follows, and a closure whose value
// nothing reads is its body.
test("a catch_unwind that heads an if let chain is a try", async () => {
  const { js, dir } = compiled("unwind-chained", `use js::JsError;
use std::panic::{AssertUnwindSafe, catch_unwind};

#[cfg_attr(rust_js, rust_js::link_name = "globalThis.render")]
fn render() -> u32 {
    unreachable!()
}

pub fn root(construct: bool) -> (Option<String>, Option<String>) {
    let mut control: Option<&'static JsError> = None;
    if let Err(sample) = catch_unwind(AssertUnwindSafe(|| {
        if construct {
            render();
        } else {
            let Err(x) = catch_unwind(|| panic!(""));
            control = x.downcast_ref::<&'static JsError>().copied();
            render();
        }
    }))
    // This is inlined manually because closure doesn't do it for us.
        && let Some(sample) = sample.downcast_ref::<&'static JsError>()
        && let Some(control) = control
        && let Some(sample_stack) = sample.stack()
    {
        return (Some(sample_stack), control.stack());
    }
    (None, None)
}

pub fn otherwise(wanted: bool) -> &'static str {
    if let Err(_) = catch_unwind(render)
        && wanted
    {
        "caught"
    } else {
        "not"
    }
}

pub fn counted(limit: u32) -> u32 {
    let mut seen = 0;
    let mut count = 0;
    if let Err(_) = catch_unwind(AssertUnwindSafe(|| {
        seen += 1;
        render()
    })) && seen <= limit
    {
        count += 100;
    }
    count + seen
}
`);
  expect(js).toContain(`  } catch (sample) {
    // This is inlined manually because closure doesn't do it for us.
    if (control) {
      const sampleStack = sample.stack;
      if (sampleStack != null) {
        return [sampleStack, control.stack];
      }
    }
  }
  return [undefined, undefined];
}`);
  // What the control's `catch` sets, the rest of its block after it, as
  // nothing else there reads the payload (ADR 0830).
  expect(js).toContain(`      try {
        throw new Error();
      } catch (x) {
        control = x;
      }
      globalThis.render();
`);
  expect(js).toContain(`  try {
    globalThis.render();
  } catch {
    if (wanted) {
      return "caught";
    }
  }
  return "not";`);
  expect(js).toContain(`  try {
    seen = (seen + 1) >>> 0;
    globalThis.render();
  } catch {
    if (seen <= limit) {`);
  expect(js).not.toContain("$catchUnwind");
  const lib = await import(join(dir, "lib.js"));
  const g = globalThis as Record<string, unknown>;
  const thrown = new TypeError("no props");
  g.render = () => {
    throw thrown;
  };
  const [sample, control] = lib.root(false);
  expect(sample).toBe(thrown.stack);
  expect(control).toStartWith("Error");
  expect(lib.root(true)).toEqual([undefined, undefined]);
  expect([lib.otherwise(true), lib.otherwise(false), lib.counted(0), lib.counted(1)]).toEqual(["caught", "not", 1, 101]);
  g.render = () => 1;
  expect([lib.root(false), lib.otherwise(true), lib.counted(1)]).toEqual([[undefined, undefined], "not", 1]);
  delete g.render;
});

// rust-js's panics unwind (ADR 0090); where the crate asked them to abort,
// `catch_unwind` would catch what ends Rust's process.
test("unwinding is refused where panics abort", () => {
  const dir = fixture("unwind-abort");
  writeFileSync(join(dir, "lib.rs"), `pub fn caught() -> bool {
    std::panic::catch_unwind(|| 1).is_ok()
}
`);
  const p = runSync([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "-C", "panic=abort"], root, 60_000);
  expect(p.code).not.toBe(0);
  expect(p.stderr).toContain("rust-js does not support unwinding where a panic aborts, `panic = \"abort\"`: rust-js's panics unwind yet");
});

// The stages read again once a body's drops are each one `try { .. }
// finally { .. }` are tidy's runner's, traced as their own (ADR 0705,
// amended): `--tidy-passes` names them `again/<stage>/<pass>`, between
// `scopes` and `finish`, and `--tidy-until` stops in them, past what
// `scopes` left and before what `finish` names.
test("a stage read again is its own in tidy's trace", () => {
  const passes = run([compiler, "--tidy-passes"]).trim().split("\n");
  const at = (name: string) => passes.indexOf(name);
  const again = passes.filter((name) => name.startsWith("again/"));
  expect(again.map((name) => name.slice(0, name.lastIndexOf("/"))).filter((s, i, all) => all.indexOf(s) === i)).toEqual([
    "again/names",
    "again/in place",
    "again/chains",
  ]);
  expect(at("scopes/returns through finally")).toBe(at(again[0]) - 1);
  expect(at("finish/temporaries named")).toBe(at(again.at(-1)!) + 1);
  const dir = fixture("tidy-again");
  const after = (pass?: string) => {
    const out = join(dir, `${pass?.replaceAll(/[/ ]/g, "_") ?? "all"}.js`);
    run([compiler, join(root, "test/corpus/drop_scopes.rs"), "-o", out, ...(pass ? ["--tidy-until", pass] : []), "--", "-Awarnings"]);
    // Without its map's name, which is its file's.
    return readFileSync(out, "utf8").replace(/\/\/# sourceMappingURL=.*/, "");
  };
  const scoped = after("scopes/returns through finally");
  const read = after(again.at(-1));
  expect(read).not.toBe(scoped);
  expect(read).not.toBe(after());
});
