// The webapi crate's coverage of TypeScript's DOM is a ratchet: each member
// it binds is listed in webapi/coverage.txt, and losing one fails here, as
// gaining one does until it's blessed (`BLESS=1`), so the diff shows it.
// See docs/research/web-platform-coverage.md.

import { expect, test } from "bun:test";
import { existsSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

import { functionsAsObjects, measure, render } from "../webapi/coverage";
import { generatorOutput, root } from "./support";

const baseline = join(root, "webapi/coverage.txt");

test("the webapi crate binds what it bound of TypeScript's DOM", async () => {
  const now = await measure();
  if (process.env.BLESS) writeFileSync(baseline, render(now));
  const before = existsSync(baseline) ? readFileSync(baseline, "utf8").split("\n").filter((l) => l && !l.startsWith("#")) : [];
  const covered = new Set(now.covered);
  // Each member bound before still is.
  expect(before.filter((x) => !covered.has(x))).toEqual([]);
  // And what's new is in the baseline, blessed.
  expect(render(now)).toBe(readFileSync(baseline, "utf8"));
}, 60_000);

// The webapi crate is what webapi/generate.ts writes of WebIDL (ADR 0024),
// each name as scripts/rust-names.ts names it (ADR 0376).
test("the webapi crate is what its generator writes", () => {
  expect(generatorOutput("webapi/generate.ts")).toBe(readFileSync(join(root, "webapi/src/lib.rs"), "utf8"));
});

// ADR 0653: a function JS gives is a `dyn Fn` of its signature, the one
// type of a JS function value, never a `JsObject`: what TypeScript types
// as a function, `onclick`'s `((ev: PointerEvent) => any) | null`, is
// `Option<&'static dyn Fn(&PointerEvent) -> Option<&'static Unknown>>`.
test("a function the webapi crate reads is a dyn Fn of its signature, not a JsObject", async () => {
  expect(await functionsAsObjects()).toEqual([]);
}, 60_000);
