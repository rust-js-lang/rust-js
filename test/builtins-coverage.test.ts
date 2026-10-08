// The js crate's coverage of TypeScript's ES library is a ratchet: each member
// it binds is listed in builtins/coverage.txt, and losing one fails here, as
// gaining one does until it's blessed (`BLESS=1`), so the diff shows it.
// See docs/decisions/0283-js-crate-covers-typescripts-es-library.md.

import { expect, test } from "bun:test";
import { existsSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

import { measure, render } from "../builtins/coverage";
import { root } from "./support";

const baseline = join(root, "builtins/coverage.txt");

test("the js crate binds what it bound of TypeScript's ES library", async () => {
  const now = await measure();
  if (process.env.BLESS) writeFileSync(baseline, render(now));
  const before = existsSync(baseline) ? readFileSync(baseline, "utf8").split("\n").filter((l) => l && !l.startsWith("#")) : [];
  const covered = new Set(now.covered);
  // Each member bound before still is.
  expect(before.filter((x) => !covered.has(x))).toEqual([]);
  // And what's new is in the baseline, blessed.
  expect(render(now)).toBe(readFileSync(baseline, "utf8"));
}, 60_000);

// The typed arrays are one shape over eleven element types, written once in
// builtins/typed_arrays.ts: what's committed is what it writes.
test("the typed arrays are what their template writes", () => {
  const file = join(root, "builtins/src/typed_arrays.rs");
  const committed = readFileSync(file, "utf8");
  const p = Bun.spawnSync([process.execPath, join(root, "builtins/typed_arrays.ts")], { cwd: root });
  expect(p.exitCode).toBe(0);
  const written = readFileSync(file, "utf8");
  writeFileSync(file, committed);
  expect(written).toBe(committed);
});

// Intl's formatters and their options, written once in builtins/intl.ts:
// what's committed is what it writes.
test("Intl is what its template writes", () => {
  const file = join(root, "builtins/src/intl.rs");
  const committed = readFileSync(file, "utf8");
  const p = Bun.spawnSync([process.execPath, join(root, "builtins/intl.ts")], { cwd: root });
  expect(p.exitCode).toBe(0);
  const written = readFileSync(file, "utf8");
  writeFileSync(file, committed);
  expect(written).toBe(committed);
});
