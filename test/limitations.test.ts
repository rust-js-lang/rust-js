// docs/guide/limitations.md is kept true: each item it says is refused at compile
// time ends with an example, compiled here as a program's `main`. One that
// compiles, a limitation lifted, fails this, and says to take its line out.

import { expect, test } from "bun:test";
import { writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildCompiler, compiler, fixture, limitations } from "./support";

test("the limitations page has examples to check", () => {
  expect(limitations().length).toBeGreaterThan(20);
});

for (const { item, example, title } of limitations()) {
  test(title, () => {
    buildCompiler();
    const dir = fixture("limitation");
    writeFileSync(join(dir, "main.rs"), `#![allow(unused)]\nfn main() {\n${example}\n}\n`);
    const result = Bun.spawnSync([compiler, join(dir, "main.rs"), "-o", join(dir, "main.js")], { cwd: dir });
    const stderr = result.stderr.toString();
    // Refused by rust-js, not rustc's error of an example written wrong: a
    // feature it lacks, `rust-js does not support`, or a rule of its own,
    // `rust-js: ..`, as another module's write of a module's variable is.
    expect(
      [result.exitCode === 0, /error: rust-js( does not support|:)/.test(stderr)],
      result.exitCode === 0 ? `it compiles: take "${item}" out of docs/guide/limitations.md` : stderr,
    ).toEqual([false, true]);
  });
}
