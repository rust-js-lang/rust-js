// How much of Next.js's public API the next crate binds is a ratchet (ADR
// 0346): each export of each public module, `+` where the crate binds it,
// `-` where it doesn't, listed in docs/next-coverage.txt. An export bound
// before and unbound now fails here, as one newly bound does until it's
// blessed (`BLESS=1`), so the diff shows it.

import { expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

import { fixture, root, run } from "./support";

const baseline = join(root, "docs/next-coverage.txt");

test("the next crate binds each Next.js export it bound", () => {
  const measured = join(fixture("next-coverage"), "coverage.txt");
  run(["bun", join(root, "scripts/next-coverage.ts"), measured]);
  const now = readFileSync(measured, "utf8");
  if (process.env.BLESS) writeFileSync(baseline, now);
  const bound = (text: string) => text.split("\n").filter((l) => l.startsWith("+ ")).map((l) => l.slice(2));
  const before = bound(readFileSync(baseline, "utf8"));
  const boundNow = new Set(bound(now));
  // Each export bound before still is.
  expect(before.filter((e) => !boundNow.has(e))).toEqual([]);
  // And what's new is in the baseline, blessed.
  expect(now).toBe(readFileSync(baseline, "utf8"));
});
