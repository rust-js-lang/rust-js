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
    '  switch (ty) {\n    case REACT_FRAGMENT_TYPE:\n      return "Fragment";\n    case REACT_PROFILER_TYPE:\n      return "Profiler";\n    case REACT_TRACING_MARKER_TYPE:\n      if (tracing) {\n        return "TracingMarker";\n      }\n      break;\n  }\n',
  );
  expect(js).toContain('    case C:\n      if (on) {\n        return "c";\n      }\n      break;\n    case B:\n      return "b";\n  }\n  return "none";\n');
  expect(js).toContain("    case C:\n      if (on) {\n        log.push(3);\n      }\n      break;\n  }\n  log.push(0);\n");
  expect(js).toContain("        break loop;\n      case C:\n        if (on) {\n          continue;\n        }\n        break;\n    }\n");
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
    match js::get(value, key).map(js::classify) {
        Some(Kind::String(text)) => Some(text.to_string()),
        _ => None,
    }
}

pub fn get_io_description(value: Option<&Unknown>) -> String {
    catch_unwind(|| {
        let Some(value) = value else {
            return String::new();
        };
        match js::classify(value) {
            Kind::Function(_) => text_at(value, "name").unwrap_or_default(),
            Kind::Object(_) | Kind::Array(_) => text_at(value, "url").unwrap_or_default(),
            Kind::String(text) => text.to_string(),
            Kind::Number(_) | Kind::BigInt(_) => js::string(Some(value)),
            _ => String::new(),
        }
    })
    .unwrap_or_default()
}

