// A push that undoes another's commit is refused (scripts/push-guard.ts,
// `.githooks/pre-push`): a commit made from a tree older than the main it's
// pushed onto, as `git reset --soft origin/main` after a fetch makes one,
// as 4163f3865 undid 12e759c57.
import { expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

import { refusal, undone } from "../scripts/push-guard";
import { fixture, root, run } from "./support";

test("a push that undoes a commit already on the remote is refused, but one that names it", () => {
  const dir = fixture("push-guard");
  const git = (...args: string[]) => run(["git", "-c", "user.name=t", "-c", "user.email=t@t", ...args], undefined, { GIT_DIR: join(dir, ".git"), GIT_WORK_TREE: dir }).trim();
  git("init", "-q", "-b", "main");
  const commit = (message: string, files: Record<string, string>) => {
    for (const [name, text] of Object.entries(files)) writeFileSync(join(dir, name), text);
    git("add", "-A");
    git("commit", "-q", "-m", message);
    return git("rev-parse", "HEAD");
  };
  commit("base", { "fix.rs": "broken\n", "mine.rs": "old\n" });
  // Another's fix, on the remote.
  const fixed = commit("the other's fix", { "fix.rs": "fixed\n" });
  // A commit of a tree from before it, the fix's file as it was, and one's own change.
  const stale = commit("one's own change", { "fix.rs": "broken\n", "mine.rs": "new\n" });
  expect(undone(dir, fixed, stale).map((u) => [u.file, u.commit])).toEqual([["fix.rs", fixed]]);
  expect(refusal(undone(dir, fixed, stale))).toContain(`fix.rs: back to before ${fixed.slice(0, 9)}`);
  // Its own change alone, and a revert that names the commit, go.
  git("checkout", "-q", fixed);
  const own = commit("one's own change", { "mine.rs": "new\n" });
  expect(undone(dir, fixed, own)).toEqual([]);
  const reverted = commit(`This reverts commit ${fixed}.`, { "fix.rs": "broken\n" });
  expect(undone(dir, own, reverted)).toEqual([]);
  // `bun install` sets git to run the guard before a push.
  expect(JSON.parse(readFileSync(join(root, "package.json"), "utf8")).scripts.prepare).toBe("git config core.hooksPath .githooks");
  expect(readFileSync(join(root, ".githooks", "pre-push"), "utf8")).toContain("scripts/push-guard.ts");
});

// A tree older than the remote that edits each file the other commit did,
// so no file is as it was before it, still undoes its lines: f7d5e10ec took
// 02df83a40's memory cap out of test/child.ts and three more files so
// (#1097). Not a commit that rewrites some of the other's lines, whose
// other lines stand.
test("a push that undoes another's lines in files it edits too is refused, but a rewrite of some of them goes", () => {
  const dir = fixture("push-guard-lines");
  const git = (...args: string[]) => run(["git", "-c", "user.name=t", "-c", "user.email=t@t", ...args], undefined, { GIT_DIR: join(dir, ".git"), GIT_WORK_TREE: dir }).trim();
  git("init", "-q", "-b", "main");
  const commit = (message: string, files: Record<string, string>) => {
    for (const [name, text] of Object.entries(files)) writeFileSync(join(dir, name), text);
    git("add", "-A");
    git("commit", "-q", "-m", message);
    return git("rev-parse", "HEAD");
  };
  const lines = (...ls: string[]) => `${ls.join("\n")}\n`;
  const ten = (first: string, ...rest: string[]) => [first, "2", "3", "4", "5", "6", "7", "8", "9", "10", ...rest];
  commit("base", { "child.ts": lines(...ten("a"), "z"), "mutations.ts": lines(...ten("1"), "z") });
  // Another's cap, a line at the end of each file.
  const cap = commit("the cap", { "child.ts": lines(...ten("a"), "cap", "z"), "mutations.ts": lines(...ten("1"), "capped", "z") });
  // A tree from before it, with one's own edits to each file, past what's
  // around the cap.
  const stale = commit("one's own change", { "child.ts": lines(...ten("A"), "z"), "mutations.ts": lines(...ten("one"), "z") });
  expect(undone(dir, cap, stale).map((u) => [u.file, u.commit])).toEqual([
    ["child.ts", cap],
    ["mutations.ts", cap],
  ]);
  // Its own edits made on the cap, and a rewrite of the cap's line in one
  // file, the other's standing, go.
  git("checkout", "-q", cap);
  const own = commit("one's own change", { "child.ts": lines(...ten("A"), "cap", "z"), "mutations.ts": lines(...ten("one"), "capped", "z") });
  expect(undone(dir, cap, own)).toEqual([]);
  const rewrite = commit("the cap said once", { "child.ts": lines(...ten("A"), "cap(1)", "z") });
  expect(undone(dir, own, rewrite)).toEqual([]);
});

test("the guard runs before a push", () => {
  expect(JSON.parse(readFileSync(join(root, "package.json"), "utf8")).scripts.prepare).toBe("git config core.hooksPath .githooks");
  expect(readFileSync(join(root, ".githooks", "pre-push"), "utf8")).toContain("scripts/push-guard.ts");
});
