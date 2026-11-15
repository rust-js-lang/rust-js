// std's items a recognizer knows by their path, each resolved to its
// `DefId` once a session (`recognition/std_paths.rs`, #1681's DefId step):
// found through `std`'s modules, or `core`'s and `alloc`'s where a crate
// loads no `std`.
import { beforeAll, expect, test } from "bun:test";
import { writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildCompiler, compiler, fixture, run } from "./support";

beforeAll(buildCompiler, 600_000);

// A `#![no_std]` crate's `str::CharIndices`, a row of the table, is core's,
// found through `core`: its `offset()` recognized, as a crate with `std`'s.
test("a #![no_std] crate's std item a row names is found through core", async () => {
  const dir = fixture("std-paths-no-std");
  writeFileSync(join(dir, "lib.rs"), [
    "#![no_std]",
    "pub fn rest(s: &str) -> usize {",
    "    let mut it = s.char_indices();",
    "    it.next();",
    "    it.offset()",
    "}",
    "",
  ].join("\n"));
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const lib = await import(join(dir, "lib.js"));
  expect(lib.rest("héllo")).toBe(1);
}, 300_000);

// A `#![no_std]` crate's `core::hint::black_box(x)` is `x`, as std's is: it's
// under `std::hint`, a module the table resolves in each of std's crates,
// core's here, as std's path writes it (#1681's module question).
test("a #![no_std] crate's item under a std module is under core's module of its path", async () => {
  const dir = fixture("std-paths-no-std-module");
  writeFileSync(join(dir, "lib.rs"), ["#![no_std]", "pub fn kept(x: u32) -> u32 {", "    core::hint::black_box(x) + 1", "}", ""].join("\n"));
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const lib = await import(join(dir, "lib.js"));
  expect(lib.kept(41)).toBe(42);
}, 300_000);
