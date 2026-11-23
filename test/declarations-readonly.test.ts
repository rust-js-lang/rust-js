// A shared borrow is lent to be read (#2392, ADR 1189): of an array, a
// slice or a `Vec`, `&[T]`, `&[T; N]`, `&Vec<T>`, its declaration is
// `readonly T[]` wherever it's written, a parameter, a result or a field,
// so TypeScript passes a `readonly` array where Rust only reads, and can't
// change one Rust lent it. A `&mut` one is the array, `T[]`, and refuses a
// `readonly` one. Shallow, as TypeScript's `readonly` is; `&str` is a string.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildCompiler, compiler, fixture, run } from "./support";
import { es2022, repoTypes, type Setting, typeChecked } from "./type-checked";

beforeAll(() => {
  buildCompiler();
}, 600_000);

const checking: Setting = { ...es2022, ...repoTypes };

test("a shared borrow of an array is a readonly one, and a &mut one isn't", () => {
  const dir = fixture("declarations-readonly");
  writeFileSync(join(dir, "lib.rs"), `pub struct Basket<'a> {
    pub items: &'a [String],
    pub owned: Vec<String>,
}

pub fn with_item(cart: &[String], item: &str) -> Vec<String> {
    let mut more = cart.to_vec();
    more.push(item.to_string());
    more
}

pub fn add_item(cart: &mut Vec<String>, item: &str) {
    cart.push(item.to_string());
}

pub fn first<'a>(cart: &'a [String]) -> &'a [String] {
    &cart[..1]
}

pub fn sum(xs: &[f64; 3]) -> f64 {
    xs.iter().sum()
}

pub fn count(xs: &Vec<u32>) -> usize {
    xs.len()
}

pub fn rows(xs: &[Vec<u8>]) -> usize {
    xs.len()
}

pub fn text(s: &str) -> usize {
    s.len()
}
`);
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--crate-type=lib", "--edition=2024"]);
  const declarations = readFileSync(join(dir, "lib.d.ts"), "utf8");
  for (const line of [
    "    items: readonly string[];",
    "    owned: string[];",
    "export function withItem(cart: readonly string[], item: string): string[];",
    "export function addItem(cart: string[], item: string): void;",
    "export function first(cart: readonly string[]): readonly string[];",
    "export function sum(xs: readonly number[]): number;",
    "export function count(xs: readonly number[]): number;",
    "export function rows(xs: readonly number[][]): number;",
    "export function text(s: string): number;",
  ]) {
    expect(declarations).toContain(line);
  }
  writeFileSync(join(dir, "use.ts"), `import { first, withItem } from "./lib.js";
const frozen: readonly string[] = ["apple"];
export const more: string[] = withItem(frozen, "pear");
export const mutable: string[] = withItem(["apple"], "pear");
export const lent: readonly string[] = first(frozen);
`);
  expect(typeChecked(dir, "use.ts", checking)).toEqual([]);
  writeFileSync(join(dir, "wrong.ts"), `import { addItem, first } from "./lib.js";
const frozen: readonly string[] = ["apple"];
addItem(frozen, "fig");
first(frozen).push("fig");
`);
  expect(typeChecked(dir, "wrong.ts", checking)).toEqual([
    expect.stringContaining("error TS4104: The type 'readonly string[]' is 'readonly' and cannot be assigned to the mutable type 'string[]'"),
    expect.stringContaining("error TS2339: Property 'push' does not exist on type 'readonly string[]'"),
  ]);
  // The JS is as it was: the array itself, lent.
  const lib = readFileSync(join(dir, "lib.js"), "utf8");
  expect(lib).toContain("export function withItem(cart, item) {");
});
