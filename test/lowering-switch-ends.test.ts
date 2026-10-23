// How a `switch`'s case ends (ADR 0629): a `break` only where it's needed,
// none after a case that leaves, a `try` each of whose arms returns
// among them, and none at the end of the last case, which JS falls out
// of. Each test is a program of its own, compiled and run.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildReact, compiler, fixture, run } from "./support";

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
