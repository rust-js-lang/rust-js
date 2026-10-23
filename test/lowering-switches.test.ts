// What the compiler writes of a `match` of constants as a `switch` (ADR
// 0629): its arms' cases, a guarded one's too. Each test is a program of its
// own, compiled and run.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildReact, compiler, fixture, run, target } from "./support";

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
