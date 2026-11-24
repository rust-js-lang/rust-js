// The push's gate (#2485, scripts/judged.ts): what a mutation run judged is
// recorded by the exact tree it judged, its untracked files' too, and a push
// judges, or refuses, each mutation CI's check judges of the change that
// isn't on record for the tree it pushes.
import { expect, test } from "bun:test";
import { mkdirSync, rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { fixture } from "./support";
import { judgedOn, judgedTrees, recordJudged, treeOf, unjudged } from "../scripts/judged";

// A tree is the working tree's files as a mutant's workspace is made of
// them: a tracked file as it is, a deleted one gone, an untracked one git
// doesn't ignore, a test the change hasn't committed, there; an ignored
// one, a build's, not. The same files committed are the same tree, and the
// checkout's own index is left as it was.
test("a working tree's tree is its files as they are, its untracked files too, not its ignored", () => {
  const dir = fixture("tree-of");
  const git = (...args: string[]) => Bun.spawnSync(["git", "-c", "user.name=t", "-c", "user.email=t@t", ...args], { cwd: dir, stdout: "pipe", stderr: "pipe" }).stdout.toString().trim();
  git("init", "-q");
  writeFileSync(join(dir, ".gitignore"), "built/\n");
  writeFileSync(join(dir, "kept.ts"), "kept");
  writeFileSync(join(dir, "gone.ts"), "gone");
  git("add", ".");
  git("commit", "-qm", "files");
  const committed = treeOf(dir);
  expect(committed).toBe(git("rev-parse", "HEAD^{tree}"));
  mkdirSync(join(dir, "built"));
  writeFileSync(join(dir, "built", "out.js"), "built");
  expect(treeOf(dir)).toBe(committed);
  writeFileSync(join(dir, "new.test.ts"), "new");
  const untracked = treeOf(dir);
  expect(untracked).not.toBe(committed);
  writeFileSync(join(dir, "kept.ts"), "changed");
  const changed = treeOf(dir);
  expect(changed).not.toBe(untracked);
  rmSync(join(dir, "gone.ts"));
  const deleted = treeOf(dir);
  expect(deleted).not.toBe(changed);
  // Nothing staged in the checkout's own index.
  expect(git("diff", "--cached", "--name-only")).toBe("");
  git("add", "--all");
  git("commit", "-qm", "the change");
  expect(git("rev-parse", "HEAD^{tree}")).toBe(deleted);
});

// What was judged is had of the tree it was judged on alone, each run's
// beside the others' of that tree, and only the newest trees are kept.
test("what a run judged is recorded by its tree, the newest trees kept", () => {
  const file = join(fixture("judged-record"), "judged.json");
  expect([...judgedOn("t1", file)]).toEqual([]);
  recordJudged("t1", ["b", "a"], file);
  recordJudged("t2", ["c"], file);
  recordJudged("t1", ["d"], file);
  expect([[...judgedOn("t1", file)], [...judgedOn("t2", file)], [...judgedOn("t3", file)]]).toEqual([["a", "b", "d"], ["c"], []]);
  for (let i = 0; i < judgedTrees - 1; i++) recordJudged(`n${i}`, ["x"], file);
  expect([[...judgedOn("t1", file)], [...judgedOn("t2", file)]]).toEqual([["a", "b", "d"], []]);
});

// Of the change's mutations, CI's set, those on record for the tree are
// judged; those that read rust-js.wasm are CI's wasm job's, said apart.
test("the change's mutations not judged on its tree are those it doesn't record", () => {
  const named = (...names: string[]) => names.map((name) => ({ name }));
  expect(unjudged({ run: named("a", "b", "c"), elsewhere: named("w") }, new Set(["b", "w"]))).toEqual({ names: ["a", "c"], wasm: ["w"] });
});
