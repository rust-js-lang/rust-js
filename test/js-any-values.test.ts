// What the js crate's functions of a value of any shape do as JS does
// them: `js::keys_in` of what may be `null` or `undefined` (ADR 0719,
// amended), and JS's loose equality, `js::loose_equals` (ADR 1012). Each
// test is a program of its own, compiled and run in Node, as no native
// Rust has these values.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

import { runSync } from "./child";
import { buildReact, compiler, fixture, root, target } from "./support";

beforeAll(buildReact, 600_000);

const withJs = ["--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];

/** `source` compiled, its JS, and the module it is, loaded. */
async function compiled(name: string, source: string): Promise<{ js: string; lib: any }> {
  const dir = fixture(name);
  writeFileSync(join(dir, "lib.rs"), source);
  const built = runSync([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs], root, 600_000);
  expect(built.code, built.stderr).toBe(0);
  return { js: readFileSync(join(dir, "lib.js"), "utf8"), lib: await import(join(dir, "lib.js")) };
}

// ADR 0719, amended (React port gap rd71): `keys_in` takes any value, an
// `Option` too, as JS's `for…in` does, which visits nothing of `null` or
// `undefined`: CSSPropertyOperations' `for (const styleName in styles)`, no
// `if` around it.
test("keys_in of an Option visits none of None", async () => {
  const { js, lib } = await compiled("js-keys-in-option", `use js::Unknown;

pub fn count(styles: Option<&Unknown>) -> u32 {
    let mut n = 0;
    for key in js::keys_in(styles) {
        n += key.len() as u32;
    }
    n
}

pub fn listed(styles: Option<&Unknown>) -> Vec<String> {
    js::keys_in(styles).collect()
}
`);
  expect(js).toContain("  let n = 0;\n  for (const key in styles) {\n");
  expect([lib.count(undefined), lib.count(null), lib.count({ ab: 1, c: 2 })]).toEqual([0, 0, 3]);
  expect([lib.listed(null), lib.listed({ a: 1 })]).toEqual([[], ["a"]]);
});

// ADR 1012 (React port gap rd78): JS's loose equality, which ReactDOMInput
// compares by on purpose, `node.value != value`, so `"0.0"` typed where the
// value is `0` isn't clobbered: `js::loose_equals(a, b)` is `a == b`, and
// coerces as JS does, where Rust's `==` of the text would differ.
test("loose_equals is JS's ==, which coerces", async () => {
  const { js, lib } = await compiled("js-loose-equals", `use js::Unknown;

pub fn clobbers(typed: &str, value: &Unknown) -> bool {
    !js::loose_equals(typed, value)
}

pub fn same(a: Option<&Unknown>, b: Option<&Unknown>) -> bool {
    js::loose_equals(a, b)
}
`);
  expect(js).toContain("  return typed != value;\n");
  expect([lib.clobbers("0.0", 0), lib.clobbers("1", 1), lib.clobbers("1", 2), lib.clobbers("a", "a"), lib.clobbers("", 0)]).toEqual([false, false, true, false, false]);
  expect([lib.same(null, undefined), lib.same(undefined, 0), lib.same("1", true), lib.same(null, false)]).toEqual([true, false, true, false]);
});
