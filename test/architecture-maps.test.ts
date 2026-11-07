// The architecture's rules of maps, split from architecture.test.ts at its
// cap (a test file is at most 1500 lines): the same input gives the same
// output (ADR 1003), so what the compiler decides is never in an order a
// process picks (ADR 1098); and a B-tree made of entries taken as sorted
// is a clone's alone (#924).
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

// The compiler's maps and sets are `collections`' (ADR 1098, #866): each
// walked in the order its entries were put in, never a hasher's. A std
// `HashMap` or `HashSet` is walked in an order its process's random keys
// pick, so what's decided walking one, ranges' fixpoint's callees, may
// differ from one compile of a program to the next.
test("the compiler's maps and sets are walked in an order the program decides", () => {
  // What the code names, not its comments or strings: std's, or a hasher
  // of a process's random keys.
  const code = (file: string) =>
    read(file)
      .split("\n")
      .map((line) => line.replace(/"(?:[^"\\]|\\.)*"/g, '""').replace(/\/\/.*$/, ""))
      .join("\n");
  const std = /\b(?:collections::(?:\{[^}]*\b)?Hash(?:Map|Set)\b|hash_(?:map|set)::|RandomState\b)|(?<!::)\bHash(?:Map|Set)\s*(?:<|::)/;
  expect(sources.filter((file) => std.test(code(file)))).toEqual([]);
  expect(read("src/collections.rs")).toContain("pub type IndexMap<K, V> = indexmap::IndexMap<K, V, rustc_hash::FxBuildHasher>;");
});

// A B-tree's `ofSorted(entries)` keeps its entries in the order given,
// asking its `cmp` nothing (#924), so it is right only of entries read
// from a B-tree of the same order: a clone's, `clone_parts`' map arm,
// its one maker. Any other caller could hand it entries out of order.
test("only a B-tree's clone makes one of sorted entries, `ofSorted`", () => {
  const makers = sources.filter((file) => /"ofSorted"/.test(read(file)));
  expect(makers).toEqual(["src/lower/std_impls.rs"]);
  const arm = read("src/lower/std_impls.rs").match(/\/\/\/ `clone_value` of a `ty` that needs a copy, part by part\.\n    fn clone_parts[\s\S]*?\n    }\n/)?.[0] ?? "";
  expect(arm.match(/"ofSorted"/g)?.length).toBe(1);
  expect(read("src/lower/std_impls.rs").match(/"ofSorted"/g)?.length).toBe(1);
  // The runtime defines it and never calls it.
  const runtime = files("src/runtime").filter((path) => path.endsWith(".js"));
  expect(runtime.filter((file) => /\.ofSorted\(/.test(read(file)))).toEqual([]);
});
