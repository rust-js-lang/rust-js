// The js crate's builtins: JS's errors, regular expressions, symbols,
// promises, Reflect and Proxy, atomics, Intl, typed arrays, dates and
// JSON, each JS's own. Each test is a program of its own, compiled and run.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

import { runSync } from "./child";
import { node } from "./programs";
import { buildReact, compiler, fixture, performanceTrack, root, run, target } from "./support";

beforeAll(buildReact, 600_000);

// ADR 0035: what a JS function threw is shown as JS shows it, `String(error)`,
// `SyntaxError: ..`, by `{:?}` and by an `expect`'s panic.
test("a JS error is shown as JS shows it", async () => {
  const withJs = ["--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("js-error");
  writeFileSync(join(dir, "lib.rs"), `use js::{Unknown, json};
pub fn parsed(text: &str) -> Option<&'static Unknown> {
    json::parse(text).unwrap()
}
pub fn kept(text: &str) -> Option<&'static Unknown> {
    let value = match json::parse(text) {
        Ok(value) => value,
        Err(_) => {
            eprintln!("invalid");
            None
        }
    };
    value
}
pub fn valid(text: &str) -> bool {
    match json::parse(text) {
        Ok(_) => true,
        Err(_) => false,
    }
}
pub fn told(text: &str) -> String {
    let told = match json::parse(text) {
        Ok(_) => String::new(),
        Err(error) => format!("{error:?}"),
    };
    told
}
pub fn kept_or_told(text: &str) -> Option<&'static Unknown> {
    let value = match json::parse(text) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("{error:?}");
            None
        }
    };
    value
}
pub fn shown(text: &str) -> String {
    match json::parse(text) {
        Ok(_) => "parsed".to_string(),
        Err(error) => format!("{error:?}"),
    }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js"), ...withJs]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toMatch(/return String\((match\._0|error)\);/);
  // A `match` of what it threw, whose `Ok` only keeps the value and
  // whose `Err` reads no error, is JS's `try` (ADR 0035).
  expect(js).toContain('  let value;\n  try {\n    value = JSON.parse(text);\n  } catch {\n    console.error("invalid");\n    value = undefined;\n  }');
  const lib = await import(join(dir, "mir.js"));
  expect(lib.parsed("[1]")).toEqual([1]);
  expect([lib.kept("[1]"), lib.kept("{")]).toEqual([[1], undefined]);
  // One whose `Err` reads the error keeps it, `$try`'s.
  expect([lib.keptOrTold("[1]"), lib.keptOrTold("{")]).toEqual([[1], undefined]);
  expect([lib.told("1"), lib.told("{").startsWith("SyntaxError")]).toEqual(["", true]);
  expect([lib.valid("[1]"), lib.valid("{")]).toEqual([true, false]);
  // Its `unwrap()` is the call, which throws what JS threw (ADR 0643).
  expect(() => lib.parsed("{")).toThrow(SyntaxError);
  expect([lib.shown("{").startsWith("SyntaxError: "), lib.shown("1")]).toEqual([true, "parsed"]);
});

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
  expect(js).toContain('$unwrapOk(\n    $try(() => JSON.parse(text)),\n    "valid JSON",');
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
// addValueToProperties' `if did_truncate { stringify_with(..) } else {
// stringify_with(value, ..) }.unwrap()` is. A branch that isn't a
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
        json::stringify_with(js::unknown("…"), None, 0)
    } else {
        json::stringify_with(value, None, 0)
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
  expect(js).toContain('return short ? JSON.stringify("…", null, 0) : JSON.stringify(value, null, 0);');
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
  expect(described).toContain("                : JSON.stringify(value, null, 0),\n");
  expect(described).not.toContain("$try");
});

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
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('function testStringCoercion(value) {\n  // For whoever follows the stack here.\n  return "" + value;\n}');
  expect(js).toContain("function willCoercionThrow(value) {\n  try {\n    testStringCoercion(value);\n    return false;\n  } catch {\n    return true;\n  }\n}");
  expect(js).toContain("    return testStringCoercion(value);\n");
  expect(js).toContain("function valid(text) {\n  try {\n    parsed(text);\n    return true;\n  } catch {\n    return false;\n  }\n}");
  expect(js).toContain("$try(() => parsed(text))");
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
  expect(js).toContain("  return $try(() => JSON.parse(text));\n}");
  expect([lib.parses(""), lib.parses("1"), lib.parses("{")]).toEqual([true, true, false]);
  // One of a Rust \`Result\`, no JS call's, is no throwing function.
  expect([lib.numeric("12"), lib.numeric("x")]).toEqual([true, false]);
});

// ADR 0283: a regular expression's match, an `Error`'s parts, symbols, the
// global functions and the weak collections are the js crate's, each a
// method or a function of the JS it names; a weak key is an object.
test("RegExp, Error, Symbol, weak collections and globals are JS's", async () => {
  const withJs = ["--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("js-objects");
  writeFileSync(join(dir, "lib.rs"), `use js::{WeakKey, dict, error, parse_int, reg_exp, symbol, weak_map, weak_ref};
pub fn matched(text: &str) -> Option<(String, u32, String, bool)> {
    let pattern = reg_exp::new(r"(?<word>[a-z]+)(\\d)", "g");
    let m = pattern.exec(text)?;
    let word = m.groups().and_then(|groups| dict::get(groups, "word").cloned()).unwrap_or_default();
    Some((m.get(2).unwrap_or_default(), m.index(), word + &m.full_match(), pattern.global()))
}
pub fn errors() -> (String, String, f64, f64) {
    let e = error::new("boom");
    (e.name(), e.message(), parse_int("42px", Some(10)), parse_int("0x1f", None))
}
pub fn symbols() -> (bool, Option<String>, Option<String>) {
    let local = symbol::new("local");
    (js::object::is(local, local), symbol::key_for(local), local.description())
}
pub struct State {
    pub n: u32,
}
unsafe impl WeakKey for State {}
pub fn weak(state: &'static State) -> (Option<u32>, bool) {
    let seen = weak_map::new::<State, u32>();
    seen.set(state, state.n);
    let r = weak_ref::new(state);
    (seen.get(state), r.deref().is_some())
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("const m = pattern.exec(text);");
  expect(js).toContain('const e = new Error("boom");');
  // A radix given or not, as TypeScript's `radix?`: `parseInt(text)` reads `0x` as hex.
  expect(js).toContain('parseInt("42px", 10), parseInt("0x1f")]');
  expect(js).toContain('const local = Symbol("local");');
  expect(js).toContain("const seen = new WeakMap();\n  seen.set(state, state.n);\n  const r = new WeakRef(state);");
  const lib = await import(join(dir, "lib.js"));
  expect(lib.matched("ab cd7")).toEqual(["7", 3, "cdcd7", true]);
  expect(lib.matched("none")).toBe(undefined);
  expect(lib.errors()).toEqual(["Error", "boom", 42, 31]);
  expect(lib.symbols()).toEqual([true, undefined, "local"]);
  expect(lib.weak({ n: 5 })).toEqual([5, true]);
});

// React's ReactCapturedValue: `createCapturedValueFromError(value: Error,
// ..)` takes an `Error` object, `js::Error`, which declares `Error`, as an
// `AggregateError` declares its own; what's thrown, a `JsError`, is
// anything, `any`. An `Error` is what may be thrown, by `Deref`.
test("an Error object is declared Error, and what's thrown any", async () => {
  const withJs = ["--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("js-error-objects");
  writeFileSync(join(dir, "lib.rs"), `use js::{AggregateError, Error, JsError, WeakKey, WeakMap, aggregate_error, error};

thread_local! {
    static STACKS: &'static WeakMap<dyn WeakKey, String> = js::weak_map::new();
}

pub fn captured(value: &'static Error, stack: &str) -> String {
    STACKS.with(|stacks| stacks.set(value, stack.to_string()));
    value.message()
}

pub fn thrown(value: &JsError) -> String {
    value.name()
}

pub fn made(message: &str) -> &'static Error {
    error::new(message)
}

pub fn all() -> &'static AggregateError {
    aggregate_error::new(&[], Some("every one"))
}

pub fn named() -> String {
    let error = made("boom");
    format!("{} {} {}", thrown(error), captured(error, "at"), all().message())
}
`);
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs]);
  const declarations = readFileSync(join(dir, "lib.d.ts"), "utf8");
  expect(declarations).toContain("export function captured(value: Error, stack: string): string;");
  expect(declarations).toContain("export function thrown(value: any): string;");
  expect(declarations).toContain("export function made(message: string): Error;");
  expect(declarations).toContain("export function all(): AggregateError;");
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('  const error = made("boom");');
  const lib = await import(join(dir, "lib.js"));
  expect(lib.named()).toBe("Error boom every one");
  expect(lib.made("x")).toBeInstanceOf(Error);
});

// ADR 0677: a weak collection is typed as TypeScript types it,
// `WeakMap<K, V>`, and what `js::Kind` holds of an object, an array, a
// function or a symbol is a key: `Kind::Object`'s a `&JsObject`, TS's
// `object`, which reads as the `Unknown` it is. React's `jsxPropsParents`
// (shared/ReactSerializationErrors.js), with no cast and no `const key`.
test("a weak map is keyed by what js::Kind holds", async () => {
  const dir = fixture("weak-keys");
  writeFileSync(join(dir, "lib.rs"), `use js::{JsObject, Kind, Unknown, WeakKey, WeakMap};

thread_local! {
    pub static PROPS_PARENTS: &'static WeakMap<JsObject, &'static Unknown> = js::weak_map::new();
    pub static CHILDREN_PARENTS: &'static WeakMap<[Option<&'static Unknown>], &'static Unknown> = js::weak_map::new();
    pub static NAMED: &'static WeakMap<dyn WeakKey, &'static str> = js::weak_map::new();
}

pub fn remember(value: Option<&'static Unknown>, parent: &'static Unknown) -> bool {
    match js::classify(value) {
        Kind::Object(props) => PROPS_PARENTS.with(|parents| parents.set(props, parent)),
        Kind::Array(children) => CHILDREN_PARENTS.with(|parents| parents.set(children, parent)),
        _ => return false,
    }
    true
}

pub fn parent(value: Option<&'static Unknown>) -> Option<&'static Unknown> {
    match js::classify(value) {
        Kind::Object(props) => PROPS_PARENTS.with(|parents| parents.get(props)),
        Kind::Array(children) => CHILDREN_PARENTS.with(|parents| parents.get(children)),
        _ => None,
    }
}

pub fn name(value: Option<&'static Unknown>) -> String {
    match js::classify(value) {
        Kind::Object(object) => {
            NAMED.with(|named| named.set(object, "object"));
            js::string(js::get(object, "name"))
        }
        Kind::Symbol(symbol) => {
            NAMED.with(|named| named.set(symbol, "symbol"));
            symbol.description().unwrap_or_default()
        }
        _ => String::new(),
    }
}

pub fn function_seen(value: Option<&'static Unknown>, seen: &'static WeakMap<dyn Fn() -> Option<&'static Unknown>, bool>) -> bool {
    match js::classify(value) {
        Kind::Function(function) => {
            let before = seen.has(function);
            seen.set(function, true);
            before
        }
        _ => false,
    }
}
`);
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("export const propsParents = new WeakMap();");
  expect(js).toMatch(/propsParents\.set\(value, parent\)/);
  expect(js).toMatch(/childrenParents\.get\(value\)/);
  expect(js).not.toContain("const key");
  const declarations = readFileSync(join(dir, "lib.d.ts"), "utf8");
  expect(declarations).toContain("export const propsParents: WeakMap<object, unknown>;");
  expect(declarations).toContain("export const childrenParents: WeakMap<(unknown | null | undefined)[], unknown>;");
  expect(declarations).toContain("export const named: WeakMap<WeakKey, string>;");
  writeFileSync(join(dir, "run.mjs"), `import * as lib from "./lib.js";
const props = { name: "props" };
const children = ["a"];
const parent = { type: "div" };
const s = Symbol("s");
const f = () => 1;
const seen = new WeakMap();
console.log(JSON.stringify([lib.remember(props, parent), lib.remember(children, parent), lib.remember("text", parent)]));
console.log(JSON.stringify([lib.parent(props) === parent, lib.parent(children) === parent, lib.parent({}), lib.parent(null)]));
console.log(JSON.stringify([lib.name(props), lib.name(s), lib.name(1), lib.named.get(props), lib.named.get(s)]));
console.log(JSON.stringify([lib.functionSeen(f, seen), lib.functionSeen(f, seen), lib.functionSeen(props, seen)]));
`);
  const ran = runSync([node!, join(dir, "run.mjs")], root, 30_000);
  expect(ran.stderr).toBe("");
  expect(ran.stdout).toBe(
    [
      `[true,true,false]`,
      `[true,true,null,null]`,
      `["props","s","","object","symbol"]`,
      `[false,true,false]`,
      "",
    ].join("\n"),
  );
});

// ADR 0283: JS's Promise's statics and methods are the js crate's, as
// ReScript's Stdlib has them; `allSettled`'s results a union tagged by
// `status` (ADR 0284).
test("a Promise's statics and methods are JS's", async () => {
  const withJs = ["--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("js-promises");
  writeFileSync(join(dir, "lib.rs"), `use js::{PromiseSettledResult, PromiseWithResolvers, error, promise};
pub async fn made() -> u32 {
    promise::new(|resolve, _| resolve(1)).then(|n: u32| promise::resolve(n + 1)).await
}
pub async fn caught() -> String {
    promise::reject::<String>(Some(error::new("boom"))).catch(|e| promise::resolve(e.message())).finally(|| {}).await
}
pub async fn paired() -> (u32, String) {
    promise::all2((promise::resolve(1), promise::resolve("a".to_string()))).await
}
pub async fn settled() -> Vec<String> {
    let all = promise::all_settled(vec![promise::resolve(1), promise::reject(Some(error::new("no")))]).await;
    all.into_iter()
        .map(|r| match r {
            PromiseSettledResult::Fulfilled { value } => value.to_string(),
            PromiseSettledResult::Rejected { reason } => reason.message(),
        })
        .collect()
}
pub async fn resolvers() -> u32 {
    let PromiseWithResolvers { promise, resolve, .. } = promise::with_resolvers::<u32>();
    resolve(7);
    promise::race(vec![promise, promise::any(vec![promise::resolve(8)])]).await
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("return await new Promise((resolve) => {\n    resolve(1);\n  }).then(");
  expect(js).toContain('Promise.reject(new Error("boom"))\n    .catch((e) => Promise.resolve(e.message))');
  expect(js).toContain('Promise.all([Promise.resolve(1), Promise.resolve("a")])');
  expect(js).toContain('r.status === "fulfilled"');
  const lib = await import(join(dir, "lib.js"));
  expect(await lib.made()).toBe(2);
  expect(await lib.caught()).toBe("boom");
  expect(await lib.paired()).toEqual([1, "a"]);
  expect(await lib.settled()).toEqual(["1", "no"]);
  expect(await lib.resolvers()).toBe(7);
});

// ADR 0283: Reflect's functions and Proxy's traps are the js crate's, of a JS
// value of any shape.
test("Reflect and Proxy are JS's", async () => {
  const withJs = ["--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("js-reflect");
  writeFileSync(join(dir, "lib.rs"), `use js::{PropertyDescriptor, ProxyHandler, Unknown, proxy, reflect, string, unknown};
pub fn reflected(o: &Unknown) -> (bool, String, usize, bool, bool) {
    let d = PropertyDescriptor { value: Some(&"fixed"), writable: Some(false), ..Default::default() };
    let defined = reflect::define_property(o, "id", &d);
    let written = reflect::set(o, "id", "other", None);
    let own = reflect::get_own_property_descriptor(o, "id").and_then(|d| d.writable) == Some(false);
    (defined, string(reflect::get(o, "id", None)), reflect::own_keys(o).len(), written, own || !reflect::set_prototype_of(o, None))
}
pub fn proxied(o: &'static Unknown) -> (String, String, bool) {
    let handler = ProxyHandler {
        get: Some(Box::new(|target, key, _| match reflect::get(target, key, None) {
            Some(value) => Some(value),
            None => Some(unknown("missing")),
        })),
        ..Default::default()
    };
    let p = proxy::revocable(o, &handler);
    let seen = (string(reflect::get(p.proxy(), "a", None)), string(reflect::get(p.proxy(), "b", None)));
    p.revoke();
    (seen.0, seen.1, reflect::get_prototype_of(o).is_none())
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  // A descriptor and a handler are dictionaries (ADR 0666).
  expect(js).toContain('const d = { value: "fixed", writable: false };');
  expect(js).toContain("const handler = {\n    get(target, key) {");
  expect(js).toContain("Reflect.setPrototypeOf(o, null)");
  expect(js).toContain("const p = Proxy.revocable(o, handler);");
  const lib = await import(join(dir, "lib.js"));
  expect(lib.reflected({ a: 1 })).toEqual([true, "fixed", 2, false, true]);
  expect(lib.proxied(Object.assign(Object.create(null), { a: 1 }))).toEqual(["1", "missing", true]);
});

// ADR 0283: Atomics' functions take an integer typed array, its element a
// Rust number of its kind, and one that a thread may wait on a
// SharedArrayBuffer's.
test("Atomics are JS's, of a view of a shared buffer", async () => {
  const withJs = ["--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("js-atomics");
  writeFileSync(join(dir, "lib.rs"), `use js::{WaitAsyncValue, WaitResult, atomics, big_int64_array, int32_array, shared_array_buffer};
pub fn counted() -> (i32, i32, i32, i64, bool, bool) {
    let shared = shared_array_buffer::new(16.0);
    let counts = int32_array::new_with_buffer(shared, None, None);
    atomics::add(counts, 0, 5);
    let before = atomics::sub(counts, 0, 2);
    atomics::compare_exchange(counts, 1, 0, 9);
    let wide = big_int64_array::new(1.0);
    atomics::store(wide, 0, 7);
    let waited = matches!(atomics::wait(counts, 0, 0, None), WaitResult::NotEqual);
    (before, atomics::load(counts, 0), atomics::load(counts, 1), atomics::or(wide, 0, 8), waited, counts.buffer().growable())
}
pub async fn waited() -> bool {
    let counts = int32_array::new_with_buffer(shared_array_buffer::new(4.0), None, None);
    match atomics::wait_async(counts, 0, 0, Some(1.0)).value {
        WaitAsyncValue::Waiting(promise) => matches!(promise.await, WaitResult::TimedOut),
        WaitAsyncValue::Ended(_) => false,
    }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("const counts = new Int32Array(shared);");
  expect(js).toContain("Atomics.compareExchange(counts, 1, 0, 9);");
  expect(js).toContain('Atomics.wait(counts, 0, 0) === "not-equal"');
  const lib = await import(join(dir, "lib.js"));
  expect(lib.counted()).toEqual([5, 3, 9, 7n, true, false]);
  expect(await lib.waited()).toBe(true);
});

// ADR 0283: Intl's formatters are the js crate's, their options structs whose
// `None` isn't given, each string union an enum.
test("Intl's formatters are JS's, their options typed", async () => {
  const withJs = ["--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("js-intl");
  writeFileSync(join(dir, "lib.rs"), `use js::date;
use js::intl::{self, *};
pub fn numbers() -> (String, String, bool, Vec<String>, String) {
    let price = number_format::new(&["en-US"], &NumberFormatOptions { style: Some(NumberStyle::Currency), currency: Some("USD"), ..Default::default() });
    let plain = number_format::new(&["en-US"], &NumberFormatOptions { use_grouping: Some(UseGrouping::Bool(false)), ..Default::default() });
    let options = plain.resolved_options();
    let parts = price.format_to_parts(-1.5).into_iter().filter(|p| p.r#type != NumberFormatPartType::Literal).map(|p| p.value).collect();
    (price.format(1234.5), plain.format(1234.0), options.use_grouping == UseGrouping::Bool(false), parts, price.format_range(3.0, 5.0))
}
pub fn dates() -> (String, String, Vec<String>) {
    let d = date::new(Some(&0.0));
    let utc = DateTimeFormatOptions { time_zone: Some("UTC"), year: Some(NumericStyle::Numeric), month: Some(MonthStyle::Long), day: Some(NumericStyle::Numeric), ..Default::default() };
    let format = date_time_format::new(&["en-US"], &utc);
    let kinds = format.format_to_parts(d).into_iter().filter(|p| p.r#type != DateTimeFormatPartType::Literal).map(|p| p.value).collect();
    (format.format(d), d.to_locale_date_string(Some(&["en-GB"]), Some(&utc)), kinds)
}
pub fn words() -> (Vec<String>, PluralCategory, String, String, Option<String>, usize) {
    let collator = collator::new(&["en"], &CollatorOptions { numeric: Some(true), ..Default::default() });
    let mut files = vec!["file10", "file2", "File1"];
    files.sort_by(|a, b| collator.compare(a, b));
    let ordinal = plural_rules::new(&["en-US"], &PluralRulesOptions { r#type: Some(PluralRuleType::Ordinal), ..Default::default() });
    let ago = relative_time_format::new(&["en"], &RelativeTimeFormatOptions { numeric: Some(RelativeTimeNumeric::Auto), ..Default::default() });
    let list = list_format::new(&["en"], &ListFormatOptions { r#type: Some(ListFormatType::Disjunction), ..Default::default() });
    let names = display_names::new(&["en"], &DisplayNamesOptions { r#type: DisplayNamesType::Region, locale_matcher: None, style: None, language_display: None, fallback: None });
    let words = segmenter::new(&["en"], &SegmenterOptions { granularity: Some(Granularity::Word), ..Default::default() });
    let count = segments::iter(words.segment("Hello, big world")).filter(|s| s.is_word_like == Some(true)).count();
    (files.iter().map(|f| f.to_string()).collect(), ordinal.select(2.0), ago.format(-1.0, RelativeTimeUnit::Day), list.format(&["a", "b"]), names.of("US"), count)
}
pub fn locales() -> (String, bool, Vec<String>, bool) {
    let maximal = locale::new("en", &LocaleOptions::default()).map(|l| l.maximize().to_string()).unwrap_or_default();
    let canonical = intl::get_canonical_locales(&["EN-us"]).unwrap_or_default();
    (maximal, locale::new("not a tag!", &LocaleOptions::default()).is_err(), canonical, intl::supported_values_of(SupportedValuesKey::Currency).len() > 10)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('new Intl.NumberFormat(["en-US"], { style: "currency", currency: "USD"');
  expect(js).toContain('const format = new Intl.DateTimeFormat(["en-US"], utc);');
  const lib = await import(join(dir, "lib.js"));
  expect(lib.numbers()).toEqual(["$1,234.50", "1234", true, ["-", "$", "1", ".", "50"], "$3.00 – $5.00"]);
  expect(lib.dates()).toEqual(["January 1, 1970", "1 January 1970", ["January", "1", "1970"]]);
  expect(lib.words()).toEqual([["File1", "file2", "file10"], "two", "yesterday", "a or b", "United States", 3]);
  expect(lib.locales()).toEqual(["en-Latn-US", true, ["en-US"], true]);
});

// ADR 0283: JS's typed arrays are the js crate's, each element a Rust number
// of its kind, a `BigInt64Array`'s an `i64`, the `BigInt` rust-js makes one.
test("typed arrays and their buffers are JS's", async () => {
  const withJs = ["--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("js-typed-arrays");
  writeFileSync(join(dir, "lib.rs"), `use js::{array_buffer, big_int64_array, data_view, float64_array, int8_array, uint8_array};
pub fn floats() -> (f64, Option<f64>, u32, String) {
    let a = float64_array::from(&[1.5, -2.0, 3.25]);
    a.set_index(1.0, 4.0);
    let doubled = a.map(|x, _| x * 2.0);
    let sorted = a.to_sorted(Some(Box::new(|x, y| y.partial_cmp(&x).unwrap())));
    let shifted = sorted.map(|x, i| x + i as f64);
    (doubled.reduce(|sum, x, _| sum + x, 0.0), a.at(-1.0), float64_array::BYTES_PER_ELEMENT, shifted.slice(Some(1.0), None).join(Some("|")))
}
pub fn bytes() -> (u32, u32, i8, Vec<u8>) {
    let buffer = array_buffer::new(8.0);
    let view = data_view::new(buffer, None, None);
    view.set_int16(0.0, -2.0, true);
    let bytes = uint8_array::new_with_buffer(buffer, None, None);
    let signed = int8_array::of((1, -1, 127));
    (buffer.byte_length(), bytes.length(), signed.get(1.0).unwrap_or(0), bytes.subarray(Some(0.0), Some(2.0)).values().collect())
}
pub fn bigs() -> (i64, bool) {
    let a = big_int64_array::from(&[1, -2, 3]);
    (a.reduce(|sum, x, _| sum + x, 0), a.includes(-2, None))
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("const a = Float64Array.from([1.5, -2, 3.25]);\n  a[1] = 4;");
  // One callback form, given the index (ADR 0443): a `_` it doesn't read is
  // left out, as JS's callback leaves it.
  expect(js).toContain("const doubled = a.map((x) => x * 2);");
  expect(js).toContain("const shifted = sorted.map((x, i) => x + i);");
  expect(js).toContain("doubled.reduce((sum, x) => sum + x, 0)");
  expect(js).toContain("const signed = Int8Array.of(1, -1, 127);");
  expect(js).toContain("view.setInt16(0, -2, true);");
  const lib = await import(join(dir, "lib.js"));
  expect(lib.floats()).toEqual([17.5, 3.25, 8, "4.25|3.5"]);
  expect(lib.bytes()).toEqual([8, 8, -1, [254, 255]]);
  expect(lib.bigs()).toEqual([2n, true]);
});

// ADR 0741: TypeScript's `ArrayBufferView`, what each typed array and a
// `DataView` deref to, of its `buffer`, `byteOffset` and `byteLength`; and
// `ArrayBuffer.isView` of any value, as React's
// `shared/ReactPerformanceTrackProperties.js` asks it.
test("an ArrayBufferView is any typed array or DataView, and isView takes any value", async () => {
  const withJs = ["--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("js-array-buffer-view");
  writeFileSync(join(dir, "lib.rs"), `use js::{ArrayBufferView, Kind, Unknown, array_buffer, data_view, float64_array, uint8_array};

pub fn bytes_of(view: &ArrayBufferView) -> Vec<u8> {
    uint8_array::new_with_buffer(view.buffer(), Some(f64::from(view.byte_offset())), Some(f64::from(view.byte_length())))
        .values()
        .collect()
}

pub fn of_each() -> (Vec<u8>, Vec<u8>, Vec<u8>) {
    let data = data_view::new(array_buffer::new(4.0), Some(1.0), Some(2.0));
    data.set_uint8(0.0, 7.0);
    let bytes = uint8_array::of((1, 2, 3));
    (bytes_of(float64_array::of((1.0,))), bytes_of(data), bytes_of(bytes.subarray(Some(1.0), None)))
}

pub fn described(value: Option<&Unknown>) -> String {
    if let Some(value) = value
        && array_buffer::is_view(Some(value))
    {
        return match js::classify(js::get(value, "length")) {
            Kind::Number(length) => format!("a view of {length}"),
            _ => "a view".to_string(),
        };
    }
    String::new()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("new Uint8Array(view.buffer, view.byteOffset, view.byteLength)");
  expect(js).toContain("ArrayBuffer.isView(value)");
  const lib = await import(join(dir, "lib.js"));
  expect(lib.ofEach()).toEqual([[0, 0, 0, 0, 0, 0, 240, 63], [7, 0], [2, 3]]);
  expect(lib.described(new Uint8Array(3))).toBe("a view of 3");
  expect(lib.described(new DataView(new ArrayBuffer(1)))).toBe("a view");
  expect(lib.described([1])).toBe("");
  expect(lib.described("text")).toBe("");
  expect(lib.described(undefined)).toBe("");
});

// ADR 0718: `String.fromCharCode(...codes)`, a rest of TypeScript's
// `number`, `f64`, which takes any number JS has as one, and a typed array
// spread as its rest arguments, as a slice is: React's
// `shared/binaryToComparableString.js` as a port writes it.
test("String.fromCharCode takes its codes as rest arguments, a typed array spread", async () => {
  const withJs = ["--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("js-from-char-code");
  writeFileSync(join(dir, "lib.rs"), `use js::{ArrayBufferView, string, uint8_array, uint16_array};

// Turns a TypedArray or ArrayBuffer into a string that can be used for comparison
// in a Map to see if the bytes are the same.
js::export_default! {
    fn binary_to_comparable_string(view: &ArrayBufferView) -> String {
        string::from_char_code(uint8_array::new_with_buffer(
            view.buffer(),
            Some(f64::from(view.byte_offset())),
            Some(f64::from(view.byte_length())),
        ))
    }
}

pub fn written() -> (String, String, String, String) {
    let units: Vec<u16> = vec![0x52, 0x75];
    let wide = uint16_array::of((0x73, 0x74));
    (string::from_char_code((72, 105)), string::from_char_code(&units), string::from_char_code(wide), string::from_char_code(()))
}

pub fn of_view() -> String {
    binary_to_comparable_string(uint8_array::of((97, 98, 99)).subarray(Some(1.0), None))
}

pub fn largest(byte: u8) -> f64 {
    js::math::max((1, 2.5, byte))
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain(
    "export default function binaryToComparableString(view) {\n  return String.fromCharCode(...new Uint8Array(view.buffer, view.byteOffset, view.byteLength));\n}",
  );
  expect(js).toContain("String.fromCharCode(72, 105)");
  expect(js).toContain("String.fromCharCode(...units)");
  expect(js).toContain("String.fromCharCode(...wide)");
  expect(js).toContain("String.fromCharCode()");
  const lib = await import(join(dir, "lib.js"));
  expect(lib.written()).toEqual(["Hi", "Ru", "st", ""]);
  expect(lib.ofView()).toBe("bc");
  expect(js).toContain("return Math.max(1, 2.5, byte);");
  expect(lib.largest(3)).toBe(3);
  expect(lib.default(new Float32Array([1]))).toBe(String.fromCharCode(...new Uint8Array(new Float32Array([1]).buffer)));
});

// ADR 0283: JS's `Date` is the js crate's, a type with its members as
// methods and its constructors and statics a module's, each the JS it names.
test("a Date is JS's, its members methods", async () => {
  const withJs = ["--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("js-date");
  writeFileSync(join(dir, "lib.rs"), `use js::{Date, date};
pub fn moment(time: f64) -> (f64, f64, f64, String) {
    let d = date::new(Some(&time));
    d.set_utc_hours(12, None, None, None);
    d.set_utc_minutes(30, Some(15), None);
    (d.get_utc_full_year(), d.get_utc_month(), d.get_time(), d.to_iso_string().unwrap())
}
pub fn invalid() -> (bool, bool, Option<String>) {
    let d = date::new(Some(&"not a date"));
    (d.get_time().is_nan(), d.to_iso_string().is_err(), d.to_json())
}
pub fn utc() -> f64 {
    date::utc(2026, Some(9), Some(8), None, None, None, None)
}
pub fn now_is_recent(d: &Date) -> bool {
    date::now() - d.get_time() < 60_000.0
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("const d = new Date(time);\n  d.setUTCHours(12);\n  d.setUTCMinutes(30, 15);");
  expect(js).toContain("return Date.UTC(2026, 9, 8);");
  expect(js).toContain("return Date.now() - d.getTime() < 60000;");
  const lib = await import(join(dir, "lib.js"));
  expect(lib.moment(0)).toEqual([1970, 0, 45_015_000, "1970-01-01T12:30:15.000Z"]);
  expect(lib.invalid()).toEqual([true, true, null]);
  expect(lib.utc()).toBe(Date.UTC(2026, 9, 8));
  expect(lib.nowIsRecent(new Date())).toBe(true);
});

// A struct whose JS is its JSON, `unsafe impl JsonText`, is written by
// `JSON.stringify`, indented, as react.dev's Sandpack template writes its
// package.json: `JSON.stringify({ .. }, null, 2)`.
test("a struct whose JS is its JSON is stringified as it is", async () => {
  const withJs = ["--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("json-text");
  writeFileSync(join(dir, "lib.rs"), `use js::{JsonText, json};
pub struct Dependencies {
    pub react: &'static str,
    #[cfg_attr(rust_js, rust_js::name = "react-dom")]
    pub react_dom: &'static str,
}
pub struct Package {
    pub name: &'static str,
    pub version: Option<&'static str>,
    pub files: Vec<String>,
    pub dependencies: Dependencies,
}
unsafe impl JsonText for Dependencies {}
unsafe impl JsonText for Package {}
pub fn manifest(version: Option<&'static str>) -> String {
    json::stringify_with(
        &Package {
            name: "react.dev",
            version,
            files: vec!["a.js".to_string()],
            dependencies: Dependencies { react: "19", react_dom: "19" },
        },
        None,
        2,
    )
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('dependencies: { react: "19", "react-dom": "19" },\n    },\n    null,\n    2,\n  );');
  const lib = await import(join(dir, "lib.js"));
  const want = { name: "react.dev", files: ["a.js"], dependencies: { react: "19", "react-dom": "19" } };
  expect(lib.manifest(undefined)).toBe(JSON.stringify(want, null, 2));
  expect(JSON.parse(lib.manifest("1")).version).toBe("1");
});

// ADR 0225: JSON is a typed value, `js::Json`, as ReScript's `JSON.t` is,
// and an object of it a `js::Dict`, ReScript's `dict`, TypeScript's
// `Record<string, T>`: a JSON `null` is `None`, a key that isn't there too,
// told apart by `dict::get`.
test("JSON is a typed value, and its objects dictionaries", async () => {
  const withWeb = ["--", "--extern", `webapi=${join(target, "libwebapi.rmeta")}`, "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("json");
  writeFileSync(join(dir, "lib.rs"), `use js::{Dict, Json, dict};
fn show(value: Option<Json>) -> String {
    match value {
        None => "null".to_string(),
        Some(Json::String(text)) => format!("'{text}'"),
        Some(Json::Number(n)) => n.to_string(),
        Some(Json::Bool(b)) => b.to_string(),
        Some(Json::Array(items)) => format!("[{}]", items.iter().map(|item| show(*item)).collect::<Vec<_>>().join(",")),
        Some(Json::Object(fields)) => format!(
            "{{{}}}",
            dict::entries(fields).into_iter().map(|(key, value)| format!("{key}:{}", show(value))).collect::<Vec<_>>().join(",")
        ),
    }
}
pub fn parsed(text: &str) -> String {
    match Json::parse(text) {
        Ok(value) => show(value),
        Err(_) => "invalid".to_string(),
    }
}
pub fn lookup(text: &str, key: &str) -> String {
    match Json::parse(text) {
        Ok(Some(Json::Object(fields))) => match dict::get(fields, key) {
            None => "missing".to_string(),
            Some(value) => show(*value),
        },
        _ => "not an object".to_string(),
    }
}
pub fn round_trip(text: &str) -> String {
    match Json::parse(text) {
        Ok(Some(value)) => Json::stringify(&value),
        _ => "null".to_string(),
    }
}
pub fn built() -> Vec<String> {
    let numbers: &Dict<f64> = dict::from_entries(vec![("a".to_string(), 1.0)]);
    dict::set(numbers, "b", 2.0);
    dict::keys(numbers)
}
pub fn counted(key: &str) -> f64 {
    let counts: &Dict<f64> = dict::from_entries(vec![("a".to_string(), 1.0)]);
    dict::get(counts, key).copied().unwrap_or(0.0)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js"), ...withWeb]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  // A lookup is the runtime's `$dictGet`, imported with its other helpers.
  expect(js).toContain('import { $dictGet, $displayF64, $someValue, $try } from "@rust-js/runtime";');
  expect(js).toContain("Object.entries(value)");
  expect(js).toMatch(/ = \$dictGet\(\w+(\._0)?, key\);/);
  expect(js).toMatch(/return JSON\.stringify\(\w+(\._0)?\);/);
  expect(js).toContain("const numbers = { a: 1 };\n  numbers.b = 2;");
  const lib = await import(join(dir, "mir.js"));
  expect([lib.parsed('{"a":[1,"x",null,true],"b":{}}'), lib.parsed("null"), lib.parsed("nope")])
    .toEqual(["{a:[1,'x',null,true],b:{}}", "null", "invalid"]);
  expect([lib.lookup('{"a":null,"b":1}', "a"), lib.lookup('{"a":null,"b":1}', "b"), lib.lookup('{"a":null}', "c"), lib.lookup('{"a":null}', "toString"), lib.lookup("[1]", "a")])
    .toEqual(["null", "1", "missing", "missing", "not an object"]);
  expect(lib.roundTrip('{"a": [1, null]}')).toBe('{"a":[1,null]}');
  expect(lib.built()).toEqual(["a", "b"]);
  // A dictionary of what isn't an Option is read as JS reads one (ADR 0225).
  expect(js).toContain("return counts[key] ?? 0;");
  expect([lib.counted("a"), lib.counted("b")]).toEqual([1, 0]);
});

// ADR 0229: `encodeURIComponent(uriComponent: string | number | boolean)`
// takes each member as TypeScript's does, the value as it is.
test("encodeURIComponent takes a string, a number or a boolean", async () => {
  const withJs = ["--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("js-encode-uri-component");
  const source = `use js::encode_uri_component;
pub fn parts(text: &str, owned: &String, n: f64, flag: bool) -> Vec<String> {
    vec![
        encode_uri_component(text),
        encode_uri_component(owned),
        encode_uri_component(n),
        encode_uri_component(flag),
    ]
}
pub fn each(texts: &[&str]) -> Vec<String> {
    texts.iter().copied().map(encode_uri_component).collect()
}
`;
  writeFileSync(join(dir, "lib.rs"), source);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("encodeURIComponent(text),\n    encodeURIComponent(owned),\n    encodeURIComponent(n),\n    encodeURIComponent(flag),\n");
  expect(js).toContain("return texts.map((component) => encodeURIComponent(component));");
  const lib = await import(join(dir, "lib.js"));
  expect(lib.parts("a b", "&", 1.5, true)).toEqual(["a%20b", "%26", "1.5", "true"]);
  expect(lib.each(["?", "#"])).toEqual(["%3F", "%23"]);
  writeFileSync(join(dir, "lib.rs"), source.replace("encode_uri_component(n)", "encode_uri_component(1_i32)"));
  const wrong = Bun.spawnSync([compiler, join(dir, "lib.rs"), "-o", join(dir, "wrong.js"), ...withJs], { cwd: dir });
  expect([wrong.exitCode === 0, wrong.stderr.toString().includes("is not a `string | number | boolean`")], wrong.stderr.toString()).toEqual([false, true]);
});

// ADR 0698: a promise's `catch` handler gives what TypeScript's
// `onrejected` may, `TResult | PromiseLike<TResult>`: a value, a promise of
// one, or, of an `Option`'s, nothing. A `catch` whose promise nothing
// reads is a statement, which rustc warns nothing of.
test("a promise's catch handler gives a value or a promise", async () => {
  const dir = fixture("js-promise-catch");
  writeFileSync(join(dir, "lib.rs"), `use js::{Promise, Unknown, error, promise};
pub fn silenced(maybe_promise: &'static Unknown) {
    unsafe { js::cast::<Promise<Option<&'static Unknown>>>(Some(maybe_promise)) }.catch(|_| {});
}
pub async fn valued() -> u32 {
    promise::reject::<u32>(Some(error::new("no"))).catch(|_| 7).await
}
pub async fn promised() -> String {
    promise::reject::<String>(Some(error::new("no"))).catch(|e| promise::resolve(e.message())).await
}
pub async fn nothing() -> Option<u32> {
    promise::reject::<Option<u32>>(Some(error::new("no"))).catch(|_| {}).await
}
`);
  const built = runSync([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target], root, 600_000);
  expect(built.code, built.stderr).toBe(0);
  expect(built.stderr).not.toContain("warning");
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("maybePromise.catch(() => {});");
  expect(js).toContain('return await Promise.reject(new Error("no")).catch(() => 7);');
  expect(js).toContain("catch(() => {})");
  writeFileSync(join(dir, "run.mjs"), `import * as lib from "./lib.js";
let unhandled = 0;
process.on("unhandledRejection", () => unhandled++);
lib.silenced(Promise.reject(new Error("async component")));
console.log(JSON.stringify([await lib.valued(), await lib.promised(), (await lib.nothing()) ?? null]));
await new Promise((resolve) => setTimeout(resolve, 10));
console.log(unhandled);
`);
  const ran = runSync([node!, join(dir, "run.mjs")], root, 30_000);
  expect(ran.stderr).toBe("");
  expect(ran.stdout).toBe('[7,"no",null]\n0\n');
});

// React's lazy() stashes a promise's resolvers in its own `let`s, `new
// Promise((resolve, reject) => { resolveDebugValue = resolve; .. })`: the
// executor runs before `new Promise` returns, so it may borrow what's
// outside it (ADR 0782). And `then(onfulfilled, onrejected)`, TypeScript's
// two handlers, is `then_with`. Its amendment: a promise of a `Copy` value
// is `Copy`, a handle JS shares, kept in a `Cell` and read again,
// `ioInfo.value.then(..)`; one of any other value is consumed, rustc's
// E0382; and what rejects one takes any value, `rejectDebugValue(error)`
// of a `mixed` error, `Promise.reject()` of none.
test("a promise's executor may set the variables around it, and then takes two handlers", async () => {
  const dir = fixture("js-promise-executor");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::Cell;

use js::{JsError, Promise, Unknown, error, js_error, promise};

pub struct Debug {
    pub resolve: &'static dyn Fn(u32),
    pub reject: &'static dyn Fn(Option<&Unknown>),
    pub value: Promise<u32>,
}

pub fn stash() -> Debug {
    let mut resolve_debug_value: Option<&'static dyn Fn(u32)> = None;
    let mut reject_debug_value: Option<&'static dyn Fn(Option<&Unknown>)> = None;
    let value = promise::new(|resolve, reject| {
        resolve_debug_value = Some(resolve);
        reject_debug_value = Some(reject);
    });
    Debug {
        resolve: resolve_debug_value.unwrap(),
        reject: reject_debug_value.unwrap(),
        value,
    }
}

pub async fn settled(fail: bool) -> String {
    let value = match fail {
        true => promise::reject::<u32>(Some(error::new("no"))),
        false => promise::resolve(7),
    };
    value.then_with(|n| n.to_string(), |e| e.message()).await
}

pub fn hidden(value: Promise<u32>) {
    let _: Promise<()> = value.then_with(|_| {}, |_| {});
}

pub struct Info {
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub value: Cell<Option<Promise<Option<&'static Unknown>>>>,
}

pub fn rejected(info: &Info, error: Option<&'static Unknown>) -> Promise<String> {
    let mut reject_debug_value: Option<&'static dyn Fn(Option<&Unknown>)> = None;
    info.value.set(Some(promise::new(|_resolve, reject| {
        reject_debug_value = Some(reject);
    })));
    let _: Promise<()> = info.value.get().unwrap().then_with(|_| {}, |_| {});
    reject_debug_value.unwrap()(error);
    let shown = info.value.get().unwrap();
    shown.then_with(|_| String::from("fulfilled"), |e| js_error::to_string(e))
}

pub fn twice(value: Promise<u32>) -> Promise<u32> {
    let _: Promise<()> = value.then_with(|_| {}, |_| {});
    value.then_resolve(|n| n + 1)
}

pub fn rethrown(error: &JsError) -> Promise<u32> {
    promise::reject(Some(error))
}

pub fn of_nothing() -> Promise<u32> {
    promise::reject(None)
}
`);
  const built = runSync([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target], root, 600_000);
  expect(built.code, built.stderr).toBe(0);
  expect(built.stderr).not.toContain("warning");
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain(`  const value = new Promise((resolve, reject) => {
    resolveDebugValue = resolve;
    rejectDebugValue = reject;
  });`);
  expect(js).toContain("return await value.then(\n    (n) => String(n),\n    (e) => e.message,\n  );");
  expect(js).toContain("  value.then(\n    () => {},\n    () => {},\n  );");
  expect(js).toContain("  $unwrap(rejectDebugValue)(error);\n");
  expect(js).toContain("  return value.then((n) => (n + 1) >>> 0);\n");
  expect(js).toContain("  return Promise.reject(error);\n");
  expect(js).toContain("  return Promise.reject();\n");
  writeFileSync(join(dir, "run.mjs"), `import * as lib from "./lib.js";
let unhandled = 0;
process.on("unhandledRejection", () => unhandled++);
const debug = lib.stash();
debug.resolve(3);
console.log(await debug.value, await lib.settled(false), await lib.settled(true));
lib.hidden(Promise.reject(new Error("hidden")));
const info = {};
console.log(await lib.rejected(info, "lost"), await lib.rejected({}, undefined), info.value instanceof Promise);
console.log(await lib.twice(Promise.resolve(1)));
console.log(await lib.rethrown(new TypeError("again")).catch(String), await lib.ofNothing().catch((e) => e));
await new Promise((resolve) => setTimeout(resolve, 10));
console.log(unhandled);
`);
  const ran = runSync([node!, join(dir, "run.mjs")], root, 30_000);
  expect(ran.stderr).toBe("");
  expect(ran.stdout).toBe("3 7 no\nlost undefined true\n2\nTypeError: again undefined\n0\n");
  // A promise of a value that isn't `Copy` is consumed: two handlers given
  // the one array it fulfils with would each change it.
  writeFileSync(join(dir, "moved.rs"), `use js::Promise;

pub fn both(value: Promise<Vec<u32>>) -> Promise<usize> {
    let _: Promise<()> = value.then_resolve(|mut v| v.push(1));
    value.then_resolve(|v| v.len())
}
`);
  const moved = runSync([compiler, join(dir, "moved.rs"), "-o", join(dir, "moved.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target], root, 600_000);
  expect(moved.code).not.toBe(0);
  expect(moved.stderr).toContain("E0382");
});

// ADR 0699: a program keeps upstream's fallback where the host has no
// `WeakMap`, `typeof WeakMap === 'function' ? WeakMap : Map`: a `js::Map`
// of what a `WeakMap` keys is a `WeakMap`, as TypeScript's `Map<K, V>` is,
// so the one value holds either.
test("a Map is a WeakMap where the host has none", async () => {
  const dir = fixture("js-possibly-weak-map");
  writeFileSync(join(dir, "lib.rs"), `use js::{JsObject, Map, Unknown, WeakMap};

thread_local! {
    pub static CACHE: &'static WeakMap<dyn Fn() -> Option<&'static Unknown>, String> =
        if js::is_defined(js::weak_map::new::<JsObject, String>) {
            js::weak_map::new()
        } else {
            js::map::new(None)
        };
}

pub fn cached(f: &dyn Fn() -> Option<&'static Unknown>, frame: String) -> Option<String> {
    CACHE.with(|cache| {
        let before = cache.get(f);
        cache.set(f, frame);
        before
    })
}

pub fn strong(object: &JsObject) -> (u32, bool, u32) {
    let map: &Map<JsObject, u32> = js::map::new(None);
    map.set(object, 1);
    let size = map.size();
    let had = map.delete(object);
    map.clear();
    (size, had, map.size())
}
`);
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('export const cache = typeof WeakMap !== "undefined" ? new WeakMap() : new Map();');
  expect(js).toContain("const before = cache.get(f);");
  expect(js).toContain("const map = new Map();");
  expect(js).toContain("const size = map.size;");
  expect(readFileSync(join(dir, "lib.d.ts"), "utf8")).toContain("export const cache: WeakMap<() => unknown | null | undefined, string>;");
  writeFileSync(join(dir, "run.mjs"), `const weak = process.argv[2] === "weak";
if (!weak) delete globalThis.WeakMap;
const lib = await import("./lib.js");
const f = () => 1;
console.log(JSON.stringify([lib.cache.constructor.name, lib.cached(f, "a") ?? null, lib.cached(f, "b"), lib.strong({})]));
`);
  for (const [host, made] of [["weak", "WeakMap"], ["none", "Map"]]) {
    const ran = runSync([node!, join(dir, "run.mjs"), host], root, 30_000);
    expect(ran.stderr).toBe("");
    expect(ran.stdout).toBe(`["${made}",null,"a",[1,true,0]]\n`);
  }
});

// ADR 0849: a `Map` takes any key, as TypeScript's `Map<K, V>` does, each
// the one JS's `SameValueZero` says: React's cache keys a primitive
// argument, an `Option<&Unknown>`, `undefined` and `null` apart, `0` and
// `-0` one, `NaN` itself, an object by itself.
test("a Map takes any key, as TypeScript's does", async () => {
  const dir = fixture("js-map-any-key");
  writeFileSync(join(dir, "lib.rs"), `use js::{Map, Unknown};

pub fn first_seen(keys: &[Option<&'static Unknown>]) -> Vec<Option<u32>> {
    let map: &Map<Option<&'static Unknown>, u32> = js::map::new(None);
    for (i, &key) in keys.iter().enumerate() {
        if !map.has(&key) {
            map.set(&key, i as u32);
        }
    }
    keys.iter().map(|key| map.get(key)).collect()
}

pub fn texts() -> (Option<f64>, bool, bool, u32) {
    let map: &Map<str, f64> = js::map::new(None);
    map.set("a", 1.0);
    (map.get("a"), map.delete("b"), map.delete("a"), map.size())
}
`);
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("if (!map.has(key)) {\n      map.set(key, i);");
  expect(js).toContain('map.set("a", 1);');
  const lib = await import(join(dir, "lib.js"));
  const object = {};
  expect(lib.firstSeen([undefined, null, 0, -0, NaN, NaN, "0", object, object, {}])).toEqual([0, 1, 2, 2, 4, 4, 6, 7, 7, 9]);
  expect(lib.texts()).toEqual([1, false, true, 0]);
});

// ADR 0850: an `Rc` is `Defined` as what it points at is: where the crate
// counts none, the value itself, which a `WeakMap` holds and gives back as
// Rust shares it, React's cache nodes; a counted one given to JS is
// refused, as JS's value would count no clone it gives back.
test("an uncounted Rc a WeakMap holds is the value, and a counted one is refused", async () => {
  const dir = fixture("js-weak-map-rc");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::Cell;
use std::rc::Rc;

use js::{JsObject, weak_map};

pub struct Node {
    n: Cell<u32>,
}

unsafe impl js::Defined for Node {}

pub fn shared(key: &JsObject) -> u32 {
    let nodes = weak_map::new::<JsObject, Rc<Node>>();
    let node = Rc::new(Node { n: Cell::new(1) });
    nodes.set(key, Rc::clone(&node));
    if let Some(held) = nodes.get(key) {
        held.n.set(2);
    }
    node.n.get()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("const node = { n: 1 };\n  nodes.set(key, node);");
  const lib = await import(join(dir, "lib.js"));
  expect(lib.shared({})).toBe(2);
  writeFileSync(join(dir, "counted.rs"), `use std::rc::Rc;

use js::{JsObject, weak_map};

pub struct Loud(pub u32);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

unsafe impl js::Defined for Loud {}

pub fn held(key: &JsObject) -> usize {
    let map = weak_map::new::<JsObject, Rc<Loud>>();
    let loud = Rc::new(Loud(1));
    map.set(key, Rc::clone(&loud));
    drop(map.get(key));
    Rc::strong_count(&loud)
}
`);
  const counted = runSync([compiler, join(dir, "counted.rs"), "-o", join(dir, "counted.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target], root, 600_000);
  expect(counted.code).not.toBe(0);
  expect(counted.stderr).toContain(
    "rust-js does not support a counted `std::rc::Rc<Loud>` given to JS or taken from it yet: JS holds the box that counts it, which counts no clone JS makes",
  );
});

// ADR 0782's amendment: what was thrown, a `&JsError`, as a value of any
// shape, `error.as_unknown()`, the value itself, which a field of any
// value stores and `js::throw` throws again: of an `Error`, an object, a
// string, and a falsy one, `0`, `""`, `undefined` and `null`.
test("what was caught is a value of any shape", async () => {
  const dir = fixture("js-error-as-unknown");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::Cell;
use std::panic::{self, AssertUnwindSafe};

use js::{JsError, Unknown};

pub struct Node {
    pub v: Cell<Option<&'static Unknown>>,
}

pub fn caught(f: &dyn Fn(), node: &Node) -> bool {
    match panic::catch_unwind(AssertUnwindSafe(f)) {
        Ok(()) => false,
        Err(error) => {
            node.v.set(error.downcast_ref::<&'static JsError>().and_then(|error| error.as_unknown()));
            true
        }
    }
}

pub fn thrown_again(node: &Node) {
    js::throw(node.v.get())
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("node.v = error;");
  const lib = await import(join(dir, "lib.js"));
  for (const thrown of [new Error("e"), { at: 1 }, "x", 0, "", undefined, null]) {
    const node: { v: unknown } = { v: "unset" };
    expect(lib.caught(() => {
      throw thrown;
    }, node)).toBe(true);
    expect(node.v).toBe(thrown);
    let again: unknown = "none";
    try {
      lib.thrownAgain(node);
    } catch (error) {
      again = error;
    }
    expect(again).toBe(thrown);
  }
});

// ADR 0860: TypeScript's `Iterator<T>`, stepped by its `next()`, whose
// `IteratorResult<T>`'s `done` is left out of a step that isn't the end, as
// React's `mapIntoArray` steps what an iterable's `@@iterator` gives, one
// with no `[Symbol.iterator]` of its own too; the loop that makes the step
// its test reads, `while (!(step = iterator.next()).done)`; and a number's
// string in a radix, `index.toString(36)`.
test("an iterator is stepped by its next, and a number written in a radix", async () => {
  const dir = fixture("js-iterator-protocol");
  writeFileSync(join(dir, "lib.rs"), `use js::Unknown;

pub fn keys(iterator: &js::Iterator<Option<&'static Unknown>>, prefix: &str) -> Vec<String> {
    let mut keys = Vec::new();
    let mut ii = 0;
    loop {
        let step = iterator.next();
        if step.done == Some(true) {
            break;
        }
        let child = step.value;
        keys.push(prefix.to_string() + &js::number::to_string(f64::from(ii), Some(36)) + "=" + &js::string(child));
        ii += 1;
    }
    keys
}

pub fn read(iterator: &js::Iterator<f64>) -> Vec<f64> {
    let mut read = Vec::new();
    let mut step = iterator.next();
    while step.done != Some(true) {
        read.push(step.value);
        step = iterator.next();
    }
    read
}

pub fn decimal(n: f64) -> String {
    js::number::to_string(n, None)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("  let step;\n  while (!(step = iterator.next()).done) {\n    const child = step.value;\n");
  expect(js).toContain("prefix + ii.toString(36)");
  expect(js).toContain("  while (!step.done) {\n");
  expect(js).toContain("return n.toString();");
  const lib = await import(join(dir, "lib.js"));
  // A program's own iterator, `next` alone, its steps' `done` left out
  // until the end, and one given `done: false`.
  const counted = (n: number, given: boolean) => {
    let i = 0;
    return { next: () => (i < n ? (given ? { value: i++, done: false } : { value: i++ }) : { value: "end", done: true }) };
  };
  const keys: string[] = lib.keys(counted(37, false), ".");
  expect(keys.slice(0, 2)).toEqual([".0=0", ".1=1"]);
  expect(keys.slice(35)).toEqual([".z=35", ".10=36"]);
  expect(lib.keys(new Map([["a", 1]]).entries(), "")).toEqual(["0=a,1"]);
  expect(lib.read(counted(3, true))).toEqual([0, 1, 2]);
  expect(lib.read([4, 5].values())).toEqual([4, 5]);
  expect([lib.decimal(1e21), lib.decimal(Infinity), lib.decimal(0.5)]).toEqual(["1e+21", "Infinity", "0.5"]);
});

// ADR 0867: an `AggregateError` is TypeScript's of ES2021, `new
// AggregateError(errors: Iterable<any>, message?: string)` and its `errors`,
// an `Error` by `Deref`, as React's `act` makes one of what was thrown; and
// its constructor runs nothing of the crate's, so a `RefCell` borrowed for
// the call is the array it holds, `ReactSharedInternals.thrownErrors`.
test("an AggregateError is ES2021's, and making one borrows nothing", async () => {
  const dir = fixture("js-aggregate-error");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::RefCell;

use js::{Unknown, aggregate_error};

pub struct Internals {
    pub thrown_errors: RefCell<Option<Vec<Option<&'static Unknown>>>>,
}

thread_local! {
    pub static INTERNALS: Internals = Internals { thrown_errors: RefCell::new(Some(Vec::new())) };
}

fn aggregate_errors(errors: &[Option<&'static Unknown>]) -> Option<&'static Unknown> {
    if errors.len() > 1 && js::is_defined(aggregate_error::new) {
        return aggregate_error::new(errors, None).as_unknown();
    }
    errors.first().copied().flatten()
}

pub fn record(error: Option<&'static Unknown>) {
    INTERNALS.with(|i| i.thrown_errors.borrow_mut().as_mut().unwrap().push(error));
}

pub fn thrown() -> Option<&'static Unknown> {
    let thrown = INTERNALS.with(|i| aggregate_errors(i.thrown_errors.borrow().as_ref().unwrap()));
    INTERNALS.with(|i| i.thrown_errors.borrow_mut().as_mut().unwrap().clear());
    thrown
}

pub fn named(errors: Vec<Option<&'static Unknown>>) -> (String, String, usize, bool) {
    let e = aggregate_error::new(&errors, Some("several"));
    (e.name(), e.message(), e.errors().len(), e.is_error())
}

js::export_default!(INTERNALS);
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("return new AggregateError(errors);");
  expect(js).toContain('const e = new AggregateError(errors, "several");');
  expect(js).toContain("e.errors.length");
  // The cell is the array, borrowed unchecked: no borrow meets another.
  expect(js).toContain("const internals = { thrownErrors: [] };");
  expect(js).toContain("aggregateErrors($unwrap(internals.thrownErrors))");
  expect(js).not.toContain("$borrow");
  const lib = await import(join(dir, "lib.js"));
  const [a, b] = [new Error("a"), "b"];
  lib.record(a);
  expect(lib.thrown()).toBe(a);
  lib.record(a);
  lib.record(b);
  const both = lib.thrown();
  expect(both).toBeInstanceOf(AggregateError);
  expect(both.errors).toEqual([a, b]);
  expect(lib.default.thrownErrors).toEqual([]);
  expect(lib.named([a, undefined])).toEqual(["AggregateError", "several", 2, true]);
});
