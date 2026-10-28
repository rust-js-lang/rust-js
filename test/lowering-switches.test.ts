// What the compiler writes of a `match` of constants as a `switch` (ADR
// 0629): its arms' cases, a guarded one's too. Each test is a program of its
// own, compiled and run.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { runSync } from "./child";
import { node } from "./programs";
import { buildReact, compiler, fixture, installRuntime, root, run, target } from "./support";

beforeAll(buildReact, 600_000);

const withJs = () => ["--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];

// ADR 0629, amended: a guarded arm is a case whose statements test the
// guard, `case C: if (on) { .. } break;`, where what Rust tries next when
// the guard fails does nothing: each later arm that could match the arm's
// constants a `_ => {}`. React's `getComponentNameFromType` writes `case
// REACT_TRACING_MARKER_TYPE: if (enableTransitionTracing) { .. }`, which
// clippy's `collapsible_match` asks for as a guard. Where a later arm
// could take the constant and does something, `C if on => 30, .., _ => 0`,
// a failed guard goes on to it, and the `match` stays an `if` chain.
test("a guarded arm of a match of constants is a case that tests its guard", async () => {
  const dir = fixture("guarded-switches");
  writeFileSync(join(dir, "lib.rs"), `use js::{SymbolFor, Unknown};

pub const REACT_FRAGMENT_TYPE: SymbolFor = SymbolFor("react.fragment");
pub const REACT_PROFILER_TYPE: SymbolFor = SymbolFor("react.profiler");
pub const REACT_TRACING_MARKER_TYPE: SymbolFor = SymbolFor("react.tracing_marker");

pub const A: u32 = 1;
pub const B: u32 = 2;
pub const C: u32 = 3;

pub fn name(ty: &Unknown, tracing: bool) -> Option<&'static str> {
    match SymbolFor::from_value(Some(ty)) {
        Some(REACT_FRAGMENT_TYPE) => return Some("Fragment"),
        Some(REACT_PROFILER_TYPE) => return Some("Profiler"),
        Some(REACT_TRACING_MARKER_TYPE) if tracing => return Some("TracingMarker"),
        _ => {}
    }
    None
}

pub fn middle(n: u32, on: bool) -> &'static str {
    match n {
        A => return "a",
        C if on => return "c",
        B => return "b",
        _ => {}
    }
    "none"
}

pub fn counts(n: u32, on: bool, log: &mut Vec<u32>) {
    match n {
        A => log.push(1),
        B => log.push(2),
        C if on => log.push(3),
        _ => {}
    }
    log.push(0);
}

pub fn looped(xs: &[u32], on: bool) -> u32 {
    let mut total = 0;
    for &x in xs {
        match x {
            A => total += 1,
            B => break,
            C if on => continue,
            _ => {}
        }
        total += 10;
    }
    total
}

#[derive(Clone, Copy)]
pub enum Tag {
    Function,
    Class,
    Host,
}

pub fn tag(t: Tag, on: bool) -> u32 {
    match t {
        Tag::Function => return 1,
        Tag::Class if on => return 2,
        Tag::Host => return 3,
        _ => {}
    }
    0
}

pub fn taken(n: u32, on: bool) -> u32 {
    match n {
        A => 10,
        C if on => 30,
        B => 20,
        _ => 0,
    }
}

pub fn again(n: u32, on: bool, log: &mut Vec<u32>) {
    match n {
        A => log.push(1),
        C if on => log.push(3),
        B => log.push(2),
        C => log.push(4),
        _ => {}
    }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs()]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain(
    '  switch (ty) {\n    case REACT_FRAGMENT_TYPE:\n      return "Fragment";\n    case REACT_PROFILER_TYPE:\n      return "Profiler";\n    case REACT_TRACING_MARKER_TYPE:\n      if (tracing) {\n        return "TracingMarker";\n      }\n  }\n',
  );
  expect(js).toContain('    case C:\n      if (on) {\n        return "c";\n      }\n      break;\n    case B:\n      return "b";\n  }\n  return "none";\n');
  expect(js).toContain("    case C:\n      if (on) {\n        log.push(3);\n      }\n  }\n  log.push(0);\n");
  // An arm that breaks the loop, which goes on past the `match`: its arms
  // an `if` chain, no label the program didn't write (#389).
  expect(js).toContain("    } else if (x === B) {\n      break;\n    } else if (x === C && on) {\n      continue;\n    }\n");
  expect(js).toContain('    case "Class":\n      if (on) {\n        return 2;\n      }\n      break;\n    case "Host":\n      return 3;\n  }\n');
  // A later arm that could take the constant, `_ => 0`, `C => ..`: an `if` chain.
  expect(js).toMatch(/export function taken\(n, on\) \{\n  if \(n === A\)/);
  expect(js).toMatch(/export function again\(n, on, log\) \{\n  if \(n === A\)/);
  const lib = await import(join(dir, "lib.js"));
  const [fragment, tracing] = [Symbol.for("react.fragment"), Symbol.for("react.tracing_marker")];
  expect([lib.name(fragment, false), lib.name(tracing, true), lib.name(tracing, false), lib.name({}, true)]).toEqual([
    "Fragment",
    "TracingMarker",
    undefined,
    undefined,
  ]);
  expect([1, 2, 3, 4].flatMap((n) => [lib.middle(n, true), lib.middle(n, false)])).toEqual(["a", "a", "b", "b", "c", "none", "none", "none"]);
  const log: number[] = [];
  [1, 2, 3, 4].forEach((n) => [true, false].forEach((on) => lib.counts(n, on, log)));
  expect(log).toEqual([1, 0, 1, 0, 2, 0, 2, 0, 3, 0, 0, 0, 0]);
  expect([lib.looped([1, 3, 4, 2, 1], true), lib.looped([1, 3, 4, 2, 1], false)]).toEqual([21, 31]);
  expect(["Function", "Class", "Host"].flatMap((t) => [lib.tag(t, true), lib.tag(t, false)])).toEqual([1, 1, 2, 0, 3, 3]);
  expect([1, 2, 3, 4].flatMap((n) => [lib.taken(n, true), lib.taken(n, false)])).toEqual([10, 10, 20, 20, 30, 0, 0, 0]);
  const again: number[] = [];
  [1, 2, 3, 4].forEach((n) => [true, false].forEach((on) => lib.again(n, on, again)));
  expect(again).toEqual([1, 1, 2, 2, 3, 4]);
});

// ADR 0668: a `match` of what a value is, `js::classify(value)`, is a
// `switch (typeof value)`, each variant its `typeof`'s case, an or-pattern's
// stacked, `_` the `default`; `Kind::Object` and `Kind::Array` together
// `case "object"`, and an arm of `Kind::Array` alone, which `typeof`
// doesn't tell, an `if` of `Array.isArray(value)` before it. A symbol is a
// `Kind::Symbol`, and `Kind::Object` a `typeof value === "object"` not an
// array. React's `getIODescription` (shared/ReactIODescription.js), its
// `try` the `catch_unwind` of ADR 0660.
test("a match of what a value is is a switch of its typeof", () => {
  const dir = fixture("typeof-switches");
  writeFileSync(join(dir, "lib.rs"), `use js::{Kind, Unknown};
use std::panic::catch_unwind;

/// \`value[key]\`, where it's text.
fn text_at(value: &Unknown, key: &str) -> Option<String> {
    match js::classify(js::get(value, key)) {
        Kind::String(text) => Some(text.to_string()),
        _ => None,
    }
}

pub fn get_io_description(value: Option<&Unknown>) -> String {
    catch_unwind(|| {
        let Some(value) = value else {
            return String::new();
        };
        match js::classify(Some(value)) {
            Kind::Function(_) => text_at(value, "name").unwrap_or_default(),
            Kind::Object(_) | Kind::Array(_) => text_at(value, "url").unwrap_or_default(),
            Kind::String(text) => text.to_string(),
            Kind::Number(_) | Kind::BigInt(_) => js::string(Some(value)),
            _ => String::new(),
        }
    })
    .unwrap_or_default()
}

pub fn describe(value: Option<&Unknown>) -> String {
    match js::classify(value) {
        Kind::Undefined | Kind::Null => "nothing".to_string(),
        Kind::Array(items) => format!("{} items", items.len()),
        Kind::String(text) => format!("text {text}"),
        Kind::Number(_) => "a number".to_string(),
        Kind::BigInt(_) => "a bigint".to_string(),
        Kind::Bool(b) => format!("{b}"),
        Kind::Symbol(symbol) => symbol.description().unwrap_or_default(),
        Kind::Function(_) => "a function".to_string(),
        Kind::Object(object) => text_at(object, "name").unwrap_or_default(),
    }
}

pub fn weighed(value: &Unknown, on: bool) -> u32 {
    let mut n = 0;
    match js::classify(Some(value)) {
        Kind::Function(_) | Kind::String(_) => n += 1,
        Kind::Number(_) => n += 2,
        Kind::Bool(_) if on => n += 3,
        _ => {}
    }
    n
}

pub fn objects(value: &Unknown) -> u32 {
    match js::classify(Some(value)) {
        Kind::Object(_) | Kind::Array(_) => 1,
        _ => 0,
    }
}

pub fn listed(value: &Unknown) -> u32 {
    match js::classify(Some(value)) {
        Kind::String(_) => 1,
        Kind::Array(_) => 2,
        _ => 0,
    }
}

pub fn tally(values: &[&Unknown]) -> [u32; 4] {
    let (mut texts, mut arrays, mut numbers, mut others) = (0, 0, 0, 0);
    for &value in values {
        match js::classify(Some(value)) {
            Kind::String(_) => texts += 1,
            Kind::Array(_) => arrays += 1,
            Kind::Number(_) => numbers += 1,
            _ => others += 1,
        }
    }
    [texts, arrays, numbers, others]
}

pub fn named(value: &Unknown) -> String {
    match js::classify(Some(value)) {
        Kind::String(text) => text.to_string(),
        Kind::Number(_) => "a number".to_string(),
        Kind::Object(object) => text_at(object, "name").unwrap_or_default(),
        _ => String::new(),
    }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs()]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain(`export function getIoDescription(value) {
  try {
    if (value == null) {
      return "";
    }
    switch (typeof value) {
      case "function":
        return textAt(value, "name") ?? "";
      case "object":
        return textAt(value, "url") ?? "";
      case "string":
        return value;
      case "number":
      case "bigint":
        return String(value);
      default:
        return "";
    }
  } catch {
    return "";
  }
}`);
  expect(js).toContain(`export function describe(value) {
  if (value == null) {
    return "nothing";
  }
  if (Array.isArray(value)) {
    return \`\${value.length} items\`;
  }
  switch (typeof value) {
    case "string":
      return \`text \${value}\`;
    case "number":
      return "a number";
    case "bigint":
      return "a bigint";
    case "boolean":
      return String(value);
    case "symbol":
      return value.description ?? "";
    case "function":
      return "a function";
    case "object":
      return textAt(value, "name") ?? "";
  }
}`);
  expect(js).toContain(`    if (Array.isArray(value)) {
      arrays = (arrays + 1) >>> 0;
    } else {
      switch (typeof value) {
        case "string":
          texts = (texts + 1) >>> 0;
          break;
        case "number":
          numbers = (numbers + 1) >>> 0;
          break;
        default:
          others = (others + 1) >>> 0;
      }
    }`);
  // An or-pattern's cases as it names them; a guarded arm's, its guard in
  // it (ADR 0636).
  expect(js).toContain(`  switch (typeof value) {
    case "function":
    case "string":
      n++;
      break;
    case "number":
      n += 2;
      break;
    case "boolean":
      if (on) {
        n += 3;
      }
  }`);
  // An \`Object\` arm whose arrays are \`_\`'s: \`case "object"\` would take
  // them, so an \`if\` chain, \`Object\` tested as an object not an array.
  expect(js).toContain(`  if (typeof value === "object" && !Array.isArray(value)) {`);
  expect(js).not.toContain(`typeof value !== "string"`);
  // Of two cases, `"object"` and a \`default\`, or a \`"string"\` beside an
  // array's \`if\`: an \`if\` chain, as ADR 0629's \`switch\` is of three,
  // a value's of two arms its conditional (ADR 0237, amended).
  expect(js).toMatch(/export function objects\(value\) \{\n  return typeof value === "object" \? 1 : 0;/);
  expect(js).toMatch(/export function listed\(value\) \{\n  if \(/);
  expect(js.slice(js.indexOf("export function listed"), js.indexOf("export function tally"))).not.toContain("switch");
  writeFileSync(join(dir, "run.mjs"), `import { getIoDescription, describe, tally, named, objects, listed } from "./lib.js";
const throwing = { get url() { throw new TypeError("no url"); } };
function fetchUser() {}
const values = [throwing, Symbol("s"), { url: "https://react.dev", name: "dev" }, ["a"], "hello", fetchUser, 42, 7n, true, {}, null, undefined];
console.log(JSON.stringify(values.map(getIoDescription)));
const defined = values.filter((v) => v != null && v !== throwing);
console.log(JSON.stringify([...defined, null, undefined].map(describe)));
console.log(JSON.stringify(tally(defined)));
console.log(JSON.stringify(defined.map(named)));
console.log(JSON.stringify([defined.map(objects), defined.map(listed)]));
`);
  const ran = runSync([node!, join(dir, "run.mjs")], root, 30_000);
  expect(ran.stderr).toBe("");
  expect(ran.stdout).toBe(
    [
      `["","","https://react.dev","","hello","fetchUser","42","7","","","",""]`,
      `["s","dev","1 items","text hello","a function","a number","a bigint","true","","nothing","nothing"]`,
      `[1,1,1,6]`,
      `["","dev","","hello","","a number","","",""]`,
      `[[0,1,1,0,0,0,0,0,1],[0,0,2,1,0,0,0,0,0]]`,
      "",
    ].join("\n"),
  );
});

// ADR 0676: `js::classify` takes an `Option<&Unknown>`, and `js::Kind` has
// `Undefined` and `Null`, so one `match` tells every JS value as `typeof`
// and `=== null` do: `Undefined` is `case "undefined"`, and `Null` sits in
// `case "object"`, `if (value === null)`, or is it, with the objects an arm
// takes beside it. React's `describeValueForErrorMessage`
// (shared/ReactSerializationErrors.js), whose `null` is `"Null"`, its
// `Object.prototype.toString`'s, and `undefined` `"undefined"`. Of one
// that's there, `classify(Some(value))`, an object's test is what it was.
test("a match of what a value is tells undefined and null", () => {
  const dir = fixture("typeof-nullish");
  writeFileSync(join(dir, "lib.rs"), `use js::{Kind, Unknown};

pub fn describe_value(value: Option<&Unknown>) -> String {
    match js::classify(value) {
        Kind::String(text) => js::json::stringify(text),
        Kind::Null | Kind::Object(_) | Kind::Array(_) => {
            if js::array::is_array(value) {
                return "[...]".to_string();
            }
            js::object::to_string(value).unwrap()
        }
        Kind::Function(_) => "function".to_string(),
        _ => js::string(value),
    }
}

pub fn named(value: Option<&Unknown>) -> String {
    match js::classify(value) {
        Kind::Array(items) => format!("{} items", items.len()),
        Kind::Object(object) => js::string(js::get(object, "name")),
        Kind::Null => "null".to_string(),
        Kind::String(text) => text.to_string(),
        Kind::Undefined => "nothing".to_string(),
        _ => String::new(),
    }
}

pub fn parsed(value: Option<&Unknown>) -> String {
    match js::classify(value) {
        Kind::Null => "null".to_string(),
        Kind::String(text) => text.to_string(),
        Kind::Number(_) => "a number".to_string(),
        _ => String::new(),
    }
}

pub fn is_object(value: Option<&Unknown>) -> bool {
    matches!(js::classify(value), Kind::Object(_))
}

pub fn is_null(value: Option<&Unknown>) -> bool {
    matches!(js::classify(value), Kind::Null)
}

pub fn is_undefined(value: Option<&Unknown>) -> bool {
    matches!(js::classify(value), Kind::Undefined)
}

pub fn there(value: &Unknown) -> bool {
    matches!(js::classify(Some(value)), Kind::Object(_))
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs()]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain(`export function describeValue(value) {
  switch (typeof value) {
    case "string":
      return JSON.stringify(value);
    case "object":
      if (Array.isArray(value)) {
        return "[...]";
      }
      return Object.prototype.toString.call(value);
    case "function":
      return "function";
    default:
      return String(value);
  }
}`);
  expect(js).toContain(`export function named(value) {
  if (Array.isArray(value)) {
    return \`\${value.length} items\`;
  }
  switch (typeof value) {
    case "object":
      if (value === null) {
        return "null";
      }
      return String(value.name);
    case "string":
      return value;
    case "undefined":
      return "nothing";
    default:
      return "";
  }
}`);
  // Where no arm takes the objects, `null`'s `if` is before the `switch`.
  expect(js).toContain(`export function parsed(value) {
  if (value === null) {
    return "null";
  }
  switch (typeof value) {
    case "string":
      return value;
    case "number":
      return "a number";
    default:
      return "";
  }
}`);
  expect(js).toContain(`export function isObject(value) {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}`);
  expect(js).toContain(`export function isNull(value) {
  return value === null;
}`);
  expect(js).toContain(`export function isUndefined(value) {
  return value === undefined;
}`);
  expect(js).toContain(`export function there(value) {
  return typeof value === "object" && !Array.isArray(value);
}`);
  // Each `===`, never `== null`, which is both.
  expect(js).not.toMatch(/ [!=]= (null|undefined)/);
  writeFileSync(join(dir, "run.mjs"), `import * as lib from "./lib.js";
const values = [null, undefined, "a long string", ["a"], { name: "dev" }, new Date(0), () => 1, 7n, 1];
for (const f of [lib.describeValue, lib.named, lib.parsed, lib.isObject, lib.isNull, lib.isUndefined]) {
  console.log(JSON.stringify(values.map((v) => f(v))));
}
console.log(JSON.stringify(values.filter((v) => v != null).map((v) => lib.there(v))));
`);
  const ran = runSync([node!, join(dir, "run.mjs")], root, 30_000);
  expect(ran.stderr).toBe("");
  expect(ran.stdout).toBe(
    [
      `["[object Null]","undefined","\\"a long string\\"","[...]","[object Object]","[object Date]","function","7","1"]`,
      `["null","nothing","a long string","1 items","dev","undefined","","",""]`,
      `["null","","a long string","","","","","","a number"]`,
      `[false,false,false,false,true,true,false,false,false]`,
      `[true,false,false,false,false,false,false,false,false]`,
      `[false,true,false,false,false,false,false,false,false]`,
      `[false,false,true,true,false,false,false]`,
      "",
    ].join("\n"),
  );
});

// ADR 0676: `js::classify` takes an `Option<&Unknown>`, the one way: of a
// value that's there, `classify(Some(value))`, which rustc says of a
// `&Unknown`.
test("classify of a value that's there is of its Some", () => {
  const dir = fixture("classify-some");
  writeFileSync(join(dir, "lib.rs"), `pub fn text(value: &js::Unknown) -> bool {
    matches!(js::classify(value), js::Kind::String(_))
}
`);
  const out = runSync([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs()], root, 120_000);
  expect(out.code).not.toBe(0);
  expect(out.stderr).toContain("expected `Option<&Unknown>`, found `&Unknown`");
  expect(out.stderr).toContain("Some(value)");
});

// ADR 0668, amended: a test that a value is one of an or-pattern's
// variants is what they are together, as their owner, `untagged.rs`, tells:
// `Kind::Object(_) | Kind::Array(_)`, an object no array is or an array, is
// `typeof value === "object"`, as React's `getIODescription` tests
// `typeof value.request === "object" && value.request !== null`, where it
// was `Array.isArray(v) || (typeof v === "object" && !Array.isArray(v))`.
// An or-pattern that leaves out what one of its variants' tests does keeps
// that test: `Object | String` is an object no array is, or a string.
test("a test of an object or an array is typeof object", () => {
  const dir = fixture("typeof-unions");
  writeFileSync(join(dir, "lib.rs"), `use js::{Kind, Unknown};

pub fn requested(value: &Unknown) -> bool {
    if let Some(request) = js::get(value, "request")
        && let Kind::Object(_) | Kind::Array(_) = js::classify(Some(request))
    {
        return true;
    }
    false
}

pub fn objects(value: &Unknown) -> u32 {
    match js::classify(Some(value)) {
        Kind::Object(_) | Kind::Array(_) => 1,
        _ => 0,
    }
}

pub fn callable_or_object(value: &Unknown) -> bool {
    matches!(js::classify(Some(value)), Kind::Function(_) | Kind::Object(_) | Kind::Array(_))
}

pub fn plain_or_text(value: &Unknown) -> bool {
    matches!(js::classify(Some(value)), Kind::Object(_) | Kind::String(_))
}

pub fn listed_or_text(value: &Unknown) -> bool {
    matches!(js::classify(Some(value)), Kind::Array(_) | Kind::String(_))
}

pub fn maybe_object(value: Option<&Unknown>) -> bool {
    matches!(js::classify(value), Kind::Object(_) | Kind::Array(_))
}

pub fn any_object(value: Option<&Unknown>) -> bool {
    matches!(js::classify(value), Kind::Null | Kind::Object(_) | Kind::Array(_))
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs()]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain(`export function requested(value) {
  const request = value.request;
  if (request != null && typeof request === "object") {
    return true;
  }
  return false;
}`);
  expect(js).toContain(`export function objects(value) {
  return typeof value === "object" ? 1 : 0;
}`);
  expect(js).toContain(`return typeof value === "function" || typeof value === "object";`);
  // In the order the pattern names them (ADR 0780).
  expect(js).toContain(`return (typeof value === "object" && !Array.isArray(value)) || typeof value === "string";`);
  expect(js).toContain(`return Array.isArray(value) || typeof value === "string";`);
  // Of what may be `null`, an object that's not, and of `null` too, what
  // `typeof` says (ADR 0676).
  expect(js).toContain(`return typeof value === "object" && value !== null;`);
  expect(js).toContain(`export function anyObject(value) {
  return typeof value === "object";
}`);
  expect(js).not.toContain(`Array.isArray(value) || (typeof value === "object"`);
  writeFileSync(join(dir, "run.mjs"), `import { requested, objects, callableOrObject, plainOrText, listedOrText, maybeObject, anyObject } from "./lib.js";
function fetchUser() {}
const values = [{}, ["a"], "text", fetchUser, 42, 7n, true, Symbol("s"), new Map()];
console.log(JSON.stringify([
  [{ request: {} }, { request: [] }, { request: null }, {}, { request: "x" }, { request: fetchUser }].map(requested),
  values.map(objects),
  values.map(callableOrObject),
  values.map(plainOrText),
  values.map(listedOrText),
  [...values, null, undefined].map(maybeObject),
  [...values, null, undefined].map(anyObject),
]));
`);
  const ran = runSync([node!, join(dir, "run.mjs")], root, 30_000);
  expect(ran.stderr).toBe("");
  expect(ran.stdout).toBe(
    `[[true,true,false,false,false,false],[1,1,0,0,0,0,0,0,1],[true,true,false,true,false,false,false,false,true],[true,false,true,false,false,false,false,false,true],[false,true,true,false,false,false,false,false,false],[true,true,false,false,false,false,false,false,true,false,false],[true,true,false,false,false,false,false,false,true,true,false]]\n`,
  );
});

// ADR 0668, amended: an arm that returns early, `if text.len() < 5 { return
// String::new(); }`, is still its case. MIR tests `Kind::String` first, and
// what's written before it, the `Function` and `Object` arms, was put first
// as an `if (typeof value !== "string") { .. }` with the string's statements
// after it, which no `switch` reads. A switch's arm's test stays as MIR
// tests it: the `switch` writes the cases as the arms are. React's
// `getIODescription`, whose string case tests its length.
test("an arm that returns early is still its case of a switch of a typeof", () => {
  const dir = fixture("typeof-switch-returns");
  writeFileSync(join(dir, "lib.rs"), `use js::{Kind, Unknown};

pub fn described(value: &Unknown) -> String {
    match js::classify(Some(value)) {
        Kind::Function(_) => match js::classify(js::get(value, "name")) {
            Kind::String(name) => name.to_string(),
            _ => String::new(),
        },
        Kind::Object(_) | Kind::Array(_) => {
            if let Kind::String(url) = js::classify(js::get(value, "url")) {
                return url.to_string();
            }
            let text = js::string(Some(value));
            if text.starts_with("[object ") {
                return String::new();
            }
            text
        }
        Kind::String(text) => {
            if text.len() < 5 {
                return String::new();
            }
            text.to_string()
        }
        Kind::Number(_) | Kind::BigInt(_) => js::string(Some(value)),
        _ => String::new(),
    }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs()]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain(`export function described(value) {
  switch (typeof value) {
    case "function": {
      const name = value.name;
      return typeof name === "string" ? name : "";
    }
    case "object": {`);
  expect(js).toContain(`      return text;
    }
    case "string":
      if ($byteLen(value) < 5) {
        return "";
      }
      return value;
    case "number":
    case "bigint":
      return String(value);
    default:
      return "";
  }
}`);
  expect(js).not.toContain(`typeof value !== "string"`);
  installRuntime(dir);
  writeFileSync(join(dir, "run.mjs"), `import { described } from "./lib.js";
function fetchUser() {}
const values = [Symbol("s"), ["a"], ["https://react.dev/learn"], { url: "/learn" }, {}, "hello", "hi", fetchUser, 42, 7n, true];
console.log(JSON.stringify(values.map(described)));
`);
  const ran = runSync([node!, join(dir, "run.mjs")], root, 30_000);
  expect(ran.stderr).toBe("");
  expect(ran.stdout).toBe(`["","a","https://react.dev/learn","/learn","","hello","","fetchUser","42","7",""]\n`);
});

// React's `getIODescription` (shared/ReactIODescription.js), its port whole,
// is upstream's `switch (typeof value)`, its arms in the program's order:
// its string case tests a length in UTF-16 units, `value.length` (ADR 0632),
// its object case an object or an array, `typeof request === "object"`
// (ADR 0668), each property read once (ADR 0672). Run in Node beside what
// upstream's JS gives of each value: a symbol, a throwing getter, arrays,
// an `Error`, each property, each kind.
test("React's getIODescription is upstream's switch of a typeof", () => {
  const dir = fixture("io-description");
  writeFileSync(join(dir, "lib.rs"), `use std::panic::catch_unwind;

use js::{JsError, Kind, Unknown};

pub fn get_io_description(value: Option<&Unknown>) -> String {
    catch_unwind(|| {
        let Some(value) = value else {
            return String::new();
        };
        match js::classify(Some(value)) {
            Kind::Function(_) => {
                if let Kind::String(name) = js::classify(js::get(value, "name")) {
                    return name.to_string();
                }
                String::new()
            }
            Kind::Object(_) | Kind::Array(_) => {
                // Test the object for a bunch of common property names that are useful identifiers.
                if unsafe { js::cast::<&JsError>(Some(value)) }.is_error() {
                    return js::string(js::get(value, "message"));
                }
                if let Kind::String(url) = js::classify(js::get(value, "url")) {
                    return url.to_string();
                }
                if let Kind::String(href) = js::classify(js::get(value, "href")) {
                    return href.to_string();
                }
                if let Kind::String(src) = js::classify(js::get(value, "src")) {
                    return src.to_string();
                }
                if let Kind::String(current_src) = js::classify(js::get(value, "currentSrc")) {
                    return current_src.to_string();
                }
                if let Kind::String(command) = js::classify(js::get(value, "command")) {
                    return command.to_string();
                }
                if let Some(request) = js::get(value, "request")
                    && let Kind::Object(_) | Kind::Array(_) = js::classify(Some(request))
                    && let Kind::String(url) = js::classify(js::get(request, "url"))
                {
                    return url.to_string();
                }
                if let Some(response) = js::get(value, "response")
                    && let Kind::Object(_) | Kind::Array(_) = js::classify(Some(response))
                    && let Kind::String(url) = js::classify(js::get(response, "url"))
                {
                    return url.to_string();
                }
                if let Some(id) = js::get(value, "id")
                    && let Kind::String(_) | Kind::Number(_) | Kind::BigInt(_) = js::classify(Some(id))
                {
                    return js::string(Some(id));
                }
                if let Kind::String(name) = js::classify(js::get(value, "name")) {
                    return name.to_string();
                }
                let str = js::string(Some(value));
                if str.starts_with("[object ")
                    || str.encode_utf16().count() < 5
                    || str.encode_utf16().count() > 500
                {
                    // This is probably not a useful description.
                    return String::new();
                }
                str
            }
            Kind::String(text) => {
                if text.encode_utf16().count() < 5 || text.encode_utf16().count() > 500 {
                    return String::new();
                }
                text.to_string()
            }
            Kind::Number(_) | Kind::BigInt(_) => js::string(Some(value)),
            // Not useful descriptors.
            _ => String::new(),
        }
    })
    .unwrap_or_default()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs()]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain(`export function getIoDescription(value) {
  try {
    if (value == null) {
      return "";
    }
    switch (typeof value) {
      case "function": {
        const name = value.name;
        if (typeof name === "string") {
          return name;
        }
        return "";
      }
      case "object": {
        // Test the object for a bunch of common property names that are useful identifiers.
        if (value instanceof Error) {
          return String(value.message);
        }
        const url = value.url;
        if (typeof url === "string") {
          return url;
        }
        const href = value.href;
        if (typeof href === "string") {
          return href;
        }
        const src = value.src;
        if (typeof src === "string") {
          return src;
        }
        const currentSrc = value.currentSrc;
        if (typeof currentSrc === "string") {
          return currentSrc;
        }
        const command = value.command;
        if (typeof command === "string") {
          return command;
        }
        const request = value.request;
        if (request != null && typeof request === "object") {
          const url = request.url;
          if (typeof url === "string") {
            return url;
          }
        }
        const response = value.response;
        if (response != null && typeof response === "object") {
          const url = response.url;
          if (typeof url === "string") {
            return url;
          }
        }
        const id = value.id;
        if (typeof id === "string" || typeof id === "number" || typeof id === "bigint") {
          return String(id);
        }
        const name = value.name;
        if (typeof name === "string") {
          return name;
        }
        const str = String(value);
        if (str.startsWith("[object ") || str.length < 5 || str.length > 500) {
          // This is probably not a useful description.
          return "";
        }
        return str;
      }
      case "string":
        if (value.length < 5 || value.length > 500) {
          return "";
        }
        return value;
      case "number":
      case "bigint":
        return String(value);
      default:
        // Not useful descriptors.
        return "";
    }
  } catch {
    return "";
  }
}`);
  expect(js).not.toContain("import");
  writeFileSync(join(dir, "run.mjs"), `import { getIoDescription } from "./lib.js";
const throwing = { get url() { throw new TypeError("no url"); } };
const reads = [];
const counted = { get request() { reads.push("request"); return { get url() { reads.push("url"); return "/counted"; } }; } };
function fetchUser() {}
const values = [
  throwing, Symbol("s"), ["a", "b"], ["https://react.dev/learn"], new TypeError("failed"),
  { url: "https://react.dev" }, { href: "/learn" }, { currentSrc: "/video.mp4" }, { command: "ls" }, { request: { url: "/api" } }, { request: null, response: { url: "/res" } },
  { request: ["x"], id: 7 }, { id: 8n }, { id: "user-1" }, { name: "dev" }, { toString: () => "a described thing" },
  {}, "hello", "hi", "\\u{1F600}\\u{1F600}\\u{1F600}", "x".repeat(501), fetchUser, () => {}, 42, 7n, true, null, undefined,
  counted,
];
console.log(JSON.stringify([values.map(getIoDescription), reads]));
`);
  const ran = runSync([node!, join(dir, "run.mjs")], root, 30_000);
  expect(ran.stderr).toBe("");
  // Upstream's, of each value but the last, which upstream reads more
  // often than once.
  expect(JSON.parse(ran.stdout)).toEqual([
    [
      "", "", "", "https://react.dev/learn", "failed", "https://react.dev", "/learn", "/video.mp4", "ls", "/api", "/res", "7", "8",
      "user-1", "dev", "a described thing", "", "hello", "", "\u{1F600}\u{1F600}\u{1F600}", "", "fetchUser", "", "42",
      "7", "", "", "", "/counted",
    ],
    ["request", "url"],
  ]);
});

// ADR 0743: a part of a chain of tests that an earlier test decides is
// left out of it: an arm of `Kind::Array(..)` whose pattern fails, then one
// of `Kind::Object(_)`, is `if (Array.isArray(value)) { .. } else if
// (typeof value !== "object" || value === null)`, not `.. ||
// Array.isArray(value)` again. React's `getArrayKind`
// (shared/ReactPerformanceTrackProperties.js).
test("a part of a chain an earlier test decides is left out", async () => {
  const dir = fixture("decided-parts");
  writeFileSync(join(dir, "lib.rs"), `use js::{Kind, Unknown};

pub fn kind(values: &[Option<&Unknown>]) -> u32 {
    let mut n = 0;
    for &value in values {
        match js::classify(value) {
            Kind::Array(&[first, _]) if matches!(js::classify(first), Kind::String(_)) => n += 1,
            Kind::Object(_) | Kind::Array(_) => return 100,
            Kind::Function(_) => return 200,
            _ => n += 10,
        }
    }
    n
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs()]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("Array.isArray(value)");
  expect(js.match(/Array\.isArray\(value\)/g)?.length).toBe(1);
  const lib = await import(join(dir, "lib.js"));
  expect([lib.kind([["a", 1], 3, "x"]), lib.kind([[1, 2]]), lib.kind([{}]), lib.kind([() => {}]), lib.kind([null, undefined])])
    .toEqual([21, 100, 100, 200, 20]);
});

// ADR 0364, amended: a value some arms give, `invoke = true`, isn't given
// first where an arm that does nothing, `_ => {}`, goes on past the match
// without it. React's `mapIntoArray` (ReactChildren.js) set `invokeCallback
// = true` before testing `$$typeof`, so every object called back (issue
// #219); it's upstream's `switch (children.$$typeof)` now.
test("React's mapIntoArray's invokeCallback is set in its cases of $$typeof", async () => {
  const dir = fixture("invoke-callback");
  writeFileSync(join(dir, "lib.rs"), `use js::{SymbolFor, Unknown};

pub const REACT_ELEMENT_TYPE: SymbolFor = SymbolFor("react.transitional.element");
pub const REACT_PORTAL_TYPE: SymbolFor = SymbolFor("react.portal");
pub const REACT_LAZY_TYPE: SymbolFor = SymbolFor("react.lazy");

pub fn kind(children: Option<&'static Unknown>) -> u32 {
    let r#type = js::type_of(children);
    let mut invoke_callback = false;
    if children.is_none() {
        invoke_callback = true;
    } else {
        match r#type.as_str() {
            "bigint" | "string" | "number" => invoke_callback = true,
            "object" => match SymbolFor::from_value(js::get(children, "$$typeof")) {
                Some(REACT_ELEMENT_TYPE | REACT_PORTAL_TYPE) => invoke_callback = true,
                Some(REACT_LAZY_TYPE) => return 2,
                _ => {}
            },
            _ => {}
        }
    }
    if invoke_callback {
        return 1;
    }
    0
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs()]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain(`    switch (type) {
      case "bigint":
      case "string":
      case "number":
        invokeCallback = true;
        break;
      case "object":
        switch (children.$$typeof) {
          case REACT_ELEMENT_TYPE:
          case REACT_PORTAL_TYPE:
            invokeCallback = true;
            break;
          case REACT_LAZY_TYPE:
            return 2;
        }
    }`);
  const lib = await import(join(dir, "lib.js"));
  const of = (name: string) => ({ $$typeof: Symbol.for(name) });
  const values = [undefined, null, "a", 1, 2n, of("react.transitional.element"), of("react.portal"), of("react.lazy")];
  expect(values.map((v) => lib.kind(v))).toEqual([1, 1, 1, 1, 1, 1, 1, 2]);
  expect([{}, of("react.memo"), [], true, () => {}].map((v) => lib.kind(v))).toEqual([0, 0, 0, 0, 0]);
});

// ADR 0364, amended: each way out of a match of symbols, whose tree tidy
// can't chain, that could tell a value some arms give was given first.
// The arm that does nothing; a loop's `break` and `continue`; a closure's
// `return`; a panic `catch_unwind` takes; a call, and a guard's, reading
// the variable through a closure. Each is what native Rust gives of the
// same program of `Option<u32>`s.
test("a value some arms of a match of symbols give is given where they give it", () => {
  const dir = fixture("default-value-symbols");
  writeFileSync(join(dir, "lib.rs"), `use std::panic::{self, AssertUnwindSafe};

use js::{SymbolFor, Unknown};

pub const A: SymbolFor = SymbolFor("a");
pub const B: SymbolFor = SymbolFor("b");
pub const C: SymbolFor = SymbolFor("c");

fn tag(value: &'static Unknown) -> Option<SymbolFor<'static>> {
    SymbolFor::from_value(Some(value))
}

pub fn kept(values: &[&'static Unknown]) -> Vec<bool> {
    let mut hit = false;
    let mut seen = Vec::new();
    for &value in values {
        match tag(value) {
            Some(A | B) => hit = true,
            Some(C) => {}
            _ => hit = false,
        }
        seen.push(hit);
    }
    seen
}

pub fn continued(values: &[&'static Unknown]) -> Vec<bool> {
    let mut hit = false;
    let mut seen = Vec::new();
    for &value in values {
        match tag(value) {
            Some(A | B) => hit = true,
            Some(C) => continue,
            _ => hit = false,
        }
        seen.push(hit);
    }
    seen.push(hit);
    seen
}

pub fn broken(values: &[&'static Unknown]) -> Vec<bool> {
    let mut hit = false;
    let mut seen = Vec::new();
    for &value in values {
        match tag(value) {
            Some(A | B) => hit = true,
            Some(C) => break,
            _ => hit = false,
        }
        seen.push(hit);
    }
    seen.push(hit);
    seen
}

pub fn returned(values: &[&'static Unknown]) -> Vec<bool> {
    let mut hit = false;
    let mut seen = Vec::new();
    for &value in values {
        let mut step = || {
            match tag(value) {
                Some(A | B) => hit = true,
                Some(C) => return,
                _ => hit = false,
            }
            seen.push(hit);
        };
        step();
        seen.push(hit);
    }
    seen
}

pub fn unwound(values: &[&'static Unknown]) -> Vec<bool> {
    let mut hit = false;
    let mut seen = Vec::new();
    for &value in values {
        let caught = panic::catch_unwind(AssertUnwindSafe(|| {
            match tag(value) {
                Some(A | B) => hit = true,
                Some(C) => panic!("a c"),
                _ => hit = false,
            }
            seen.push(hit);
        }));
        seen.push(caught.is_ok());
        seen.push(hit);
    }
    seen
}

pub fn observed(values: &[&'static Unknown]) -> Vec<u32> {
    let mut seen = Vec::new();
    for &value in values {
        let mut level = 0;
        let look = || level * 10;
        match tag(value) {
            Some(A | B) => level = 1,
            Some(C) => level = 2,
            _ => {
                seen.push(look());
                level = 3;
            }
        }
        seen.push(level);
    }
    seen
}

pub fn guarded(values: &[&'static Unknown], limit: u32) -> Vec<u32> {
    let mut seen = Vec::new();
    for &value in values {
        let mut level = limit;
        let below = |n: u32| n < level;
        match tag(value) {
            Some(A | B) => level = 1,
            Some(C) if below(3) => level = 3,
            _ => level = 5,
        }
        seen.push(level);
    }
    seen
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs()]);
  installRuntime(dir);
  writeFileSync(join(dir, "run.mjs"), `import * as lib from "./lib.js";
const [a, b, c, x] = ["a", "b", "c", "x"].map((name) => Symbol.for(name));
const values = [a, c, x, a, {}, x, b, c];
console.log(JSON.stringify([
  lib.kept(values),
  lib.continued(values),
  lib.broken(values),
  lib.returned(values),
  lib.unwound(values),
  lib.observed(values),
  [lib.guarded(values, 9), lib.guarded(values, 2)],
]));
`);
  const ran = runSync([node!, join(dir, "run.mjs")], root, 30_000);
  // Native Rust's, of `Some(1)`, `Some(2)`, `Some(3)` and `Some(9)` for `a`,
  // `b`, `c` and `x`, and `None` for `{}`.
  expect(JSON.parse(ran.stdout)).toEqual([
    [true, true, false, true, false, false, true, true],
    [true, false, true, false, false, true, true],
    [true, true],
    [true, true, true, false, false, true, true, false, false, false, false, true, true, true],
    [true, true, true, false, true, false, true, false, true, true, true, false, true, false, false, true, false, true, true, true, false, true],
    [1, 2, 0, 3, 1, 0, 3, 0, 3, 1, 2],
    [[1, 3, 5, 1, 5, 5, 1, 3], [1, 5, 5, 1, 5, 5, 1, 5]],
  ]);
});

// ADR 0629, amended: React's `resolveThenable` (ReactChildren.js). A last
// `_`'s statements, which MIR placed after the chain of the arms before
// it, are its `default`, up to its last statement written there; what
// follows the `match` stays after the `switch`. And a `match` of an
// `Option`'s text, `Some("fulfilled")`, whose `_ => {}` does nothing, is a
// `switch` of the value as JS writes it: no test of `None`, which no case
// takes, no temporary and no `default: break;`.
test("React's resolveThenable is upstream's switch, its last arm its default", async () => {
  const dir = fixture("thenable-switch");
  writeFileSync(join(dir, "lib.rs"), `use std::marker::PhantomData;

use js::{JsObject, Unknown};

pub struct Thenable(PhantomData<JsObject>);

unsafe impl js::Defined for Thenable {}

impl Thenable {
    #[cfg_attr(rust_js, rust_js::link_name = "get status")]
    pub fn status(&self) -> Option<String> {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "set status")]
    pub fn set_status(&self, status: &str) {
        let _ = status;
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "get value")]
    pub fn value(&self) -> Option<&'static Unknown> {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "get reason")]
    pub fn reason(&self) -> Option<&'static Unknown> {
        unreachable!()
    }
}

pub fn resolve(thenable: &'static Thenable) -> Option<&'static Unknown> {
    match thenable.status().as_deref() {
        Some("fulfilled") => return thenable.value(),
        Some("rejected") => js::throw(thenable.reason()),
        _ => {
            if js::type_of(js::get(thenable, "status")) == "string" {
                thenable.set_status("seen");
            } else {
                thenable.set_status("pending");
            }

            // Check one more time in case the thenable resolved synchronously.
            match thenable.status().as_deref() {
                Some("fulfilled") => return thenable.value(),
                Some("rejected") => js::throw(thenable.reason()),
                _ => {}
            }
        }
    }
    js::throw(Some(js::unknown(thenable)))
}

pub fn check(thenable: &'static Thenable) -> Option<&'static Unknown> {
    thenable.set_status("pending");
    match thenable.status().as_deref() {
        Some("fulfilled") => return thenable.value(),
        Some("rejected") => js::throw(thenable.reason()),
        _ => {}
    }
    js::throw(Some(js::unknown(thenable)))
}

pub fn code(thenable: &'static Thenable) -> u32 {
    match thenable.status().as_deref() {
        Some("fulfilled") => 1,
        Some("rejected") => 2,
        _ => {
            thenable.set_status("seen");
            3
        }
    }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs()]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain(`export function resolve(thenable) {
  switch (thenable.status) {
    case "fulfilled":
      return thenable.value;
    case "rejected":
      throw thenable.reason;
    default:
      if (typeof thenable.status === "string") {
        thenable.status = "seen";
      } else {
        thenable.status = "pending";
      }

      // Check one more time in case the thenable resolved synchronously.
      switch (thenable.status) {
        case "fulfilled":
          return thenable.value;
        case "rejected":
          throw thenable.reason;
      }
  }
  throw thenable;
}`);
  expect(js).toContain(`    default:
      thenable.status = "seen";
      return 3;
  }
}`);
  expect(js).toContain(`export function check(thenable) {
  thenable.status = "pending";
  switch (thenable.status) {
    case "fulfilled":
      return thenable.value;
    case "rejected":
      throw thenable.reason;
  }
  throw thenable;
}`);
  const lib = await import(join(dir, "lib.js"));
  const thrown = (f: () => unknown) => {
    try {
      return ["returned", f()];
    } catch (e) {
      return ["threw", e];
    }
  };
  const settling = { status: "x", set value(_: unknown) {} } as { status: unknown; value?: unknown };
  const later = {
    _status: undefined as unknown,
    get status() {
      return this._status;
    },
    set status(s: unknown) {
      this._status = s === "pending" ? "fulfilled" : s;
    },
    value: 7,
  };
  expect([
    thrown(() => lib.resolve({ status: "fulfilled", value: 1 })),
    thrown(() => lib.resolve({ status: "rejected", reason: 2 })),
    thrown(() => lib.resolve(later)),
  ]).toEqual([["returned", 1], ["threw", 2], ["returned", 7]]);
  const pending = {} as { status?: string };
  expect(thrown(() => lib.resolve(pending))).toEqual(["threw", pending]);
  expect(pending.status).toBe("pending");
  expect(thrown(() => lib.resolve(settling))).toEqual(["threw", settling]);
  expect(settling.status).toBe("seen");
  const checked = { status: "x" } as { status?: string };
  expect([thrown(() => lib.check(checked)), checked.status]).toEqual([["threw", checked], "pending"]);
  const unknown = { status: undefined } as { status?: string };
  expect([lib.code({ status: "fulfilled" }), lib.code({ status: "rejected" }), lib.code(unknown), unknown.status]).toEqual([1, 2, 3, "seen"]);
});

// An or-pattern's cases of an `Option`'s strings, `x === "d" || x ===
// "i"`, each say `x != null`, so the `None` arm beside them is the
// `switch`'s `default`, not an early return (#255).
test("an or-pattern of an Option's strings leaves its None arm the default", () => {
  const dir = fixture("switch-or-pattern-default");
  writeFileSync(join(dir, "lib.rs"), `pub fn either(flag: Option<&str>, v: &mut Vec<u32>) {
    match flag {
        Some("d") | Some("i") => v.push(1),
        _ => v.push(2),
    }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  expect(readFileSync(join(dir, "mir.js"), "utf8")).toContain(`export function either(flag, v) {
  switch (flag) {
    case "d":
    case "i":
      v.push(1);
      break;
    default:
      v.push(2);
  }
}`);
});

// ADR 0956: React's commitAttachRef, a `switch` of a case per tag, two
// arms of which end in the same value: each case's statements as written,
// not their tail shared after a smaller `switch` of an `if` chain's `else`.
test("arms of a switch that end alike are each their case's", async () => {
  const dir = fixture("switch-arms-sharing-a-tail");
  writeFileSync(
    join(dir, "lib.rs"),
    `use std::cell::Cell;

pub struct Fiber {
    pub tag: Cell<u8>,
    pub state_node: Cell<Option<&'static js::Unknown>>,
}

pub const HOST_COMPONENT: u8 = 5;
pub const FRAGMENT: u8 = 7;
pub const VIEW_TRANSITION_COMPONENT: u8 = 30;
pub const ENABLE_VIEW_TRANSITION: bool = true;

#[cfg_attr(rust_js, rust_js::link_name = "rustJsPublic", rust_js::nullable)]
fn get_public_instance(instance: Option<&'static js::Unknown>) -> Option<&'static js::Unknown> {
    let _ = instance;
    unreachable!()
}

#[cfg_attr(rust_js, rust_js::link_name = "rustJsFragment")]
fn create_fragment_instance(fiber: &Fiber) -> &'static js::Unknown {
    let _ = fiber;
    unreachable!()
}

pub fn attach(finished_work: &Fiber) -> Option<&'static js::Unknown> {
    let instance_to_use = match finished_work.tag.get() {
        HOST_COMPONENT => get_public_instance(finished_work.state_node.get()),
        VIEW_TRANSITION_COMPONENT => {
            if ENABLE_VIEW_TRANSITION {
                get_public_instance(None)
            } else {
                finished_work.state_node.get()
            }
        }
        FRAGMENT => {
            if finished_work.state_node.get().is_none() {
                finished_work
                    .state_node
                    .set(Some(create_fragment_instance(finished_work)));
            }
            finished_work.state_node.get()
        }
        _ => finished_work.state_node.get(),
    };
    instance_to_use
}
`,
  );
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs()]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("  switch (finishedWork.tag) {\n    case HOST_COMPONENT:\n");
  expect(js).toContain(
    "    case FRAGMENT:\n      finishedWork.stateNode ??= rustJsFragment(finishedWork);\n      instanceToUse = finishedWork.stateNode;\n      break;\n    default:\n      instanceToUse = finishedWork.stateNode;\n  }\n",
  );
  const global = globalThis as Record<string, unknown>;
  global.rustJsPublic = (x: unknown) => ["public", x ?? null];
  global.rustJsFragment = () => "fragment";
  const lib = await import(join(dir, "lib.js"));
  const node = {};
  expect([
    lib.attach({ tag: 5, stateNode: node }),
    lib.attach({ tag: 30, stateNode: node }),
    lib.attach({ tag: 7, stateNode: null }),
    lib.attach({ tag: 7, stateNode: node }),
    lib.attach({ tag: 1, stateNode: node }),
  ]).toEqual([["public", node], ["public", null], "fragment", node, node]);
});

// ADR 0956, amended: React's unwindWork, `case SuspenseListComponent: {
// ..; if (..) { .. } return null; } default: return null;`, an arm whose
// statements go on to the value it ends with: that value its case's, as
// the program wrote it, not shared with `_`'s after the `switch`.
test("an arm of a switch that goes on to its value returns it, and `_`'s is its default", async () => {
  const dir = fixture("switch-arm-going-on-to-its-value");
  writeFileSync(
    join(dir, "lib.rs"),
    `#[cfg_attr(rust_js, rust_js::nullable)]
pub fn unwind(tag: u8, value: u32, log: &mut Vec<u32>) -> Option<u32> {
    match tag {
        5 => {
            log.push(value);
            None
        }
        25 => {
            if value > 2 {
                log.push(value);
            }
            None
        }
        _ => None,
    }
}
`,
  );
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs()]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain(
    "    case 25:\n      if (value > 2) {\n        log.push(value);\n      }\n      return null;\n    default:\n      return null;\n  }\n}\n",
  );
  const lib = await import(join(dir, "lib.js"));
  const log: number[] = [];
  expect([5, 25, 25, 1].map((tag, i) => lib.unwind(tag, i + 1, log))).toEqual([null, null, null, null]);
  expect(log).toEqual([1, 3]);
});

// ADR 0905, amended: React's popToNextHostParent, `while
// (hydrationParentFiber) { switch (hydrationParentFiber.tag) { case
// HostComponent: ..; return; case HostRoot: ..; return; default: .. } }`.
// MIR joins the arms' `return`s into one after the `switch`, which printed
// each case's `break;` and a `return;` after it: each case of an arm the
// program wrote a `return` in ends with it, as written, and the `default`
// that goes on, with no `continue`.
test("a case of an arm that returns ends with its return", async () => {
  const dir = fixture("switch-arms-returning");
  writeFileSync(
    join(dir, "lib.rs"),
    `use std::rc::Rc;

pub struct Fiber {
    pub tag: u8,
    pub parent: Option<Rc<Fiber>>,
}

pub fn pop_to_host(fiber: &Fiber, log: &mut Vec<u8>) {
    let mut parent = fiber.parent.clone();
    while let Some(current) = parent {
        match current.tag {
            5 | 13 => {
                log.push(1);
                return;
            }
            3 | 27 => {
                log.push(2);
                return;
            }
            _ => {
                parent = current.parent.clone();
            }
        }
    }
}

pub fn fiber(tag: u8, parent: Option<Rc<Fiber>>) -> Rc<Fiber> {
    Rc::new(Fiber { tag, parent })
}
`,
  );
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs()]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain(
    "      case 27:\n        log.push(2);\n        return;\n      default:\n        parent = parent.parent;\n    }\n  }\n}\n",
  );
  const lib = await import(join(dir, "lib.js"));
  const log: number[] = [];
  const root = lib.fiber(3, null);
  const host = lib.fiber(13, lib.fiber(1, root));
  for (const parent of [null, root, host, lib.fiber(1, lib.fiber(2, null))]) {
    lib.popToHost(lib.fiber(0, parent), log);
  }
  expect(log).toEqual([2, 1]);
});
