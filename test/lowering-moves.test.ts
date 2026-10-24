// Where a temporary MIR made is read: in its place, where what the
// program does stays the same (src/lower/mir/tidy/moves.rs, ADR 0750).
// Each test is a program of its own, compiled and run.

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

// React's addValueToProperties pushes `[prefix + '\xa0\xa0'.repeat(indent)
// + propertyName, objectName === 'Object' ? .. : objectName]`. MIR makes
// the pair's first part before the branch of its second: once that's a
// conditional, the part is read in the pair, past `properties.push`, the
// method its array has, not held, `const value$1 = ..`.
test("a call's value is read in a method's arguments, past the method", async () => {
  const { js, lib } = await compiled("read-past-a-method", `pub fn pushed(name: &str, object_name: &str, indent: usize, out: &mut Vec<(String, String)>) {
    out.push((
        "-".repeat(indent) + name,
        if object_name == "Object" {
            if indent < 3 { String::new() } else { "\\u{2026}".to_string() }
        } else {
            object_name.to_string()
        },
    ));
}
`);
  expect(js).toContain('  out.push([\n    "-".repeat(indent) + name,\n    objectName !== "Object" ? objectName : indent < 3 ? "" : "…",\n  ]);\n');
  expect(js).not.toMatch(/const (value|_\d+)/);
  const out: [string, string][] = [];
  lib.pushed("a" as never, "Object" as never, 1 as never, out as never);
  lib.pushed("b" as never, "Map" as never, 4 as never, out as never);
  lib.pushed("c" as never, "Object" as never, 4 as never, out as never);
  expect(out).toEqual([["-a", ""], ["----b", "Map"], ["----c", "…"]]);
});

// React's addObjectDiffToProperties: `if (readReactElementTypeof(nextValue)
// === REACT_ELEMENT_TYPE) { .. continue; } else { .. }`, a test MIR holds
// before the blocks its arms' jumps leave, `const _183 = ..; bb200: {
// bb291: { if (_183) ..`. Entering a block runs nothing: the test is read
// where it's tested, not held as a `const value`.
test("a temporary is read in the block after it that reads it", async () => {
  const { js, lib } = await compiled("read-in-a-block", `use js::{SymbolFor, Unknown};

const ELEMENT: SymbolFor = SymbolFor("react.element");

fn tag(value: &Unknown) -> Option<&'static Unknown> {
    js::get(value, "tag")
}

pub fn counted(values: &[&Unknown], out: &mut Vec<String>) -> usize {
    let mut other = 0;
    for &value in values {
        if matches!(SymbolFor::from_value(tag(value)), Some(ELEMENT)) {
            if js::get(value, "skip").is_some() {
                out.push("skipped".to_string());
                continue;
            }
        } else {
            let kind = js::object::to_string(Some(value)).unwrap();
            if kind == "[object Array]" {
                out.push(kind);
                continue;
            }
        }
        other += 1;
    }
    other
}
`);
  expect(js).toContain("      if (tag(value) === ELEMENT) {\n");
  expect(js).not.toMatch(/const (value\$?\d*|_\d+) = /);
  const element = Symbol.for("react.element");
  const out: string[] = [];
  const values = [{ tag: element }, { tag: element, skip: 1 }, [], {}];
  expect(lib.counted(values as never, out as never)).toBe(2);
  expect(out).toEqual(["skipped", "[object Array]"]);
});
