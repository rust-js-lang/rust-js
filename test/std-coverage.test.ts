// How much of std's data structures rust-js lowers is a ratchet (ADR 0314):
// each stable inherent method of each, `+` where rust-js knows it, `-` where
// it refuses it, listed in docs/std-coverage.txt. A method known before and
// refused now fails here, as one newly known does until it's blessed
// (`BLESS=1`), so the diff shows it. Beside each, after a tab, what its body
// in std is made of (ADR 0557): `safe`, MIR rust-js could
// lower down to its vocabulary; `spec`, reaching a std-private trait's
// method on a type parameter; `opaque`, reaching a body the toolchain
// ships no MIR of; `memory`, reaching addressable memory; or `none`, no
// MIR of its own. That's the toolchain's, measured, not a promise: it's
// held equal so a toolchain upgrade's change shows.

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
  const known = (text: string) =>
    text
      .split("\n")
      .filter((l) => l.startsWith("+ "))
      .map((l) => l.slice(2).split("\t")[0]);
  const before = known(readFileSync(baseline, "utf8"));
  const knownNow = new Set(known(now));
  // Each method known before still is.
  expect(before.filter((m) => !knownNow.has(m))).toEqual([]);
  // And what's new is in the baseline, blessed.
  expect(now).toBe(readFileSync(baseline, "utf8"));
});

test("each std method's body is measured as one of the five kinds", () => {
  const lines = readFileSync(baseline, "utf8").split("\n").filter((l) => /^[+-] /.test(l));
  expect(lines.length).toBeGreaterThan(500);
  for (const line of lines) expect(line, line).toMatch(/^[+-] \S+\t(?:safe|spec|opaque|memory|none)$/);
  // The safe stratum is real: the methods a compiled std would take over
  // are the many, not the few, and what reaches memory stays measured.
  const kinds = lines.map((l) => l.split("\t")[1]);
  expect(kinds.filter((k) => k === "safe").length).toBeGreaterThan(100);
  expect(kinds.filter((k) => k === "memory").length).toBeGreaterThan(100);
  // The stratum the draft is about: Iterator's provided methods are safe,
  // the many, which a compiled std would take over.
  const iterator = readFileSync(baseline, "utf8").split("\n# ").find((section) => section.startsWith("Iterator "));
  expect(iterator).toBeDefined();
  expect(iterator!.split("\n").filter((l) => /^\+ .*\tsafe$/.test(l) || /^- .*\tsafe$/.test(l)).length).toBeGreaterThan(40);
});
