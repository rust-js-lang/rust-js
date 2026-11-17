// Whether code rust-js doesn't compile sees a value, another crate (a
// library's consumer) or JS, is asked of one owner, `library::escapes` and
// `library::seen_outside` (#2015): six checks answered it apart and
// disagreed. Each that's left is named here with its issue, a ratchet that
// only falls. The export rules themselves, which say what a module exports
// and so what JS sees (pipeline.rs, classes.rs, bindings.rs, consts.rs and
// the like), and std's stability, are its sources, not its consumers, and
// out of this test's scope; so is `js_given.rs`, which owns `js_facing`'s
// rule.

import { expect, test } from "bun:test";
import { readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";
import { root } from "./support";

const read = (path: string) => readFileSync(join(root, path), "utf8");
const rust = (directory: string) =>
  readdirSync(join(root, directory), { recursive: true, encoding: "utf8" })
    .filter((file) => file.endsWith(".rs"))
    .map((file) => `${directory}/${file}`)
    .sort();
const count = (file: string, pattern: RegExp) => (read(file).match(pattern) ?? []).length;

// rustc's own answer of what another crate reaches, `effective_visibilities()`
// and `is_exported`, which disagree with `library::reachable` of a trait
// impl's method: `library.rs` alone, which owns it, and those named.
const rustcReach: Record<string, string> = {
  "src/lower/library.rs": "the owner",
  "src/lower/analysis/fn_pointers.rs": "#2024: a type JS can name",
  "src/lower/analysis/collected.rs": "#2024: an inherent impl's constant JS can name (`exported_value_const`)",
};

// What an analysis asks of an item, a type, a field, a `static` or a key,
// through `visibility(..).is_public()`: what JS can reach of it, #2024's
// owner once it's written, and so many each.
const itemPublic: Record<string, [number, string]> = {
  "src/lower/analysis/js_given.rs": [1, "js_facing's rule, `exported_function`, its owner"],
  "src/lower/analysis/naming.rs": [1, "a `pub use`, the module's export, an export rule"],
  "src/lower/analysis/cell_borrows.rs": [1, "#2024: keys"],
  "src/lower/analysis/cell_flow.rs": [1, "#2024: a key"],
  "src/lower/analysis/lazy_locals.rs": [1, "#2024: a key"],
  "src/lower/analysis/nulled_params/js_nones.rs": [1, "#2024: a public type's field"],
  "src/lower/analysis/nulled_params/read_alike.rs": [1, "#2024: a key"],
  "src/lower/analysis/owned_by_return.rs": [1, "#2024: a field"],
  "src/lower/analysis/scopes.rs": [1, "#2024: a type"],
  "src/lower/instances.rs": [3, "#2024: whether JS or another crate reaches a generic owner, `seen_whole`'s `Shared` and `Counted`, a `pub` trait's impl by its exported dictionary too, and a trait"],
  "src/lower/analysis/wholes.rs": [3, "#2024: a public type's field (two calls), a key"],
};

test("rustc's own answer of what another crate reaches is asked only where #2015 names", () => {
  const asking = [...rust("src/lower"), "src/lower.rs"].filter((file) => /effective_visibilities\(|\.is_exported\(/.test(read(file)));
  expect(asking.filter((file) => !(file in rustcReach))).toEqual([]);
});

test("an analysis asks an item's `is_public()` only as #2015's ratchet allows", () => {
  const counted = [...rust("src/lower/analysis"), "src/lower/instances.rs"]
    .map((file) => [file, count(file, /visibility\([^)]*\)\.is_public\(\)/g)] as const)
    .filter(([, n]) => n > 0);
  expect(counted.filter(([file, n]) => n > (itemPublic[file]?.[0] ?? 0)).map(([file, n]) => `${file}: ${n}, at most ${itemPublic[file]?.[0] ?? 0}`)).toEqual([]);
});
