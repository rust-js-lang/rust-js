// What the CI workflow runs where (DEVELOPMENT.md): the suite split into
// shards, the mutations a change touches, and each branch's latest run.

import { expect, test } from "bun:test";
import { latestRuns } from "../scripts/ci";
import { changedMutations } from "../scripts/mutations";
import { shard } from "../scripts/shard";

const files = ["a", "b", "c", "d", "e", "f", "g"].map((name) => `test/${name}.test.ts`);

test("shards split the files between them, each once, whatever the order given", () => {
  const shards = [1, 2, 3].map((i) => shard(files, i, 3));
  expect(shards.flat().sort()).toEqual(files);
  expect(shards.map((s) => s.length).sort()).toEqual([2, 2, 3]);
  expect(shard([...files].reverse(), 2, 3)).toEqual(shards[1]);
  expect(() => shard(files, 4, 3)).toThrow("a shard is 1 to 3");
});

const mutation = (name: string, file: string, find = "a", replace = "b") => ({ name, breaks: "", file, find, replace, tests: ["test/x.test.ts"] });

// A change's mutations: those of a file it changes, and those it adds or
// edits, which a change to their tests' files alone wouldn't name.
test("the changed mutations are a changed file's, and the new or edited", () => {
  const base = [mutation("kept", "src/a.rs"), mutation("edited", "src/b.rs"), mutation("touched", "src/c.rs")];
  const now = [mutation("kept", "src/a.rs"), mutation("edited", "src/b.rs", "a", "c"), mutation("touched", "src/c.rs"), mutation("added", "src/a.rs")];
  expect(changedMutations(now, base, ["src/c.rs", "docs/x.md"]).map((m) => m.name)).toEqual(["edited", "touched", "added"]);
  expect(changedMutations(now, now, [])).toEqual([]);
});

const run = (headBranch: string, databaseId: number, status: string, conclusion: string) =>
  ({ headBranch, databaseId, status, conclusion, createdAt: new Date(databaseId * 1000).toISOString(), displayTitle: headBranch });

test("each branch's latest run, newest first", () => {
  const runs = [run("a", 1, "completed", "failure"), run("b", 2, "completed", "success"), run("a", 3, "in_progress", "")];
  expect(latestRuns(runs).map((r) => [r.headBranch, r.databaseId])).toEqual([["a", 3], ["b", 2]]);
});
