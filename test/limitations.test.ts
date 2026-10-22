// docs/guide/limitations.md is kept true: each item it says is refused at compile
// time ends with an example, compiled here as a program's `main`. One that
// compiles, a limitation lifted, fails this, and says to take its line out.

import { expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildCompiler, compiler, fixture, root } from "./support";

/** Each `- item: \`example\`` line of the page, its item and its example. */
function examples(): { item: string; example: string }[] {
  const page = readFileSync(join(root, "docs/guide/limitations.md"), "utf8");
  return [...page.matchAll(/^- (.+?): `([^`]+)`$/gm)].map((m) => ({ item: m[1], example: m[2] }));
}

test("the limitations page has examples to check", () => {
  expect(examples().length).toBeGreaterThan(20);
});

for (const { item, example } of examples()) {
  test(`refused, as docs/guide/limitations.md says: ${item}`, () => {
    buildCompiler();
    const dir = fixture("limitation");
    writeFileSync(join(dir, "main.rs"), `#![allow(unused)]\nfn main() {\n${example}\n}\n`);
    const result = Bun.spawnSync([compiler, join(dir, "main.rs"), "-o", join(dir, "main.js")], { cwd: dir });
    const stderr = result.stderr.toString();
    // Refused by rust-js, not rustc's error of an example written wrong.
    expect(
      [result.exitCode === 0, stderr.includes("rust-js does not support")],
      result.exitCode === 0 ? `it compiles: take "${item}" out of docs/guide/limitations.md` : stderr,
    ).toEqual([false, true]);
  });
}
