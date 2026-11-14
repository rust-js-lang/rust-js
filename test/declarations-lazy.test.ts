// What a crate's declarations say of a `LazyCell` or a `LazyLock` it
// exports (#1752, ADR 1149): the cell its JS gives as it is, `{ init }`, is
// `@rust-js/std`'s opaque `LazyLock<T>`, not `any`: TypeScript takes one only
// of what rust-js gave, checks what it holds, and gives it back, its value
// read in Rust.

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

test("a LazyCell or a LazyLock is declared std's opaque type of what it holds", async () => {
  const dir = fixture("declarations-lazy");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::LazyCell;
use std::sync::LazyLock;

pub struct Point {
    pub x: f64,
    pub y: f64,
}

pub static ORIGIN: LazyLock<Point> = LazyLock::new(|| Point { x: 0.0, y: 0.0 });

pub static PRIMES: LazyLock<Vec<u32>> = LazyLock::new(|| vec![2, 3, 5]);

pub fn x_of(point: &LazyLock<Point>) -> f64 {
    point.x
}

pub fn made() -> LazyCell<u32> {
    LazyCell::new(|| 41)
}

pub fn counted(cell: &LazyCell<u32>) -> u32 {
    **cell + 1
}
`);
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const declarations = readFileSync(join(dir, "lib.d.ts"), "utf8");
  expect(declarations).toContain('import type { LazyCell, LazyLock } from "@rust-js/std";');
  expect(declarations).toContain("export const ORIGIN: LazyLock<Point>;");
  expect(declarations).toContain("export const PRIMES: LazyLock<number[]>;");
  expect(declarations).toContain("export function xOf(point: LazyLock<Point>): number;");
  expect(declarations).toContain("export function made(): LazyCell<number>;");
  expect(declarations).toContain("export function counted(cell: LazyCell<number>): number;");
  expect(declarations).not.toContain("any");
  writeFileSync(join(dir, "use.ts"), `import type { LazyLock } from "@rust-js/std";
import { counted, made, ORIGIN, xOf, type Point } from "./lib.js";
export const kept: LazyLock<Point> = ORIGIN;
export const x: number = xOf(kept);
export const one: number = counted(made());
`);
  expect(typeChecked(dir, "use.ts")).toEqual([]);
  writeFileSync(join(dir, "wrong.ts"), `import { counted, PRIMES, xOf } from "./lib.js";
xOf(PRIMES);
xOf({ init: () => ({ x: 0, y: 0 }) });
counted({ value: 1 });
`);
  expect(typeChecked(dir, "wrong.ts")).toEqual([
    expect.stringContaining("error TS2345: Argument of type 'LazyLock<number[]>' is not assignable to parameter of type 'LazyLock<Point>'"),
    expect.stringContaining("error TS2353: Object literal may only specify known properties, and 'init' does not exist in type 'LazyLock<Point>'"),
    expect.stringContaining("error TS2353: Object literal may only specify known properties, and 'value' does not exist in type 'LazyCell<number>'"),
  ]);
  const lib = await import(join(dir, "lib.js"));
  expect([lib.xOf(lib.ORIGIN), lib.counted(lib.made())]).toEqual([0, 42]);
});
