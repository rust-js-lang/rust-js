// What `#[rust_js::types]` says a type is to TypeScript, read by its one
// reader (ADR 0811): what it reads, declared as written, and what it
// can't, an error saying what it reads.

import { expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { runSync } from "./child";
import { compiler, fixture, root, run } from "./support";

// What `#[rust_js::types]` says is a TypeScript type the compiler reads, or
// an error saying what it reads, where it's written: never a declaration
// made of what it couldn't read (ADR 0811).
test("a #[rust_js::types] the compiler can't read is an error saying what it reads", () => {
  const dir = fixture("declarations-written-unread");
  const written = ["Map<string", "{ run<T>(f: () => T): T", "string |", "typeof Object.assign", "A &"];
  for (const types of written) {
    writeFileSync(join(dir, "lib.rs"), `use std::marker::PhantomData;

#[cfg_attr(rust_js, rust_js::types = ${JSON.stringify(types)})]
pub struct Task(PhantomData<u8>);
`);
    for (const command of [[compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")], [compiler, "--rustc", "--edition=2024", "--crate-type=lib", "--crate-name=dep", "--emit=metadata", "--target=wasm32-unknown-unknown", `--out-dir=${dir}`, join(dir, "lib.rs")]]) {
      const refused = runSync(command, root, 60_000);
      expect(refused.code).not.toBe(0);
      expect(refused.stderr).toContain(`rust-js: \`#[rust_js::types = ${JSON.stringify(types)}]\` isn't a TypeScript type rust-js reads`);
      expect(refused.stderr).toContain("a name, `Element`, of a module, `react#ReactNode`");
    }
  }
});

// One attribute of what a type is: a type's, a trait's, what a type it
// bounds is, and a trait's associated type's, what its projection is
// (#1817); on a function, a method or a constant it's refused, never left
// unread, so no second form grows beside it.
test("a #[rust_js::types] on what isn't a type is refused, naming what it says", () => {
  const dir = fixture("declarations-written-not-a-type");
  for (const item of ["pub fn made() {}", "pub const LIMIT: u8 = 1;", "pub trait Listen<E> {\n    #[cfg_attr(rust_js, rust_js::types = \"string\")]\n    fn listen(&self);\n}"]) {
    const attribute = item.startsWith("pub trait") ? "" : `#[cfg_attr(rust_js, rust_js::types = "string")]\n`;
    writeFileSync(join(dir, "lib.rs"), `${attribute}${item}\n`);
    const refused = runSync([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")], root, 60_000);
    expect([refused.code === 0, refused.stderr]).toEqual([
      false,
      expect.stringContaining("rust-js: `#[rust_js::types]` says what a type is to TypeScript: a struct's, an enum's, an alias's, a trait's, what a type it bounds is, or a trait's associated type's, what its projection is; not a"),
    ]);
  }
});

// An intersection, `string[] & { index: number }`, a `RegExpExecArray`'s,
// is read and declared as written (#303, ADR 0811, amended).
test("a #[rust_js::types] intersection is declared as written", () => {
  const dir = fixture("declarations-written-intersection");
  writeFileSync(join(dir, "lib.rs"), `use std::marker::PhantomData;

#[cfg_attr(rust_js, rust_js::types = "string[] & { index: number }")]
pub struct Found(PhantomData<u8>);

pub fn found() -> Option<&'static Found> {
    None
}
`);
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  expect(readFileSync(join(dir, "lib.d.ts"), "utf8")).toContain("export function found(): (string[] & {\n    index: number;\n}) | null | undefined;");
});
