// What a crate's declarations say of an iterator at the JS boundary
// (#1994, #1999, ADR 1169): a parameter bound by `Iterator` takes any of
// JS's iterables, `Iterable<T>`; one bound by `DoubleEndedIterator` or
// `ExactSizeIterator` an array or the iterator `@rust-js/std` declares
// that knows its back or its length; and an `impl Iterator` given back is
// that iterator, so one function's result is another's argument.

import { expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { compiler, fixture, root, run } from "./support";

/** TypeScript's errors in `file` of `dir`, strict, its declarations checked too. */
function typeChecked(dir: string, file: string): string[] {
  const config = join(dir, "tsconfig.json");
  writeFileSync(config, JSON.stringify({
    compilerOptions: { module: "esnext", moduleResolution: "bundler", target: "es2022", lib: ["es2022", "esnext.iterator"], strict: true, noEmit: true, skipLibCheck: false, exactOptionalPropertyTypes: true, typeRoots: [join(root, "node_modules/@types")] },
    files: [file],
  }));
  const checked = Bun.spawnSync([process.execPath, join(root, "node_modules/typescript/bin/tsc"), "-p", config], { cwd: dir });
  return checked.stdout.toString().split("\n").filter((line) => line.includes("error TS"));
}

test("an iterator at the JS boundary is declared what a JS caller gives and what it's given back", () => {
  const dir = fixture("declarations-iterators");
  writeFileSync(join(dir, "lib.rs"), `pub fn upto(n: u32) -> impl DoubleEndedIterator<Item = u32> + ExactSizeIterator {
    0..n
}

pub fn evens(v: Vec<u32>) -> impl DoubleEndedIterator<Item = u32> {
    v.into_iter().filter(|x| x % 2 == 0)
}

pub fn counted(n: u32) -> impl Iterator<Item = u32> {
    0..n
}

pub fn last_of(items: impl DoubleEndedIterator<Item = u32>) -> Option<u32> {
    let mut items = items;
    items.next_back()
}

pub fn left(items: impl ExactSizeIterator<Item = u32>) -> usize {
    items.len()
}

pub fn total<I: Iterator<Item = u32>>(items: I) -> u32 {
    items.sum()
}
`);
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const declarations = readFileSync(join(dir, "lib.d.ts"), "utf8");
  expect(declarations).toContain('import type { DoubleEndedIterator, ExactSizeIterator } from "@rust-js/std";');
  expect(declarations).toContain("export function upto(n: number): DoubleEndedIterator<number> & ExactSizeIterator<number>;");
  expect(declarations).toContain("export function evens(v: number[]): DoubleEndedIterator<number>;");
  expect(declarations).toContain("export function counted(n: number): IterableIterator<number>;");
  expect(declarations).toContain("export function lastOf(items: readonly number[] | DoubleEndedIterator<number>): number | null | undefined;");
  expect(declarations).toContain("export function left(items: readonly number[] | ExactSizeIterator<number>): number;");
  expect(declarations).toContain("export function total(items: Iterable<number>): number;");
  expect(declarations).not.toContain("any");
  expect(declarations).not.toContain("unknown");
  writeFileSync(join(dir, "use.ts"), `import { counted, evens, lastOf, left, total, upto } from "./lib.js";
export const last: number | null | undefined = lastOf(upto(4));
export const filtered: number | null | undefined = lastOf(evens([1, 2, 3, 4]));
export const count: number = left(upto(4));
export const sum: number = total(upto(4)) + total(counted(3)) + total(new Set([1, 2]));
export const spread: number[] = [...upto(4)];
export const ofArray: number = left([1, 2, 3]);
const back: IteratorResult<number> = upto(3).back();
export const length: number = upto(3).len() + (back.done ? 0 : back.value);
`);
  expect(typeChecked(dir, "use.ts")).toEqual([]);
  writeFileSync(join(dir, "wrong.ts"), `import { counted, lastOf, left } from "./lib.js";
lastOf(counted(3));
left(new Set([1, 2]));
`);
  expect(typeChecked(dir, "wrong.ts")).toEqual([
    expect.stringContaining("error TS2345: Argument of type 'IterableIterator<number>' is not assignable to parameter of type 'DoubleEndedIterator<number> | readonly number[]'"),
    expect.stringContaining("error TS2345: Argument of type 'Set<number>' is not assignable to parameter of type 'ExactSizeIterator<number> | readonly number[]'"),
  ]);
});
