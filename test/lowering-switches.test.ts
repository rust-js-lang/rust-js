// What the compiler writes of a `match` of constants as a `switch` (ADR
// 0629): its arms' cases, a guarded one's too. Each test is a program of its
// own, compiled and run.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { runSync } from "./child";
import { node } from "./programs";
import { buildReact, compiler, fixture, root, run, target } from "./support";

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
  expect(js).toContain("        break loop;\n      case C:\n        if (on) {\n          continue;\n        }\n    }\n");
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
  // array's \`if\`: an \`if\` chain, as ADR 0629's \`switch\` is of three.
  expect(js).toMatch(/export function objects\(value\) \{\n  if \(/);
  expect(js).toMatch(/export function listed\(value\) \{\n  if \(/);
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
