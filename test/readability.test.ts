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
// made this file a merge conflict of every change (ADR 0565, amended). So
// the shares compared are of the files the file lists and the repository
// has, like for like, each file's counts listed: a case added is listed
// where it's blessed next, and moves no share before then.

import { expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

import { SHAPES, compared, measure, measuredOf, report, type Measured } from "../scripts/readability";
import { root } from "./support";

const baseline = join(root, "docs/readability.txt");

/** The generated JS the repository commits: each corpus case's, and each
 * snapshot's, which their tests hold equal to what the compiler writes. */
function committed(): string[] {
  const corpus = [...new Bun.Glob("test/corpus/*.js").scanSync(root)];
  const snapshots = [...new Bun.Glob("test/snapshots/**/*.js").scanSync(root)];
  return [...corpus, ...snapshots].sort().map((file) => join(root, file));
}

/** Of `before` and `now`, each shape that grew, and each that shrank by one
 * in ten thousand lines or more, like for like. */
function moved(before: Map<string, Measured>, now: Map<string, Measured>) {
  const shares = compared(before, now);
  const line = ([name, share]: [string, number]) => `${name}: ${shares.before.get(name)} to ${share} in 10,000 lines`;
  return {
    grown: [...shares.now].filter(([name, share]) => share > shares.before.get(name)!).map(line),
    better: [...shares.now].filter(([name, share]) => share <= shares.before.get(name)! - 1).map(line),
    added: shares.added,
  };
}

test("the generated JS reads no worse than it read: no shape's share grew", () => {
  const files = committed();
  expect(files.length).toBeGreaterThan(500);
  const now = new Map(files.map((file) => [file.slice(root.length + 1), measure(readFileSync(file, "utf8").split("\n"))]));
  if (process.env.BLESS) writeFileSync(baseline, report(now));
  const { grown, better, added } = moved(measuredOf(readFileSync(baseline, "utf8")), now);
  if (added.length > 0) console.log(`not yet in docs/readability.txt, counted once it's blessed: ${added.join(", ")}`);
  expect(grown, "the generated JS reads worse: read the construct, and bless it only where its ADR says why").toEqual([]);
  expect(better, "the generated JS reads better: run `bun run bless` and review the diff").toEqual([]);
});

// Like for like: a case added moves no share, whatever it has; one more of
// a shape in a case both have grows its share.
test("a case added moves no share, and a shape added to a case grows one", () => {
  const of = (lines: number, temporaries: number): Measured => ({
    lines,
    counts: new Map(SHAPES.map(([name]) => [name, name === "temporaries" ? temporaries : 0])),
  });
  const before = new Map([["a.js", of(1000, 10)]]);
  expect(moved(before, new Map([...before, ["b.js", of(9000, 0)]]))).toEqual({ grown: [], better: [], added: ["b.js"] });
  expect(moved(before, new Map([...before, ["b.js", of(10, 10)]])).grown).toEqual([]);
  expect(moved(before, new Map([["a.js", of(1000, 11)]])).grown).toEqual(["temporaries: 100 to 110 in 10,000 lines"]);
  expect(moved(before, new Map([["a.js", of(1000, 9)]])).better).toEqual(["temporaries: 100 to 90 in 10,000 lines"]);
  // Read back as it's written.
  expect(measuredOf(report(before))).toEqual(before);
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
  const blessed = readFileSync(baseline, "utf8");
  expect(blessed.split("\n").filter((line) => SHAPES.some(([name]) => line.startsWith(`${name}\t`))).length).toBe(SHAPES.length);
});
