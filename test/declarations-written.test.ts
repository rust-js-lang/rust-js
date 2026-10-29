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
