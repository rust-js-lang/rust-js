// The checks CI runs of each push to `main` (DEVELOPMENT.md), read from
// here, and of another branch started from here, by GitHub's CLI:
//
//   bun run ci:check [branch] [--shards=4] [--since=<rev>]  # check a branch as it's pushed; main's push checks it already
//   bun run ci:status                       # each branch's latest run, and what failed, and the nightly's
//   bun run ci:bless [branch]               # apply its latest run's bless patches
//   bun scripts/ci.ts test-timings <times>  # in CI: the test files' times `plan` gave every shard, for them to balance by
//   bun scripts/ci.ts mutation-timings <times>   # in CI: the mutations' times `plan` gave every shard, for them to balance by
//   TIMES=<times> bun scripts/ci.ts merge-mutation-timings <ran>  # in CI: those and this run's shards' merged, for the next plan
//   bun scripts/ci.ts nets                  # in CI: which of a job of nets' nets ran, and which failed, from its `STEPS`
//   bun scripts/ci.ts prune-cache [--dry-run]    # in CI, after each check, nightly and test map: each cache family's newest 8 kept
//
// A run whose snapshots or rustc's lists differ uploads what blessing them
// writes, as `bless-*` artifacts: applied here, to be read as a local bless
// is, before they're committed.

import { appendFileSync, existsSync, mkdirSync, readFileSync, readdirSync, rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { gunzipSync, gzipSync } from "node:zlib";

import { changedCases, changePicks, ROWS_PARTS, rowsShards, touchesAsync, touchesConsumers, touchesCrates, touchesDeclarations, touchesFormatting, touchesIntervals, touchesIterators, touchesMemory, touchesNumbers, touchesPort, touchesRustc, touchesStdNet, touchesTwins, touchesWasm } from "./iteratorSlice";
import { changedSince, controlRuns, corpusCaseNames, type Mutation, mutationFile, mutations, TESTS_AS_IT_IS, testSet, wasmSplit } from "./mutations";
import { outcome, type Selection, selectChange, suiteFiles } from "./select";
import { downloaded } from "./artifacts";
import { browsers, caseFiles, dealtByGroup, drivesBrowser, fewestShards, type ShardFacts, shardFacts, shardTimings, testFileSeconds, timingsByShard, timingsUnder } from "./shard";
import { timed, timings } from "./timing";

const root = join(import.meta.dir, "..");
const workflow = "check.yml";

export type Run = {
  headBranch: string;
  databaseId: number;
  status: string;
  conclusion: string;
  createdAt: string;
  displayTitle: string;
  headSha?: string;
  updatedAt?: string;
};

/** Each branch's latest run, the newest first. */
export function latestRuns(runs: Run[]): Run[] {
  const newest = [...runs].sort((a, b) => b.createdAt.localeCompare(a.createdAt));
  return newest.filter((run, i) => newest.findIndex((other) => other.headBranch === run.headBranch) === i);
}

/** The commit of `branch`'s last check whose mutations passed, which a
 * check of it runs the mutations of what changed since: a green run, or one
 * that failed elsewhere and whose every mutation job passed,
 * `mutationsPassed`. A cancelled run, a running one, or one whose mutations
 * failed or never ran checked none of them, so its changes stay in the
 * next run's. */
export function checkedSince(runs: Run[], branch: string, mutationsPassed: (run: Run) => boolean = () => false): string | undefined {
  return [...runs]
    .sort((a, b) => b.createdAt.localeCompare(a.createdAt))
    .find(
      (run) =>
        run.headBranch === branch &&
        run.status === "completed" &&
        (run.conclusion === "success" || (run.conclusion === "failure" && mutationsPassed(run))),
    )?.headSha;
}

/** The artifact the wasm job keeps its mutations' results in, beside the
 * shards' `mutation-results-<k>` (check.yml's `wasm` job, #2346). */
export const wasmResults = "mutation-results-wasm";

/** What a run's mutation shards found, each `mutation-results-<k>`'s, of
 * the commit it ran at (`mutations.ts`'s `Results`), and how many shards
 * ran, each completed, passed or not. */
export type RunResults = {
  run: Run;
  shards: number;
  completed: number;
  results: { commit: string; results: Record<string, string> }[];
  latent?: string[];
  /** The wasm job's, `mutation-results-wasm`'s: the mutations the shards
   * left to it, judged where rust-js.wasm is (#2346). */
  wasm?: { commit: string; results: Record<string, string> };
};

/** The last check of `main` this one's mutations follow: the newest whose
 * every mutation shard completed and kept what it found, each of that run's
 * own commit; and of it, what it ran and didn't catch, which runs again
 * beside what changed since. A run of a shard cancelled, or missing, or of
 * another commit's results, is passed over, so a mutation counts as caught
 * only where it was, at the commit its run checked; what changed since is
 * the change's, as ever (ADR 0093). `undefined` where none is, and the
 * last check whose mutations passed is the base, as before. The nightly
 * runs every mutation, whatever this says. */
export function resultsBase(newestFirst: RunResults[]): { since: string; retry: string[] } | undefined {
  for (const { run, shards, completed, results, latent = [], wasm } of newestFirst) {
    if (run.status !== "completed" || !run.headSha || shards === 0 || completed !== shards || results.length !== shards) continue;
    if (!results.every((r) => r.commit === run.headSha)) continue;
    // What the shards left to the wasm job is caught where its run, of the
    // same commit, caught it; else, its job not run or not catching it, it
    // runs again (#2346).
    const caughtThere = wasm?.commit === run.headSha ? Object.entries(wasm.results).filter(([, result]) => result === "caught").map(([name]) => name) : [];
    const passed = [...latent, ...caughtThere];
    // Not one it judged latent, surviving at its base too: no push's, filed
    // in the survivors' issue, and the nightly runs it. Retried, the 6–7 of
    // the crate tests held a shard up in every check, 40% of a check's
    // runner-minutes the mutations'. A push that changes its site or its
    // tests still runs it, `--changed`'s, whatever the retry list says.
    const retry = results.flatMap((r) => Object.entries(r.results).filter(([name, result]) => result !== "caught" && !passed.includes(name)).map(([name]) => name));
    return { since: run.headSha, retry: [...new Set(retry)].sort() };
  }
  return undefined;
}

/** `main`'s last checks, the newest first, each with its mutation shards'
 * results (`resultsBase`): of at most `limit` completed runs that ran
 * mutations, of at most `looked` in all. One whose change touched none ran
 * no shard (`plan`'s `mutations`), and isn't counted. */
function runResults(limit = 6, looked = 30): RunResults[] {
  const out: RunResults[] = [];
  let ran = 0;
  for (const run of runs().filter((r) => r.headBranch === "main" && r.status === "completed").slice(0, looked)) {
    if (ran === limit) break;
    const jobs: { name: string; conclusion: string }[] = JSON.parse(gh(["run", "view", String(run.databaseId), "--json=jobs"])).jobs;
    const shardJobs = jobs.filter((job) => /^mutations \/ mutations \(\d+\/\d+\)$/.test(job.name));
    if (shardJobs.length === 0) continue;
    ran++;
    const completed = shardJobs.filter((job) => job.conclusion === "success" || job.conclusion === "failure").length;
    const dir = join(root, "target", "mutation-results", String(run.databaseId));
    const results: RunResults["results"] = [];
    const latent: string[] = [];
    let wasm: RunResults["wasm"];
    // Its artifacts first, as a download of none asks again, three times.
    const kept = () =>
      (JSON.parse(gh(["api", `repos/{owner}/{repo}/actions/runs/${run.databaseId}/artifacts?per_page=100`, "--jq", "[.artifacts[].name]"])) as string[])
        .some((name) => name.startsWith("mutation-results-"));
    if (shardJobs.length > 0 && completed === shardJobs.length && kept()) {
      try {
        if (!downloaded(run.databaseId, ["--pattern=mutation-results-*", "--pattern=mutation-latent-*"], dir)) throw new Error(`run ${run.databaseId}'s results`);
        // Each shard's, `mutation-results-<k>`; and the wasm job's, apart (#2346).
        for (const shard of readdirSync(dir).filter((name) => /^mutation-results-\d+$/.test(name))) results.push(JSON.parse(readFileSync(join(dir, shard, "results.json"), "utf8")));
        if (existsSync(join(dir, wasmResults, "results.json"))) wasm = JSON.parse(readFileSync(join(dir, wasmResults, "results.json"), "utf8"));
        for (const shard of readdirSync(dir).filter((name) => name.startsWith("mutation-latent-"))) {
          const kept: { latent: { name: string }[] } = JSON.parse(readFileSync(join(dir, shard, "latent.json"), "utf8"));
          latent.push(...kept.latent.map(({ name }) => name));
        }
      } catch {}
    }
    out.push({ run, shards: shardJobs.length, completed, results, latent, wasm });
    if (resultsBase(out)) break;
  }
  return out;
}

/** Whether each of `run`'s mutation jobs ran and passed, as GitHub lists
 * them: `mutations / mutations (k/n)`. A run that ran none passed none. */
export function mutationJobsPassed(jobs: { name: string; conclusion: string }[]): boolean {
  const mutations = jobs.filter((job) => job.name.startsWith("mutations /"));
  return mutations.length > 0 && mutations.every((job) => job.conclusion === "success");
}

/** `mutationJobsPassed` of a run, asked of GitHub. */
function runMutationsPassed(run: Run): boolean {
  const jobs: { name: string; conclusion: string }[] = JSON.parse(gh(["run", "view", String(run.databaseId), "--json=jobs"])).jobs;
  return mutationJobsPassed(jobs);
}

/** What `gh` prints. A read is asked again, up to three times, as GitHub's
 * API at times answers a 500 with no body, `unexpected end of JSON input`,
 * which failed a nightly's test shard before its suite ran (#606); a write,
 * a workflow started, is asked once. Each retry is said on stderr: `plan`'s
 * stdout is GitHub's step outputs, where a line not `name=value` fails the
 * step (#606). */
function gh(args: string[], { write = false } = {}): string {
  for (let attempt = 1; ; attempt++) {
    const p = Bun.spawnSync(["gh", ...args], { cwd: root, stdout: "pipe", stderr: "pipe" });
    if (p.exitCode === 0) return p.stdout.toString();
    const failed = `gh ${args.join(" ")} failed:\n${p.stderr.toString()}`;
    if (write || attempt === 3) throw new Error(failed);
    console.error(`${failed.trimEnd()}\nasking again in ${5 * attempt} s`);
    Bun.sleepSync(5000 * attempt);
  }
}

function runs(of = workflow, limit = 100): Run[] {
  return JSON.parse(gh(["run", "list", `--workflow=${of}`, `--limit=${limit}`, "--json=headBranch,databaseId,status,conclusion,createdAt,displayTitle,headSha,updatedAt"]));
}

/** How long each finished green check of `main` took, kept with the
 * loop's own steps, once each (scripts/timing.ts): a check that grows
 * slower is seen as the loop's are. */
function timeChecks(all: Run[]) {
  const kept = new Set(timings().flatMap((t) => (t.step === "CI check of main" && t.detail ? [t.detail] : [])));
  const green = all
    .filter((run) => run.headBranch === "main" && run.conclusion === "success" && run.updatedAt)
    .sort((a, b) => a.createdAt.localeCompare(b.createdAt));
  for (const run of green) {
    const id = `run ${run.databaseId}`;
    if (kept.has(id)) continue;
    const seconds = (Date.parse(run.updatedAt!) - Date.parse(run.createdAt)) / 1000;
    console.log(timed("CI check of main", seconds, { detail: id }));
  }
}

function status() {
  // The latest nightly.yml run of `main`, every test there is, first.
  // None until it's on `main`, which GitHub reads a schedule from.
  let nightly: Run[] = [];
  try {
    nightly = runs("nightly.yml", 1).map((run) => ({ ...run, headBranch: "nightly" }));
  } catch {}
  const all = runs();
  timeChecks(all);
  for (const run of [...nightly, ...latestRuns(all)]) {
    const result = run.status === "completed" ? run.conclusion : run.status;
    let failed = "";
    if (run.conclusion === "failure") {
      const jobs: { name: string; conclusion: string }[] = JSON.parse(gh(["run", "view", String(run.databaseId), "--json=jobs"])).jobs;
      failed = jobs.filter((job) => job.conclusion === "failure").map((job) => job.name).join(", ");
    }
    console.log([run.headBranch, result, failed, `gh run view ${run.databaseId}`].filter(Boolean).join("\t"));
    // What broke what, where a failed check attributed it (scripts/attribute.ts).
    if (run.conclusion === "failure") for (const name of ["attribution", "attribution-mutations"]) for (const line of attributionOf(run.databaseId, name)) console.log(`  ${line}`);
  }
}

/** The lines of a run's attribution artifact `name`, the suite's or the
 * mutations', or none. */
function attributionOf(run: number, name: string): string[] {
  const dir = join(root, "target", "ci-attribution", String(run), name);
  if (!downloaded(run, [`--name=${name}`], dir)) return [];
  try {
    return (JSON.parse(readFileSync(join(dir, `${name}.json`), "utf8")) as { lines: string[] }).lines;
  } catch {
    return [];
  }
}

/** The machines mutations may be split across: the workflows' choices
 * and matrix are these, as `test/ci.test.ts` holds them. The nightly takes
 * the most, its every mutation on each of the Free plan's 20 at once
 * but the few its other jobs hold for their first minutes. */
export const MUTATION_SHARDS = [1, 2, 4, 8, 16];

/** What a mutation shard takes beside its mutations: its checkout, its
 * packages, the toolchain and the caches restored (m1-lisa's measure of
 * main's checks, 2026-11-19). */
export const SHARD_SETUP_SECONDS = 60;
/** And the compiler as it is, built before any mutant: 87 to 130 s of
 * each shard's `phases.json` (main's check 44441824204). */
export const SHARD_COMPILER_SECONDS = 120;
/** And where one of its mutations survived at its last check's base too,
 * latent (`latent-survivors.ts`), surviving again: the survivors run again
 * at the base, the compiler built whole there, 283 to 320 s of main's
 * checks 44830680353 and 44774641229, after its mutants (#2220). */
export const SHARD_BASE_SECONDS = 300;
/** And beside them, where a test one of its mutations runs drives a
 * browser (`drivesBrowser`): Chromium installed. */
export const SHARD_BROWSER_SECONDS = 420;
/** The longest a check's mutation shard is meant to take, about the suite's
 * shards' time, 11 and 15 min of main's check 44441824204: its machines
 * are the fewest that keep each under it. Twelve minutes, so a shard whose
 * test drives a browser, 10 min before its first mutant, still has room. */
export const SHARD_BUDGET_SECONDS = 720;

/** Machines for a check's mutations, `chosen`: the fewest of
 * `MUTATION_SHARDS` whose every shard, as the shards deal them by each's
 * share of its shard, `seconds` (`dealtByGroup`, each's untimed by its
 * file's, `groupOf`, the catalog's `mutationFile` as the shards' own), is
 * expected to take `SHARD_BUDGET_SECONDS` or less: its mutations' shares,
 * its setup, the compiler as it is, its mutations' tests as it is, of the
 * corpus's `cases` (`testsAsItIsRuns`), the survivors' run at the base where it's dealt one the last checks found
 * latent, `latent`, and a browser's where one of its tests drives one,
 * `browser`. Sized by count, twenty a machine, one shard of a
 * change's slow mutations took 21 min while others took 4 (main's checks
 * of 2026-11-19); else the most. */
export function mutationShards(
  chosen: { name: string; tests?: string[]; snapshots?: boolean; env?: Record<string, string> }[],
  seconds: ReadonlyMap<string, number>,
  browser: (file: string) => boolean,
  groupOf: (name: string) => string | undefined,
  latent: ReadonlySet<string> = new Set(),
  cases: readonly string[] = [],
): string {
  return String(mutationShardPlan(chosen, seconds, browser, groupOf, latent, cases).count);
}

/** The jobs the account runs at once, the Free plan's: a check's jobs past
 * them wait for others' to end (#2257). */
export const CONCURRENT_JOBS = 20;

/** A check's jobs that run beside its mutation shards besides its test
 * shards and its `iterator-rows` machines: `build`, `lint`,
 * `iterator-forms`, the nets, `wasm` and the rustc tests' map, about as
 * many as a check of a change to the compiler runs (check.yml). */
export const OWN_JOBS_BESIDE_SHARDS = 6;

/** Of in-flight runs, each with its jobs, the jobs of runs other than
 * `ownRun` that hold one of the account's slots or wait for one: queued or
 * in progress (#2257). */
export function heldJobs(runs: readonly { id: number; jobs: readonly { status: string }[] }[], ownRun: number): number {
  return runs
    .filter((run) => run.id !== ownRun)
    .reduce((held, run) => held + run.jobs.filter((job) => job.status === "queued" || job.status === "in_progress").length, 0);
}

/** How many of the account's jobs a check's mutation shards may take, of
 * `held` by other checks and `own`, its other jobs beside them: at least
 * one (#2257). */
export function freeJobs(held: number, own: number): number {
  return Math.max(1, CONCURRENT_JOBS - held - own);
}

/** `mutationShards`' machines, at most `cap`, the jobs free (`freeJobs`):
 * the fewest of `MUTATION_SHARDS` at or under it within the budget, else
 * the fewest of them that end soonest (`fewestShards`); with how many its
 * time alone would take, `wanted`, and when each's slowest shard is
 * expected to end, `at` and `wantedAt`, so a check says what the cap cost
 * it (#2257). Fewer, longer shards where others hold jobs also pay each
 * shard's setup fewer times. */
export function mutationShardPlan(
  chosen: { name: string; tests?: string[]; snapshots?: boolean; env?: Record<string, string> }[],
  seconds: ReadonlyMap<string, number>,
  browser: (file: string) => boolean,
  groupOf: (name: string) => string | undefined,
  latent: ReadonlySet<string> = new Set(),
  cases: readonly string[] = [],
  cap = Number.POSITIVE_INFINITY,
): { count: number; wanted: number; at: number; wantedAt: number } {
  const drives = new Map(chosen.map((m) => [m.name, m.tests !== undefined && browser(m.tests[0]!)]));
  const byName = new Map(chosen.map((m) => [m.name, m]));
  const expected = (count: number) => {
    // Its load holds each mutation's share and its tests as the compiler is,
    // dealt with them (`controlRuns`, #2304).
    const { shards, load } = dealtByGroup(chosen.map((m) => m.name), count, seconds, groupOf, (name) => controlRuns(byName.get(name)!, cases));
    return shards.map(
      (names, i) =>
        SHARD_SETUP_SECONDS +
        SHARD_COMPILER_SECONDS +
        TESTS_AS_IT_IS.shard +
        load[i]! +
        (names.some((name) => latent.has(name)) ? SHARD_BASE_SECONDS : 0) +
        (names.some((name) => drives.get(name)) ? SHARD_BROWSER_SECONDS : 0),
    );
  };
  const slowest = (count: number) => Math.max(0, ...expected(count));
  const wanted = fewestShards(MUTATION_SHARDS, expected, SHARD_BUDGET_SECONDS);
  const within = MUTATION_SHARDS.filter((count) => count <= cap);
  const count = fewestShards(within.length > 0 ? within : [1], expected, SHARD_BUDGET_SECONDS);
  return { count, wanted, at: slowest(count), wantedAt: slowest(wanted) };
}

/** A check's summary of its mutation shards' count: of the jobs free, and
 * where the cap took fewer than its time wanted, what that cost, its
 * slowest shard's expected end against the wanted count's (#2257). */
export function shardSummary(plan: { count: number; wanted: number; at: number; wantedAt: number }, held: number | undefined, own: number): string {
  if (held === undefined) return `Mutation shards: ${plan.count}, as their time asks (the jobs other checks hold unread).\n`;
  const free = freeJobs(held, own);
  const of = `${free} of the account's ${CONCURRENT_JOBS} jobs free (${held} held by other checks, ${own} its own beside them)`;
  if (plan.count >= plan.wanted) return `Mutation shards: ${plan.count}, as their time asks; ${of}.\n`;
  return `Mutation shards: ${plan.count} where their time asks ${plan.wanted}; ${of}: expected to end in ${Math.round(plan.at)} s where ${plan.wanted} would in ${Math.round(plan.wantedAt)} s.\n`;
}

/** The jobs other in-flight runs hold or wait for, `heldJobs`' of what
 * GitHub lists now; `undefined` where it can't say, and no cap is made. */
function heldByOthers(): number | undefined {
  try {
    const ids = new Set<number>();
    for (const status of ["in_progress", "queued"]) {
      const listed = JSON.parse(gh(["api", `repos/{owner}/{repo}/actions/runs?status=${status}&per_page=50`, "--jq", "[.workflow_runs[].id]"])) as number[];
      listed.forEach((id) => ids.add(id));
    }
    const runs = [...ids].map((id) => ({
      id,
      jobs: JSON.parse(gh(["api", `repos/{owner}/{repo}/actions/runs/${id}/jobs?per_page=100`, "--jq", "[.jobs[] | {status}]"])) as { status: string }[],
    }));
    return heldJobs(runs, Number(process.env.GITHUB_RUN_ID ?? 0));
  } catch {
    return undefined;
  }
}

/** Start the check of `branch`, as it's pushed: the local one, if it's
 * ahead, isn't what would be checked. Its mutations are of what changed
 * since `since`: of `main`, by default, since its last finished check, or
 * the commit before; of another branch, since it left `main`. A check of
 * `main` is its push's already: one started here waits behind the one
 * running, and cancels none (check.yml). */
async function check(branch: string, shards: string | undefined, since?: string) {
  const git = (args: string[]) => Bun.spawnSync(["git", ...args], { cwd: root }).stdout.toString().trim();
  git(["fetch", "--quiet", "origin", branch]);
  const local = git(["rev-parse", branch]);
  const pushed = git(["rev-parse", `origin/${branch}`]);
  if (!pushed || pushed !== local) throw new Error(`${branch} isn't as it's pushed: push it first, git push -u origin ${branch}`);
  const base = since ?? (branch === "main" ? (checkedSince(runs(), branch, runMutationsPassed) ?? `${local}~1`) : "origin/main");
  // As many machines as the mutations it runs need, which are this
  // checkout's, as it's pushed.
  const chosen = await changedSince(base);
  const count = chosen.length;
  const timings = plannedMutationTimings();
  const machines = mutationShards(
    chosen,
    timings === "" ? new Map() : decodedTimings(timings, mutations.map((m) => m.name)),
    (file) => drivesBrowser(root, file),
    mutationFile,
    new Set(),
    corpusCaseNames(),
  );
  // The suite's machines are the workflow's own, unless `--shards=` says.
  const suite = shards === undefined ? [] : ["-f", `test_shards=${shards}`];
  gh(["workflow", "run", workflow, `--ref=${branch}`, ...suite, "-f", `since=${base}`, "-f", `mutation_shards=${machines}`], { write: true });
  const on = shards === undefined ? "the workflow's machines" : `${shards} machine${shards === "1" ? "" : "s"}`;
  console.log(`started ${workflow} of ${branch} at ${local.slice(0, 10)}, the suite on ${on}, the ${count} mutations of what changed since ${base.slice(0, 10)} on ${machines}: bun run ci:status`);
}

/** Of a check, in CI: what its mutations are of, what changed since `main`'s
 * last check whose mutations passed, or the commit before, or since `given`,
 * a check started by hand's; how many machines they take; and whether what
 * changed is the iterators' code, which runs the iterator matrix's slice,
 * or the formatting code or the numbers', which run the formatting grid's
 * and the numbers grid's
 * (scripts/iteratorSlice.ts): as GitHub's step outputs. */
async function plan(given?: string, testShardsGiven?: string) {
  let since = given ?? "HEAD~1";
  let retry: string[] = [];
  // What the last checks found latent, whose shard runs them again at its
  // base where they survive again (`SHARD_BASE_SECONDS`).
  let latent = new Set<string>();
  if (given === undefined) {
    try {
      // What the last check's mutations left, else the last that passed.
      const ran = runResults();
      latent = new Set(ran.flatMap((r) => r.latent ?? []));
      const base = resultsBase(ran);
      since = base?.since ?? checkedSince(runs(), "main", runMutationsPassed) ?? since;
      retry = base?.retry ?? [];
    } catch {}
  }
  let touched: Mutation[] = [];
  try {
    touched = await changedSince(since);
  } catch {
    since = "HEAD~1";
    touched = await changedSince(since);
  }
  // The mutations run only where the change touches one, its site, its
  // list or its tests: what the last check didn't catch, of code as it
  // was, isn't caught now either. A run of none is passed over by the
  // next one's `resultsBase`, which keeps the base and what to retry.
  const mutating = touched.length > 0;
  const chosen = [...touched, ...mutations.filter((m) => retry.includes(m.name) && !touched.includes(m))];
  const timings = plannedMutationTimings();
  const seconds = timings === "" ? new Map<string, number>() : decodedTimings(timings, mutations.map((m) => m.name));
  const changed = Bun.spawnSync(["git", "diff", "--name-only", since, "HEAD"], { cwd: root }).stdout.toString().split("\n").filter(Boolean);
  console.log(`since=${since}`);
  console.log(`retry=${retry.join(",")}`);
  console.log(`iterators=${touchesIterators(changed)}`);
  console.log(`formatting=${touchesFormatting(changed)}`);
  console.log(`numbers=${touchesNumbers(changed)}`);
  console.log(`declarations=${touchesDeclarations(changed)}`);
  console.log(`async=${touchesAsync(changed)}`);
  console.log(`consumers=${touchesConsumers(changed)}`);
  console.log(`std=${touchesStdNet(changed)}`);
  console.log(`twins=${touchesTwins(changed)}`);
  console.log(`cases=${changedCases(changed).length > 0}`);
  console.log(`crates=${touchesCrates(changed)}`);
  console.log(`port=${touchesPort(changed)}`);
  console.log(`memory=${touchesMemory(changed)}`);
  console.log(`intervals=${touchesIntervals(changed)}`);
  console.log(`mutations=${mutating}`);
  console.log(`mutation_timings=${timings}`);
  const testTimes = plannedTestTimings();
  console.log(`test_timings=${testTimes ? encodedTestTimings(testTimes.seconds) : ""}`);
  console.log(`wasm=${touchesWasm(changed)}`);
  // Whether any chosen mutation needs rust-js.wasm, which the wasm job
  // judges after it builds it (`wasmSplit`, #2346).
  console.log(`wasm_mutations=${mutating && wasmSplit(chosen, true).run.length > 0}`);
  console.log(`rustc=${touchesRustc(changed)}`);
  // The suite's selection (scripts/select.ts): by the newest test map of an
  // ancestor the cache kept, the diff from its commit, `since`'s change the
  // run's own; written for the test shards, `selection.json`, and said.
  let selection: Selection & { map?: string };
  try {
    selection = selectChange(since);
  } catch (error) {
    selection = { all: true, files: [], cases: [], forms: [], why: `the selector failed: ${error instanceof Error ? error.message.split("\n")[0] : error}` };
  }
  mkdirSync(join(root, "target"), { recursive: true });
  writeFileSync(join(root, "target", "selection.json"), JSON.stringify(selection));
  console.log(`tests=${testsOutput(selection)}`);
  // As many as a check started by hand or the nightly says, else as many
  // as its selection's expected time needs; check.yml's matrix is the list.
  const testShardCount = testShards(selection, testTimes, testShardsGiven);
  console.log(`test_shards=${testShardCount}`);
  // The iterator rows each push names since the commit it moved `main`
  // from (`RUST_JS_ROWS_BASE`, #2340), on machines of their own,
  // `iterator-rows`, as many as the pick needs: none, and no job, of a
  // push that names none (#2374).
  const rowsMachines = rowsShards(changePicks(ROWS_PARTS));
  console.log(`rows=${rowsMachines > 0}`);
  console.log(`rows_shards=${rowsMachines}`);
  console.log(`rows_shard_list=${shardList(String(Math.max(1, rowsMachines)))}`);
  // The mutations' machines, at most the jobs free beside other checks' and
  // this one's own, which wait past the account's 20 (#2257).
  const held = heldByOthers();
  const own = Number(testShardCount) + rowsMachines + OWN_JOBS_BESIDE_SHARDS;
  const shardPlan = mutationShardPlan(
    chosen,
    seconds,
    (file) => drivesBrowser(root, file),
    mutationFile,
    latent,
    corpusCaseNames(),
    held === undefined ? Number.POSITIVE_INFINITY : freeJobs(held, own),
  );
  console.log(`shards=${shardPlan.count}`);
  if (process.env.GITHUB_STEP_SUMMARY) appendFileSync(process.env.GITHUB_STEP_SUMMARY, shardSummary(shardPlan, held, own));
  console.log(`test_shard_list=${shardList(testShardCount)}`);
  // The batch a failure is attributed in: since the newest check of
  // `main` that completed, red or green (scripts/attribute.ts).
  let batchBase = "";
  try {
    batchBase = completedBase(runs(), Bun.spawnSync(["git", "rev-parse", "HEAD"], { cwd: root }).stdout.toString().trim()) ?? "";
  } catch {}
  console.log(`batch_base=${batchBase}`);
  // The commits it checks, a push's whose check a newer one replaced
  // waiting among them (GitHub keeps a group's newest waiting run alone,
  // check.yml): named in the run's summary, as its failures are by commit.
  if (batchBase && process.env.GITHUB_STEP_SUMMARY) {
    appendFileSync(process.env.GITHUB_STEP_SUMMARY, batchSummary(batchCommits(batchBase)));
  }
  const said = `The suite's selection: ${outcome(selection)}${selection.map ? `, by the test map of ${selection.map.slice(0, 9)}` : ""}`;
  console.error(said);
  if (process.env.GITHUB_STEP_SUMMARY) appendFileSync(process.env.GITHUB_STEP_SUMMARY, `${said}\n`);
}

/** The commits after `base` to HEAD, the newest first, each its short sha
 * and subject. */
function batchCommits(base: string): { sha: string; subject: string }[] {
  const out = Bun.spawnSync(["git", "log", "--format=%h%x09%s", `${base}..HEAD`], { cwd: root }).stdout.toString().trim();
  return out === "" ? [] : out.split("\n").map((line) => ({ sha: line.split("\t")[0]!, subject: line.split("\t").slice(1).join("\t") }));
}

/** A check's summary of the commits it checks, its batch: more than one
 * where a push's own check was replaced while it waited, each still
 * checked here, its failures attributed by commit (scripts/attribute.ts). */
export function batchSummary(commits: { sha: string; subject: string }[]): string {
  if (commits.length <= 1) return "";
  const listed = commits.map(({ sha, subject }) => `- ${sha} ${subject.slice(0, 100)}`).join("\n");
  return `### This check covers ${commits.length} commits\n\nEach a push's; a check of an older one replaced while it waited is this one, and a failure here is attributed to the commit that broke it.\n\n${listed}\n`;
}

/** The jobs of a push's check that run whatever it changes, each with its
 * reason; every other is gated, its `if` reading `plan`'s outputs or
 * another job's result (docs/dev-loop.md's "CI spend", test/ci.test.ts). */
export const alwaysRun: Record<string, string> = {
  plan: "it decides every other job's gate, and the suite's selection",
  build: "beside plan, the release build nets and iterator-forms restore: gated, it would wait for plan, a minute onto nets' leg, the check's longest; restored from its cache it's half a minute",
  lint: "typecheck, rustfmt, clippy, cargo test and the Vite example's build: a change to any file can break one, and it starts beside plan",
};

/** The commit of the newest check of `main` that completed, red or green,
 * but HEAD's own: a batch's base, the commits after it its check runs over. */
export function completedBase(all: Run[], head: string): string | undefined {
  return [...all]
    .sort((a, b) => b.createdAt.localeCompare(a.createdAt))
    .find((run) => run.headBranch === "main" && run.status === "completed" && (run.conclusion === "success" || run.conclusion === "failure") && run.headSha && run.headSha !== head)?.headSha;
}

/** The machines the suite may be split across: check.yml's test matrix
 * is `plan`'s list of them (`shardList`). */
export const TEST_SHARDS = [1, 2, 3, 4, 5, 6];

/** What a test shard takes beside its run of the suite: its checkout,
 * the toolchain, packages and the browser, the compiler from its cache,
 * and what it uploads after: 98 and 37 s before its run of check
 * 44480685461's first and third shards (896bcafd0), each some 10 s after;
 * its second's install of the browser hung 7.7 min. */
export const TEST_SHARD_SETUP_SECONDS = 100;

/** How long a test shard of a selection is expected to take, of `count`,
 * its rest's files' shares of their wall, `load` (`testFileSeconds`): its
 * setup; then the longer of its rest and its pool of the files of cases,
 * which run beside each other, the pool's parts' seconds at what its wall
 * took a second of them (`ShardFacts`), the corpus's cases selected and
 * the metamorphic sample's a `1/count` of each; then its browsers' files,
 * after both; all of it at the slowest pace the kept runs measured
 * (`ShardFacts`' `pace`), as a check waits on its slowest shard. Of
 * check 44480685461's three shards the pool was each one's longest leg,
 * 468 to 667 s of their 516 to 745 s runs, the rest 301 to 396, the
 * browsers 10 to 79 after. */
export function testShardSeconds(load: number, count: number, facts: ShardFacts, { cases, metamorphic, pages }: { cases: number; metamorphic: boolean; pages: number }): number {
  const pooled = (cases * facts.secondsPerCase + (metamorphic ? facts.metamorphic : 0)) / count;
  return facts.pace * (TEST_SHARD_SETUP_SECONDS + Math.max(load, pooled * facts.poolWallPerSecond) + Math.ceil(pages / count) * facts.browserSeconds);
}

/** How many machines the suite's shards of a selection take, by the
 * kept runs' times of its files and what their shards measured, `times`
 * (`plannedTestTimings`): the fewest of `TEST_SHARDS` each of whose
 * shards, as `scripts/test.ts` deals them (`testShardFiles`, of
 * `dealtByGroup`), is expected to take `SHARD_BUDGET_SECONDS` or less
 * (`testShardSeconds`, `fewestShards`, the mutations' rule), a file no run
 * timed by the timed ones' median. Where no run kept them, by count: one of a small
 * selection, at most 60 files and 400 corpus cases, two of more. As many
 * as `given`, a check started by hand's or the nightly's, where it's
 * given. */
export function testShards(
  selection: Selection,
  times?: { seconds: ReadonlyMap<string, number>; facts: ShardFacts },
  given?: string,
  suite: () => string[] = suiteFiles,
  corpusCases: () => number = () => corpusCaseNames().length,
): string {
  if (given !== undefined) return given;
  if (times === undefined || times.seconds.size === 0) return !selection.all && selection.files.length <= 60 && selection.cases.length <= 400 ? "1" : "2";
  const cases: readonly string[] = Object.values(caseFiles);
  const chosen = selection.all ? suite() : selection.files;
  const files = chosen.filter((file) => !cases.includes(file) && !browsers.includes(file));
  const pool = {
    cases: selection.all ? corpusCases() : selection.cases.length,
    metamorphic: selection.all || selection.files.includes(caseFiles.metamorphic),
    pages: chosen.filter((file) => browsers.includes(file)).length,
  };
  // The deal `testShardFiles` gives each shard, and its load.
  const expected = (count: number) =>
    dealtByGroup(files, count, times.seconds, () => "suite").load.map((load) => testShardSeconds(load, count, times.facts, pool));
  return String(fewestShards(TEST_SHARDS, expected, SHARD_BUDGET_SECONDS));
}

/** check.yml's matrix of `count` shards, `[1, .., count]`: the test
 * shards', and `iterator-rows`'. */
export function shardList(count: string): string {
  return JSON.stringify(Array.from({ length: Number(count) }, (_, i) => i + 1));
}

/** `plan`'s `tests` output: what the suite's shards run of a selection,
 * `all`, `some`, or `none`, where the shards are skipped. */
export function testsOutput(selection: Selection): "all" | "some" | "none" {
  if (selection.all) return "all";
  return selection.files.length === 0 && selection.cases.length === 0 ? "none" : "some";
}

/** check.yml's jobs of nets, each a machine whose nets run at once:
 * the iterator nets, the program nets, and the rest. */
export const NET_JOBS = ["nets-iterators", "nets-programs", "nets"] as const;

/** Each net of check.yml's jobs of nets (`NET_JOBS`), by its step's id,
 * and its name: the steps that run in the background, as the workflow
 * lists them. */
export function nets(workflow = readFileSync(join(root, ".github/workflows/check.yml"), "utf8")): { id: string; name: string }[] {
  type Step = { id?: string; name?: string; background?: boolean };
  const jobs = (Bun.YAML.parse(workflow) as { jobs: Record<string, { steps?: Step[] } | undefined> }).jobs;
  const steps: Step[] = NET_JOBS.flatMap((job) => jobs[job]?.steps ?? []);
  return steps.filter((step) => step.background).map((step) => ({ id: step.id!, name: step.name! }));
}

/** A job of nets' summary, of its steps' outcomes (GitHub's `steps`
 * context): each net that failed by name first, then each that passed;
 * one skipped, as its change didn't touch it, isn't said. */
export function netsSummary(outcomes: Record<string, { outcome?: string }>, all = nets()): string {
  const of = (outcome: string) => all.filter((net) => outcomes[net.id]?.outcome === outcome).map((net) => net.name);
  const failed = of("failure"), passed = of("success");
  return [
    failed.length > 0 ? `### ${failed.length} net${failed.length === 1 ? "" : "s"} failed: ${failed.join(", ")}` : "### Every net that ran passed",
    "",
    ...failed.map((name) => `- **${name}**: failed; its step's log says why`),
    ...passed.map((name) => `- ${name}: passed`),
    "",
  ].join("\n");
}

/** An artifact a run uploaded, as GitHub lists them, the newest first. */
export type Artifact = { name: string; run: number; createdAt: string; expired: boolean };

/** The run whose shards' timings this run's are balanced by: the newest
 * that uploaded any, `timings-<shard>`, still kept; or none, when no run
 * has, and the shards split by file count, as a first run's do. */
export function timingsRuns(artifacts: Artifact[], count = TIMINGS_RUNS): number[] {
  const newest = [...artifacts].filter((a) => /^timings-\d+$/.test(a.name) && !a.expired).sort((a, b) => b.createdAt.localeCompare(a.createdAt));
  return [...new Set(newest.map((a) => a.run))].slice(0, count);
}

/** How many runs' test times `plan` reads, each file's the newest run's
 * that timed it: one run's alone, a selection's of a few files, left the
 * rest untimed, and d1a21768d's check of everything, its first sized by
 * time, planned 2 shards of the last run's 144. */
export const TIMINGS_RUNS = 6;

/** In CI, `plan`'s: the test files' times of the last run's `timings-*`
 * artifacts, read once and handed to every test shard alike as its output
 * (`encodedTestTimings`), as the mutations' times are: each shard
 * downloaded them itself, and one GitHub's storage turned away three times
 * failed (check of 7f324c2ec, `503 Egress is over the account limit`).
 * Where none can be read it says so, and every shard deals by file count.
 * The runs are those whose first shard kept its timings, `timings-1`, as
 * each that ran the suite has: GitHub lists those by name, where its list
 * of all the repository's artifacts fails as often as not with a 500 that
 * three asks don't get past (#606). */
function plannedTestTimings(): { seconds: Map<string, number>; facts: ShardFacts } | undefined {
  try {
    const listed: { name: string; workflow_run: { id: number }; created_at: string; expired: boolean }[] = JSON.parse(
      gh(["api", "repos/{owner}/{repo}/actions/artifacts?per_page=100&name=timings-1", "--jq", "[.artifacts[] | {name, workflow_run, created_at, expired}]"]),
    );
    const runs = timingsRuns(listed.map((a) => ({ name: a.name, run: a.workflow_run.id, createdAt: a.created_at, expired: a.expired })));
    if (runs.length === 0) {
      console.error("no run has kept its timings yet: the test shards split by file count");
      return undefined;
    }
    // Each file's time the newest run's that timed it.
    const paths: string[] = [];
    for (const run of runs) {
      const dir = join(root, "target", "test-timings-ran", String(run));
      if (!downloaded(run, ["--pattern=timings-*"], dir)) {
        console.error(`run ${run}'s timings couldn't be read: passed over`);
        continue;
      }
      const files = readdirSync(dir, { recursive: true, withFileTypes: true }).filter((entry) => entry.isFile() && entry.name.endsWith(".json"));
      paths.push(...files.map((entry) => join(entry.parentPath, entry.name)).sort());
    }
    // Of the runs that kept what their shards measured, each file's share
    // of its rest's wall (`testFileSeconds`), and the pool's and browsers'
    // measures (`shardFacts`).
    const seconds = testFileSeconds(paths, true);
    const facts = shardFacts(paths);
    if (seconds.size === 0 || facts === undefined) {
      console.error("no run kept its shards' measures yet: the test shards split by file count");
      return undefined;
    }
    console.error(`the test shards balance by runs ${runs.join(", ")}'s timings, ${seconds.size} files', each the newest that timed it, expected at their slowest pace, ${facts.pace.toFixed(2)}`);
    return { seconds, facts };
  } catch (error) {
    console.error(`the test files' times couldn't be read, ${error instanceof Error ? error.message : error}: the test shards split by file count`);
    return undefined;
  }
}

/** Test files' times, as `bun test --timings` writes them, `{ version: 1,
 * files: { [file]: ms } }`, gzipped, base64: one line of a job's output,
 * every shard's alike. A few KB of some 360 files. */
export function encodedTestTimings(seconds: ReadonlyMap<string, number>): string {
  const files = Object.fromEntries([...seconds].sort(([a], [b]) => a.localeCompare(b)).map(([file, s]) => [file, Math.round(s * 1000)]));
  return gzipSync(JSON.stringify({ version: 1, files })).toString("base64");
}

/** `encodedTestTimings`' times, in seconds by file: none of `""`. */
export function decodedTestTimings(encoded: string): Map<string, number> {
  if (encoded === "") return new Map();
  const { files } = JSON.parse(gunzipSync(Buffer.from(encoded, "base64")).toString()) as { files: Record<string, number> };
  return new Map(Object.entries(files).map(([file, ms]) => [file, ms / 1000]));
}

/** In CI, before a test shard runs: the times `plan` gave every shard
 * alike, as `target/shard-timings/plan.json`, by which `bun run test` deals
 * each shard its files (scripts/test.ts); none where it gave none. */
function writeTestTimings(encoded: string) {
  const dir = join(root, shardTimings);
  rmSync(dir, { recursive: true, force: true });
  if (encoded === "") {
    console.log("plan read no timings: the shards split by file count");
    return;
  }
  mkdirSync(dir, { recursive: true });
  const file = join(dir, "plan.json");
  writeFileSync(file, gunzipSync(Buffer.from(encoded, "base64")));
  console.log(`the shards balance by ${testFileSeconds([file]).size} files' times, in ${shardTimings}`);
}

/** The artifact of a run's merged mutations' times, each its share of its
 * shard (`shareOfShard`), named here alone: rustc-tests.yml uploads it by
 * the name `merge-mutation-timings` gives it, and `plan` reads it by this.
 * Renamed as its times' meaning changed: `mutation-timings-merged`'s were
 * each mutation's from its tests' asking for a slot, a nightly's minutes
 * behind thousands of others, which no plan should size by. */
export const MERGED_MUTATION_TIMES = "mutation-shares-merged";

/** The run whose merged mutations' times a check's `plan` reads: the
 * newest of `main` that kept `MERGED_MUTATION_TIMES` (the
 * `mutation-timings` job's, rustc-tests.yml), still kept; or none, and the
 * shards deal in turn. A merged file carries what each run before it
 * timed, the nightly's every mutation and each check's since, the later
 * time of a mutation over the earlier, so one small file is all a plan
 * fetches: by each shard, a check's downloads of six runs' every shard's
 * times, one of which GitHub's storage turned away (check of 6d9fcfb45). */
export function mergedTimingsRun(artifacts: (Artifact & { branch: string })[]): number | undefined {
  return [...artifacts]
    .filter((a) => a.name === MERGED_MUTATION_TIMES && !a.expired && a.branch === "main")
    .sort((a, b) => b.createdAt.localeCompare(a.createdAt))[0]?.run;
}

/** Mutations' times, `{ name: seconds }`, as one line a job's output holds
 * for every shard alike: each of `names` (the catalog, the checkout's
 * every mutation, which every job of a run has the same of) in turn, its
 * tenths of a second, or nothing where it has none; gzipped, base64.
 * Handed so, not downloaded by each shard: a shard that couldn't read
 * them would deal the mutations otherwise than the others, some run twice
 * and some by none. About 20 KB of a nightly's 14,000. */
export function encodedTimings(times: ReadonlyMap<string, number>, names: string[]): string {
  const catalog = [...new Set(names)].sort();
  const line = catalog.map((name) => (times.has(name) ? String(Math.round(times.get(name)! * 10)) : "")).join(",");
  return Buffer.from(gzipSync(`${catalog.length}:${line}`)).toString("base64");
}

/** `encodedTimings`' times again, of the same catalog `names`; refused
 * where it's another's, a count that differs. */
export function decodedTimings(encoded: string, names: string[]): Map<string, number> {
  const catalog = [...new Set(names)].sort();
  const text = gunzipSync(Buffer.from(encoded, "base64")).toString();
  const colon = text.indexOf(":");
  const count = Number(text.slice(0, colon));
  if (count !== catalog.length) throw new Error(`mutations' times of a catalog of ${count}, not this checkout's ${catalog.length}`);
  const times = new Map<string, number>();
  text.slice(colon + 1).split(",").forEach((tenths, i) => {
    if (tenths !== "") times.set(catalog[i]!, Number(tenths) / 10);
  });
  return times;
}

/** The values `gh api --paginate --jq '.items[] | ..'` prints, one a line. */
export function jsonLines<T>(text: string): T[] {
  return text.split("\n").filter((line) => line.trim() !== "").map((line) => JSON.parse(line) as T);
}

/** Of `plan`: the merged mutations' times of the newest run that kept
 * them (`mergedTimingsRun`), encoded for the shards (`encodedTimings`); or
 * nothing, said why, and every shard deals in turn. Never a failure: a
 * plan without them is a slower one. */
function plannedMutationTimings(): string {
  try {
    const listed: { name: string; workflow_run: { id: number; head_branch: string }; created_at: string; expired: boolean }[] = jsonLines(
      gh(["api", "--paginate", `repos/{owner}/{repo}/actions/artifacts?per_page=100&name=${MERGED_MUTATION_TIMES}`, "--jq", ".artifacts[] | {name, workflow_run, created_at, expired}"]),
    );
    const run = mergedTimingsRun(listed.map((a) => ({ name: a.name, run: a.workflow_run.id, branch: a.workflow_run.head_branch, createdAt: a.created_at, expired: a.expired })));
    if (run === undefined) {
      console.error("no run of main kept its merged mutations' times: the mutation shards deal in turn");
      return "";
    }
    const dir = join(root, "target", MERGED_MUTATION_TIMES);
    if (!downloaded(run, [`--name=${MERGED_MUTATION_TIMES}`], dir)) {
      console.error(`run ${run}'s merged mutations' times couldn't be read: the mutation shards deal in turn`);
      return "";
    }
    return encodedTimings(timingsUnder(dir), mutations.map((m) => m.name));
  } catch (error) {
    console.error(`the merged mutations' times couldn't be read, ${error instanceof Error ? error.message : error}: the mutation shards deal in turn`);
    return "";
  }
}

/** In CI, before a mutation shard runs: the times `plan` gave every shard
 * alike (`encodedTimings`), as `target/mutation-timings/timings.json`,
 * which `bun scripts/mutations.ts --timings=` balances the shards by. */
function writeMutationTimings(encoded: string) {
  const dir = join(root, "target", "mutation-timings");
  rmSync(dir, { recursive: true, force: true });
  mkdirSync(dir, { recursive: true });
  const times = decodedTimings(encoded, mutations.map((m) => m.name));
  writeFileSync(join(dir, "timings.json"), timingsJson(times));
  console.log(`the mutation shards balance by ${times.size} mutations' times, in target/mutation-timings`);
}

/** Times as a mutation shard writes them, `{ name: { build, tests } }`,
 * each all its build's, to a tenth of a second, as dealing needs: what
 * `timingsUnder` reads. */
const timingsJson = (times: ReadonlyMap<string, number>) =>
  JSON.stringify(Object.fromEntries([...times].map(([name, seconds]) => [name, { build: Math.round(seconds * 10) / 10, tests: 0 }]))) + "\n";

/** `base`, a plan's times, with what this run's shards timed in `ran`
 * over it, each `timings.json` under it: the times the next plan reads. */
export function mergedTimings(base: ReadonlyMap<string, number>, ran: ReadonlyMap<string, number>): Map<string, number> {
  return new Map([...base, ...ran]);
}

/** Each shard's times, `shards`, as its machine would be had it run at the
 * plan's speed, `base`: divided by the median of its mutations' times over
 * the plan's, of those the plan timed at over half a second, at least 20, and
 * held to 0.5–2. GitHub's machines differ: nightly 44963035271's shards 1,
 * 13, 14, 15 and 16 took 1.22 to 1.32 times what the plan gave each of their
 * mutations, at the median, its others 0.89 to 1.01, a whole shard's alike,
 * so the newest time of a mutation was its machine's as much as its own, and
 * the next deal took it so (#2304 (b)). A mutation that grew, against its
 * shard's others, keeps what it grew by. */
export function normalizedShardTimes(base: ReadonlyMap<string, number>, shards: ReadonlyMap<string, ReadonlyMap<string, number>>): Map<string, number> {
  const out = new Map<string, number>();
  for (const own of shards.values()) {
    const ratios = [...own]
      .filter(([name]) => (base.get(name) ?? 0) > 0.5)
      .map(([name, seconds]) => seconds / base.get(name)!)
      .sort((a, b) => a - b);
    const speed = ratios.length < 20 ? 1 : Math.min(2, Math.max(0.5, ratios[ratios.length >> 1]!));
    for (const [name, seconds] of own) out.set(name, seconds / speed);
  }
  return out;
}

/** In CI, after a run's mutation shards (rustc-tests.yml's
 * `mutation-timings`): the times its plan gave them (`encoded`, none of a
 * first nightly's) and each shard's own under `ran`, merged into
 * target/`MERGED_MUTATION_TIMES`/timings.json, and its artifact's name and
 * path said as GitHub's step outputs, which upload it for the next plan.
 * None where no shard's times were read: no file and no outputs, so no
 * upload, and the next plan reads an older run's. */
function mergeMutationTimings(encoded: string, ran: string) {
  const shards = timingsByShard(ran);
  if (shards.size === 0) {
    console.error(`no shard's times under ${ran}: nothing merged`);
    return;
  }
  const base = encoded === "" ? new Map<string, number>() : decodedTimings(encoded, mutations.map((m) => m.name));
  // Each shard's at the plan's speed, its machine's own taken out.
  const own = normalizedShardTimes(base, shards);
  const merged = mergedTimings(base, own);
  const out = join("target", MERGED_MUTATION_TIMES, "timings.json");
  mkdirSync(join(root, out, ".."), { recursive: true });
  writeFileSync(join(root, out), timingsJson(merged));
  console.error(`${merged.size} mutations' times merged, ${own.size} of this run's shards over ${base.size} of its plan's, in ${out}`);
  console.log(`artifact=${MERGED_MUTATION_TIMES}`);
  console.log(`path=${out}`);
}

/** The Actions cache's entries each family keeps, the newest by when they
 * were saved: a key restored exactly is of a run in flight, of the newest
 * commits' sources, and one restored by its family's prefix takes the
 * newest. 8 of a test map, about 6 MB each, of a binary, about 18 MB. */
export const keptCacheEntries = 8;

/** A cache key's family: the key less what's new each run or each set of
 * inputs, its trailing hashes, `hashFiles`' and a commit's, and a run's id
 * and attempt. `rust-js-release-Linux-<hash>` is `rust-js-release-Linux`'s,
 * `native-outcomes-Linux-test-1-of-2-<run>-<attempt>` is
 * `native-outcomes-Linux-test-1-of-2`'s, `test-map-Linux-<toolchain>-<sha>`
 * is `test-map-Linux`'s. */
export function cacheFamily(key: string): string {
  return key.replace(/(-(?:[0-9a-f]{40,}|\d{9,}(?:-\d+)?))+$/, "");
}

export type CacheEntry = { id: number; key: string; ref: string; createdAt: string; sizeInBytes: number };

/** Of the Actions cache's entries, those past each family's newest `keep`,
 * a family a ref's: a branch's entries never push out main's. */
export function cachePast(entries: CacheEntry[], keep: number): CacheEntry[] {
  const families = Map.groupBy(entries, (entry) => `${entry.ref}\0${cacheFamily(entry.key)}`);
  return [...families.values()].flatMap((family) => family.toSorted((a, b) => Date.parse(b.createdAt) - Date.parse(a.createdAt)).slice(keep));
}

/** CI's, after each check, nightly and test map (cache-prune.yml): the
 * Actions cache's entries past each family's newest `keptCacheEntries`
 * removed, or, `dryRun`, listed. */
async function pruneCache(dryRun: boolean) {
  const listed = Bun.spawnSync(["gh", "cache", "list", "--limit", "10000", "--json", "id,key,ref,createdAt,sizeInBytes"], { cwd: root, stdout: "pipe", stderr: "inherit" });
  if (listed.exitCode !== 0) throw new Error("gh cache list failed");
  const entries: CacheEntry[] = JSON.parse(listed.stdout.toString());
  const past = cachePast(entries, keptCacheEntries);
  const gb = (list: CacheEntry[]) => (list.reduce((sum, { sizeInBytes }) => sum + sizeInBytes, 0) / 1e9).toFixed(2);
  let failed = 0;
  // Sixteen at a time: each is a request of its own.
  for (let at = 0; at < past.length; at += 16) {
    await Promise.all(
      past.slice(at, at + 16).map(async ({ id, key, ref }) => {
        if (dryRun) return console.log(`would remove ${key} (${ref})`);
        const removed = Bun.spawn(["gh", "cache", "delete", String(id)], { cwd: root, stdout: "ignore", stderr: "inherit" });
        // One another run removed first is gone as asked.
        if ((await removed.exited) !== 0) failed++;
        else console.log(`removed ${key} (${ref})`);
      }),
    );
  }
  console.log(`${entries.length} entries, ${gb(entries)} GB; ${dryRun ? "would remove" : "removed"} ${past.length - failed}, ${gb(past)} GB, past each family's newest ${keptCacheEntries}${failed ? `; ${failed} not removed` : ""}`);
}

function bless(branch: string) {
  const run = latestRuns(runs()).find((r) => r.headBranch === branch && r.status === "completed");
  if (!run) throw new Error(`no completed ${workflow} run of ${branch}`);
  const dir = join(root, "target", "ci-bless", String(run.databaseId));
  const got = downloaded(run.databaseId, ["--pattern=bless-*"], dir);
  const patches = readdirSync(dir, { recursive: true, withFileTypes: true })
    .filter((entry) => entry.isFile() && entry.name.endsWith(".patch"))
    .map((entry) => join(entry.parentPath, entry.name));
  if (!got && patches.length === 0) {
    console.log(`run ${run.databaseId} of ${branch} has no bless patches: nothing to apply`);
    return;
  }
  for (const patch of patches) {
    const applied = Bun.spawnSync(["git", "apply", "--index", patch], { cwd: root, stdout: "inherit", stderr: "inherit" });
    if (applied.exitCode !== 0) throw new Error(`${patch} doesn't apply: is the branch at run ${run.databaseId}'s commit?`);
    console.log(`applied ${patch}`);
  }
  console.log("Read every change (git diff --cached) before committing it, as a local bless's.");
}

if (import.meta.main) {
  const [command, ...rest] = process.argv.slice(2);
  const branch = rest.find((arg) => !arg.startsWith("--"))
    ?? Bun.spawnSync(["git", "branch", "--show-current"], { cwd: root }).stdout.toString().trim();
  const shards = rest.find((arg) => arg.startsWith("--shards="))?.slice("--shards=".length);
  const since = rest.find((arg) => arg.startsWith("--since="))?.slice("--since=".length);
  const testShardsGiven = rest.find((arg) => arg.startsWith("--test-shards="))?.slice("--test-shards=".length) || undefined;
  try {
    if (command === "status") status();
    else if (command === "check") await check(branch, shards, since);
    else if (command === "bless") bless(branch);
    else if (command === "plan") await plan(since, testShardsGiven);
    else if (command === "test-timings") writeTestTimings(rest[0] ?? "");
    else if (command === "mutation-timings") writeMutationTimings(rest[0] ?? "");
    else if (command === "merge-mutation-timings") mergeMutationTimings(process.env.TIMES ?? "", rest[0] ?? "");
    else if (command === "nets") console.log(netsSummary(JSON.parse(process.env.STEPS ?? "{}")));
    else if (command === "prune-cache") await pruneCache(rest.includes("--dry-run"));
    else {
      console.error("usage: bun scripts/ci.ts check [branch] [--shards=1|2|4] [--since=<rev>] | status | bless [branch] | plan [--since=<rev>] [--test-shards=<n>] | test-timings <times> | mutation-timings <times> | merge-mutation-timings <ran> | nets | prune-cache [--dry-run]");
      process.exit(2);
    }
  } catch (error) {
    console.error(error instanceof Error ? error.message : error);
    process.exit(1);
  }
}
