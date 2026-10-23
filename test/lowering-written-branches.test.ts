// What the compiler writes of an `if` the program wrote, as it wrote it:
// its test as written, not negated, its branches in the order written, an
// `else if` chain kept a chain past an arm that leaves, and a `continue`
// the program wrote where it wrote it, with its comment. Each test is a
// program of its own, compiled and run.

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

// React's isSimpleObject: `if (!descriptor.enumerable) { if ((name ===
// 'key' || name === 'ref') && typeof descriptor.get === 'function') { //
// ..\n continue; } return false; }`. The `continue` the program wrote is
// where it wrote it, its comment before it, and its `if`'s test as written;
// not `if ((name !== "key" && name !== "ref") || ..) { return false; }`, a
// `let … else`'s guard, with the comment gone.
test("a continue the program wrote keeps its if, its test and its comment", async () => {
  const { js, lib } = await compiled("written-continues", `use std::collections::HashMap;

pub struct Descriptor {
    pub enumerable: Option<bool>,
    pub get: Option<u32>,
}

pub fn is_simple(names: &[String], descriptors: &HashMap<String, Descriptor>) -> bool {
    for name in names {
        let Some(descriptor) = descriptors.get(name) else {
            return false;
        };
        if !descriptor.enumerable.unwrap_or_default() {
            if (name == "key" || name == "ref") && descriptor.get.is_some() {
                // React adds key and ref getters to props objects to issue warnings.
                // Those getters will not be transferred to the client, but that's ok,
                // so we'll special case them.
                continue;
            }
            return false;
        }
    }
    true
}

pub fn simple(names: Vec<String>, enumerable: Vec<bool>, getters: Vec<bool>) -> bool {
    let descriptors = (names.iter().cloned())
        .zip(enumerable.into_iter().zip(getters))
        .map(|(name, (enumerable, getter))| {
            let descriptor = Descriptor { enumerable: Some(enumerable), get: getter.then_some(1) };
            (name, descriptor)
        })
        .collect();
    is_simple(&names, &descriptors)
}
`);
  expect(js).toContain(
    '      if ((name === "key" || name === "ref") && descriptor.get != null) {\n        // React adds key and ref getters to props objects to issue warnings.\n',
  );
  expect(js).toContain("        // so we'll special case them.\n        continue;\n      }\n      return false;\n    }\n");
  const simple = (names: string[], enumerable: boolean[], getters: boolean[]) => lib.simple(names as never, enumerable as never, getters as never);
  expect([
    simple(["a", "key"], [true, false], [false, true]),
    simple(["a", "key"], [true, false], [false, false]),
    simple(["ref", "b"], [false, false], [true, true]),
    simple(["a"], [true], [false]),
  ]).toEqual([true, false, false, true]);
});

// isSimpleObject itself, of the js crate's reflection: its `let … else`
// guard, then its `if` of a `continue`, which ends the loop's body. Its
// `else`, MIR's `continue` the loop's turn ends with anyway, is no `else`
// (ADR 0684), not `} else { continue; }`, left past the `return` as a
// `continue;` the program didn't write.
test("a loop's body that ends in an if of a continue has no continue after it", async () => {
  const { js, lib } = await compiled("written-continues-js", `use js::Unknown;

pub fn is_simple_object(object: &Unknown) -> bool {
    let names = js::object::get_own_property_names(object);
    for name in &names {
        let Some(descriptor) = js::object::get_own_property_descriptor(object, name) else {
            return false;
        };
        if !descriptor.enumerable.unwrap_or_default() {
            if (name == "key" || name == "ref") && descriptor.get.is_some() {
                // React adds key and ref getters to props objects to issue warnings.
                continue;
            }
            return false;
        }
    }
    true
}
`);
  expect(js).toContain(
    '      if ((name === "key" || name === "ref") && descriptor.get) {\n        // React adds key and ref getters to props objects to issue warnings.\n        continue;\n      }\n      return false;\n    }\n  }\n  return true;\n',
  );
  const props = Object.defineProperty({ a: 1 }, "key", { get: () => 1, enumerable: false });
  const hidden = Object.defineProperty({}, "b", { value: 1, enumerable: false });
  expect([lib.isSimpleObject(props as never), lib.isSimpleObject(hidden as never), lib.isSimpleObject({ a: 1 } as never)]).toEqual([true, false, true]);
});

// ADR 0697: of `if a || v[i] != w[j] { return 1; }`, an index in bounds
// MIR reads into a temporary between the tests, the read is the test's
// alone where nothing after it reads it (`unread_bound`, as `and_tests`
// leaves one): `if (j === 0 || v[i] !== w[j])`, no `const value = v[i];`
// left after it.
test("a read a test takes in place is gone where nothing else reads it", async () => {
  const { js, lib } = await compiled(
    "unread-bound-read",
    `pub fn differs(v: &[i32], w: &[i32], i: usize, j: usize) -> usize {
    if i < v.len() && j < w.len() {
        if j == 0 || v[i] != w[j] {
            return 1;
        }
    }
    0
}
`,
  );
  expect(js).toContain("    if (j === 0 || v[i] !== w[j]) {\n      return 1;\n    }\n  }\n  return 0;\n");
  expect(js).not.toContain("const value");
  const differs = lib.differs as unknown as (v: number[], w: number[], i: number, j: number) => number;
  expect([differs([1, 2], [1, 2], 1, 1), differs([1, 2], [1, 3], 1, 1), differs([1], [1], 0, 0), differs([], [1], 0, 0)]).toEqual([0, 1, 1, 0]);
});
