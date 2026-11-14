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
// shrinks, M2 moving each into what a body gives back or `Settled`.
const cellsLeft = ["ref_cells", "own_callbacks", "module_fns"];

test("the crate's facts are read alike by each body: no cell but those still to move", () => {
  const source = read("src/lower/crate_facts.rs");
  const start = source.indexOf("pub(in crate::lower) struct CrateFacts");
  const struct = source.slice(start, source.indexOf("\n}\n", start));
  const cells = [...struct.matchAll(/pub\(in crate::lower\) (\w+): [^\n]*\b(?:Cell|RefCell|OnceCell)</g)].map((m) => m[1]);
  expect(cells).toEqual(cellsLeft);
});

test("what the bodies lowered before settled is the pipeline's to write, a body's to read", () => {
  // A body holds it by a shared reference alone.
  expect(read("src/lower.rs")).toContain("    settled: &'a Settled,\n");
  const writers = lowering
    .filter((file) => /\bsettled\.\w+\.(?:insert|extend|push|remove|clear)\(/.test(read(file)));
  expect(writers).toEqual(["src/lower/pipeline.rs"]);
});
