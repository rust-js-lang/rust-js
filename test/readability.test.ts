// How the generated JS reads is a gate, as what it computes is (ADR 0565):
// each shape a person wouldn't write, `scripts/readability.ts`'s, counted
// over the JS this repository commits as the compiler writes it now, the
// corpus's and the snapshots', and listed in docs/readability.txt with its
// share, how many in ten thousand lines. A share that grew fails here: a
// construct a change adds is read first, and blessed (`BLESS=1`, `bun run
// bless`) only where its ADR says why; one that shrank by one in ten
// thousand lines or more is blessed too, so the file says what got better,
// where it did. A case added, which moves each share a little either way,
// is no reason to bless: three agents each blessing for their own cases
// made this file a merge conflict of every change (ADR 0565, amended).

import { expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

import { SHAPES, measureFiles, report, shares } from "../scripts/readability";
import { root } from "./support";

const baseline = join(root, "docs/readability.txt");

/** The generated JS the repository commits: each corpus case's, and each
 * snapshot's, which their tests hold equal to what the compiler writes. */
function committed(): string[] {
  const corpus = [...new Bun.Glob("test/corpus/*.js").scanSync(root)];
  const snapshots = [...new Bun.Glob("test/snapshots/**/*.js").scanSync(root)];
  return [...corpus, ...snapshots].sort().map((file) => join(root, file));
}

test("the generated JS reads no worse than it read: no shape's share grew", () => {
  const files = committed();
  expect(files.length).toBeGreaterThan(500);
  const measured = measureFiles(files);
  const now = report(files.length, measured);
  if (process.env.BLESS) writeFileSync(baseline, now);
  const before = shares(readFileSync(baseline, "utf8"));
  const grown = [...shares(now)]
    .filter(([name, share]) => share > (before.get(name) ?? 0))
    .map(([name, share]) => `${name}: ${before.get(name)} to ${share} in 10,000 lines`);
  expect(grown, "the generated JS reads worse: read the construct, and bless it only where its ADR says why").toEqual([]);
  const better = [...shares(now)]
    .filter(([name, share]) => share <= (before.get(name) ?? 0) - 1)
    .map(([name, share]) => `${name}: ${before.get(name)} to ${share} in 10,000 lines`);
  expect(better, "the generated JS reads better: run `bun run bless` and review the diff").toEqual([]);
});

test("each shape is a line's, and the gate counts them all", async () => {
  expect(SHAPES.map(([name]) => name)).toEqual([
    "temporaries",
    "labeled blocks",
    "while (true)",
    "$iter",
    "Iterator.from",
    "return;",
    "renamed x$1",
  ]);
  const lines = ["  bb36: {", "  while (true) {", "  const _3 = x;", "  return;", "  const it = $iter(v);", "  const x$1 = Iterator.from(xs);"];
  const { counts } = (await import("../scripts/readability")).measure(lines);
  expect([...counts.values()]).toEqual([1, 1, 1, 1, 1, 1, 1]);
  expect(shares(readFileSync(baseline, "utf8")).size).toBe(SHAPES.length);
});
