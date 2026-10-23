// The checks CI runs of each push to `main` (DEVELOPMENT.md), read from
// here, and of another branch started from here, by GitHub's CLI:
//
//   bun run ci:check [branch] [--shards=4] [--since=<rev>]  # check it as it's pushed, again
//   bun run ci:status                       # each branch's latest run, and what failed, and the nightly's
//   bun run ci:bless [branch]               # apply its latest run's bless patches
//   bun scripts/ci.ts timings               # in CI: the last run's shards' timings, for this run's to balance by
//
// A run whose snapshots or rustc's lists differ uploads what blessing them
// writes, as `bless-*` artifacts: applied here, to be read as a local bless
// is, before they're committed.

import { mkdirSync, readdirSync, rmSync } from "node:fs";
import { join } from "node:path";

import { changedSince } from "./mutations";
import { shardTimings } from "./shard";
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

/** The commit `branch`'s last finished check was of, which a check of it
 * runs the mutations of what changed since: a cancelled run, or a running
 * one, checked nothing. */
export function checkedSince(runs: Run[], branch: string): string | undefined {
  return [...runs]
    .sort((a, b) => b.createdAt.localeCompare(a.createdAt))
    .find((run) => run.headBranch === branch && run.status === "completed" && run.conclusion !== "cancelled")?.headSha;
}

function gh(args: string[]): string {
  const p = Bun.spawnSync(["gh", ...args], { cwd: root, stdout: "pipe", stderr: "pipe" });
  if (p.exitCode !== 0) throw new Error(`gh ${args.join(" ")} failed:\n${p.stderr.toString()}`);
  return p.stdout.toString();
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
    console.log(timed("CI check of main", seconds, id));
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
  }
}

/** Start the check of `branch`, as it's pushed: the local one, if it's
 * ahead, isn't what would be checked. Its mutations are of what changed
 * since `since`: of `main`, by default, since its last finished check, or
 * the commit before; of another branch, since it left `main`. */
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

async function check(branch: string, shards: string, since?: string) {
  const git = (args: string[]) => Bun.spawnSync(["git", ...args], { cwd: root }).stdout.toString().trim();
  git(["fetch", "--quiet", "origin", branch]);
  const local = git(["rev-parse", branch]);
  const pushed = git(["rev-parse", `origin/${branch}`]);
  if (!pushed || pushed !== local) throw new Error(`${branch} isn't as it's pushed: push it first, git push -u origin ${branch}`);
  const base = since ?? (branch === "main" ? (checkedSince(runs(), branch) ?? `${local}~1`) : "origin/main");
  // As many machines as the mutations it runs need, which are this
  // checkout's, as it's pushed.
  const count = (await changedSince(base)).length;
  const machines = mutationShards(count);
  gh(["workflow", "run", workflow, `--ref=${branch}`, "-f", `test_shards=${shards}`, "-f", `since=${base}`, "-f", `mutation_shards=${machines}`]);
  console.log(`started ${workflow} of ${branch} at ${local.slice(0, 10)}, the suite on ${shards} machine${shards === "1" ? "" : "s"}, the ${count} mutations of what changed since ${base.slice(0, 10)} on ${machines}: bun run ci:status`);
}

/** Of a push, in CI: what its mutations are of, what changed since `main`'s
 * last finished check, or the commit before, and how many machines they
 * take, as GitHub's step outputs. */
async function plan() {
  let since = "HEAD~1";
  try {
    since = checkedSince(runs(), "main") ?? since;
  } catch {}
  let count = 0;
  try {
    count = (await changedSince(since)).length;
  } catch {
    since = "HEAD~1";
    count = (await changedSince(since)).length;
  }
  console.log(`since=${since}`);
  console.log(`shards=${mutationShards(count)}`);
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
 * `target/shard-timings/`, which `bun run test` gives `bun test --shard`
 * (scripts/test.ts). */
function fetchTimings() {
  const listed: { name: string; workflow_run: { id: number }; created_at: string; expired: boolean }[] = JSON.parse(
    gh(["api", "repos/{owner}/{repo}/actions/artifacts?per_page=100", "--jq", "[.artifacts[] | {name, workflow_run, created_at, expired}]"]),
  );
  const run = timingsRun(listed.map((a) => ({ name: a.name, run: a.workflow_run.id, createdAt: a.created_at, expired: a.expired })));
  const dir = join(root, shardTimings);
  rmSync(dir, { recursive: true, force: true });
  if (run === undefined) {
    console.log("no run has kept its timings yet: the shards split by file count");
    return;
  }
  mkdirSync(dir, { recursive: true });
  gh(["run", "download", String(run), "--pattern=timings-*", `--dir=${dir}`]);
  const files = readdirSync(dir, { recursive: true, withFileTypes: true }).filter((entry) => entry.isFile() && entry.name.endsWith(".json"));
  console.log(`the shards balance by run ${run}'s timings, ${files.length} file${files.length === 1 ? "" : "s"} in ${shardTimings}`);
}

function bless(branch: string) {
  const run = latestRuns(runs()).find((r) => r.headBranch === branch && r.status === "completed");
  if (!run) throw new Error(`no completed ${workflow} run of ${branch}`);
  const dir = join(root, "target", "ci-bless", String(run.databaseId));
  rmSync(dir, { recursive: true, force: true });
  mkdirSync(dir, { recursive: true });
  const download = Bun.spawnSync(["gh", "run", "download", String(run.databaseId), "--pattern=bless-*", `--dir=${dir}`], { cwd: root, stdout: "pipe", stderr: "pipe" });
  const patches = readdirSync(dir, { recursive: true, withFileTypes: true })
    .filter((entry) => entry.isFile() && entry.name.endsWith(".patch"))
    .map((entry) => join(entry.parentPath, entry.name));
  if (download.exitCode !== 0 && patches.length === 0) {
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
  const shards = rest.find((arg) => arg.startsWith("--shards="))?.slice("--shards=".length) ?? "1";
  const since = rest.find((arg) => arg.startsWith("--since="))?.slice("--since=".length);
  try {
    if (command === "status") status();
    else if (command === "check") await check(branch, shards, since);
    else if (command === "bless") bless(branch);
    else if (command === "plan") await plan();
    else if (command === "timings") fetchTimings();
    else {
      console.error("usage: bun scripts/ci.ts check [branch] [--shards=1|2|4] [--since=<rev>] | status | bless [branch] | plan | timings");
      process.exit(2);
    }
  } catch (error) {
    console.error(error instanceof Error ? error.message : error);
    process.exit(1);
  }
}
