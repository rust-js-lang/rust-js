// std's bodies as a package (ADR 0552): `@rust-js/std`,
// released with the compiler as `@rust-js/runtime` is. Its JS and its
// manifest are made by the compiler from the bodies the toolchain ships,
// `bun run std`, and committed in `std/`: the compiler builds the manifest
// in, so what it imports can't drift from what the package has.

import { beforeAll, expect, test } from "bun:test";
import { cpSync, mkdirSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { buildCompiler, compiler, fixture, root, run } from "./support";

beforeAll(buildCompiler, 600_000);

// The committed package is what the compiler writes of std's bodies now:
// each module's JS, and the manifest, whose paths are relative to it.
test("the committed @rust-js/std is std's bodies as the compiler compiles them", () => {
  const dir = fixture("std-package");
  mkdirSync(join(dir, "std"), { recursive: true });
  cpSync(join(root, "std", "lib.rs"), join(dir, "std", "lib.rs"));
  run([
    compiler,
    join(dir, "std", "lib.rs"),
    "-o",
    join(dir, "std", "index.js"),
    "--library",
    "--manifest",
    join(dir, "std", "manifest.json"),
    "--std-bodies",
    "--",
    "--crate-name",
    "rust_js_std",
  ]);
  const manifest = JSON.parse(readFileSync(join(dir, "std", "manifest.json"), "utf8"));
  const committed = JSON.parse(readFileSync(join(root, "std", "manifest.json"), "utf8"));
  // The library contract, what the compiler builds in, and each module's
  // file, relative: the same wherever the checkout is, but for the crate's
  // hash, which rustc makes of where its source is.
  const contract = ({ crate_hash, inputs, ...rest }: Record<string, unknown>) => rest;
  expect(contract(manifest.library)).toEqual(contract(committed.library));
  expect(manifest.modules.map((m: { module: string[]; file: string }) => [m.module, m.file])).toEqual(
    committed.modules.map((m: { module: string[]; file: string }) => [m.module, m.file]),
  );
  // Each module's JS, but for its first line, which names the input as
  // the command did, `std/lib.rs` of `bun run std`.
  const body = (file: string) => readFileSync(file, "utf8").split("\n").slice(1).join("\n");
  for (const { file } of manifest.modules) {
    expect(body(join(dir, "std", file)), file).toBe(body(join(root, "std", file)));
  }
  // Each item is std's: keyed as rustc keys it, in a module of the package.
  expect(committed.library.items.length).toBeGreaterThan(10);
  expect(committed.library.items.every((item: { rust_path: string }) => item.rust_path.includes("core::iter"))).toBe(true);
  const pkg = JSON.parse(readFileSync(join(root, "std", "package.json"), "utf8"));
  const version = readFileSync(join(root, "Cargo.toml"), "utf8").match(/^version = "([^"]+)"/m)![1];
  expect([pkg.name, pkg.version]).toEqual(["@rust-js/std", version]);
});

// What the compiler writes imports the package by its name, each module by
// its path: `@rust-js/std/iter.js`, as `@rust-js/runtime` is imported.
test("a Peekable's next is std's own, imported from @rust-js/std", () => {
  const dir = fixture("std-package-import");
  const file = join(dir, "lib.rs");
  Bun.write(file, "pub fn first(v: &[u32]) -> Option<u32> { let mut p = v.iter().copied().peekable(); p.next() }\n");
  run([compiler, file, "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('from "@rust-js/std/iter.js"');
  expect(js).toContain("peekableIterator_next(");
  expect(js).toContain("iterator_peekable(");
});
