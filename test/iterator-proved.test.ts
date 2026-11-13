// The iterator traits' methods whose hand-written form may stand in for
// std's body (`PROVED_LINKS`, ADR 1136): each a row of the iterator tables
// (ADRs 1090, 1092), which run it against native Rust in both builds. A
// method listed with no row fails here: the list mirrors the tables, held
// by running both.
import { expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { adapters, consumers, stepAdapters, stepConsumers } from "./iterator-matrix";
import { root } from "./support";

const proved = (): string[] => {
  const text = readFileSync(join(root, "src/lower/recognition/compiled_std.rs"), "utf8");
  const list = text.match(/const PROVED_LINKS: &\[\(&str, &str\)\] = &\[([\s\S]*?)\n\];/)?.[1] ?? "";
  return [...list.matchAll(/\("\w+", "(\w+)"\)/g)].map((m) => m[1]);
};

// A table's row by the method it steps: `collect` of each collect's row,
// `vec`'s; a row's name its method, a variant's after a space, `flat_map
// array`'s `flat_map`, `fold holding`'s `fold`.
const rowMethod = (name: string) => (name === "vec" ? "collect" : name.split(" ")[0]);
const tabled = new Set([...adapters, ...stepAdapters, ...consumers, ...stepConsumers].map((row) => rowMethod(row.name)));

test("each method a chain's hand-written form may use is a row of the iterator tables", () => {
  const listed = proved();
  expect(listed.length).toBeGreaterThan(30);
  expect(listed.filter((method) => !tabled.has(method))).toEqual([]);
});
