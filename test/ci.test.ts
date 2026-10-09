// What the CI workflow runs where (DEVELOPMENT.md): the suite split into
// shards, the mutations a change touches, and each branch's latest run.

import { expect, test } from "bun:test";
import { checkedSince, latestRuns, mutationShards } from "../scripts/ci";
import { changedLines, changedMutations, enclosingFunction, testBlocks } from "../scripts/mutations";
import { shard } from "../scripts/shard";

const files = ["a", "b", "c", "d", "e", "f", "g"].map((name) => `test/${name}.test.ts`);

test("shards split the files between them, each once, whatever the order given", () => {
  const shards = [1, 2, 3].map((i) => shard(files, i, 3));
  expect(shards.flat().sort()).toEqual(files);
  expect(shards.map((s) => s.length).sort()).toEqual([2, 2, 3]);
  expect(shard([...files].reverse(), 2, 3)).toEqual(shards[1]);
  expect(() => shard(files, 4, 3)).toThrow("a shard is 1 to 3");
});

const mutation = (name: string, file: string, find = "a", replace = "b", tests = ["test/y.test.ts"]) => ({ name, breaks: "", file, find, replace, tests });

const source = `use std::fmt;

fn first() -> u32 {
    let a = 1;
    a + 1
}

impl Thing {
    pub(super) fn second(&self) -> u32 {
        let b = 2;
        b * 2
    }

    fn short(&self) -> u32 { 3 }
}
`;

const tests = `import { test } from "bun:test";

const shared = 1;

// The first.
test("adds", () => {
  expect(1 + 1).toBe(2);
});

// The second,
// of two lines.
test.skipIf(false)("multiplies", () => {
  expect(2 * 2).toBe(4);
});
`;

// A change's mutations (ADR 0093): those it adds or edits, those in a
// function it changes, and those whose test, or corpus case, it changes.
test("the changed mutations are of the functions and tests a change touches", () => {
  const files: Record<string, string> = { "src/a.rs": source, "test/x.test.ts": tests };
  const read = (file: string) => files[file];
  const base = [
    mutation("in-first", "src/a.rs", "a + 1"),
    mutation("in-second", "src/a.rs", "b * 2"),
    mutation("edited", "src/a.rs", "let a = 1;", "let a = 2;"),
    mutation("adds-test", "src/b.rs", "x", "y", ["test/x.test.ts", "-t", "adds"]),
    mutation("multiplies-test", "src/b.rs", "x", "y", ["test/x.test.ts", "-t", "multiplies"]),
    mutation("case", "src/b.rs", "x", "y", ["test/corpus.test.ts", "-t", "array_map"]),
    mutation("gone", "src/a.rs", "no longer here"),
  ];
  const now = [...base.slice(0, 2), mutation("edited", "src/a.rs", "let a = 1;", "let a = 3;"), ...base.slice(3), mutation("added", "src/c.rs")];
  const names = (changes: { file: string; lines: [number, number][] }[]) => changedMutations(now, base, changes, read).map((m) => m.name);
  // A line of \`second\`'s: its mutation, the edited and added ones, and one
  // that no longer applies, to say so.
  expect(names([{ file: "src/a.rs", lines: [[10, 10]] }])).toEqual(["in-second", "edited", "gone", "added"]);
  // The second test's comment, then what both tests share.
  expect(names([{ file: "test/x.test.ts", lines: [[11, 11]] }])).toEqual(["edited", "multiplies-test", "added"]);
  expect(names([{ file: "test/x.test.ts", lines: [[3, 3]] }])).toEqual(["edited", "adds-test", "multiplies-test", "added"]);
  expect(names([{ file: "test/corpus/array_map.rs", lines: [[1, 1]] }, { file: "docs/x.md", lines: [[1, 1]] }])).toEqual(["edited", "case", "added"]);
  expect(changedMutations(now, now, [], read)).toEqual([]);
});

test("a function is where rustfmt and Prettier lay it out", () => {
  expect(enclosingFunction(source, 5, 5)).toEqual([3, 6]);
  expect(enclosingFunction(source, 11, 11)).toEqual([9, 12]);
  expect(enclosingFunction(source, 14, 14)).toEqual([14, 14]);
  // In none: its lines, and ten each side.
  expect(enclosingFunction(source, 1, 1)).toEqual([1, 11]);
});

test("a test file's tests are their comments and bodies", () => {
  expect(testBlocks(tests)).toEqual([
    { name: "adds", first: 5, last: 9 },
    { name: "multiplies", first: 10, last: 15 },
  ]);
});

test("a diff's changed lines are where the file is changed now", () => {
  const diff = [
    "diff --git a/src/a.rs b/src/a.rs",
    "--- a/src/a.rs",
    "+++ b/src/a.rs",
    "@@ -3,0 +4,2 @@ fn first() {",
    "+one",
    "+two",
    "@@ -9 +11 @@",
    "-old",
    "+new",
    "@@ -20,2 +21,0 @@",
    "diff --git a/gone.rs b/gone.rs",
    "--- a/gone.rs",
    "+++ /dev/null",
    "@@ -1 +0,0 @@",
  ].join("\n");
  expect(changedLines(diff)).toEqual([{ file: "src/a.rs", lines: [[4, 5], [11, 11], [21, 22]] }]);
});

// A check's mutations on as many machines as keeps each near forty.
test("a check's mutations are split to about forty a machine", () => {
  expect([0, 40, 41, 80, 81, 160, 161, 400].map(mutationShards)).toEqual(["1", "1", "2", "2", "4", "4", "8", "8"]);
});

const run = (headBranch: string, databaseId: number, status: string, conclusion: string, headSha = `sha${databaseId}`) =>
  ({ headBranch, databaseId, status, conclusion, createdAt: new Date(databaseId * 1000).toISOString(), displayTitle: headBranch, headSha });

test("each branch's latest run, newest first", () => {
  const runs = [run("a", 1, "completed", "failure"), run("b", 2, "completed", "success"), run("a", 3, "in_progress", "")];
  expect(latestRuns(runs).map((r) => [r.headBranch, r.databaseId])).toEqual([["a", 3], ["b", 2]]);
});

// A check of `main`, which changes are pushed to, runs the mutations of
// what changed since the commit its last finished check was of: one that
// was cancelled, or is running, checked nothing.
test("main's mutations are of what changed since its last finished check", () => {
  const runs = [run("main", 1, "completed", "success"), run("main", 2, "completed", "failure"), run("main", 3, "completed", "cancelled"), run("main", 4, "in_progress", ""), run("b", 5, "completed", "success")];
  expect(checkedSince(runs, "main")).toBe("sha2");
  expect(checkedSince(runs, "c")).toBeUndefined();
});
