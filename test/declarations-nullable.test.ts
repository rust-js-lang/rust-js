// A nullable field's `None` is `null`, and its declaration says so, a
// `Cell`'s too, `title: string | null` (ADR 0586): where its value is any
// value, `unknown` absorbs it (#1850), so this is of a type that isn't.

import { expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { compiler, fixture, run } from "./support";

test("a nullable Cell field of text is declared its text or null", () => {
  const dir = fixture("declarations-nullable-cell");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::Cell;

pub struct Context {
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub title: Cell<Option<&'static str>>,
}

pub fn make() -> Context {
    Context { title: Cell::new(None) }
}
`);
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  expect(readFileSync(join(dir, "lib.d.ts"), "utf8")).toContain("export interface Context {\n    title: string | null;\n}");
  expect(readFileSync(join(dir, "lib.js"), "utf8")).toContain("title: null");
});
