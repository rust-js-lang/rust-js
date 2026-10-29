// What the compiler writes of a `match` of the registry's symbols, React's
// `$$typeof`s: a `switch` of the value, its `default` the `_` arm's and
// `None`'s (ADR 0629, amended), and the value some arms give, given where
// they give it (ADR 0364, amended). Each test is a program of its own,
// compiled and run.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { runSync } from "./child";
import { node } from "./programs";
import { buildReact, compiler, fixture, installRuntime, root, run, target } from "./support";

beforeAll(buildReact, 600_000);

const withJs = () => ["--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];

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
// can't chain, that could tell a value some arms give was given first;
// and its `switch`, whose `default` takes `None` (ADR 0629, amended).
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

pub fn stopped(values: &[&'static Unknown]) -> Vec<u32> {
    let mut seen = Vec::new();
    for &value in values {
        let mut level = 0;
        match tag(value) {
            Some(A | B) => level = 1,
            Some(C) => continue,
            _ => {
                seen.push(0);
                if seen.len() > 3 {
                    break;
                }
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
  // The `_` arm's statements the `default`'s, `None` among what it takes,
  // the `switch` no `if` of the `Some`'s and no block's (#221).
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain(`    switch (tag(value)) {
      case A:
      case B:
        level = 1;
        break;
      case C:
        level = 2;
        break;
      default:
        seen.push(look());
        level = 3;
    }`);
  expect(js).not.toContain("symbolFor != null) {\n        switch");
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
  lib.stopped(values),
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
    [1, 0, 0, 1, 0],
  ]);
});
