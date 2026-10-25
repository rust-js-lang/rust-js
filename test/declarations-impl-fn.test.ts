// What a crate's declarations (ADR 0196) say of an `impl Fn` parameter:
// the callback it says, of its own item's type parameters, whatever an
// item declared before it took the same index for.

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
