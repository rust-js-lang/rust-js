// What the compiler writes of an early exit the program wrote, as it wrote
// it: a loop's `break` in the `if` it's in, not an `else` of the rest
// ending in a `continue`, nor an early `continue` the program never wrote
// (ADR 0730). Each test is a program of its own, compiled and run.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildReact, compiler, fixture, run, target } from "./support";

beforeAll(buildReact, 600_000);

/** `rust` compiled in a fixture of `name`, against the js crate: its JS,
 * and its module. */
async function compiled(name: string, rust: string): Promise<{ js: string; lib: Record<string, (...args: never[]) => unknown> }> {
  const dir = fixture(name);
  writeFileSync(join(dir, "lib.rs"), rust);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  return { js: readFileSync(join(dir, "lib.js"), "utf8"), lib: await import(join(dir, "lib.js")) };
}

// React's addObjectDiffToProperties (shared/ReactPerformanceTrackProperties.js):
// `if (checked > OBJECT_WIDTH_LIMIT) { ..; break; }` then the rest of the
// body. MIR has the rest as the `if`'s `else`, ending in a `continue`, and
// the `break` after it; and the rest's own `if (!(key in next))` is as
// written, not `if (key in next) { continue; }`.
test("a loop's early break is in the if the program wrote it in", async () => {
  const { js, lib } = await compiled("early-breaks-in-then", `const LIMIT: usize = 3;

pub fn checked(keys: &[String], next: &[String], log: &mut Vec<String>) -> bool {
    let mut equal = true;
    for (checked, key) in keys.iter().enumerate() {
        if checked > LIMIT {
            log.push("too many".to_string());
            equal = false;
            break;
        }

        if !next.contains(key) {
            log.push(key.clone());
            equal = false;
        }
    }

    let mut next_checked = 0;
    for key in next {
        if next_checked > LIMIT {
            log.push("too many".to_string());
            equal = false;
            break;
        }
        if keys.contains(key) {
            log.push(key.clone());
            if key.is_empty() {
                continue;
            }
        } else {
            equal = false;
        }
        next_checked += 1;
    }
    equal
}
`);
  expect(js).toContain('    if (checked > LIMIT) {\n      log.push("too many");\n      equal = false;\n      break;\n    }\n');
  expect(js).toContain("    if (!next.includes(key)) {\n      log.push(key);\n      equal = false;\n    }\n  }\n");
  expect(js).toContain('    if (nextChecked > LIMIT) {\n      log.push("too many");\n      equal = false;\n      break;\n    }\n    if (keys.includes(key)) {\n');
  expect(js).toContain("    nextChecked = (nextChecked + 1) >>> 0;\n  }\n");
  expect(js).not.toContain("} else {\n      if (");
  const results = [
    [["a", "b"], ["a", "c"]],
    [["a", "b", "c", "d", "e", "f"], ["a", "", "b", "c", "d", "e", "f"]],
  ].map(([keys, next]) => {
    const log: string[] = [];
    return [lib.checked(keys as never, next as never, log as never), log];
  });
  expect(results).toEqual([
    [false, ["b", "a"]],
    [false, ["too many", "a", "b", "c", "too many"]],
  ]);
});