pub fn describe(value: &Unknown) -> String {
    match js::classify(value) {
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
    match js::classify(value) {
        Kind::Function(_) | Kind::String(_) => n += 1,
        Kind::Number(_) => n += 2,
        Kind::Bool(_) if on => n += 3,
        _ => {}
    }
    n
}

pub fn objects(value: &Unknown) -> u32 {
    match js::classify(value) {
        Kind::Object(_) | Kind::Array(_) => 1,
        _ => 0,
    }
}

pub fn listed(value: &Unknown) -> u32 {
    match js::classify(value) {
        Kind::String(_) => 1,
        Kind::Array(_) => 2,
        _ => 0,
    }
}

pub fn tally(values: &[&Unknown]) -> [u32; 4] {
    let (mut texts, mut arrays, mut numbers, mut others) = (0, 0, 0, 0);
    for value in values {
        match js::classify(value) {
            Kind::String(_) => texts += 1,
            Kind::Array(_) => arrays += 1,
            Kind::Number(_) => numbers += 1,
            _ => others += 1,
        }
    }
    [texts, arrays, numbers, others]
}

pub fn named(value: &Unknown) -> String {
    match js::classify(value) {
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
          break;
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
      break;
  }`);
  // An \`Object\` arm whose arrays are \`_\`'s: \`case "object"\` would take
  // them, so an \`if\` chain, \`Object\` tested as an object not an array.
  expect(js).toContain(`  if (typeof value === "object" && !Array.isArray(value)) {`);
  expect(js).not.toContain(`typeof value !== "string"`);
  // Of two cases, `"object"` and a \`default\`, or a \`"string"\` beside an
  // array's \`if\`: an \`if\` chain, as ADR 0629's \`switch\` is of three.
  expect(js).toMatch(/export function objects\(value\) \{\n  if \(/);
  expect(js).toMatch(/export function listed\(value\) \{\n  if \(/);
  expect(js.slice(js.indexOf("export function listed"), js.indexOf("export function tally"))).not.toContain("switch");
  writeFileSync(join(dir, "run.mjs"), `import { getIoDescription, describe, tally, named, objects, listed } from "./lib.js";
const throwing = { get url() { throw new TypeError("no url"); } };
function fetchUser() {}
const values = [throwing, Symbol("s"), { url: "https://react.dev", name: "dev" }, ["a"], "hello", fetchUser, 42, 7n, true, {}, null, undefined];
console.log(JSON.stringify(values.map(getIoDescription)));
const defined = values.filter((v) => v != null && v !== throwing);
console.log(JSON.stringify(defined.map(describe)));
console.log(JSON.stringify(tally(defined)));
console.log(JSON.stringify(defined.map(named)));
console.log(JSON.stringify([defined.map(objects), defined.map(listed)]));
`);
  const ran = runSync([node!, join(dir, "run.mjs")], root, 30_000);
  expect(ran.stderr).toBe("");
  expect(ran.stdout).toBe(
    [
      `["","","https://react.dev","","hello","fetchUser","42","7","","","",""]`,
      `["s","dev","1 items","text hello","a function","a number","a bigint","true",""]`,
      `[1,1,1,6]`,
      `["","dev","","hello","","a number","","",""]`,
      `[[0,1,1,0,0,0,0,0,1],[0,0,2,1,0,0,0,0,0]]`,
      "",
    ].join("\n"),
  );
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
        && let Kind::Object(_) | Kind::Array(_) = js::classify(request)
    {
        return true;
    }
    false
}

pub fn objects(value: &Unknown) -> u32 {
    match js::classify(value) {
        Kind::Object(_) | Kind::Array(_) => 1,
        _ => 0,
    }
}

pub fn callable_or_object(value: &Unknown) -> bool {
    matches!(js::classify(value), Kind::Function(_) | Kind::Object(_) | Kind::Array(_))
}

pub fn plain_or_text(value: &Unknown) -> bool {
    matches!(js::classify(value), Kind::Object(_) | Kind::String(_))
}

pub fn listed_or_text(value: &Unknown) -> bool {
    matches!(js::classify(value), Kind::Array(_) | Kind::String(_))
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
  if (typeof value === "object") {
    return 1;
  }
  return 0;
}`);
  expect(js).toContain(`return typeof value === "function" || typeof value === "object";`);
  expect(js).toContain(`return typeof value === "string" || (typeof value === "object" && !Array.isArray(value));`);
  expect(js).toContain(`return typeof value === "string" || Array.isArray(value);`);
  expect(js).not.toContain(`Array.isArray(value) || (typeof value === "object"`);
  writeFileSync(join(dir, "run.mjs"), `import { requested, objects, callableOrObject, plainOrText, listedOrText } from "./lib.js";
function fetchUser() {}
const values = [{}, ["a"], "text", fetchUser, 42, 7n, true, Symbol("s"), new Map()];
console.log(JSON.stringify([
  [{ request: {} }, { request: [] }, { request: null }, {}, { request: "x" }, { request: fetchUser }].map(requested),
  values.map(objects),
  values.map(callableOrObject),
  values.map(plainOrText),
  values.map(listedOrText),
]));
`);
  const ran = runSync([node!, join(dir, "run.mjs")], root, 30_000);
  expect(ran.stderr).toBe("");
  expect(ran.stdout).toBe(
    `[[true,true,false,false,false,false],[1,1,0,0,0,0,0,0,1],[true,true,false,true,false,false,false,false,true],[true,false,true,false,false,false,false,false,true],[false,true,true,false,false,false,false,false,false]]\n`,
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
    match js::classify(value) {
        Kind::Function(_) => match js::get(value, "name").map(js::classify) {
            Some(Kind::String(name)) => name.to_string(),
            _ => String::new(),
        },
        Kind::Object(_) | Kind::Array(_) => {
            if let Some(Kind::String(url)) = js::get(value, "url").map(js::classify) {
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
      if (typeof name === "string") {
        return name;
      }
      return "";
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
        match js::classify(value) {
            Kind::Function(_) => match js::get(value, "name").map(js::classify) {
                Some(Kind::String(name)) => name.to_string(),
                _ => String::new(),
            },
            Kind::Object(_) | Kind::Array(_) => {
                // Test the object for a bunch of common property names that are useful identifiers.
                if unsafe { js::cast::<&JsError>(Some(value)) }.is_error() {
                    return js::string(js::get(value, "message"));
                }
                if let Some(Kind::String(url)) = js::get(value, "url").map(js::classify) {
                    return url.to_string();
                }
                if let Some(Kind::String(href)) = js::get(value, "href").map(js::classify) {
                    return href.to_string();
                }
                if let Some(Kind::String(src)) = js::get(value, "src").map(js::classify) {
                    return src.to_string();
                }
                if let Some(Kind::String(current_src)) =
                    js::get(value, "currentSrc").map(js::classify)
                {
                    return current_src.to_string();
                }
                if let Some(Kind::String(command)) = js::get(value, "command").map(js::classify) {
                    return command.to_string();
                }
                if let Some(request) = js::get(value, "request")
                    && let Kind::Object(_) | Kind::Array(_) = js::classify(request)
                    && let Some(Kind::String(url)) = js::get(request, "url").map(js::classify)
                {
                    return url.to_string();
                }
                if let Some(response) = js::get(value, "response")
                    && let Kind::Object(_) | Kind::Array(_) = js::classify(response)
                    && let Some(Kind::String(url)) = js::get(response, "url").map(js::classify)
                {
                    return url.to_string();
                }
                if let Some(id) = js::get(value, "id")
                    && let Kind::String(_) | Kind::Number(_) | Kind::BigInt(_) = js::classify(id)
                {
                    return js::string(Some(id));
                }
                if let Some(Kind::String(name)) = js::get(value, "name").map(js::classify) {
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
