// An iterator collected into a `Vec` is `Array.from(it)`, as JS writes it,
// never ES2025's `it.toArray()` (the user's decision, bw8/cm7; ADR 0055,
// amended). Each test is a program of its own, compiled and run.

import { expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { compiler, fixture, run } from "./support";

/** `rust` compiled in a fixture of `name`: its JS, and its module. */
async function compiled(name: string, rust: string): Promise<{ js: string; lib: Record<string, (...args: never[]) => unknown> }> {
  const dir = fixture(name);
  writeFileSync(join(dir, "lib.rs"), rust);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--crate-type", "lib"]);
  return { js: readFileSync(join(dir, "lib.js"), "utf8"), lib: await import(join(dir, "lib.js")) };
}

// A generic `IntoIterator`'s, an array or a JS iterator as its caller has
// it, stepped to its end: JS's iterator of it, `Iterator.from(items)`, not
// `$intoIter(items)`, which knows where it is (ADR 0475). A runtime
// helper's, an array or an iterable, needs no `Iterator.from` at all.
test("an iterator collected is Array.from of it", async () => {
  const { js, lib } = await compiled(
    "collected",
    `pub fn all<I: IntoIterator<Item = u32>>(items: I) -> Vec<u32> {
    items.into_iter().collect()
}

pub fn evens(n: u32) -> Vec<u32> {
    (0..n).filter(|i| i % 2 == 0).collect()
}

pub fn halves(s: &str) -> Vec<&str> {
    s.rsplit('=').collect()
}
`,
  );
  expect(js).not.toContain("toArray");
  expect(js).toContain("export function all(items) {\n  return Array.from(Iterator.from(items));\n}");
  expect(js).toContain("return Array.from($lazyRange(0, n).filter((i) => i % 2 === 0));");
  expect(js).toMatch(/return Array\.from\(\$rsplit\w*\(s, /);
  expect((lib.all as (items: number[]) => number[])([1, 2, 3])).toEqual([1, 2, 3]);
  expect((lib.evens as (n: number) => number[])(5)).toEqual([0, 2, 4]);
  expect((lib.halves as (s: string) => string[])("a=b")).toEqual(["b", "a"]);
});
