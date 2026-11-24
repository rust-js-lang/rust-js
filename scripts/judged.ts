// What the mutation runs judged, by the exact tree each judged (#2485, the
// lead's push gate): `bun run push` lands a change only where each mutation
// CI's check judges of it (`changedSince`, the one selection) was judged on
// the tree it pushes, caught, or a survivor at the change's base too,
// latent, as CI's `--base` rules. #2512 landed with its rule changed twice
// during its push, its 13 mutations never run again on the tree pushed, and
// #2408's survivors reached `main` so. check:fast's job judges them; the
// record lets a round whose tree was judged already, a push again after a
// refusal, or one whose author judged them, skip them.
import { spawnSync } from "node:child_process";
import { copyFileSync, mkdirSync, mkdtempSync, readFileSync, renameSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { sourceFilesOf } from "./source-mutations";

/** `git` in `root`, its output trimmed, or an error naming the command. */
function git(root: string, args: string[], env: Record<string, string> = {}, input?: string): string {
  const p = spawnSync("git", args, { cwd: root, encoding: "utf8", env: { ...process.env, ...env }, input, maxBuffer: 64 * 1024 * 1024 });
  if (p.status !== 0) throw new Error(`git ${args.join(" ")} failed in ${root}:\n${p.stderr}`);
  return p.stdout.trim();
}

/** The tree `root`'s working tree is, as git would write it: the files a
 * mutant's workspace is made of (`sourceFilesOf`), each tracked file as it
 * is, a deleted one gone, and each untracked one git doesn't ignore, a test
 * the change hasn't committed; `HEAD`'s tree where nothing differs. Written
 * by an index of its own, the checkout's left as it is. */
export function treeOf(root: string): string {
  const scratch = mkdtempSync(join(tmpdir(), "tree-of-"));
  try {
    const env = { GIT_INDEX_FILE: join(scratch, "index") };
    // The checkout's, so `add` hashes only what changed since it was written.
    copyFileSync(resolve(root, git(root, ["rev-parse", "--git-path", "index"])), env.GIT_INDEX_FILE);
    // What's tracked as it is, a deleted file gone; then what isn't.
    git(root, ["add", "--update"], env);
    git(root, ["add", "--pathspec-from-file=-", "--pathspec-file-nul"], env, sourceFilesOf(root).join("\0"));
    return git(root, ["write-tree"], env);
  } finally {
    rmSync(scratch, { recursive: true, force: true });
  }
}

/** How many trees the record keeps, the newest: a push's rounds, and the
 * runs of a change before it. */
export const judgedTrees = 16;

type Judged = { tree: string; judged: string[] }[];

function read(file: string): Judged {
  try {
    return JSON.parse(readFileSync(file, "utf8")) as Judged;
  } catch {
    return [];
  }
}

/** The mutations judged on `tree`, as `file` records them. */
export function judgedOn(tree: string, file: string): Set<string> {
  return new Set(read(file).find((entry) => entry.tree === tree)?.judged ?? []);
}

/** `names` recorded judged on `tree`, beside what was, in `file`: that
 * tree's entry first, the oldest dropped past `judgedTrees`. */
export function recordJudged(tree: string, names: string[], file: string) {
  const was = read(file);
  const own = new Set([...(was.find((entry) => entry.tree === tree)?.judged ?? []), ...names]);
  const now = [{ tree, judged: [...own].sort() }, ...was.filter((entry) => entry.tree !== tree)].slice(0, judgedTrees);
  mkdirSync(dirname(file), { recursive: true });
  const writing = `${file}.${process.pid}`;
  writeFileSync(writing, JSON.stringify(now) + "\n");
  renameSync(writing, file);
}

/** Of `chosen`, the change's mutations as `wasmSplit` parts them, those
 * not judged on its tree, `judged`; and those that read rust-js.wasm,
 * which CI's wasm job judges, apart. */
export function unjudged({ run, elsewhere }: { run: { name: string }[]; elsewhere: { name: string }[] }, judged: Set<string>): { names: string[]; wasm: string[] } {
  return { names: run.map((m) => m.name).filter((name) => !judged.has(name)), wasm: elsewhere.map((m) => m.name) };
}

/** Of the mutations CI's check judges of the change since `base` in
 * `root` (`changedSince`), those not judged on its tree as it is
 * (`unjudged`). */
export async function unjudgedSince(root: string, base: string): Promise<{ tree: string; names: string[]; wasm: string[] }> {
  const { changedSince, judgedFile, wasmSplit } = await import("./mutations");
  const tree = treeOf(root);
  return { tree, ...unjudged(wasmSplit(await changedSince(base), false), judgedOn(tree, judgedFile)) };
}
