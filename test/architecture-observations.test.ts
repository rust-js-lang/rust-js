// What a body's lowering found is given back, never written into what
// every body shares (#1671 change 4, M2): `CrateFacts` is the crate's facts
// found before any body is lowered, read alike by each; what the bodies
// lowered before settled is `Settled`, which only the pipeline writes,
// between one body and the next. Split from architecture.test.ts at its
// cap (a test file is at most 1500 lines).
import { expect, test } from "bun:test";
import { readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";
import { root } from "./support";

const read = (path: string) => readFileSync(join(root, path), "utf8");
// The lowering's sources, in name order, as macOS lists them and Linux,
// CI's, doesn't.
const lowering = readdirSync(join(root, "src", "lower"), { recursive: true, encoding: "utf8" })
  .filter((file) => file.endsWith(".rs"))
  .map((file) => `src/lower/${file}`)
  .sort();

// Each field of `CrateFacts` a body may change, a cell: a list that only
// shrinks, M2 moving each into what a body gives back or `Settled`, now
// empty. A fact found the first time a body asks is a `Lazy`, below.
const cellsLeft: string[] = [];

test("the crate's facts are read alike by each body: no cell but those still to move", () => {
  const source = read("src/lower/crate_facts.rs");
  const start = source.indexOf("pub(in crate::lower) struct CrateFacts");
  const struct = source.slice(start, source.indexOf("\n}\n", start));
  const cells = [...struct.matchAll(/pub\(in crate::lower\) (\w+): [^\n]*\b(?:Cell|RefCell|OnceCell)</g)].map((m) => m[1]);
  expect(cells).toEqual(cellsLeft);
});

// A `Lazy` is found of the crate's facts alone, its one finder each: the
// `RefCell`s' (drops/ref_cells.rs), at about 2% of a compile (ADR 1148).
test("each of the crate's facts a body finds the first time it asks is a Lazy, of one finder", () => {
  const finders = lowering.filter((file) => /\.get_or_find\(/.test(read(file)));
  expect(finders).toEqual(["src/lower/drops/ref_cells.rs"]);
  const source = read("src/lower/crate_facts.rs");
  const start = source.indexOf("pub(in crate::lower) struct CrateFacts");
  const struct = source.slice(start, source.indexOf("\n}\n", start));
  expect([...struct.matchAll(/pub\(in crate::lower\) (\w+): &'a Lazy</g)].map((m) => m[1])).toEqual(["ref_cells"]);
});

test("what the bodies lowered before settled is the pipeline's to write, a body's to read", () => {
  // A body holds it by a shared reference alone.
  expect(read("src/lower.rs")).toContain("    settled: &'a Settled<'tcx>,\n");
  const writers = lowering
    .filter((file) => /\bsettled\.\w+\.(?:insert|extend|push|remove|clear)\(/.test(read(file)));
  expect(writers).toEqual(["src/lower/pipeline.rs"]);
});
