// The checks CI runs of each push to `main` (DEVELOPMENT.md), read from
// here, and of another branch started from here, by GitHub's CLI:
//
//   bun run ci:check [branch] [--shards=4] [--since=<rev>]  # check a branch as it's pushed; main's push checks it already
//   bun run ci:status                       # each branch's latest run, and what failed, and the nightly's
//   bun run ci:bless [branch]               # apply its latest run's bless patches
//   bun scripts/ci.ts timings               # in CI: the last run's shards' timings, for this run's to balance by
//   bun scripts/ci.ts mutation-timings <times>   # in CI: the mutations' times `plan` gave every shard, for them to balance by
//   TIMES=<times> bun scripts/ci.ts merge-mutation-timings <ran> <out> # in CI: those and this run's shards' merged, for the next plan
//   bun scripts/ci.ts nets                  # in CI: which of the `nets` job's nets ran, and which failed, from its `STEPS`
//
// A run whose snapshots or rustc's lists differ uploads what blessing them
// writes, as `bless-*` artifacts: applied here, to be read as a local bless
// is, before they're committed.

import { appendFileSync, mkdirSync, readFileSync, readdirSync, rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { gunzipSync, gzipSync } from "node:zlib";

import { changedCases, touchesAsync, touchesConsumers, touchesCrates, touchesDeclarations, touchesFormatting, touchesIterators, touchesMemory, touchesNumbers, touchesPort, touchesRustc, touchesStdNet, touchesTwins, touchesWasm } from "./iteratorSlice";
import { changedSince, mutations } from "./mutations";
import { outcome, type Selection, selectChange } from "./select";
import { downloaded } from "./artifacts";
import { shardTimings, timingsUnder } from "./shard";
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

/** What a run's mutation shards found, each `mutation-results-<k>`'s, of
 * the commit it ran at (`mutations.ts`'s `Results`), and how many shards
 * ran, each completed, passed or not. */
export type RunResults = { run: Run; shards: number; completed: number; results: { commit: string; results: Record<string, string> }[]; latent?: string[] };

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
  for (const { run, shards, completed, results, latent = [] } of newestFirst) {
    if (run.status !== "completed" || !run.headSha || shards === 0 || completed !== shards || results.length !== shards) continue;
    if (!results.every((r) => r.commit === run.headSha)) continue;
    // Not one it judged latent, surviving at its base too: no push's, filed
    // in the survivors' issue, and the nightly runs it. Retried, the 6–7 of
    // the crate tests held a shard up in every check, 40% of a check's
    // runner-minutes the mutations'. A push that changes its site or its
    // tests still runs it, `--changed`'s, whatever the retry list says.
    const retry = results.flatMap((r) => Object.entries(r.results).filter(([name, result]) => result !== "caught" && !latent.includes(name)).map(([name]) => name));
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
    // Its artifacts first, as a download of none asks again, three times.
    const kept = () =>
      (JSON.parse(gh(["api", `repos/{owner}/{repo}/actions/runs/${run.databaseId}/artifacts?per_page=100`, "--jq", "[.artifacts[].name]"])) as string[])
        .some((name) => name.startsWith("mutation-results-"));
    if (shardJobs.length > 0 && completed === shardJobs.length && kept()) {
      try {
        if (!downloaded(run.databaseId, ["--pattern=mutation-results-*", "--pattern=mutation-latent-*"], dir)) throw new Error(`run ${run.databaseId}'s results`);
        for (const shard of readdirSync(dir).filter((name) => name.startsWith("mutation-results-"))) results.push(JSON.parse(readFileSync(join(dir, shard, "results.json"), "utf8")));
        for (const shard of readdirSync(dir).filter((name) => name.startsWith("mutation-latent-"))) {
          const kept: { latent: { name: string }[] } = JSON.parse(readFileSync(join(dir, shard, "latent.json"), "utf8"));
          latent.push(...kept.latent.map(({ name }) => name));
        }
      } catch {}
    }
    out.push({ run, shards: shardJobs.length, completed, results, latent });
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

/** Machines for a check's mutations, as many as keeps each near twenty,
 * five minutes or so, as long as the suite on its two. */
export function mutationShards(count: number): string {
  const wanted = Math.ceil(count / 20);
  return String(MUTATION_SHARDS.find((n) => n >= wanted) ?? MUTATION_SHARDS.at(-1));
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
  const count = (await changedSince(base)).length;
  const machines = mutationShards(count);
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
async function plan(given?: string) {
  let since = given ?? "HEAD~1";
  let retry: string[] = [];
  if (given === undefined) {
    try {
      // What the last check's mutations left, else the last that passed.
      const base = resultsBase(runResults());
      since = base?.since ?? checkedSince(runs(), "main", runMutationsPassed) ?? since;
      retry = base?.retry ?? [];
    } catch {}
  }
  let count = 0;
  try {
    count = (await changedSince(since)).length;
  } catch {
    since = "HEAD~1";
    count = (await changedSince(since)).length;
  }
  // The mutations run only where the change touches one, its site, its
  // list or its tests: what the last check didn't catch, of code as it
  // was, isn't caught now either. A run of none is passed over by the
  // next one's `resultsBase`, which keeps the base and what to retry.
  const mutating = count > 0;
  count += retry.length;
  const changed = Bun.spawnSync(["git", "diff", "--name-only", since, "HEAD"], { cwd: root }).stdout.toString().split("\n").filter(Boolean);
  console.log(`since=${since}`);
  console.log(`retry=${retry.join(",")}`);
  console.log(`shards=${mutationShards(count)}`);
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
  console.log(`mutations=${mutating}`);
  console.log(`mutation_timings=${plannedMutationTimings()}`);
  console.log(`wasm=${touchesWasm(changed)}`);
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
  console.log(`test_shards=${testShards(selection)}`);
  // The batch a failure is attributed in: since the newest check of
  // `main` that completed, red or green (scripts/attribute.ts).
  let batchBase = "";
  try {
    batchBase = completedBase(runs(), Bun.spawnSync(["git", "rev-parse", "HEAD"], { cwd: root }).stdout.toString().trim()) ?? "";
  } catch {}
  console.log(`batch_base=${batchBase}`);
  const said = `The suite's selection: ${outcome(selection)}${selection.map ? `, by the test map of ${selection.map.slice(0, 9)}` : ""}`;
  console.error(said);
  if (process.env.GITHUB_STEP_SUMMARY) appendFileSync(process.env.GITHUB_STEP_SUMMARY, `${said}\n`);
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

/** How many machines the suite's shards of a selection take: one of a
 * small one, at most 60 files and 400 corpus cases, whose setup would be
 * most of a second machine's minutes; two of more, everything's too. */
export function testShards(selection: Selection): "1" | "2" {
  return !selection.all && selection.files.length <= 60 && selection.cases.length <= 400 ? "1" : "2";
}

/** `plan`'s `tests` output: what the suite's shards run of a selection,
 * `all`, `some`, or `none`, where the shards are skipped. */
export function testsOutput(selection: Selection): "all" | "some" | "none" {
  if (selection.all) return "all";
  return selection.files.length === 0 && selection.cases.length === 0 ? "none" : "some";
}

/** Each net of check.yml's `nets` job, by its step's id, and its name:
 * the steps that run in the background, as the workflow lists them. */
export function nets(workflow = readFileSync(join(root, ".github/workflows/check.yml"), "utf8")): { id: string; name: string }[] {
  type Step = { id?: string; name?: string; background?: boolean };
  const steps: Step[] = (Bun.YAML.parse(workflow) as { jobs: { nets?: { steps: Step[] } } }).jobs.nets?.steps ?? [];
  return steps.filter((step) => step.background).map((step) => ({ id: step.id!, name: step.name! }));
}

/** The `nets` job's summary, of its steps' outcomes (GitHub's `steps`
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
export function timingsRun(artifacts: Artifact[]): number | undefined {
  return [...artifacts]
    .filter((a) => /^timings-\d+$/.test(a.name) && !a.expired)
    .sort((a, b) => b.createdAt.localeCompare(a.createdAt))[0]?.run;
}

/** In CI, before the suite: the last run's `timings-*` artifacts, into
 * `target/shard-timings/`, by which `bun run test` deals each shard its files
 * (scripts/test.ts). The runs are those whose first shard kept its timings,
 * `timings-1`, as each that ran the suite has: GitHub lists those by name,
 * where its list of all the repository's artifacts fails as often as not
 * with a 500 that three asks don't get past (#606). */
function fetchTimings() {
  const listed: { name: string; workflow_run: { id: number }; created_at: string; expired: boolean }[] = JSON.parse(
    gh(["api", "repos/{owner}/{repo}/actions/artifacts?per_page=100&name=timings-1", "--jq", "[.artifacts[] | {name, workflow_run, created_at, expired}]"]),
  );
  const run = timingsRun(listed.map((a) => ({ name: a.name, run: a.workflow_run.id, createdAt: a.created_at, expired: a.expired })));
  const dir = join(root, shardTimings);
  rmSync(dir, { recursive: true, force: true });
  if (run === undefined) {
    console.log("no run has kept its timings yet: the shards split by file count");
    return;
  }
  // Every shard deals by what it reads here, so one that can't read them
  // fails rather than deal otherwise than the others.
  if (!downloaded(run, ["--pattern=timings-*"], dir)) throw new Error(`run ${run}'s timings couldn't be read`);
  const files = readdirSync(dir, { recursive: true, withFileTypes: true }).filter((entry) => entry.isFile() && entry.name.endsWith(".json"));
  console.log(`the shards balance by run ${run}'s timings, ${files.length} file${files.length === 1 ? "" : "s"} in ${shardTimings}`);
}

/** The run whose merged mutations' times a check's `plan` reads: the
 * newest of `main` that kept `mutation-timings-merged` (the
 * `mutation-timings` job's, rustc-tests.yml), still kept; or none, and the
 * shards deal in turn. A merged file carries what each run before it
 * timed, the nightly's every mutation and each check's since, the later
 * time of a mutation over the earlier, so one small file is all a plan
 * fetches: by each shard, a check's downloads of six runs' every shard's
 * times, one of which GitHub's storage turned away (check of 6d9fcfb45). */
export function mergedTimingsRun(artifacts: (Artifact & { branch: string })[]): number | undefined {
  return [...artifacts]
    .filter((a) => a.name === "mutation-timings-merged" && !a.expired && a.branch === "main")
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
      gh(["api", "--paginate", "repos/{owner}/{repo}/actions/artifacts?per_page=100&name=mutation-timings-merged", "--jq", ".artifacts[] | {name, workflow_run, created_at, expired}"]),
    );
    const run = mergedTimingsRun(listed.map((a) => ({ name: a.name, run: a.workflow_run.id, branch: a.workflow_run.head_branch, createdAt: a.created_at, expired: a.expired })));
    if (run === undefined) {
      console.error("no run of main kept its merged mutations' times: the mutation shards deal in turn");
      return "";
    }
    const dir = join(root, "target", "mutation-timings-merged");
    if (!downloaded(run, ["--name=mutation-timings-merged"], dir)) {
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

/** In CI, after a run's mutation shards (rustc-tests.yml's
 * `mutation-timings`): the times its plan gave them (`encoded`, none of a
 * first nightly's) and each shard's own under `ran`, merged into `out`,
 * uploaded as `mutation-timings-merged` for the next plan. None where no
 * shard's times were read: no file, so no upload, and the next plan reads
 * an older run's. */
function mergeMutationTimings(encoded: string, ran: string, out: string) {
  const own = timingsUnder(ran);
  if (own.size === 0) {
    console.log(`no shard's times under ${ran}: nothing merged`);
    return;
  }
  const base = encoded === "" ? new Map<string, number>() : decodedTimings(encoded, mutations.map((m) => m.name));
  const merged = mergedTimings(base, own);
  mkdirSync(join(out, ".."), { recursive: true });
  writeFileSync(out, timingsJson(merged));
  console.log(`${merged.size} mutations' times merged, ${own.size} of this run's shards over ${base.size} of its plan's, in ${out}`);
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
  try {
    if (command === "status") status();
    else if (command === "check") await check(branch, shards, since);
    else if (command === "bless") bless(branch);
    else if (command === "plan") await plan(since);
    else if (command === "timings") fetchTimings();
    else if (command === "mutation-timings") writeMutationTimings(rest[0] ?? "");
    else if (command === "merge-mutation-timings") mergeMutationTimings(process.env.TIMES ?? "", rest[0] ?? "", rest[1] ?? "");
    else if (command === "nets") console.log(netsSummary(JSON.parse(process.env.STEPS ?? "{}")));
    else {
      console.error("usage: bun scripts/ci.ts check [branch] [--shards=1|2|4] [--since=<rev>] | status | bless [branch] | plan | timings | mutation-timings <times> | merge-mutation-timings <ran> <out> | nets");
      process.exit(2);
    }
  } catch (error) {
    console.error(error instanceof Error ? error.message : error);
    process.exit(1);
  }
}
