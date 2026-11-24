// An `async fn`'s parameters are checked as any others are (#2403 step
// 2b(d), ADR 1192, amended): first in its own body, so a value JS gives
// that isn't of its type rejects the promise, as an async function's every
// throw does. A Rust caller calls its body, `f$`, as of any checked
// function. Its pattern keeps its shape, each part checked by its path
// (`boundary-patterns.test.ts`).
import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { compiler, fixture, run } from "./support";

const source = `pub struct Size {
    pub w: u32,
    pub h: u32,
}

pub async fn later(n: u32) -> u32 {
    n + 1
}

pub async fn measured(size: Size) -> u32 {
    size.w + size.h
}

pub async fn summed((a, b): (u32, u32)) -> u32 {
    a + b
}

pub async fn awaits() -> u32 {
    later(1).await + summed((2, 3)).await
}
`;

let lib: Record<string, (...args: unknown[]) => unknown>;
let js: string;

beforeAll(async () => {
  const dir = fixture("boundary-async");
  writeFileSync(join(dir, "lib.rs"), source);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--crate-type=lib", "--edition=2024"]);
  lib = await import(join(dir, "lib.js"));
  js = readFileSync(join(dir, "lib.js"), "utf8");
}, 600_000);

const rejected = async (call: () => unknown) => {
  try {
    await call();
  } catch (error) {
    return `${(error as Error).name}: ${(error as Error).message}`;
  }
  return "nothing rejected";
};

test("an async fn's parameter that isn't of its type rejects its promise", async () => {
  expect(await lib.later!(1)).toBe(2);
  const promise = lib.later!(-1);
  expect(promise).toBeInstanceOf(Promise);
  expect(await rejected(() => promise)).toBe("RangeError: rust-js: later(n): expected a u32, got -1");
});

test("an async fn's struct is checked whole, its path in the error", async () => {
  expect(await lib.measured!({ w: 2, h: 3 })).toBe(5);
  expect(await rejected(() => lib.measured!({ w: 2 }))).toBe("TypeError: rust-js: measured(size.h): expected a u32, got undefined");
});

test("its pattern keeps its shape, each part checked by its path", async () => {
  expect(js).toContain("export async function summed([a, b]) {");
  expect(await rejected(() => lib.summed!([2, "3"]))).toBe('TypeError: rust-js: summed(__arg0[1]): expected a u32, got "3"');
});

test("a Rust caller calls its body, unchecked", async () => {
  expect(js).toMatch(/later\$\(1\)/);
  expect(await lib.awaits!()).toBe(7);
});
