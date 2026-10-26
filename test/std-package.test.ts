// std's bodies as a package (ADR 0561): `@rust-js/std`,
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
  expect(committed.library.items.length).toBeGreaterThan(4);
  expect(committed.library.items.every((item: { rust_path: string }) => item.rust_path.includes("core::iter"))).toBe(true);
  const pkg = JSON.parse(readFileSync(join(root, "std", "package.json"), "utf8"));
  const version = readFileSync(join(root, "Cargo.toml"), "utf8").match(/^version = "([^"]+)"/m)![1];
  expect([pkg.name, pkg.version]).toEqual(["@rust-js/std", version]);
  // A compile of std's bodies, 6.2 s alone since std's iterator adapters
  // compile (ADRs 0908, 0917): past 30 s beside check:fast's other jobs.
}, 120_000);

// What the compiler writes imports the package by its name, each module by
// its path: `@rust-js/std/iter.js`, as `@rust-js/runtime` is imported. A
// `Peekable` is made by std's own `peekable`, and stepped as the JS iterator
// its struct is, `$next`, its `next` the compiled one (ADR 0561).
test("a Peekable is made by std's own peekable, imported from @rust-js/std, and stepped as a JS iterator", () => {
  const dir = fixture("std-package-import");
  const file = join(dir, "lib.rs");
  Bun.write(file, "pub fn first(v: &[u32]) -> Option<u32> { let mut p = v.iter().copied().peekable(); p.next() }\n");
  run([compiler, file, "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('import { iterator_peekable } from "@rust-js/std/iter.js"');
  expect(js).toContain("$next(p)");
  expect(js).not.toContain("peekableIterator_next");
  const std = readFileSync(join(root, "std", "iter.js"), "utf8");
  expect(std).toContain("return $stdIterator(\n    { iter: self },\n    {\n      next: (iterator) =>");
  // Its hint asks what it steps by the dictionary it's given (ADR 0561).
  expect(std).toContain("hint: (peekable) => $peekableHint(peekable, SelfIterator),");
});

// ADR 0564: a std body's JS is tidied as the app's is, though its optimized
// MIR leaves what the app's MIR doesn't: `next_back`'s test of `option` after
// its early return of it, which can't hold, is gone, as is a `bool`
// temporary tested once, and a tail written into two branches.
test("a std body's JS has no test its early return decided, no once-read bool, no tail written twice", () => {
  const std = readFileSync(join(root, "std", "iter.js"), "utf8");
  const nextBack = std.slice(std.indexOf("export function peekableDoubleEndedIterator_nextBack("), std.indexOf("export function peekableDoubleEndedIterator_rfold("));
  expect(nextBack.match(/if \(option != null\)/g)).toHaveLength(1);
  expect(nextBack).toContain("return option;\n    }\n    const x = v.value;\n    v.value = undefined;\n    return x;\n");
  expect(std).not.toMatch(/tmp\$?\d* = !tmp/);
  // No statement line written twice in a row at the two ends of an `if`/`else`.
  // The `return` is indented one level past the `}` and the `else {` that
  // the formatter writes on one line.
  expect(std).not.toMatch(/\n(\s*)\s{2}(return [^\n]*;)\n\1} else \{[^]*?\n\1\s{2}\2\n/);
});
