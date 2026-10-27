// What the compiler writes of a chain of `js::strict_equals` tests of one
// subject: a `switch` (ADR 0958, the user's decision). Each test is a
// program of its own, compiled and run.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildReact, compiler, fixture, run, target } from "./support";

beforeAll(buildReact, 600_000);

const withJs = () => ["--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];

// React's ReactTestSelectors: its selectors' tags are JS values no `match`
// of constants can name, so the port tests them with `js::strict_equals`,
// an `if` chain of them, which upstream writes as `switch
// (selector.$$typeof)`. Of a chain of values, whose `return`s MIR's
// structure makes guards of, `selectorToString`'s, and of one whose
// branches go on, `matchSelector`'s. A chain that tests two subjects, or two
// ways only, stays as written; so does a guard the program wrote in the last
// `else`, its `default`'s, and what's after a chain in an `else`, after the
// `switch` the chain is.
test("React's matchSelector's chain of strict_equals tests is a switch", async () => {
  const dir = fixture("strict-equals-switches");
  writeFileSync(
    join(dir, "lib.rs"),
    `use std::cell::Cell;

use js::Unknown;

thread_local! {
    static COMPONENT_TYPE: Cell<&'static Unknown> = Cell::new(js::unknown_of(0b000));
    static ROLE_TYPE: Cell<&'static Unknown> = Cell::new(js::unknown_of(0b010));
    static TEXT_TYPE: Cell<&'static Unknown> = Cell::new(js::unknown_of(0b011));
}

pub struct Selector {
    pub r#typeof: &'static Unknown,
    pub value: &'static Unknown,
}

pub fn selector_to_string(selector: &Selector) -> String {
    if js::strict_equals(selector.r#typeof, COMPONENT_TYPE.get()) {
        "component".to_string()
    } else if js::strict_equals(selector.r#typeof, ROLE_TYPE.get()) {
        "role".to_string()
    } else if js::strict_equals(selector.r#typeof, TEXT_TYPE.get()) {
        "text".to_string()
    } else {
        panic!("Invalid selector type specified.");
    }
}

pub fn match_selector(selector: &Selector, tag: u8) -> bool {
    if js::strict_equals(selector.r#typeof, COMPONENT_TYPE.get()) {
        if tag == 1 {
            return true;
        }
    } else if js::strict_equals(selector.r#typeof, ROLE_TYPE.get()) {
        return tag == 2;
    } else if js::strict_equals(selector.r#typeof, TEXT_TYPE.get()) {
        if tag == 3 {
            return true;
        }
    } else {
        panic!("Invalid selector type specified.");
    }
    false
}

pub fn kind(selector: &Selector) -> u8 {
    if js::strict_equals(selector.r#typeof, COMPONENT_TYPE.get()) {
        1
    } else if js::strict_equals(selector.r#typeof, ROLE_TYPE.get()) {
        2
    } else {
        3
    }
}

pub fn mixed(selector: &Selector) -> u8 {
    if js::strict_equals(selector.r#typeof, COMPONENT_TYPE.get()) {
        1
    } else if js::strict_equals(selector.value, ROLE_TYPE.get()) {
        2
    } else if js::strict_equals(selector.value, TEXT_TYPE.get()) {
        3
    } else if js::strict_equals(selector.value, COMPONENT_TYPE.get()) {
        4
    } else {
        5
    }
}

pub fn two_ways(selector: &Selector, log: &mut Vec<u8>) {
    if js::strict_equals(selector.r#typeof, COMPONENT_TYPE.get()) {
        log.push(1);
    } else if js::strict_equals(selector.r#typeof, ROLE_TYPE.get()) {
        log.push(2);
    }
}

pub fn guarded(
    s: &'static Unknown,
    a: &'static Unknown,
    b: &'static Unknown,
    c: &'static Unknown,
    log: &mut Vec<u8>,
) -> u8 {
    if js::strict_equals(s, a) {
        return 1;
    } else if js::strict_equals(s, b) {
        return 2;
    } else {
        if !js::strict_equals(s, c) {
            return 3;
        }
        log.push(4);
    }
    4
}

pub fn negated(s: &'static Unknown, a: &'static Unknown, b: &'static Unknown, c: &'static Unknown) -> u8 {
    if js::strict_equals(s, a) {
        1
    } else if js::strict_equals(s, b) {
        2
    } else if !js::strict_equals(s, c) {
        3
    } else {
        4
    }
}

pub fn tail(s: &'static Unknown, a: &'static Unknown, b: &'static Unknown, log: &mut Vec<u8>) -> u8 {
    if js::strict_equals(s, a) {
        return 1;
    } else {
        if js::strict_equals(s, b) {
            log.push(2);
        } else if js::strict_equals(s, COMPONENT_TYPE.get()) {
            log.push(3);
        } else {
            log.push(4);
        }
        log.push(5);
    }
    6
}
`,
  );
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs()]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain(`export function selectorToString(selector) {
  switch (selector.typeof) {
    case componentType:
      return "component";
    case roleType:
      return "role";
    case textType:
      return "text";
    default:
      throw new Error("Invalid selector type specified.");
  }
}`);
  expect(js).toContain(`export function matchSelector(selector, tag) {
  switch (selector.typeof) {
    case componentType:
      if (tag === 1) {
        return true;
      }
      break;
    case roleType:
      return tag === 2;
    case textType:
      if (tag === 3) {
        return true;
      }
      break;
    default:
      throw new Error("Invalid selector type specified.");
  }
  return false;
}`);
  expect(js).toContain("  if (selector.typeof === componentType) {\n    return 1;\n  }\n  if (selector.value === roleType) {");
  expect(js).toContain("    case roleType:\n      return 2;\n    default:\n      return 3;\n  }\n}");
  expect(js).toContain("export function twoWays(selector, log) {\n  if (selector.typeof === componentType) {");
  expect(js).toContain("    case b:\n      return 2;\n    default:\n      return s !== c ? 3 : 4;\n  }\n}");
  // The `else` the program wrote, followed by its value: the `default`,
  // the value after the `switch` (ADR 0879, amended).
  expect(js).toContain("    default:\n      if (s !== c) {\n        return 3;\n      }\n      log.push(4);\n  }\n  return 4;\n}");
  expect(js).toContain(
    "  if (s === a) {\n    return 1;\n  } else {\n    switch (s) {\n      case b:\n        log.push(2);\n        break;\n      case componentType:\n        log.push(3);\n        break;\n      default:\n        log.push(4);\n    }\n    log.push(5);\n  }\n  return 6;\n}",
  );
  const lib = await import(join(dir, "lib.js"));
  expect([0, 2, 3].map((t) => lib.selectorToString({ typeof: t }))).toEqual(["component", "role", "text"]);
  expect(() => lib.selectorToString({ typeof: 9 })).toThrow("Invalid selector type specified.");
  expect([lib.matchSelector({ typeof: 0 }, 1), lib.matchSelector({ typeof: 0 }, 2), lib.matchSelector({ typeof: 2 }, 2), lib.matchSelector({ typeof: 3 }, 3), lib.matchSelector({ typeof: 3 }, 1)]).toEqual([true, false, true, true, false]);
  expect(() => lib.matchSelector({ typeof: 9 }, 1)).toThrow("Invalid selector type specified.");
  expect([0, 2, 3].map((t) => lib.kind({ typeof: t }))).toEqual([1, 2, 3]);
  expect([0, 2, 3, 0, 9].map((value, i) => lib.mixed({ typeof: i === 0 ? 0 : 9, value }))).toEqual([1, 2, 3, 4, 5]);
  const log: number[] = [];
  lib.twoWays({ typeof: 0 }, log);
  lib.twoWays({ typeof: 2 }, log);
  lib.twoWays({ typeof: 9 }, log);
  expect([[1, 2, 3, 9].map((s) => lib.negated(s, 1, 2, 3)), log]).toEqual([[1, 2, 4, 3], [1, 2]]);
  expect([[1, 2, 3, 9].map((s) => lib.guarded(s, 1, 2, 3, log)), log]).toEqual([[1, 2, 4, 3], [1, 2, 4]]);
  const tailed: number[] = [];
  expect([1, 2, 0, 9].map((s) => lib.tail(s, 1, 2, tailed))).toEqual([1, 6, 6, 6]);
  expect(tailed).toEqual([2, 5, 3, 5, 4, 5]);
});

