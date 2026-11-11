// A state that changes over time is one enum with named transitions, never
// loose flags (#872, #873, #870's `$closing`, #871): two `bool`s that move
// together hold a combination no transition makes, and every reader repeats
// the rules of which follows which. This counts each file's loose flags, of
// the compiler and the runtime: a `Cell<bool>`, a struct of two `bool` fields
// or more, and a runtime file that sets two properties or more to `true` or
// `false`. The counts are the audit's of 2026-11-11, `test/state-flags.txt`:
// a ratchet, which only shrinks. A new flag is an enum's state instead, or
// says why it isn't one, on the line above it: `// Flags, not a state: ..`.
import { expect, test } from "bun:test";
import { readFileSync, readdirSync } from "node:fs";
import { join, relative } from "node:path";
import { root } from "./support";

const read = (path: string) => readFileSync(join(root, path), "utf8");
// In name order, as macOS lists them and Linux, CI's, doesn't; each by its
// path from the root.
const files = (directory: string): string[] => readdirSync(directory, { withFileTypes: true })
  .sort((a, b) => (a.name < b.name ? -1 : a.name > b.name ? 1 : 0))
  .flatMap(entry => entry.isDirectory() ? files(join(directory, entry.name)) : [relative(root, join(directory, entry.name))]);

const reason = /\/\/ Flags, not a state: \S/;
/** Whether the line `at`, or one of the `above` lines before it, says why it's flags. */
const said = (lines: string[], at: number, above = 1) =>
  lines.slice(Math.max(0, at - above), at + 1).some((line) => reason.test(line));

/** The loose flags of a compiler file: each `Cell<bool>` and each struct of
 * two `bool` fields or more, but those that say why they're flags. */
function rustFlags(text: string): number {
  const lines = text.split("\n");
  let count = 0;
  lines.forEach((line, i) => {
    if (!line.trim().startsWith("//") && line.includes("Cell<bool>") && !said(lines, i)) count++;
  });
  for (const m of text.matchAll(/^\s*(?:pub(?:\([^)]*\))? )?struct \w+[^{;(]*\{([^}]*)\}/gm)) {
    const at = text.slice(0, m.index).split("\n").length - 1;
    // A struct's comment is above its attributes, `#[derive(..)]`.
    if ((m[1].match(/:\s*bool,/g) ?? []).length >= 2 && !said(lines, at, 4)) count++;
  }
  return count;
}

/** The loose flags of a runtime file: the properties it sets to `true` or
 * `false`, where it sets two or more, but those that say why they're flags. */
function runtimeFlags(text: string): number {
  const lines = text.split("\n");
  const names = new Set<string>();
  lines.forEach((line, i) => {
    for (const m of line.matchAll(/\.(\w+) = (?:true|false)\b/g)) {
      if (!said(lines, i)) names.add(m[1]);
    }
  });
  return names.size >= 2 ? names.size : 0;
}

const audited: Record<string, number> = Object.fromEntries(
  read("test/state-flags.txt").split("\n").filter((line) => line && !line.startsWith("#"))
    .map((line) => line.split("\t")).map(([file, n]) => [file, Number(n)]),
);

test("a state is an enum, and loose flags only shrink", () => {
  const counts: [string, number][] = [
    ...files(join(root, "src")).filter((file) => file.endsWith(".rs")).map((file) => [file, rustFlags(read(file))] as [string, number]),
    ...files(join(root, "src", "runtime")).filter((file) => file.endsWith(".js")).map((file) => [file, runtimeFlags(read(file))] as [string, number]),
  ];
  const changed = counts
    .filter(([file, now]) => now !== (audited[file] ?? 0))
    .map(([file, now]) => now > (audited[file] ?? 0)
      ? `${file}: ${now} loose flags, the audit's ${audited[file] ?? 0}: make the state an enum with named transitions, or say why it isn't one on the line above it, \`// Flags, not a state: ..\``
      : `${file}: ${now} loose flags, fewer than the audit's ${audited[file]}: lower test/state-flags.txt to ${now}`);
  const gone = Object.keys(audited).filter((file) => !counts.some(([f]) => f === file))
    .map((file) => `${file}: no such file: take it off test/state-flags.txt`);
  expect([...changed, ...gone]).toEqual([]);
});

// What it counts, and what it lets be.
test("loose flags are counted, and flags that say why aren't", () => {
  expect(rustFlags("struct A {\n    must: Cell<bool>,\n    running: Cell<bool>,\n}\n")).toBe(2);
  expect(rustFlags("#[derive(Default)]\nstruct B {\n    a: bool,\n    b: bool,\n}\n")).toBe(1);
  expect(rustFlags("struct C {\n    a: bool,\n    n: u32,\n}\n")).toBe(0);
  expect(rustFlags("// Flags, not a state: two facts of one crate, each its own answer.\n#[derive(Default)]\nstruct D {\n    a: bool,\n    b: bool,\n}\n")).toBe(0);
  expect(rustFlags("struct E {\n    // Flags, not a state: whether a write may fail, decided once.\n    may: Cell<bool>,\n}\n")).toBe(0);
  expect(runtimeFlags("s.held = true;\ns.closesAtEnd = false;\n")).toBe(2);
  expect(runtimeFlags("s.first = false;\ns.first = true;\n")).toBe(0);
  expect(runtimeFlags("// Flags, not a state: held by its owner.\ns.held = true;\ns.done = false;\n")).toBe(0);
});
