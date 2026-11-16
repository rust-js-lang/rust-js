// Which commit of a batched check broke each test it failed
// (scripts/attribute.ts): the commits run at, and the first a failing test
// fails at from there on.

import { expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";

import { attribution, commitsToRun, mostCommits } from "../scripts/attribute";
import { completedBase, type Run } from "../scripts/ci";
import { root } from "./support";

test("a batch's failing tests run at its commits, at most six, the oldest and the newest among them", () => {
  const commits = Array.from({ length: 10 }, (_, i) => `c${i}`);
  expect(commitsToRun(commits.slice(0, 4))).toEqual(["c0", "c1", "c2", "c3"]);
  const picked = commitsToRun(commits);
  expect([picked.length, picked[0], picked.at(-1)]).toEqual([mostCommits, "c0", "c9"]);
});

test("each failing test broke at the first commit it fails at from there on", () => {
  const runs = [
    { commit: "c1", failing: [] },
    { commit: "c2", failing: ["test/a.test.ts", "test/b.test.ts"] },
    { commit: "c3", failing: ["test/a.test.ts", "test/b.test.ts", "test/c.test.ts"] },
    { commit: "c4", failing: ["test/a.test.ts", "test/c.test.ts", "test/d.test.ts"] },
  ];
  expect(attribution(runs, ["test/a.test.ts", "test/c.test.ts", "test/b.test.ts"])).toEqual([
    { unit: "test/a.test.ts", commit: "c2", since: "batch" },
    { unit: "test/c.test.ts", commit: "c3", since: "batch" },
    // Fails no more at the newest: the check's own failure alone.
    { unit: "test/b.test.ts", commit: "c4", since: "passes" },
  ]);
  expect(attribution([{ commit: "c1", failing: ["test/a.test.ts"] }, { commit: "c2", failing: ["test/a.test.ts"] }], ["test/a.test.ts"])).toEqual([{ unit: "test/a.test.ts", commit: "c1", since: "before" }]);
});

test("a batch's base is the newest check of main that completed, red or green, not HEAD's own, nor a cancelled one", () => {
  const run = (sha: string, at: string, conclusion: string, status = "completed", headBranch = "main"): Run => ({ headBranch, databaseId: 1, status, conclusion, createdAt: at, displayTitle: "", headSha: sha });
  const all = [run("head", "5", ""), run("s4", "4", "cancelled"), run("s3", "3", "failure"), run("s2", "2", "success"), run("o9", "9", "success", "completed", "other")];
  expect(completedBase(all, "head")).toBe("s3");
  const check = readFileSync(join(root, ".github/workflows/check.yml"), "utf8");
  expect(check).toContain("bun scripts/attribute.ts plan junit ${{ needs.plan.outputs.batch_base || 'HEAD~1' }}");
  expect(check).toContain("bun scripts/attribute.ts report target/attribute/results");
});
