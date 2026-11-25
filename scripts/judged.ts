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

/** What a judged mutation's verdict is of, besides the compiler: its own
 * file, the list it's in, and the test files it names, each corpus case
 * besides where it names the corpus's or is judged by its snapshots, and
 * each Rust source where it's held by the compiler's unit tests. */
export type Judgeable = { name: string; file: string; list: string; tests?: string[]; unitTests?: string; snapshots?: boolean };

/** Whether `incoming`, the files a rebase's commits changed, touch what
 * `m`'s verdict is of (`Judgeable`). */
export function touchedBy(m: Judgeable, incoming: ReadonlySet<string>): boolean {
  if (incoming.has(m.file) || incoming.has(m.list)) return true;
  const tests = (m.tests ?? []).filter((arg) => /^test\/.+\.tsx?$/.test(arg));
  if (tests.some((test) => incoming.has(test))) return true;
  const corpus = m.snapshots === true || tests.includes("test/corpus.test.ts");
  if (corpus && [...incoming].some((file) => file.startsWith("test/corpus/"))) return true;
  return m.unitTests !== undefined && [...incoming].some((file) => file.endsWith(".rs"));
}

/** Of `from`, a round's tree, the mutations `file` records judged there
 * that `incoming`, the files the commits a rebase brought changed, leave
 * alone (`touchedBy`), recorded judged on `to`, the rebased tree, as the
 * round's check is carried on past a rebase (#2484): a rebase then makes a
 * push judge again only what the commits it brought could change, not its
 * whole set. CI's check judges the pushed commit's whole set again, so a
 * carry it disagrees with turns CI red; it never lands a change unjudged.
 * What it carried, of how many judged there. */
export function carryJudged(from: string, to: string, incoming: string[], among: Judgeable[], file: string): { carried: string[]; of: number } {
  const judged = judgedOn(from, file);
  const brought = new Set(incoming);
  const carried = among.filter((m) => judged.has(m.name) && !touchedBy(m, brought)).map((m) => m.name);
  if (carried.length > 0) recordJudged(to, carried, file);
  return { carried, of: judged.size };
}

/** What `bun run push` asks of the mutations, in a process of its own, so
 * push loads no mutation list: a rebase that edits one, as other agents'
 * commits do, made push run again from the rebased tree (`staleModules`),
 * its lists as they were before it (#2578's follow-up, the lead's ask).
 * Each answer is a line of JSON.
 *
 *   bun scripts/judged.ts files                    each mutation's file
 *   bun scripts/judged.ts unjudged <base>          `unjudgedSince`
 *   bun scripts/judged.ts carry <checked> <from> <to>
 *                                                  `carryJudged` of the tree
 *                                                  of the commit a round
 *                                                  checked onto this one's,
 *                                                  past what `from..to`
 *                                                  brought */
if (import.meta.main) {
  const [command, ...args] = process.argv.slice(2);
  const root = process.cwd();
  const answer = async (): Promise<unknown> => {
    const { lists, judgedFile } = await import("./mutations");
    if (command === "files" && args.length === 0) return [...new Set(lists.flatMap((list) => list.mutations.map((m) => m.file)))];
    if (command === "unjudged" && args.length === 1) return await unjudgedSince(root, args[0]!);
    if (command === "carry" && args.length === 3) {
      const [checked, from, to] = args as [string, string, string];
      const among = lists.flatMap(({ path, mutations }) => mutations.map((m) => ({ ...m, list: `scripts/mutations/${path}` })));
      const incoming = git(root, ["diff", "--name-only", from, to]).split("\n").filter(Boolean);
      return carryJudged(git(root, ["rev-parse", `${checked}^{tree}`]), treeOf(root), incoming, among, judgedFile);
    }
    throw new Error(`bun scripts/judged.ts files | unjudged <base> | carry <checked> <from> <to>, not ${[command, ...args].join(" ")}`);
  };
  console.log(JSON.stringify(await answer()));
}