// What a `switch` would change stays an `if` chain: a subject a call reads,
// which the chain calls once a test; a case's value that does something,
// which each test does where the one before fails, and a `switch` too, but
// of a subject read once; a `break` of the loop the chain is in, which a
// `switch` would take; and `if`s the program wrote apart, each a guard,
// no chain.
test("a chain of strict_equals tests a switch would change stays", async () => {
  const dir = fixture("strict-equals-ifs");
  writeFileSync(
    join(dir, "lib.rs"),
    `use js::Unknown;

#[cfg_attr(rust_js, rust_js::link_name = "rustJsNext")]
fn next() -> &'static Unknown {
    unreachable!()
}

#[cfg_attr(rust_js, rust_js::link_name = "rustJsLog")]
fn log(n: u8) {
    let _ = n;
    unreachable!()
}

pub fn called(a: &'static Unknown, b: &'static Unknown, c: &'static Unknown) -> u8 {
    if js::strict_equals(next(), a) {
        1
    } else if js::strict_equals(next(), b) {
        2
    } else if js::strict_equals(next(), c) {
        3
    } else {
        4
    }
}

pub fn valued(s: &'static Unknown, a: &'static Unknown, b: &'static Unknown) -> u8 {
    if js::strict_equals(s, a) {
        1
    } else if js::strict_equals(s, next()) {
        2
    } else if js::strict_equals(s, b) {
        3
    } else {
        4
    }
}

pub fn looped(items: &[&'static Unknown], a: &'static Unknown, b: &'static Unknown) -> u8 {
    let mut n = 0;
    for &item in items {
        if js::strict_equals(item, a) {
            n += 1;
        } else if js::strict_equals(item, b) {
            break;
        } else {
            n += 10;
        }
    }
    n
}

pub fn guarded(s: &'static Unknown, a: &'static Unknown, b: &'static Unknown) -> u8 {
    if js::strict_equals(s, a) {
        return 1;
    }
    if js::strict_equals(s, b) {
        return 2;
    }
    if js::strict_equals(s, a) {
        return 4;
    }
    log(3);
    3
}
`,
  );
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs()]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).not.toContain("switch");
  const global = globalThis as Record<string, unknown>;
  const reads = [1, 2, 3, 7, 5];
  global.rustJsNext = () => reads.shift();
  const logged: number[] = [];
  global.rustJsLog = (n: number) => logged.push(n);
  const lib = await import(join(dir, "lib.js"));
  expect([lib.called(9, 9, 3), lib.valued(5, 1, 6), lib.valued(5, 1, 5)]).toEqual([3, 4, 2]);
  expect([lib.looped([1, 3, 1, 2, 1], 1, 2), lib.guarded(1, 1, 2), lib.guarded(2, 1, 2), lib.guarded(5, 1, 2), logged]).toEqual([12, 1, 2, 3, [3]]);
});
