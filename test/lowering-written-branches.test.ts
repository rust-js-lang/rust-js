// What the compiler writes of an `if` the program wrote, as it wrote it:
// its test as written, not negated, its branches in the order written, an
// `else if` chain kept a chain past an arm that leaves, and a `continue`
// the program wrote where it wrote it, with its comment. Each test is a
// program of its own, compiled and run.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildReact, compiler, fixture, run } from "./support";

beforeAll(buildReact, 600_000);

/** `rust` compiled in a fixture of `name`: its JS, and its module. */
async function compiled(name: string, rust: string): Promise<{ js: string; lib: Record<string, (...args: never[]) => unknown> }> {
  const dir = fixture(name);
  writeFileSync(join(dir, "lib.rs"), rust);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  return { js: readFileSync(join(dir, "lib.js"), "utf8"), lib: await import(join(dir, "lib.js")) };
}

// React's describeObjectForErrorMessage (shared/ReactSerializationErrors.js):
// `if (__DEV__ && jsxChildrenParents.has(objectOrArray)) { .. } else { ..
// }`, each branch a loop. MIR jumps from the `&&`'s tests to the `else`, so
// its `then` is the block's; the `if` is as written, not `if (!__DEV__ ||
// !jsxChildrenParents.has(..)) { .. } else { .. }`.
test("an if of an && keeps its test and its branches' order, each a loop", async () => {
  const { js, lib } = await compiled("written-and-branches", `use std::collections::HashSet;

pub const DEV: bool = true;

pub fn described(parents: &HashSet<u32>, key: u32, items: &[u32]) -> String {
    let mut str;
    if DEV && parents.contains(&key) {
        // Print JSX Children
        str = "<".to_string();
        for &item in items {
            let substr = if item > 3 { format!("{{{item}}}") } else { item.to_string() };
            if substr.len() < 15 {
                str += &substr;
            } else {
                str += "{...}";
            }
        }
        str += ">";
    } else {
        // Print Array
        str = "[".to_string();
        for &item in items {
            str += &item.to_string();
        }
        str += "]";
    }
    str
}
`);
  expect(js).toContain('  if (DEV && parents.has(key)) {\n    // Print JSX Children\n    str = "<";\n');
  expect(js).toContain('  } else {\n    // Print Array\n    str = "[";\n');
  expect(js).not.toContain("!DEV");
  const both = new Set([1]);
  expect([lib.described(both as never, 1 as never, [1, 5] as never), lib.described(both as never, 2 as never, [1, 5] as never)]).toEqual(["<1{5}>", "[15]"]);
});

// The same function's `else if` chain: an element, a client reference that
// returns, JSX, an object. An arm that leaves keeps the chain's `else`
// where an arm before it doesn't leave (ADR 0237, amended), as ESLint's
// `no-else-return` allows; a chain whose arms each leave up to it is still
// each `if` after the last.
test("an else-if chain keeps its else past an arm that returns", async () => {
  const { js, lib } = await compiled("written-else-if-chains", `use std::collections::HashSet;

pub const DEV: bool = true;

pub fn described(parents: &HashSet<u32>, key: u32) -> String {
    let mut str;
    if key == 1 {
        str = "element".to_string();
    } else if key == 2 {
        return "client".to_string();
    } else if DEV && parents.contains(&key) {
        // Print JSX
        str = "<jsx>".to_string();
    } else {
        // Print Object
        str = "{".to_string();
        str += "}";
    }
    str
}

pub fn guarded(key: u32) -> u32 {
    if key == 1 {
        return 10;
    } else if key == 2 {
        return 20;
    } else if key > 5 {
        println!("big");
    }
    key
}
`);
  expect(js).toContain(
    '  if (key === 1) {\n    str = "element";\n  } else if (key === 2) {\n    return "client";\n  } else if (DEV && parents.has(key)) {\n    // Print JSX\n',
  );
  expect(js).toContain('  } else {\n    // Print Object\n    str = "{";\n');
  // Each arm up to the last leaves: each `if` after the last (ADR 0237).
  expect(js).toContain("  if (key === 1) {\n    return 10;\n  }\n  if (key === 2) {\n    return 20;\n  }\n  if (key > 5) {\n");
  const parents = new Set([3]);
  expect([1, 2, 3, 4].map((key) => lib.described(parents as never, key as never))).toEqual(["element", "client", "<jsx>", "{}"]);
  expect([1, 2, 3].map((key) => lib.guarded(key as never))).toEqual([10, 20, 3]);
});

