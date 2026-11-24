// The std paths' rule of the architecture (#1681's change 2, the DefId
// step): what a recognizer knows of std's items it asks of the `StdPath`
// table, by `DefId`, `crates/rust-js-lower/src/recognition/std_paths.rs` (#1681), not of
// a path's text. Each file's asks of the text, `std_path(`, a `"std::`
// literal, and `def_path_str(` tested against text, are at most
// `test/std-path-text.txt`'s: a ratchet the passes onto the table lower, and
// each file's left of it says why it's text by design.
import { expect, test } from "bun:test";
import { readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";
import { root } from "./support";

const read = (path: string) => readFileSync(join(root, path), "utf8");
// In name order, as macOS lists them and Linux, CI's, doesn't.
const files = (directory: string): string[] => readdirSync(join(root, directory), { withFileTypes: true })
  .sort((a, b) => (a.name < b.name ? -1 : a.name > b.name ? 1 : 0))
  .flatMap(entry => entry.isDirectory() ? files(`${directory}/${entry.name}`) : [`${directory}/${entry.name}`]);

// The table's own file, whose rows are std's paths' text.
const owner = "crates/rust-js-lower/src/recognition/std_paths.rs";

const allowed: Record<string, number> = Object.fromEntries(
  read("test/std-path-text.txt").split("\n").filter((line) => line && !line.startsWith("#"))
    .map((line) => line.split("\t")).map(([file, n]) => [file, Number(n)]),
);

// A line's asks of a path's text: a call of `std_path(`, a `"std::` literal,
// or `def_path_str(` on a line that tests what it gives, `==`, `!=`,
// `matches!`, a `match`, `starts_with`, `ends_with`, `contains` or
// `strip_prefix`. Comments say what they will.
const asks = (text: string) =>
  text.split("\n").filter((line) => !line.trim().startsWith("//")).reduce((n, line) => {
    const called = (line.match(/(?<![\w:])std_path\(/g) ?? []).length - (/\bfn std_path\(/.test(line) ? 1 : 0);
    const literal = (line.match(/"std::/g) ?? []).length;
    const tested = /def_path_str\(/.test(line) && /==|!=|matches!|\bmatch\b|starts_with|ends_with|contains\(|strip_prefix/.test(line) ? 1 : 0;
    return n + called + literal + tested;
  }, 0);

test("std's items are asked of the StdPath table, not a path's text", () => {
  const grown = [...files("crates/rust-js-lower/src").filter((f) => f !== "crates/rust-js-lower/src/lib.rs"), ...files("crates/rust-js-mir/src"), "src/pipeline.rs", ...files("src/pipeline")].filter((file) => file.endsWith(".rs") && file !== owner)
    .map((file) => [file, asks(read(file)), allowed[file] ?? 0] as const)
    .filter(([, now, was]) => now > was)
    .map(([file, now, was]) => `${file}: ${now} asks of a path's text, the ratchet's ${was}: ask ${owner}'s table by DefId`);
  expect(grown).toEqual([]);
});

// A file the ratchet lists has asks still: one that has none is taken off
// it, so the list says what's left.
test("the std path text ratchet lists only what's left", () => {
  const stale = Object.keys(allowed).filter((file) => asks(read(file)) === 0);
  expect(stale).toEqual([]);
});

// `cmp::max` and `cmp::min` are known by one answer, `cmp_extreme`, and
// `Ord`'s methods beside it by `ord_extreme`, both recognition/traits.rs':
// sorts.rs's `is_cmp_max_or_min` asked their diagnostic items again, and
// loop_ranges.rs read both by name (#2174).
test("std's max and min are asked of cmp_extreme and ord_extreme alone", () => {
  const names = /"cmp_(?:ord_)?(?:max|min)"|\bCmp(?:Max|Min)Fn\b/;
  const askers = files("crates").filter((file) => file.endsWith(".rs") && names.test(read(file)));
  expect(askers).toEqual(["crates/rust-js-lower/src/recognition/std_paths.rs", "crates/rust-js-lower/src/recognition/traits.rs"]);
});

// Whether an `Option`'s method is `copied()` or `cloned()` is one answer,
// `option_cloned_kind`, which says which: inherent.rs read the name
// alone and loop_ranges.rs the method of an `Option` (#2174).
test("an Option's copied or cloned is asked of option_cloned_kind alone", () => {
  // A `copied`/`cloned` name tested against `Option`, its lang or
  // diagnostic item or an `option` guard: but the method recognizer's own
  // table of `Option`'s methods (inherent.rs), which maps every one to its
  // `Std`, and `option_cloned_kind` itself.
  const asks = /"(?:copied|cloned)"(?: \| "(?:copied|cloned)")?[^\n]*(?:sym::Option|LangItem::Option|if option\b)|item_name\([^)]*\)\.as_str\(\) == "copied"/;
  const askers = files("crates").filter((file) => file.endsWith(".rs") && asks.test(read(file)));
  expect(askers).toEqual(["crates/rust-js-lower/src/recognition/inherent.rs"]);
  expect(read("crates/rust-js-lower/src/recognition/loop_ranges.rs")).toMatch(/fn option_cloned_kind\(/);
});
