// What the tests' own code holds to: one way to do each thing a test does
// often, which each test file asks rather than writing again.

import { expect, test } from "bun:test";
import { readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { root } from "./support";

// TypeScript's check of what generated declarations mean to TypeScript
// that imports them is test/type-checked.ts's `typeChecked`, each test's
// settings its parameters: 33 test files had each run `tsc` their own way,
// a tsconfig written and spawned, or its command line, which drifted apart
// in what each checked (#2103).
test("only test/type-checked.ts runs TypeScript's compiler", () => {
  const dir = join(root, "test");
  const runners = readdirSync(dir)
    .filter((name) => /\.tsx?$/.test(name) && name !== "type-checked.ts" && name !== "test-helpers.test.ts")
    .filter((name) => readFileSync(join(dir, name), "utf8").includes("typescript/bin/tsc"));
  expect(runners).toEqual([]);
});
