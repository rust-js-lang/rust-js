// A decision's number (scripts/adr.ts): the next free one, and a branch's
// own moved, with only the lines that branch added that name it.

import { expect, test } from "bun:test";
import { addedLines, nextNumber, numbers, renumbered } from "../scripts/adr";

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
