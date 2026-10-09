// How much of std's data structures rust-js lowers is a ratchet (ADR 0314):
// each stable inherent method of each, `+` where rust-js knows it, `-` where
// it refuses it, listed in docs/std-coverage.txt. A method known before and
// refused now fails here, as one newly known does until it's blessed
// (`BLESS=1`), so the diff shows it.

import { expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

import { compiler, fixture, root, run } from "./support";

const baseline = join(root, "docs/std-coverage.txt");

test("rust-js knows each std data structure method it knew", () => {
  const dir = fixture("std-coverage");
  writeFileSync(join(dir, "lib.rs"), "pub fn nothing() {}\n");
  const measured = join(dir, "coverage.txt");
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--std-coverage", measured]);
  const now = readFileSync(measured, "utf8");
  if (process.env.BLESS) writeFileSync(baseline, now);
  const known = (text: string) => text.split("\n").filter((l) => l.startsWith("+ ")).map((l) => l.slice(2));
  const before = known(readFileSync(baseline, "utf8"));
  const knownNow = new Set(known(now));
  // Each method known before still is.
  expect(before.filter((m) => !knownNow.has(m))).toEqual([]);
  // And what's new is in the baseline, blessed.
  expect(now).toBe(readFileSync(baseline, "utf8"));
});
