// Which commit of a batched check broke each test it failed
// (scripts/attribute.ts): the commits run at, and the first a failing test
// fails at from there on.

import { expect, test } from "bun:test";
import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";

import { attribution, commitsToRun, mostCommits, mutationLines, runnerFiles, runnerOf, siteLines } from "../scripts/attribute";
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

// A failed check's mutations, by blame, no rerun: each it didn't catch, not
// latent, the newest commit of the batch that touched it; a job that failed
// with no verdict, the newest that touched the runner (main's checks
// 43871190255 and 43872319768, red with no commit named).
test("a failed mutations job names the batch commit that touched each mutation it missed, or the runner", () => {
  const touching = (name: string | undefined, job: string) => (name === "a-survivor" ? ["c3", "c1"] : name === undefined && runnerOf(job) === "shard" ? ["c2"] : []);
  const said = (commit: string) => `${commit} (its subject)`;
  const failed = [
    { job: "mutations / mutations (1/2)", verdicts: { "a-survivor": "SURVIVED: its tests passed", caught: "caught", "old-one": "SURVIVED: its tests passed", "a-latent": "SURVIVED: its tests passed" } },
    { job: "mutations / mutations (2/2)" },
  ];
  expect(mutationLines(failed, ["a-latent"], touching, said)).toEqual([
    "mutation a-survivor survived: its site, list or tests changed at c3 (its subject)",
    "mutation old-one survived: nothing of it changed in the batch, so it's latent, before it",
    "mutations / mutations (2/2) failed with no mutation's verdict: the runner changed at c2 (its subject)",
  ]);
  // The survivors' job by its own files: a shard's runner changed, not it.
  expect(mutationLines([{ job: "mutations / survivors" }], [], touching, said)).toEqual(["mutations / survivors failed with no mutation's verdict: its runner unchanged in the batch, so an earlier change's, its machine's or GitHub's"]);
  expect(siteLines("a\nb\nc\nd\n", "b\nc")).toEqual([2, 3]);
  expect(siteLines("a\n", "z")).toBeUndefined();
  // The runner's files are there, and the job reads the run's verdicts.
  for (const file of Object.values(runnerFiles).flat()) expect([file, existsSync(join(root, file))]).toEqual([file, true]);
  const check = readFileSync(join(root, ".github/workflows/check.yml"), "utf8");
  expect(check).toContain("bun scripts/attribute.ts mutations target/attribute/mutations ${{ needs.plan.outputs.batch_base || 'HEAD~1' }}");
  expect(check).toContain("needs.mutations.result == 'failure'");
});
