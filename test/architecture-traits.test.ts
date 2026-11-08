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

// A function's drop slots are numbered one way (#887): its type parameters,
// then its associated types in `drop_items`' order (`item_slot`), then the
// types built of them; `drop_slot` gives an index's type and `slot_index`
// a type's index, in traits/drop_params.rs alone. A body is given its drops
// by their slots' types, never under a number of its own: `u32::MAX - n` for
// a lent associated type's and `u32::MAX / 2` for a copied default's impl's
// had each met another numbering, and the analysis, the manifest's built
// slots and the type facts had each counted past the parameters themselves.
test("a drop slot is numbered by drop_params.rs alone", () => {
  const defines = /\bfn (item_slot|slot_item|slot_index|drop_slot)\b/;
  expect(sources.filter(file => defines.test(read(file)))).toEqual(["src/lower/traits/drop_params.rs"]);
  const counts = /generics_of\([^)]*\)\s*\.count\(\)( as u32)?\s*\+|checked_sub\(count\)|\bfn slot_of\b/;
  expect(sources.filter(file => counts.test(read(file)))).toEqual(["src/lower/traits/drop_params.rs"]);
  const lowering = sources.filter(file => /^src\/lower\/(drops|traits)/.test(file) || file === "src/lower/drops.rs");
  expect(lowering.filter(file => /u32::MAX/.test(read(file)))).toEqual([]);
});

// A trait's arguments of a type, its `Self` and each parameter, are
// `args_of`'s alone (recognition/traits.rs): each parameter its kind's, a
// lifetime erased. One built by repeating the type, of a trait of a
// lifetime, `Spans<'a>`, was a trait ref rustc can't select by, and
// rust-js crashed (#951).
test("a trait's arguments of a type are args_of's alone", () => {
  const repeated = files("src/lower")
    .filter((file) => file.endsWith(".rs") && file !== "src/lower/recognition/traits.rs")
    .flatMap((file) => (/mk_args_from_iter\(\s*std::iter::repeat_n\(/.test(read(file)) ? [file] : []));
  expect(repeated).toEqual([]);
});
