// How a `switch`'s case ends (ADR 0629): a `break` only where it's needed,
// none after a case that leaves, a `try` each of whose arms returns
// among them, and none at the end of the last case, which JS falls out
// of. Each test is a program of its own, compiled and run.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildReact, compiler, fixture, run, target } from "./support";

beforeAll(buildReact, 600_000);

// React's describeElementType: `case REACT_VIEW_TRANSITION_TYPE: if
// (enableViewTransition) { return 'ViewTransition'; } }`, its last case
// falling out of the `switch`; and its lazy case, `try { return ..; }
// catch { return ''; }`, which never goes on to the next.
test("a case needs no break where it leaves or it's the last", async () => {
  const dir = fixture("switch-ends");
  writeFileSync(join(dir, "lib.rs"), `use std::panic::catch_unwind;

pub const SUSPENSE: u32 = 1;
pub const VIEW: u32 = 2;
pub const LAZY: u32 = 3;
pub const MEMO: u32 = 4;
pub const ENABLE_VIEW: bool = true;

fn init(payload: u32) -> u32 {
    if payload == 0 {
        panic!("not loaded");
    }
    payload
}

pub fn described(kind: u32, payload: u32) -> String {
    match kind {
        SUSPENSE => return "Suspense".to_string(),
        VIEW if ENABLE_VIEW => {
            return "ViewTransition".to_string();
        }
        _ => {}
    }
    if payload > 100 {
        match kind {
            LAZY => {
                // Lazy may contain any component type so we recursively resolve it.
                return catch_unwind(|| described(init(payload - 100), 0)).unwrap_or_default();
            }
            MEMO => return "Memo".to_string(),
            _ => {}
        }
    }
    String::new()
}

pub fn counted(kind: u32, on: bool, log: &mut Vec<u32>) {
    match kind {
        SUSPENSE => log.push(1),
        VIEW if on => log.push(2),
        LAZY => log.push(3),
        _ => {}
    }
    log.push(0);
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('    case VIEW:\n      if (ENABLE_VIEW) {\n        return "ViewTransition";\n      }\n  }\n');
  expect(js).toContain('        } catch {\n          return "";\n        }\n      case MEMO:\n');
  // A case before another still ends in a `break`, and the last that's left
  // out doesn't.
  expect(js).toContain("      log.push(2);\n      }\n      break;\n    case LAZY:\n      log.push(3);\n  }\n");
  const lib = await import(join(dir, "lib.js"));
  const runs = [1, 2, 3, 4].map((kind) => {
    const log: number[] = [];
    lib.counted(kind, true, log);
    return log;
  });
  expect([lib.described(2, 0), lib.described(3, 101), lib.described(3, 100), lib.described(4, 101), runs]).toEqual([
    "ViewTransition",
    "Suspense",
    "",
    "Memo",
    [[1, 0], [2, 0], [3, 0], [0]],
  ]);
});

// React's addValueToProperties (shared/ReactPerformanceTrackProperties.js):
// `let desc; if (..) { .. } else { switch (typeof value) { case
// "function": .. } } properties.push(..)`. A case's arm whose inner match
// gives `desc` jumps where the `switch` ends, nothing between: its own
// `break`, not a label around the `if` (ADR 0731).
test("a case's break past the switch is the case's own", async () => {
  const dir = fixture("switch-ends-past");
  writeFileSync(join(dir, "lib.rs"), `use js::{Kind, Unknown};

pub fn described(value: Option<&Unknown>, log: &mut Vec<String>) {
    let desc = if let Some(value) = value
        && let Kind::Object(_) | Kind::Array(_) = js::classify(Some(value))
    {
        "object".to_string()
    } else {
        match js::classify(value) {
            Kind::Null => "null".to_string(),
            Kind::Function(function) => match js::classify(js::get(js::unknown(function), "name")) {
                Kind::String(name) if !name.is_empty() => name.to_string() + "() {}",
                _ => "() => {}".to_string(),
            },
            Kind::Undefined => "undefined".to_string(),
            Kind::Bool(value) => if value { "true" } else { "false" }.to_string(),
            _ => js::string(value),
        }
    };
    log.push(desc);
}

// A match of three arms in a case, a switch's own: its block's break past
// it, the case's own.
pub fn numbered(value: Option<&Unknown>, log: &mut Vec<String>) {
    let desc = if let Some(value) = value
        && let Kind::Object(_) = js::classify(Some(value))
    {
        "object".to_string()
    } else {
        match js::classify(value) {
            Kind::Function(function) => match js::classify(js::get(js::unknown(function), "name")) {
                Kind::String(name) if !name.is_empty() => name.to_string() + "() {}",
                Kind::Number(_) => "numbered() {}".to_string(),
                _ => "() => {}".to_string(),
            },
            Kind::Undefined => "undefined".to_string(),
            _ => js::string(value),
        }
    };
    log.push(desc);
}

// A loop's in a case is the loop's: the block's own, still.
pub fn scanned(value: Option<&Unknown>, log: &mut Vec<String>) {
    let desc = 'desc: {
        match js::classify(value) {
            Kind::Null => "null".to_string(),
            Kind::String(text) => {
                for c in text.chars() {
                    if c == '!' {
                        break 'desc "bang".to_string();
                    }
                    log.push(c.to_string());
                }
                "plain".to_string()
            }
            Kind::Undefined => "undefined".to_string(),
            _ => js::string(value),
        }
    };
    log.push(desc);
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  // Its match of two arms, an `if` and its `else`, as upstream's (ADR 0745).
  expect(js).toContain('        if (typeof name === "string" && name) {\n          desc = name + "() {}";\n        } else {\n          desc = "() => {}";\n        }\n        break;\n');
  expect(js.slice(0, js.indexOf("function scanned"))).not.toMatch(/bb\d+/);
  const lib = await import(join(dir, "lib.js"));
  const log: string[] = [];
  for (const value of [{}, null, function named() {}, () => {}, undefined, true, 5]) lib.described(value, log);
  expect(log).toEqual(["object", "null", "named() {}", "() => {}", "undefined", "true", "5"]);
  const numbered: string[] = [];
  const counted = Object.defineProperty(function () {}, "name", { value: 3 });
  for (const value of [function named() {}, counted, () => {}, undefined, 5]) lib.numbered(value, numbered);
  expect(numbered).toEqual(["named() {}", "numbered() {}", "() => {}", "undefined", "5"]);
  const scanned: string[] = [];
  for (const value of ["a!b", "ab", null]) lib.scanned(value, scanned);
  expect(scanned).toEqual(["a", "bang", "a", "b", "plain", "null"]);
});
