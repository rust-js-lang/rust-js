// A decision's number (scripts/adr.ts): the next free one, and a branch's
// own moved, with only the lines that branch added that name it.

import { expect, test } from "bun:test";
import { readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";

import { addedLines, AMENDED_MOST, amendments, mostAmended, nextNumber, numbers, ownerRows, renumbered } from "../scripts/adr";

test("the next number is one past the highest either side takes", () => {
  const main = numbers(["0001-a.md", "0790-b.md", "README.md", "draft"]);
  const branch = numbers(["0001-a.md", "0791-c.md"]);
  expect(main).toEqual([1, 790]);
  expect(nextNumber(main, branch)).toBe("0792");
  expect(nextNumber([], [])).toBe("0001");
});

test("a number is renumbered where it's a number of its own", () => {
  expect(renumbered("(ADR 0774), [0774](0774-room.md), ADRs 0315, 0774", "0774", "0812")).toBe(
    "(ADR 0812), [0812](0812-room.md), ADRs 0315, 0812",
  );
  expect(renumbered("10774 07740 0774", "0774", "0812")).toBe("10774 07740 0812");
});

test("a diff's added lines are each file's, in the file as it is now", () => {
  const diff = [
    "diff --git a/x.md b/x.md",
    "--- a/x.md",
    "+++ b/x.md",
    "@@ -3,0 +4,2 @@",
    "+one",
    "+two",
    "@@ -10 +12 @@",
    "-old",
    "+new",
    "diff --git a/y.rs b/y.rs",
    "--- a/y.rs",
    "+++ b/y.rs",
    "@@ -5,2 +5,0 @@",
    "-gone",
    "-gone",
  ].join("\n");
  const added = addedLines(diff);
  expect([...(added.get("x.md") ?? [])]).toEqual([4, 5, 12]);
  expect([...(added.get("y.rs") ?? [])]).toEqual([]);
});

// A decision amended again and again is a list of cases (#1465): the
// audit counts each one's amendments, by its headings.
test("a decision's amendments are its headings that say so", () => {
  const text = "# 0001. A rule\n\n## Decision\n\nAmended below.\n\n## Amendment: a case (#2)\n\n## Amended (2026-11-06, #3): another\n\n### Amendment of a part\n\n## Amends\n\nADR 0002.\n";
  expect(amendments(text)).toBe(2);
  const texts = new Map([
    ["0001-a.md", text],
    ["0002-b.md", "# 0002. Another\n"],
    ["0003-c.md", `${text}\n## Amendment: a third\n`],
  ]);
  expect(mostAmended(texts, 2)).toEqual([
    ["0003-c.md", 3],
    ["0001-a.md", 2],
  ]);
  expect(mostAmended(texts, 4)).toEqual([]);
});

test("the semantic owners' rows are read with their lengths, the longest first", () => {
  const short = "| A short concept | `a.rs` | `ask` |";
  const long = `| A long concept, ${"its cases, ".repeat(8)}| \`b.rs\` | \`ask\` |`;
  const page = ["# Architecture", "### Semantic owners", "", "| Concept | Owner | What the others ask |", "|---|---|---|", short, long, "", "**Calls.** The rest.", "| not | a | row |"].join("\n");
  expect(ownerRows(page)).toEqual([
    [`A long concept, ${"its cases, ".repeat(8)}`.trimEnd(), long.length],
    ["A short concept", short.length],
  ]);
  expect(() => ownerRows("# Architecture\n")).toThrow("no table of semantic owners");
});

// Each decision amended `AMENDED_MOST` times or more has its row in the
// audit's table, docs/bug-clusters.md's "The decisions amended most": what
// its amendments have in common, and the refactor that would end them or
// why none is wanted. Whoever amends one past that adds its row, so a rule
// that keeps growing by cases is named where the next refactor is chosen.
test("a decision amended most has its row in the audit's table", () => {
  const root = join(import.meta.dir, "..");
  const dir = join(root, "docs", "decisions");
  const texts = new Map(
    readdirSync(dir)
      .filter((name) => /^\d{4}-.*\.md$/.test(name))
      .map((name) => [name, readFileSync(join(dir, name), "utf8")]),
  );
  const page = readFileSync(join(root, "docs", "bug-clusters.md"), "utf8");
  const from = page.indexOf("## The decisions amended most");
  expect(from).toBeGreaterThan(0);
  const rows = page.slice(from, page.indexOf("\n## ", from + 1)).split("\n").filter((line) => /^\| \d{4}/.test(line));
  const named = new Set(rows.flatMap((row) => row.split(" | ")[0].match(/\d{4}/g) ?? []));
  const most = mostAmended(texts, AMENDED_MOST).map(([name]) => name.slice(0, 4));
  expect(most.length).toBeGreaterThan(5);
  expect(most.filter((number) => !named.has(number))).toEqual([]);
  // And each row says what would end them: none is left to fill in.
  expect(rows.filter((row) => row.split(" | ").length < 4 || row.split(" | ").some((cell) => cell.trim() === ""))).toEqual([]);
});
