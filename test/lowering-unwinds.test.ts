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
    match js::get(value, key).map(js::classify) {
        Some(Kind::String(text)) => Some(text.to_string()),
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
        match js::classify(value) {
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
    match value.map(js::classify) {
        Some(Kind::Object(object)) => text_at(object, "url").unwrap_or_default(),
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
  // panic's form, is a panic, whoever threw it.
  expect([lib.fromJs(2), lib.fromJs(-1), lib.fromJs(0)]).toEqual(["4", "RangeError: negative", "a panic, zero"]);
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
  expect(js).toContain(`  } catch (thrown) {
    const payload = $unwound(thrown);
    throw $unwinding(payload);
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
