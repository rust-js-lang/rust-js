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
// Of the base's crate and MIR's, and the pipeline (#2299's cut).
const lowering = ([["crates/rust-js-lower/src", "crates/rust-js-lower/src"], ["crates/rust-js-mir/src", "crates/rust-js-mir/src"], ["src/pipeline", "src/pipeline"]] as const)
  .flatMap(([dir, prefix]) => readdirSync(join(root, dir), { recursive: true, encoding: "utf8" }).filter((file) => file.endsWith(".rs")).map((file) => `${prefix}/${file}`))
  .concat(["src/pipeline.rs"])
  .sort();

// Each field of `CrateFacts` a body may change, a cell: a list that only
// shrinks, M2 moving each into what a body gives back or `Settled`, now
// empty. A fact found the first time a body asks is a `Lazy`, below.
const cellsLeft: string[] = [];

test("the crate's facts are read alike by each body: no cell but those still to move", () => {
  const source = read("crates/rust-js-lower/src/crate_facts.rs");
  const start = source.indexOf("pub struct CrateFacts");
  const struct = source.slice(start, source.indexOf("\n}\n", start));
  const cells = [...struct.matchAll(/pub\(crate\) (\w+): [^\n]*\b(?:Cell|RefCell|OnceCell)</g)].map((m) => m[1]);
  expect(cells).toEqual(cellsLeft);
});

// A `Lazy` is found of the crate's facts alone, its one finder each: the
// `RefCell`s' (drops/ref_cells.rs), at about 2% of a compile (ADR 1148).
test("each of the crate's facts a body finds the first time it asks is a Lazy, of one finder", () => {
  const finders = lowering.filter((file) => /\.get_or_find\(/.test(read(file)));
  expect(finders).toEqual(["crates/rust-js-lower/src/drops/ref_cells.rs"]);
  const source = read("crates/rust-js-lower/src/crate_facts.rs");
  const start = source.indexOf("pub struct CrateFacts");
  const struct = source.slice(start, source.indexOf("\n}\n", start));
  expect([...struct.matchAll(/pub (\w+): &'a Lazy</g)].map((m) => m[1])).toEqual(["ref_cells"]);
});

test("what the bodies lowered before settled is the pipeline's to write, a body's to read", () => {
  // A body holds it by a shared reference alone.
  expect(read("crates/rust-js-lower/src/lib.rs")).toContain("    pub settled: &'a Settled<'tcx>,\n");
  const writers = lowering
    .filter((file) => /\bsettled\.\w+\.(?:insert|extend|push|remove|clear)\(/.test(read(file)));
  expect(writers).toEqual(["src/pipeline.rs"]);
});

// Why lowering stopped is a value, `Refusal`, the pipeline reads (#1797 step
// (1), ADR 1148, amended): made in refusal.rs alone, by `refuse`, `error`
// and `emitted`, and `R`'s error is it, with no conversion from rustc's
// `ErrorGuaranteed`, so a lowering can't give an error it emitted itself as
// its refusal, as 12 places did, `Err(self.tcx.dcx().span_err(..))`.
test("a refusal is refusal.rs's to make, and R's error is it, from nothing else", () => {
  const all = [...lowering, "crates/rust-js-lower/src/lib.rs"];
  expect(all.filter((file) => file !== "crates/rust-js-lower/src/refusal.rs" && /\bRefusal::\w+\(/.test(read(file)))).toEqual([]);
  expect(read("crates/rust-js-lower/src/lib.rs")).toContain("type R<T> = Result<T, refusal::Refusal>;");
  expect(all.filter((file) => /impl\s+From<[^>]*ErrorGuaranteed>\s+for\s+(?:(?:crate|rust_js_lower)::refusal::)?Refusal/.test(read(file)))).toEqual([]);
  expect(all.filter((file) => /Err\(\s*(?:self\.|cx\.)?tcx\s*\.dcx\(\)/.test(read(file)))).toEqual([]);
});
