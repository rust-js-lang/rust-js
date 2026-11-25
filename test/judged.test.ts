// The push's gate (#2485, scripts/judged.ts): what a mutation run judged is
// recorded by the exact tree it judged, its untracked files' too, and a push
// judges, or refuses, each mutation CI's check judges of the change that
// isn't on record for the tree it pushes.
import { expect, test } from "bun:test";
import { mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { fixture, root } from "./support";
import { carryJudged, judgedOn, judgedTrees, recordJudged, touchedBy, treeOf, unjudged } from "../scripts/judged";
import { askJudged } from "../scripts/push";

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

// A rebase past a round carries the round's judging onto the rebased tree
// of each mutation whose own file, list and tests the commits it brought
// leave alone: lily agent's 2c-1 push judged its same 351 mutations again
// in each of three rounds past a moved `main`, 442–445 s each, none of the
// incoming commits touching its files (2ab5bb3f9). One whose file, list,
// named test, corpus case or Rust unit tests they touch is judged again.
test("a rebase over commits that leave a judged mutation's file, list and tests alone carries it onto the rebased tree", () => {
  const file = join(fixture("judged-carry"), "judged.json");
  rmSync(file, { force: true });
  const among = [
    { name: "own", file: "crates/a.rs", list: "scripts/mutations/a.ts", tests: ["test/a.test.ts", "-t", "a"] },
    { name: "of-a-test", file: "crates/b.rs", list: "scripts/mutations/b.ts", tests: ["test/b.test.ts"] },
    { name: "of-a-list", file: "crates/c.rs", list: "scripts/mutations/c.ts", tests: ["test/c.test.ts"] },
    { name: "of-its-file", file: "crates/d.rs", list: "scripts/mutations/d.ts", tests: ["test/d.test.ts"] },
    { name: "of-a-case", file: "crates/e.rs", list: "scripts/mutations/e.ts", tests: ["test/corpus.test.ts", "-t", "x.rs"] },
    { name: "of-units", file: "crates/f.rs", list: "scripts/mutations/f.ts", unitTests: "f::tests" },
    { name: "unjudged", file: "crates/g.rs", list: "scripts/mutations/g.ts", tests: ["test/g.test.ts"] },
  ];
  recordJudged("checked", ["own", "of-a-test", "of-a-list", "of-its-file", "of-a-case", "of-units"], file);
  // Over a commit of another file alone: all it judged, carried.
  expect(carryJudged("checked", "past-docs", ["docs/x.md"], among, file)).toEqual({ carried: ["own", "of-a-test", "of-a-list", "of-its-file", "of-a-case", "of-units"], of: 6 });
  // Over one of each's test, list, file, corpus case and Rust: none of them.
  const brought = ["test/b.test.ts", "scripts/mutations/c.ts", "crates/d.rs", "test/corpus/y.rs", "src/z.rs"];
  expect(carryJudged("checked", "past-theirs", brought, among, file)).toEqual({ carried: ["own"], of: 6 });
  expect([...judgedOn("past-theirs", file)]).toEqual(["own"]);
  expect(touchedBy(among[0]!, new Set(["test/a.test.ts"]))).toBe(true);
});

// push asks scripts/judged.ts of the mutations in a process of its own,
// loading no mutation list itself: a rebase's edit of one made it run again
// from the rebased tree, each of lily agent's 2c-1 push's three (2ab5bb3f9).
test("push loads no mutation list itself", () => {
  const push = readFileSync(join(root, "scripts", "push.ts"), "utf8");
  expect(push.match(/import\(["']\.\/(?:mutations|judged)["']\)|from ["']\.\/(?:mutations|judged)["']/g)).toBeNull();
  expect(push).toContain('askJudged(["unjudged", base])');
  // As push asks it: the mutations' files, of the lists as they are.
  expect((askJudged(["files"], root) as string[]).includes("scripts/push.ts")).toBe(true);
});
