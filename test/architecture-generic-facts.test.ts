// The generic facts' rules of the architecture, split from
// architecture.test.ts at its cap (a test file is at most 1500 lines): what
// generic code needs of its type parameters is one catalogue (ADR 1115).
import { expect, test } from "bun:test";
import { readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";
import { root } from "./support";

const read = (path: string) => readFileSync(join(root, path), "utf8");
// In name order, as macOS lists them and Linux, CI's, doesn't.
const files = (directory: string): string[] => readdirSync(join(root, directory), { withFileTypes: true })
  .sort((a, b) => (a.name < b.name ? -1 : a.name > b.name ? 1 : 0))
  .flatMap(entry => entry.isDirectory() ? files(`${directory}/${entry.name}`) : [`${directory}/${entry.name}`]);

// What generic code needs of its type parameters is one catalogue,
// `generic_facts.rs`' table, each fact's route and owner (ADR 1115). Each
// file's reads of a type parameter, `ty::Param(`, `has_param()` and
// `is_unknown(`, are the audit's of 2026-11-10 at most: a ratchet, which a
// migration onto the catalogue lowers. A new read asks the fact's owner the
// catalogue names, or adds its fact to the catalogue first.
const typeParameterReads: Record<string, number> = Object.fromEntries(
  read("test/type-parameter-reads.txt").split("\n").filter((line) => line && !line.startsWith("#"))
    .map((line) => line.split("\t")).map(([file, n]) => [file, Number(n)]),
);

test("a type parameter's facts are asked of their owner", () => {
  const counted = (text: string) =>
    text.split("\n").filter((line) => !line.trim().startsWith("//"))
      .reduce((n, line) => n + (line.match(/ty::Param\(|has_param\(\)|is_unknown\(/g) ?? []).length, 0);
  // The catalogue's own file asks them for its owners.
  const grown = files("src/lower").filter((file) => file.endsWith(".rs") && file !== "src/lower/generic_facts.rs")
    .map((file) => [file, counted(read(file)), typeParameterReads[file] ?? 0] as const)
    .filter(([, now, was]) => now > was)
    .map(([file, now, was]) => `${file}: ${now} reads of a type parameter, the audit's ${was}: ask the owner src/lower/generic_facts.rs names`);
  expect(grown).toEqual([]);
});

test("each fact in the generic facts' catalogue names an owner that's there", () => {
  const rows = read("src/lower/generic_facts.rs").split("\n")
    .map((line) => line.match(/^\/\/! \|.*\| (Given|Form|Instances) \| `(\S+) (\w+)` \|$/))
    .filter((match) => match !== null);
  expect(rows.length).toBeGreaterThan(10);
  const missing = rows.filter(([, , file, fn]) => !new RegExp(`fn ${fn}\\b`).test(read(file)))
    .map(([, , file, fn]) => `${file} ${fn}`);
  expect(missing).toEqual([]);
});
