// A mutation that survives on the commit a push is checked against, its base,
// isn't the push's: the push didn't make it survive, so it doesn't fail the
// push's check (ADR 0093, amended). `scripts/mutations.ts --base=<sha>` asks
// this of each survivor: each is run again at the base, in the run's own
// checkout, by that commit's own runner. One that survives there too is latent,
// listed in `target/mutants/latent.json` and filed in one tracking issue
// (`.github/workflows/rustc-tests.yml`'s `survivors`); one caught there, not
// there at all, or whose run there doesn't end in its time is the push's, and
// fails it, the safe way.
import { existsSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { run } from "../test/child";
import { namedMapEnv } from "./testMap";

/** What `results.json` holds of a run, of the commit it ran at. */
/** A run's verdicts, by mutation, and of each caught, the first test that
 * failed with it, `test/corpus.test.ts > alias_reads.rs` (`catchingTest`). */
export type Results = { commit: string; results: Record<string, string>; caughtBy?: Record<string, string> };

/** The survivors of `here`, a run's results, that `atBase`, the same
 * mutations' at the base, says survive there too: latent. Each other one,
 * caught there, inconclusive or not run, is the push's. */
export function latentOf(here: Record<string, string>, atBase: Record<string, string>): { latent: string[]; ours: string[] } {
  const survived = Object.keys(here).filter((name) => here[name].startsWith("SURVIVED"));
  const latent = survived.filter((name) => atBase[name]?.startsWith("SURVIVED"));
  return { latent, ours: survived.filter((name) => !latent.includes(name)) };
}

/** How long the run at the base may take: past it, each is the push's. */
export const baseLimit = 40 * 60_000;

/** What a run keeps of its own in `work` that its runner at the base, in
 * the same checkout, writes over: put back once that's run. */
const runsOwn = ["results.json", "timings.json"];

/** Each of `names`' results at `base`, by its runner there, run in `root`,
 * the run's own checkout, checked out at the base and back: its crate built
 * where the run's was, into the same target, a build of what changed since
 * the base, where a worktree of its own, another path, built the crate whole
 * again, 283 to 320 s of a CI shard (#2222). Its runner takes the checkout's
 * mutants' lock, so the run gives it up first. None where the checkout has
 * changes of its own, which checking out the base would lose, where it
 * can't be set up, or where its run doesn't end within `baseLimit`; and
 * then `why`, which a CI log shows: six of a check's eight shards found
 * none and said nothing of it, each survivor the push's (#2341). Judged
 * by `map`, the commit of the test map the run read, where it read one: no
 * ancestor of the base, a commit of the push's, kept no map of the base in
 * CI, and the snapshot judge's error there took each verdict with it. */
export async function resultsAt(root: string, work: string, base: string, names: string[], map?: string): Promise<{ results: Record<string, string>; why?: string }> {
  const git = (args: string[]) => run(["git", ...args], root, 120_000);
  const tail = (exit: { stdout: string; stderr: string }) => `${exit.stdout}\n${exit.stderr}`.trim().split("\n").slice(-30).join("\n");
  const head = (await git(["rev-parse", "HEAD"])).stdout.trim();
  const status = await git(["status", "--porcelain", "--untracked-files=no"]);
  if (head === "" || status.code !== 0) return { results: {}, why: `git can't say the checkout's state: ${tail(status)}` };
  if (status.stdout.trim() !== "") return { results: {}, why: `the checkout has changes of its own, which checking out the base would lose:\n${status.stdout.trim()}` };
  const kept = runsOwn.filter((name) => existsSync(join(work, name))).map((name) => [name, readFileSync(join(work, name), "utf8")] as const);
  for (const name of runsOwn) rmSync(join(work, name), { force: true });
  try {
    const checkout = await git(["checkout", "--quiet", "--detach", base]);
    if (checkout.code !== 0) return { results: {}, why: `git can't check out ${base}: ${tail(checkout)}` };
    const install = await run(["bun", "install", "--frozen-lockfile"], root, 600_000);
    if (install.code !== 0) return { results: {}, why: `its packages don't install at ${base}: ${tail(install)}` };
    // Of those the base has: one it doesn't is the push's, and its runner
    // refuses a name it doesn't know.
    const listed = await run(["bun", "scripts/mutations.ts", "--list"], root, 600_000);
    const there = new Set(listed.stdout.split("\n").map((line) => line.split("\t")[0]));
    const known = names.filter((name) => there.has(name));
    if (known.length === 0) return { results: {}, why: `its runner knows none of them (\`--list\` exited ${listed.code}): ${tail(listed)}` };
    // By the map the run read, where it read one (`namedMapEnv`).
    const ran = await run(["bun", "scripts/mutations.ts", ...known], root, baseLimit, map === undefined ? {} : { [namedMapEnv]: map });
    const at = join(work, "results.json");
    if (ran.timedOut || ran.signal) return { results: {}, why: `its run didn't end within ${baseLimit / 60_000} min: ${tail(ran)}` };
    if (!existsSync(at)) return { results: {}, why: `its run wrote no results.json (exited ${ran.code}): ${tail(ran)}` };
    const results = (JSON.parse(readFileSync(at, "utf8")) as Results).results;
    // Its tail too, where none survived there: what it judged and why.
    const survived = known.filter((name) => results[name]?.startsWith("SURVIVED"));
    return survived.length < known.length ? { results, why: `its run's end (exited ${ran.code}):\n${tail(ran)}` } : { results };
  } finally {
    // Back as the run found it: the checkout, which was clean, its
    // packages, and what the run wrote of its own.
    await git(["checkout", "--quiet", "--force", "--detach", head]);
    await run(["bun", "install", "--frozen-lockfile"], root, 600_000);
    for (const name of runsOwn) rmSync(join(work, name), { force: true });
    for (const [name, text] of kept) writeFileSync(join(work, name), text);
  }
}

/** The latent survivors of a run's `results` at `base`, written to
 * `work/latent.json` with what each breaks, for the tracking issue. */
export async function latentSurvivors(
  root: string,
  work: string,
  base: string,
  results: Record<string, string>,
  breaks: (name: string) => string,
  map?: string,
): Promise<string[]> {
  const survived = Object.keys(results).filter((name) => results[name].startsWith("SURVIVED"));
  if (survived.length === 0) return [];
  if (map !== undefined) console.log(`the survivors at ${base.slice(0, 9)}, judged by the test map of ${map.slice(0, 9)}, this run's`);
  const at = await resultsAt(root, work, base, survived, map);
  const { latent, ours } = latentOf(results, at.results);
  // Each of the push's, as the base judged it, and why the base's run said
  // what it did: a misfire's cause in the log, never silence (#2341).
  for (const name of ours) console.log(`at ${base.slice(0, 9)}, ${name}: ${at.results[name] ?? "not run there"}`);
  if (ours.length > 0 && at.why) console.log(`the run at ${base.slice(0, 9)}: ${at.why}`);
  writeFileSync(join(work, "latent.json"), JSON.stringify({ base, latent: latent.map((name) => ({ name, breaks: breaks(name) })) }, null, 2) + "\n");
  if (latent.length > 0) console.log(`latent, surviving at ${base} too, not this push's: ${latent.join(", ")}`);
  if (ours.length > 0) console.log(`this push's, caught at ${base.slice(0, 9)} or not run there: ${ours.join(", ")}`);
  return latent;
}
