// What a crate's declarations (ADR 0196) say of an `impl Fn` parameter,
// and of a type parameter bound by `Fn`: the callback it says, of its own
// item's type parameters, whatever an item declared before it took the
// same index for.

import { expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { compiler, fixture, run } from "./support";

// React's scheduler, ported, declares `unstable_next<T>(eventHandler: impl
// FnOnce() -> T)`, then `unstable_wrapCallback<A, T>(callback: impl Fn(A)
// -> T + 'static)`, whose `T` is its parameter 1, which `unstable_next`'s
// `impl FnOnce() -> T` is: its declaration read `(a: A) => () => T`.
test("an impl Fn parameter is declared of its own item's parameters", () => {
  const dir = fixture("declarations-impl-fn-own");
  writeFileSync(join(dir, "Cargo.toml"), '[package]\nname = "app"\nversion = "0.0.0"\nedition = "2024"\n');
  writeFileSync(
    join(dir, "lib.rs"),
    "pub fn next<T>(handler: impl FnOnce() -> T) -> T {\n    handler()\n}\n\npub fn wrap<A, T>(callback: impl Fn(A) -> T + 'static) -> impl Fn(A) -> T {\n    move |argument| callback(argument)\n}\n",
  );
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const declarations = readFileSync(join(dir, "lib.d.ts"), "utf8");
  expect(declarations).toContain("export function next<T>(handler: () => T): T;");
  expect(declarations).toContain("export function wrap<A, T>(callback: (a: A) => T): (a: A) => T;");
});

// A named type parameter bound by `Fn`, `F: Fn(u32) -> u32`, is the
// function it says too, as an `impl Fn` is: its JS calls what it's given,
// so a TypeScript caller is held to a function of that signature (#661).
// An impl's is its type's parameter, declared as its type takes it.
test("a type parameter bound by Fn is declared as its function", () => {
  const dir = fixture("declarations-impl-fn-named");
  writeFileSync(join(dir, "Cargo.toml"), '[package]\nname = "app"\nversion = "0.0.0"\nedition = "2024"\n');
  writeFileSync(
    join(dir, "lib.rs"),
    `pub fn twice<F: Fn(u32) -> u32>(f: F, n: u32) -> u32 {
    f(f(n))
}

pub fn applied<T, U, F>(items: Vec<T>, f: F) -> Vec<U>
where
    F: Fn(&T) -> U,
{
    items.iter().map(f).collect()
}

pub fn counted<F: FnMut()>(mut f: F, times: u32) {
    for _ in 0..times {
        f();
    }
}

pub struct Holder<F> {
    pub f: F,
}

impl<F: Fn() -> u32> Holder<F> {
    pub fn call(&self) -> u32 {
        (self.f)()
    }
}
`,
  );
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const declarations = readFileSync(join(dir, "lib.d.ts"), "utf8");
  expect(declarations).toContain("export function twice(f: (value: number) => number, n: number): number;");
  expect(declarations).toContain("export function applied<T, U>(items: T[], f: (t: T) => U): U[];");
  expect(declarations).toContain("export function counted(f: () => void, times: number): void;");
  expect(declarations).toContain("call<F>(holder: Holder<F>): number;");
});
