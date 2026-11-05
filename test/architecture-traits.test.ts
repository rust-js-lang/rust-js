// The trait rules of the architecture, split from architecture.test.ts at
// its cap (a test file is at most 1500 lines): how a trait's default copied
// into an impl is put in the impl's terms (ADR 0049).
import { expect, test } from "bun:test";
import { readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";
import { root } from "./support";

const read = (path: string) => readFileSync(join(root, path), "utf8");
// In name order, as macOS lists them and Linux, CI's, doesn't.
const files = (directory: string): string[] => readdirSync(join(root, directory), { withFileTypes: true })
  .sort((a, b) => (a.name < b.name ? -1 : a.name > b.name ? 1 : 0))
  .flatMap(entry => entry.isDirectory() ? files(`${directory}/${entry.name}`) : [`${directory}/${entry.name}`]);
const sources = files("src").filter(path => path.endsWith(".rs"));

// A copied default is put in its impl's terms one way (#804): its own
// parameters numbered after the impl's (`default_args`), where they're known
// (`default_typing_env`), each defined by given.rs and asked by the lowering
// of its body, its slots and the analysis of what it asks. Three places had
// mapped it, each its own way: the lowering's own parameters numbered as the
// trait's, which the impl's of that number were taken for, an environment of
// the impl's beside the body's, and the analysis's of its own.
test("a copied default is put in its impl's terms by default_args alone", () => {
  const defines = /\bfn (default_args|default_typing_env)\b/;
  expect(sources.filter(file => defines.test(read(file)))).toEqual(["src/lower/traits/given.rs"]);
  const asks = /\bdefault_(args|typing_env)\(/;
  expect(sources.filter(file => asks.test(read(file)))).toEqual([
    "src/lower/analysis/type_facts.rs",
    "src/lower/traits/drop_params.rs",
    "src/lower/traits/impls.rs",
  ]);
  // Nor a second environment beside the body's, the impl's, which a call
  // resolved in alone, missing a bound of the default's own parameter (ADR
  // 0560): `impl_typing_env` and the impl's `self_env`, which this replaced.
  expect(sources.filter(file => /\bimpl_typing_env\b|\bself_env\b/.test(read(file)))).toEqual([]);
});
