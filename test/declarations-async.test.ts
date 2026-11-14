// What a crate's declarations (ADR 0196) say of an `async fn`: the
// `Promise` its JS gives (ADR 0029), of what it resolves to, its `Future`'s
// `Output`, a generic one's and nothing's too, `Promise<void>` (#1769).
// TypeScript that awaits one is checked against it, and the JS resolves to
// what it says.

import { expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { compiler, fixture, root, run } from "./support";

/** TypeScript's errors in `file` of `dir`, strict, its declarations checked too. */
function typeChecked(dir: string, file: string): string[] {
  const config = join(dir, "tsconfig.json");
  writeFileSync(config, JSON.stringify({
    compilerOptions: { module: "esnext", moduleResolution: "bundler", target: "es2022", strict: true, noEmit: true, skipLibCheck: false, exactOptionalPropertyTypes: true, typeRoots: [join(root, "node_modules/@types")] },
    files: [file],
  }));
  const checked = Bun.spawnSync([process.execPath, join(root, "node_modules/typescript/bin/tsc"), "-p", config], { cwd: dir });
  return checked.stdout.toString().split("\n").filter((line) => line.includes("error TS"));
}

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
  expect(typeChecked(dir, "use.ts")).toEqual([]);
  writeFileSync(join(dir, "wrong.ts"), `import { names } from "./lib.js";
export const wrong: number[] = await names();
`);
  expect(typeChecked(dir, "wrong.ts")).toEqual([expect.stringContaining("error TS2322")]);
  const lib = await import(join(dir, "lib.js"));
  expect([await lib.names(), await lib.first([3, 4]), await lib.nothing()]).toEqual([["a", "b"], 3, undefined]);
});
