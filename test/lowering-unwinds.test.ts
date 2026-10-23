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
// catch (thrown) { .. }`, its `let` the shadow its arm sets again.
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
  } catch (thrown) {
    let x = $unwound(thrown);`);
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
  expect(js).toContain("const _restore = { previous, dispatcher: previous };");
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
  expect(js).toContain(`export function control() {
  let control;
  try {
    throw new Error();
  } catch (x) {
    control = x;
  }
  return control?.stack;
}`);
  expect(js).toContain(`  } catch (x) {
    control = x;
  }`);
  expect(js).toContain(`  try {
    return globalThis.render();
  } catch (payload) {
    throw payload;
  }`);
  // Read otherwise, the payload is kept, and thrown again as it was.
  expect(js).toContain("    throw thrown;\n");
  // What a call throws may be any value: its downcast's test is kept.
  expect(js).toContain('payload.impl.typeId() === "&js::JsError" ? payload.value : payload.error');
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
