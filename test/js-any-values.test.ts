// What the js crate's functions of a value of any shape do as JS does
// them: `js::keys_in` of what may be `null` or `undefined` (ADR 0719,
// amended), JS's loose equality, `js::loose_equals` (ADR 1012), and its
// global `eval`, `js::eval` (ADR 1033, amended), and a `bigint` of any size
// `classify` tells (ADR 1057). Each
// test is a program of its own, compiled and run in Node, as no native
// Rust has these values.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

import { runSync } from "./child";
import { buildReact, compiler, fixture, root, target } from "./support";

beforeAll(buildReact, 600_000);

const withJs = ["--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];

/** `source` compiled, its JS, and the module it is, loaded. */
async function compiled(name: string, source: string): Promise<{ js: string; lib: any; warnings: string[] }> {
  const dir = fixture(name);
  writeFileSync(join(dir, "lib.rs"), source);
  const built = runSync([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs], root, 600_000);
  expect(built.code, built.stderr).toBe(0);
  const warnings = built.stderr.split("\n").filter((line) => line.startsWith("warning:") && !/warnings? emitted/.test(line));
  return { js: readFileSync(join(dir, "lib.js"), "utf8"), lib: await import(join(dir, "lib.js")), warnings };
}

// ADR 0719, amended (React port gap rd71): `keys_in` takes any value, an
// `Option` too, as JS's `for…in` does, which visits nothing of `null` or
// `undefined`: CSSPropertyOperations' `for (const styleName in styles)`, no
// `if` around it.
test("keys_in of an Option visits none of None", async () => {
  const { js, lib } = await compiled("js-keys-in-option", `use js::Unknown;

pub fn count(styles: Option<&Unknown>) -> u32 {
    let mut n = 0;
    for key in js::keys_in(styles) {
        n += key.len() as u32;
    }
    n
}

pub fn listed(styles: Option<&Unknown>) -> Vec<String> {
    js::keys_in(styles).collect()
}
`);
  expect(js).toContain("  let n = 0;\n  for (const key in styles) {\n");
  expect([lib.count(undefined), lib.count(null), lib.count({ ab: 1, c: 2 })]).toEqual([0, 0, 3]);
  expect([lib.listed(null), lib.listed({ a: 1 })]).toEqual([[], ["a"]]);
});

// ADR 1012 (React port gap rd78): JS's loose equality, which ReactDOMInput
// compares by on purpose, `node.value != value`, so `"0.0"` typed where the
// value is `0` isn't clobbered: `js::loose_equals(a, b)` is `a == b`, and
// coerces as JS does, where Rust's `==` of the text would differ.
test("loose_equals is JS's ==, which coerces", async () => {
  const { js, lib } = await compiled("js-loose-equals", `use js::Unknown;

pub fn clobbers(typed: &str, value: &Unknown) -> bool {
    !js::loose_equals(typed, value)
}

pub fn same(a: Option<&Unknown>, b: Option<&Unknown>) -> bool {
    js::loose_equals(a, b)
}
`);
  expect(js).toContain("  return typed != value;\n");
  expect([lib.clobbers("0.0", 0), lib.clobbers("1", 1), lib.clobbers("1", 2), lib.clobbers("a", "a"), lib.clobbers("", 0)]).toEqual([false, false, true, false, false]);
  expect([lib.same(null, undefined), lib.same(undefined, 0), lib.same("1", true), lib.same(null, false)]).toEqual([true, false, true, false]);
});

// rd444: \`Kind::Bool(true)\` of a value of any shape is \`x === true\`, as
// React's isRecoverableError writes \`error[REACT_RECOVERABLE_TYPE] === true\`,
// not \`!!(typeof x === "boolean" && x)\`, its match's \`typeof\` and its
// payload's truth; \`Kind::Bool(false)\` \`x === false\`.
test("a value of any shape that's true is === true", async () => {
  const { js, lib } = await compiled("js-kind-bool", `use js::{Kind, Unknown};

pub fn branded(error: Option<&'static Unknown>) -> bool {
    matches!(js::classify(js::get(error, "brand")), Kind::Bool(true))
}

pub fn off(value: Option<&'static Unknown>) -> bool {
    matches!(js::classify(value), Kind::Bool(false))
}

pub fn flag(value: Option<&'static Unknown>) -> bool {
    matches!(js::classify(value), Kind::Bool(_))
}
`);
  expect(js).toContain("  return error.brand === true;\n");
  expect(js).toContain("  return value === false;\n");
  expect(js).toContain('  return typeof value === "boolean";\n');
  const values = [true, false, 1, 0, "true", "", null, undefined, {}];
  expect(values.map((brand) => lib.branded({ brand }))).toEqual(values.map((v) => v === true));
  expect(values.map(lib.off)).toEqual(values.map((v) => v === false));
  expect(values.map(lib.flag)).toEqual(values.map((v) => typeof v === "boolean"));
});

// React's ReactDOMEventReplaying walks its form replaying queue, a JS
// array, three items at a time, `formReplayingQueue[i + 1]`, `i += 3`: an
// `f64` is a property key, TypeScript's `number`, as ADR 0994 gives
// WebIDL's numbers, so the index is JS's number as it is, no `>>> 0`
// (React port gap rd101).
test("an f64 is a property key, its index as JS writes it", async () => {
  const { js, lib } = await compiled("f64-property-key", `use js::Unknown;

pub fn replay(queue: &Unknown, length: f64) -> u32 {
    let mut found = 0;
    let mut i = 0.0;
    while i < length {
        if js::get(queue, &(i + 1.0)).is_some() {
            js::set(queue, &(i + 2.0), "seen");
            found += 1;
        }
        i += 3.0;
    }
    found
}
`);
  expect(js).toContain("  for (let i = 0; i < length; i += 3) {\n    if (queue[i + 1] != null) {\n      queue[i + 2] = \"seen\";\n");
  const queue = [0, "a", null, 1, null, null, 2, "b", 0];
  expect(lib.replay(queue, queue.length)).toBe(2);
  expect(queue).toEqual([0, "a", "seen", 1, null, null, 2, "b", "seen"]);
});

// React DOM's startViewTransition waits on `Array<Promise<any>>`, fonts'
// `ready` and images' promises, and races `Promise.all(..)` against a
// timeout's promise. A promise of any value is `promise.into_unknown()`,
// the promise itself, what it fulfils any value (rd211).
test("a promise of any value is the promise itself", async () => {
  const { js, lib } = await compiled("js-promise-of-any", `use js::{Promise, Unknown, promise};

#[cfg_attr(rust_js, rust_js::link_name = "globalThis.ready")]
fn ready() -> Promise<&'static js::JsObject> {
    unreachable!()
}

pub async fn blocked(image: Promise<()>, ms: f64) -> Option<&'static Unknown> {
    let blocking: Vec<Promise<Option<&'static Unknown>>> = vec![ready().into_unknown(), image.into_unknown()];
    let timeout = promise::new(move |resolve: &'static dyn Fn(Option<&'static Unknown>), _| {
        js::set_timeout(move || resolve(None), Some(ms));
    });
    promise::race(vec![promise::all(blocking).into_unknown(), timeout]).await
}
`);
  expect(js).toContain("const blocking = [globalThis.ready(), image];");
  expect(js).toContain("return await Promise.race([Promise.all(blocking), timeout]);");
  (globalThis as Record<string, unknown>).ready = () => Promise.resolve("fonts");
  expect(await lib.blocked(Promise.resolve(), 50)).toEqual(["fonts", undefined]);
  expect(await lib.blocked(new Promise(() => {}), 1)).toBe(undefined);
});

// rd331c: JS's unary `+` of any value, react-art's `Mode.Surface(+width,
// +height, ..)`, `js::to_number`: not `Number(value)`, which makes `1` of
// `1n`, where `+1n` throws. Of a value of any shape a `Result`, as
// `concat_text`'s is, whose `unwrap()` is the call; of text, a number.
test("to_number is JS's unary +, which throws of a BigInt", async () => {
  const { js, lib } = await compiled("js-to-number", `use js::Unknown;

pub fn sized(width: &Unknown, height: Option<&Unknown>) -> f64 {
    js::to_number(width).unwrap() * js::to_number(height).unwrap()
}

pub fn numeric(value: &Unknown) -> bool {
    js::to_number(value).is_ok()
}

pub fn parsed(text: &str) -> f64 {
    js::to_number(text) + 1.0
}

// A match's group or the next, as React's Flight parses a stack frame's
// line, \`+(parsed[3] || parsed[6])\`: \`NaN\` of neither.
pub fn line_of(first: Option<String>, second: Option<&str>) -> f64 {
    js::to_number(first.filter(|line| !line.is_empty()).as_deref().or(second))
}

pub fn owned_line(line: Option<String>) -> f64 {
    js::to_number(line)
}

pub struct Size {
    pub width: &'static Unknown,
}

// A field taken apart from a borrowed struct, a \`&&Unknown\`, as react-art's
// \`const { height, width } = this.props;\` is.
pub fn width_of(size: &Size) -> f64 {
    let Size { width } = size;
    js::to_number(width).unwrap()
}
`);
  expect(js).toContain("return +width;");
  expect(lib.widthOf({ width: "7" })).toBe(7);
  expect(js).toContain("return +width * +height;");
  expect(js).toContain("function numeric(value) {\n  try {\n    +value;\n    return true;\n  } catch {\n    return false;\n  }\n}");
  expect(js).toContain("return +text + 1;");
  expect(js).not.toContain("Number(");
  expect([lib.sized("2", 3), lib.sized({ valueOf: () => 4 }, "0x2"), lib.sized(2, undefined)]).toEqual([6, 8, NaN]);
  expect(() => lib.sized(1n, 1)).toThrow(TypeError);
  expect(() => lib.sized(1, Symbol("s"))).toThrow(TypeError);
  const throwing = { valueOf() { throw new Error("no"); } };
  expect([1n, Symbol("s"), throwing, "a", null, {}].map(lib.numeric)).toEqual([false, false, false, true, true, true]);
  expect([lib.parsed("  12 "), lib.parsed(""), lib.parsed("0x10"), lib.parsed("1px")]).toEqual([13, 1, 17, NaN]);
  expect(js).toContain("return +(first || second);");
  expect(js).toContain("return +line;");
  expect([lib.lineOf("3", "4"), lib.lineOf("", "4"), lib.lineOf(undefined, "5"), lib.lineOf(undefined, undefined)]).toEqual([3, 4, 5, NaN]);
  expect([lib.ownedLine("7"), lib.ownedLine(undefined)]).toEqual([7, NaN]);
});

// rd339: JS's global `eval`, `js::eval`, called as React's
// ReactClientDebugConfig checks it's there, `(0, eval)('null')`: an
// indirect eval, which runs in the global scope, as Rust's call can't see
// its caller's variables; by its name alone it'd be a direct one, which
// reads them, `source` here. Its `Result` caught where the program tests it.
test("eval is the global eval, called indirectly", async () => {
  const { js, lib } = await compiled("js-eval", `use std::cell::Cell;

use js::Unknown;

thread_local! {
    static HAS_CONFIRMED_EVAL: Cell<bool> = const { Cell::new(false) };
}

pub fn check_eval_availability_once_dev() -> bool {
    if !HAS_CONFIRMED_EVAL.get() {
        HAS_CONFIRMED_EVAL.set(true);
        if js::eval("null").is_err() {
            return false;
        }
    }
    true
}

pub fn evaluated(source: &str) -> Option<&'static Unknown> {
    js::eval(source).unwrap()
}

#[cfg_attr(rust_js, rust_js::link_name = "(0, tagged)")]
fn tagged() -> u32 {
    unreachable!()
}

pub fn shadowing(tagged: u32) -> u32 {
    tagged + crate::tagged()
}
`);
  expect(js).toContain('(0, eval)("null");');
  expect(js).toContain("(0, eval)(source)");
  expect(lib.checkEvalAvailabilityOnceDev()).toBe(true);
  expect([lib.evaluated("1 + 2"), lib.evaluated("typeof source")]).toEqual([3, "undefined"]);
  expect(() => lib.evaluated("(")).toThrow(SyntaxError);
  // A variable of the name a path reads unbound is another's.
  (globalThis as { tagged?: () => number }).tagged = () => 5;
  expect(lib.shadowing(1)).toBe(6);
});

// ADR 0967: React's DEV copyWithDelete (ReactFiberReconciler.js), `const
// updated = isArray(obj) ? obj.slice() : {...obj};`, then `updated.splice(key,
// 1)` or `delete updated[key]`, and `updated[key] = ..` of a string or a
// number key. `js::object::spread` is `{ ...value }`, which defines a
// `__proto__` key as its own, where `Object.assign({}, value)` sets the
// prototype; `js::array::splice` is `value.splice(start, n)`, of numbers as
// JS's, a negative start from the end; `js::set` takes a key as `js::get`
// does.
test("an object spread, a splice and a set by any key are JS's", async () => {
  const { js, lib, warnings } = await compiled("js-object-spread", `use js::{Kind, Unknown};

fn copy_with_delete_impl(obj: &'static Unknown, path: &[&'static Unknown], index: usize) -> &'static Unknown {
    let key = path[index];
    let updated: &Unknown = match js::classify(Some(obj)) {
        Kind::Array(items) => js::unknown_of(items.to_vec()),
        _ => js::object::spread(Some(obj)),
    };
    if index + 1 == path.len() {
        if js::array::is_array(Some(updated)) {
            if let Kind::Number(at) = js::classify(Some(key)) {
                js::array::splice(updated, at, 1.0, ());
            }
        } else {
            js::delete(updated, key);
        }
        return updated;
    }
    let inner = js::get(obj, key).expect("a path");
    js::set(updated, key, copy_with_delete_impl(inner, path, index + 1));
    updated
}

pub fn copy_with_delete(obj: &'static Unknown, path: &[&'static Unknown]) -> &'static Unknown {
    copy_with_delete_impl(obj, path, 0)
}

pub fn copied(value: Option<&Unknown>) -> &'static js::JsObject {
    js::object::spread(value)
}

// What's put in its place too, Flight's unbadgeConsole's
// \`args.splice(offset, 4, unbadgedFormat)\`, past the end only to it.
pub fn unbadged(args: &Unknown, offset: f64, format: &str) {
    js::array::splice(args, offset, 4.0, (format,));
}

pub fn keyframes() -> Vec<&'static js::JsObject> {
    vec![js::object::spread(None), js::object::spread(None)]
}
`);
  expect(warnings).toEqual([]);
  expect(js).toContain("  const updated = Array.isArray(obj) ? obj.slice() : { ...obj };\n");
  expect(js).toContain("        updated.splice(key, 1);\n");
  expect(js).not.toContain("$f64ToInt");
  expect(js).toContain("      delete updated[key];\n");
  expect(js).toContain("  args.splice(offset, 4, format);\n");
  const args = ["%c%s%c", "style", " Server ", "", "a"];
  lib.unbadged(args as never, 0 as never, "x" as never);
  expect(args).toEqual(["x", "a"]);
  const short = ["f", "b"];
  lib.unbadged(short as never, 1 as never, "y" as never);
  expect(short).toEqual(["f", "y"]);
  expect(js).toContain("  updated[key] = copyWithDeleteImpl(inner, path, (index + 1) >>> 0);\n");
  const state = { a: { b: [1, 2, 3], c: 4 }, d: 5 };
  const copy = lib.copyWithDelete(state as never, ["a", "b", 1] as never) as any;
  expect(copy).toEqual({ a: { b: [1, 3], c: 4 }, d: 5 });
  expect(state).toEqual({ a: { b: [1, 2, 3], c: 4 }, d: 5 });
  expect(lib.copyWithDelete(state as never, ["a", "c"] as never)).toEqual({ a: { b: [1, 2, 3] }, d: 5 });
  // A negative start counts from the end, as JS's `splice` does; a
  // fractional one is its whole part.
  expect((lib.copyWithDelete(state as never, ["a", "b", -1] as never) as any).a.b).toEqual([1, 2]);
  expect((lib.copyWithDelete(state as never, ["a", "b", 1.5] as never) as any).a.b).toEqual([1, 3]);
  // A `__proto__` key is the copy's own, its prototype `Object.prototype`.
  const keyed = JSON.parse('{"__proto__": {"x": 1}, "y": 2}');
  const spread = lib.copied(keyed as never) as any;
  expect([Object.getPrototypeOf(spread) === Object.prototype, Object.keys(spread), spread.x]).toEqual([true, ["__proto__", "y"], undefined]);
  expect(lib.copied(undefined as never)).toEqual({});
  // Of \`None\`, a new object of nothing, \`{}\`: animateGesture's \`[{}, {}]\` (rd210).
  expect(js).toContain("  return [{}, {}];\n");
  const [a, b] = lib.keyframes() as unknown[];
  expect([a, a === b]).toEqual([{}, false]);
});

// rd489 (P0): an `Option` of an `Option` held as its payload keeps its
// `Some(None)` as `null` (ADR 1017), so the inner `None` it gave on was
// `null`: `first(undefined, f)` called `f(null)`, and React's Flight port
// resolved a chunk whose value is `undefined` with `null`, `Some(*value)`
// written `chunk.value ?? null`. A value of any shape tells `undefined`
// from `null` (ADR 0676): one whose inner `Option` is given on keeps its
// box, which holds which it was (ADR 1017, amended).
test("the None an Option of an Option gives of any value is undefined", async () => {
  const { js, lib } = await compiled("js-nested-option-any", `use js::Unknown;

pub fn first(v: Option<&'static Unknown>, f: &dyn Fn(Option<&'static Unknown>)) {
    let o = if v.is_some() { Some(v) } else { Some(None) };
    if let Some(v) = o {
        f(v);
    }
}

pub struct Chunk {
    pub fulfilled: bool,
    pub value: Option<&'static Unknown>,
}

pub fn settle(chunk: &Chunk, resolve: &dyn Fn(Option<&'static Unknown>)) {
    let value = if chunk.fulfilled { Some(chunk.value) } else { None };
    if let Some(value) = value {
        resolve(value);
    }
}

pub fn kind(chunk: &Chunk) -> &'static str {
    let value = if chunk.fulfilled { Some(chunk.value) } else { None };
    match value {
        Some(value) => match js::classify(value) {
            js::Kind::Undefined => "undefined",
            js::Kind::Null => "null",
            _ => "value",
        },
        None => "pending",
    }
}
`);
  expect(js).not.toContain("?? null");
  const given = (call: (f: (v: unknown) => void) => void) => {
    const seen: unknown[] = [];
    call((v) => seen.push(v));
    return seen;
  };
  expect([undefined, null, 0, "a"].map((v) => given((f) => lib.first(v, f)))).toStrictEqual([[undefined], [undefined], [0], ["a"]]);
  const chunks = [undefined, null, 0, { a: 1 }].map((value) => ({ fulfilled: true, value }));
  expect(chunks.map((chunk) => given((f) => lib.settle(chunk, f)))).toStrictEqual([[undefined], [null], [0], [{ a: 1 }]]);
  expect(given((f) => lib.settle({ fulfilled: false, value: 1 }, f))).toEqual([]);
  expect([...chunks, { fulfilled: false }].map(lib.kind)).toEqual(["undefined", "null", "value", "value", "pending"]);
});

// rd512 (ADR 1057): a `bigint` `classify` tells is a `&js::BigInt`, of any
// size, never an `i64`, which wraps past 2^63: its digits are
// `n.toString(radix)`, React's Flight reply client's `value.toString(10)`,
// and its number `Number(n)`, the reply server's estimate of its digits,
// `Infinity` of one past `f64::MAX`.
test("a bigint classify tells is one of any size, its toString and Number", async () => {
  const { js, lib } = await compiled("bigint-kind", `use js::{Kind, Unknown, classify};

pub fn sent(value: &Unknown) -> String {
    match classify(Some(value)) {
        Kind::BigInt(n) => format!("$n{}", n.to_string(Some(10))),
        _ => String::new(),
    }
}

pub fn hex(value: &Unknown) -> String {
    match classify(Some(value)) {
        Kind::BigInt(n) => n.to_string(Some(16)),
        _ => String::new(),
    }
}

pub fn digits(value: &Unknown) -> f64 {
    match classify(Some(value)) {
        Kind::BigInt(n) => js::number::from_big_int(n).abs().log10().floor() + 1.0,
        _ => 0.0,
    }
}
`);
  expect(js).toContain("value.toString(10)");
  expect(js).toContain("value.toString(16)");
  expect(js).toContain("Math.abs(Number(value))");
  expect([lib.sent(2n ** 70n), lib.sent(-12n), lib.sent(1)]).toEqual(["$n1180591620717411303424", "$n-12", ""]);
  expect(lib.hex(2n ** 70n)).toBe("400000000000000000");
  expect([lib.digits(-(10n ** 30n)), lib.digits(10n ** 400n)]).toEqual([31, Infinity]);
});
