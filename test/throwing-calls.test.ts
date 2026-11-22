// A throwing JS call's `Result`: `.unwrap()` of one is the call itself
// (ADR 0643), and of each branch's (ADR 0747), and a function whose result
// is one is a throwing function (ADR 0644). Each test is a program of its
// own, compiled and run. Split from test/js-builtins.test.ts by concept.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

import { performanceTrack } from "./ports-of";
import { buildReact, compiler, fixture, run, target } from "./support";
import { underLoad } from "../scripts/timing";

beforeAll(buildReact, 600_000);

// ADR 0643: `.unwrap()` of a throwing JS call is the call itself, its throw
// what JS threw: `JSON.parse(text)`, `"" + value`, `await p`. An
// `.expect(msg)` keeps its message, `$unwrapOk`'s; and a `Result` held,
// unwrapped later, is `$try`'s.
test("unwrap of a throwing JS call is the call", async () => {
  const withJs = ["--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("unwrap-throwing");
  writeFileSync(join(dir, "lib.rs"), `use js::{Promise, Unknown, json};
pub fn parsed(text: &str) -> Option<&'static Unknown> {
    json::parse(text).unwrap()
}
pub fn coerced(value: Option<&Unknown>) -> String {
    js::concat_text(value).unwrap()
}
pub async fn awaited(p: Promise<f64>) -> f64 {
    js::settle(p).await.unwrap()
}
pub fn expected(text: &str) -> Option<&'static Unknown> {
    json::parse(text).expect("valid JSON")
}
pub fn held(text: &str, first: bool) -> Option<&'static Unknown> {
    let parsed = json::parse(text);
    if first {
        println!("parsing");
    }
    parsed.unwrap()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("function parsed(text) {\n  return JSON.parse(text);\n}");
  expect(js).toContain('function coerced(value) {\n  return "" + value;\n}');
  expect(js).toContain("async function awaited(p) {\n  return await p;\n}");
  expect(js).toContain('$unwrapOk(\n    $try(() => JSON.parse(text)),\n    "lib.rs:12:23",\n    "valid JSON",');
  expect(js).toContain("const parsed = $try(() => JSON.parse(text));");
  const lib = await import(join(dir, "lib.js"));
  expect(lib.parsed("[1]")).toEqual([1]);
  expect(() => lib.parsed("{")).toThrow(SyntaxError);
  expect(() => lib.coerced(Symbol("s"))).toThrow(TypeError);
  expect(lib.coerced(1)).toBe("1");
  expect(await lib.awaited(Promise.resolve(2))).toBe(2);
  expect(lib.awaited(Promise.reject(new RangeError("no")))).rejects.toThrow(RangeError);
  expect(() => lib.expected("{")).toThrow("valid JSON: SyntaxError");
  expect(() => lib.held("{", false)).toThrow("called `Result::unwrap()` on an `Err` value: SyntaxError");
});

// ADR 0747: a throwing call's `Result` made in each branch of an `if`, and
// unwrapped where it's made, is each call itself, as React's ported
// addValueToProperties' `if did_truncate { stringify(..) } else {
// stringify(value) }.unwrap()` is. A branch that isn't a
// throwing call, or one that runs something past its call, keeps `$try`'s.
test("unwrap of a throwing call each branch makes is each call", async () => {
  const withJs = ["--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("unwrap-branches");
  writeFileSync(join(dir, "lib.rs"), `use js::{Unknown, json};
pub fn either(text: &str, other: &str, first: bool) -> Option<&'static Unknown> {
    if first { json::parse(text) } else { json::parse(other) }.unwrap()
}
pub fn shown(value: &Unknown, short: bool) -> Option<String> {
    if short {
        json::stringify(js::unknown("…"))
    } else {
        json::stringify(value)
    }
    .unwrap()
}
pub fn or_none(text: &str, first: bool) -> Option<&'static Unknown> {
    if first { json::parse(text) } else { Ok(None) }.unwrap()
}
fn nothing() -> Result<Option<&'static Unknown>, &'static js::JsError> {
    Ok(None)
}
pub fn or_nothing(text: &str, first: bool) -> Option<&'static Unknown> {
    if first { json::parse(text) } else { nothing() }.unwrap()
}
pub fn logged(text: &str, first: bool) -> Option<&'static Unknown> {
    if first {
        let parsed = json::parse(text);
        println!("parsed");
        parsed
    } else {
        json::parse("null")
    }
    .unwrap()
}
struct Loud;
impl Drop for Loud {
    fn drop(&mut self) {
        println!("dropped");
    }
}
impl Loud {
    fn text(&self) -> &'static str {
        "{"
    }
}
pub fn dropped(first: bool) -> Option<&'static Unknown> {
    if first { json::parse(Loud.text()) } else { json::parse("null") }.unwrap()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("return first ? JSON.parse(text) : JSON.parse(other);");
  expect(js).toContain('return (short ? JSON.stringify("…") : JSON.stringify(value)) ?? undefined;');
  expect(js.match(/\$unwrapOk\(/g)?.length).toBe(4);
  const lib = await import(join(dir, "lib.js"));
  expect([lib.either("[1]", "{", true), lib.either("{", "2", false)]).toEqual([[1], 2]);
  expect(() => lib.either("{", "2", true)).toThrow(SyntaxError);
  expect(lib.shown(() => {}, false)).toBeUndefined();
  expect(lib.shown(1n, true)).toBe('"…"');
  expect(() => lib.shown(1n, false)).toThrow(TypeError);
  expect(() => lib.orNone("{", true)).toThrow("called `Result::unwrap()` on an `Err` value: SyntaxError");
  expect(lib.orNone("{", false)).toBeUndefined();
  expect(() => lib.orNothing("{", true)).toThrow("called `Result::unwrap()` on an `Err` value: SyntaxError");
  expect(lib.orNothing("{", false)).toBeUndefined();
  expect(() => lib.logged("{", true)).toThrow("called `Result::unwrap()` on an `Err` value: SyntaxError");
  // What a branch drops runs past its call: the call's throw is caught.
  expect(() => lib.dropped(true)).toThrow("called `Result::unwrap()` on an `Err` value: SyntaxError");
  // React's: what may be `undefined` keeps its `unwrap()`, `$unwrap`.
  const described = performanceTrack()("addValueToProperties");
  expect(described).toContain("desc = $unwrap(\n              didTruncate\n                ? JSON.stringify(");
  expect(described).toContain("                : JSON.stringify(value),\n");
  expect(described).not.toContain("$try");
}, underLoad(60_000));

// ADR 0644: a function whose result is a throwing call's `Result`, each
// return, is a throwing function, as React's `testStringCoercion` is: its
// JS returns the call's value, and its calls are throwing calls, a `try` of
// `is_err()`, the call of `unwrap()`, `$try` of `?`. A `pub` one keeps its
// `Result`, which JS reads; one that returns its own call keeps `$try`.
test("a function whose result is a throwing call's is a throwing function", async () => {
  const withJs = ["--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("throwing-functions");
  writeFileSync(join(dir, "lib.rs"), `use js::{JsError, Unknown, json};

fn test_string_coercion(value: Option<&Unknown>) -> Result<String, &'static JsError> {
    // For whoever follows the stack here.
    js::concat_text(value)
}

fn will_coercion_throw(value: Option<&Unknown>) -> bool {
    test_string_coercion(value).is_err()
}

pub fn check_key_string_coercion(value: Option<&Unknown>) -> Option<String> {
    if will_coercion_throw(value) {
        eprintln!("unsupported");
        return Some(test_string_coercion(value).unwrap());
    }
    None
}

fn parsed(text: &str) -> Result<Option<&'static Unknown>, &'static JsError> {
    json::parse(text)
}

pub fn valid(text: &str) -> bool {
    match parsed(text) {
        Ok(_) => true,
        Err(_) => false,
    }
}

pub fn present(text: &str) -> Result<bool, &'static JsError> {
    let value = parsed(text)?;
    Ok(value.is_some())
}

pub fn coerced(value: Option<&Unknown>) -> Result<String, &'static JsError> {
    js::concat_text(value)
}

fn nested(value: Option<&Unknown>, depth: u32) -> Result<String, &'static JsError> {
    if depth == 0 {
        js::concat_text(value)
    } else {
        nested(value, depth - 1)
    }
}

pub fn deep(value: Option<&Unknown>) -> bool {
    nested(value, 2).is_err()
}

fn or_none(text: &str) -> Result<Option<&'static Unknown>, &'static JsError> {
    if text.is_empty() {
        return Ok(None);
    }
    json::parse(text)
}

pub fn parses(text: &str) -> bool {
    or_none(text).is_ok()
}

fn number(text: &str) -> Result<u32, std::num::ParseIntError> {
    text.parse()
}

pub fn numeric(text: &str) -> bool {
    number(text).is_ok()
}

fn read(text: &str) -> Result<Option<&'static Unknown>, &'static JsError> {
    json::parse(text)
}

fn through(f: fn(&str) -> Result<Option<&'static Unknown>, &'static JsError>, text: &str) -> bool {
    f(text).is_ok()
}

pub fn as_value(text: &str) -> bool {
    through(read, text)
}

// One whose result is another throwing function's call, which the call
// graph finds first, its callee (#1671 change 4 M3).
fn relayed(text: &str) -> Result<Option<&'static Unknown>, &'static JsError> {
    parsed(text)
}

pub fn relays(text: &str) -> bool {
    relayed(text).is_ok()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('function testStringCoercion(value) {\n  // For whoever follows the stack here.\n  return "" + value;\n}');
  expect(js).toContain("function willCoercionThrow(value) {\n  try {\n    testStringCoercion(value);\n    return false;\n  } catch {\n    return true;\n  }\n}");
  expect(js).toContain("    return testStringCoercion(value);\n");
  expect(js).toContain("function valid(text) {\n  try {\n    parsed(text);\n    return true;\n  } catch {\n    return false;\n  }\n}");
  expect(js).toContain("$try(() => parsed(text))");
  expect(js).toContain("function relayed(text) {\n  return parsed(text);\n}");
  expect(js).toContain('function coerced(value) {\n  return $try(() => "" + value);\n}');
  expect(js).toContain('$try(() => "" + value)');
  const lib = await import(join(dir, "lib.js"));
  expect([lib.checkKeyStringCoercion("a"), lib.checkKeyStringCoercion(1)]).toEqual([undefined, undefined]);
  expect(() => lib.checkKeyStringCoercion(Symbol("s"))).toThrow(TypeError);
  expect([lib.valid("[1]"), lib.valid("{")]).toEqual([true, false]);
  expect([lib.present("1"), lib.present("null")]).toEqual([{ TAG: "Ok", _0: true }, { TAG: "Ok", _0: false }]);
  expect(lib.present("{").TAG).toBe("Err");
  expect([lib.coerced(1), lib.coerced(Symbol("s")).TAG]).toEqual([{ TAG: "Ok", _0: "1" }, "Err"]);
  expect([lib.deep(1), lib.deep(Symbol("s"))]).toEqual([false, true]);
  // One that makes its own \`Ok\` too keeps its \`Result\`.
  expect(js).toMatch(/function orNone\(text\) \{\n(?:(?!\n\}\n)[\s\S])*\n  return \$try\(\(\) => JSON\.parse\(text\)\);\n\}/);
  expect([lib.parses(""), lib.parses("1"), lib.parses("{")]).toEqual([true, true, false]);
  // One of a Rust \`Result\`, no JS call's, is no throwing function.
  expect([lib.numeric("12"), lib.numeric("x")]).toEqual([true, false]);
  // Nor one named as a value, whose callers read its \`Result\` (#1670).
  expect(js).toContain("function read(text) {\n  return $try(() => JSON.parse(text));\n}");
  expect([lib.asValue("[1]"), lib.asValue("{")]).toEqual([true, false]);
});

// #1675: a namespace's method that's another module's function (#566), its
// JS that function, re-exported by the method's name, reaches it as the
// method is reached: by a call of the method, through it as a value and by
// JS, each of which reads its `Result`. So the function is no throwing
// function, though its one call the crate's MIR makes is the method's
// body's: it gives its `Result`, as Rust's does.
test("a function a namespace's method is is reached as the method is", async () => {
  const withJs = ["--", "--crate-type", "lib", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("throwing-forwarded");
  writeFileSync(join(dir, "lib.rs"), `mod inner {
    use js::{JsError, Unknown, json};

    pub(crate) fn read(text: &str) -> Result<Option<&'static Unknown>, &'static JsError> {
        json::parse(text)
    }
}

use js::{JsError, Unknown};

#[cfg_attr(rust_js, rust_js::namespace)]
pub struct Parsing;

#[cfg_attr(rust_js, rust_js::namespace)]
impl Parsing {
    pub fn read(&self, text: &str) -> Result<Option<&'static Unknown>, &'static JsError> {
        inner::read(text)
    }
}

pub fn parsing() -> &'static Parsing {
    &Parsing
}

pub fn direct(text: &str) -> bool {
    Parsing.read(text).is_ok()
}

pub fn through(text: &str) -> bool {
    let f: fn(&Parsing, &str) -> Result<Option<&'static Unknown>, &'static JsError> = Parsing::read;
    f(&Parsing, text).is_ok()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs]);
  expect(readFileSync(join(dir, "lib.js"), "utf8")).toContain('export { read } from "./inner.js";');
  expect(readFileSync(join(dir, "inner.js"), "utf8")).toContain("return $try(() => JSON.parse(text));");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.direct("[1]"), lib.direct("{"), lib.through("[1]"), lib.through("{")]).toEqual([true, false, true, false]);
  expect(lib.parsing().read("[1]")).toEqual({ TAG: "Ok", _0: [1] });
  expect(lib.parsing().read("{").TAG).toBe("Err");
});

// #2280: a namespace's method that's another module's function is
// re-exported by its name only where the method is exported, as a plain
// function is: a crate-private one's isn't, whose function stays only its
// crate's to call, so a JS importer never reaches its throwing form, where
// the method's contract is a `Result`. A `pub` one's is, its function its
// `Result` form.
test("a crate-private namespace's method that's a function isn't re-exported", async () => {
  const withJs = ["--", "--crate-type", "lib", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("throwing-forwarded-private");
  writeFileSync(join(dir, "lib.rs"), `mod inner {
    use js::{JsError, Unknown, json};

    pub(crate) fn read(text: &str) -> Result<Option<&'static Unknown>, &'static JsError> {
        json::parse(text)
    }

    pub(crate) fn load(text: &str) -> Result<Option<&'static Unknown>, &'static JsError> {
        json::parse(text)
    }
}

use js::{JsError, Unknown};

#[cfg_attr(rust_js, rust_js::namespace)]
pub(crate) struct Parsing;

#[cfg_attr(rust_js, rust_js::namespace)]
impl Parsing {
    pub(crate) fn read(&self, text: &str) -> Result<Option<&'static Unknown>, &'static JsError> {
        inner::read(text)
    }
}

#[cfg_attr(rust_js, rust_js::namespace)]
pub struct Loading;

#[cfg_attr(rust_js, rust_js::namespace)]
impl Loading {
    pub fn load(&self, text: &str) -> Result<Option<&'static Unknown>, &'static JsError> {
        inner::load(text)
    }
}

pub fn read_ok(text: &str) -> bool {
    Parsing.read(text).is_ok()
}

pub fn load_ok(text: &str) -> bool {
    Loading.load(text).is_ok()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).not.toContain("export { read }");
  expect(js).toContain('export { load } from "./inner.js";');
  expect(readFileSync(join(dir, "inner.js"), "utf8")).toContain("return $try(() => JSON.parse(text));");
  const lib = await import(join(dir, "lib.js"));
  expect(lib.read).toBeUndefined();
  expect([lib.readOk("[1]"), lib.readOk("{")]).toEqual([true, false]);
  expect([lib.loadOk("[1]"), lib.loadOk("{")]).toEqual([true, false]);
  expect(lib.load("[1]")).toEqual({ TAG: "Ok", _0: [1] });
  expect(lib.load("{").TAG).toBe("Err");
});
