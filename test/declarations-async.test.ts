// What a crate's declarations (ADR 0196) say of an `async fn`: the
// `Promise` its JS gives (ADR 0029), of what it resolves to, its `Future`'s
// `Output`, a generic one's and nothing's too, `Promise<void>` (#1769).
// TypeScript that awaits one is checked against it, and the JS resolves to
// what it says.

import { expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { compiler, fixture, root, run } from "./support";
import { es2022, repoTypes, type Setting, typeChecked } from "./type-checked";

// What this file's checks of its declarations are made under (`typeChecked`).
const checking: Setting = { ...es2022, ...repoTypes };

test("an async fn is declared the Promise of what it resolves to", async () => {
  const dir = fixture("declarations-async");
  writeFileSync(join(dir, "lib.rs"), `pub async fn names() -> Vec<String> {
    vec!["a".to_string(), "b".to_string()]
}

pub async fn first<T: Clone>(items: Vec<T>) -> Option<T> {
    items.first().cloned()
}

pub async fn nothing() {}
`);
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const declarations = readFileSync(join(dir, "lib.d.ts"), "utf8");
  expect(declarations).toContain("export function names(): Promise<string[]>;");
  expect(declarations).toContain("export function first<T>(items: T[]): Promise<T | null | undefined>;");
  expect(declarations).toContain("export function nothing(): Promise<void>;");
  writeFileSync(join(dir, "use.ts"), `import { first, names, nothing } from "./lib.js";
export const all: string[] = await names();
export const one: number | null | undefined = await first([1, 2]);
export const done: void = await nothing();
`);
  expect(typeChecked(dir, "use.ts", checking)).toEqual([]);
  writeFileSync(join(dir, "wrong.ts"), `import { names } from "./lib.js";
export const wrong: number[] = await names();
`);
  expect(typeChecked(dir, "wrong.ts", checking)).toEqual([expect.stringContaining("error TS2322")]);
  const lib = await import(join(dir, "lib.js"));
  expect([await lib.names(), await lib.first([3, 4]), await lib.nothing()]).toEqual([["a", "b"], 3, undefined]);
});
