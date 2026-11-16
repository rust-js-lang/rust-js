// Which commit of a batched check broke each test it failed: a check of
// `main` runs once over the commits since the last completed check, and
// where it fails, each failing test file and corpus case runs again at each
// commit of that batch, on its own machine, and the first commit it fails at
// is said to have broken it (check.yml's `attribute`).
//
//   bun scripts/attribute.ts plan <junit dir> <base>   the failing tests (target/attribute/failing.json) and the commits to run them at, `commits=`
//   (the suite's shards' reports, and the slices' and nets' of
//   scripts/iteratorSlice.ts, `slices/`: each failing test of a slice run
//   again by its run's file and settings, `runs`; a failed step of the
//   nets' that no rerun can say, the newest commit that touched its inputs)
//   (check.yml's `attribute` runs them at each commit by `bun test`, each
//   commit's own harness, and writes which still fail, `result.json`)
//   bun scripts/attribute.ts report <results dir>      attribution.json and the summary's lines, "<test> broke at <sha> (<subject>)"
//
// Where its mutations fail, no rerun: each mutation a shard didn't catch,
// not latent, is said changed by the batch's newest commit that touched its
// site's lines, its list or its tests (`git log -L`, a blame); a shard or
// the survivors' job that failed with no verdict, by the newest that touched
// the runner's own files (check.yml's `attribute-mutations`).
//
//   bun scripts/attribute.ts mutations <artifacts dir> <base> [head]   attribution-mutations.json and the summary's lines; of a past run, its head

import { appendFileSync, existsSync, mkdirSync, readdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

import { lists, type Mutation, testResults } from "./mutations";
import { cratePaths, portPaths, type SliceReport } from "./iteratorSlice";
import { failedIn } from "./selector-audit";

const root = join(import.meta.dir, "..");
const out = join(root, "target", "attribute");

/** The most commits a batch's failures are run again at, each a machine. */
export const mostCommits = 6;

/** The failing tests' units, `test/x.test.ts` or a corpus case's
 * `test/corpus/x.rs`; and of each slice run, its failing tests, run again
 * as `bun test <file> -t <pattern>` with its settings, `unit` its name. */
export type Failing = { files: string[]; cases: string[]; runs: SliceRerun[] };

/** A slice run's failing tests, run again at a commit: every part's at once. */
export type SliceRerun = { unit: string; file: string; env: Record<string, string>; pattern: string };

/** Of the slices' reports, `[report, its SliceReport, its JUnit]`, each
 * run's failing tests, a rerun a run: the metamorphic rewrites' none, as
 * their cases are the change's own, said by `unattributable`. */
export function sliceReruns(reports: [SliceReport, string][]): SliceRerun[] {
  const failing = new Map<string, { report: SliceReport; tests: Set<string> }>();
  for (const [report, junit] of reports) {
    if (report.of === "metamorphic") continue;
    const tests = [...testResults(junit)].filter(([, passed]) => !passed).map(([name]) => name);
    if (tests.length === 0) continue;
    const key = JSON.stringify([report.name, report.file, report.env]);
    const run = failing.get(key) ?? { report, tests: new Set<string>() };
    for (const name of tests) run.tests.add(name);
    failing.set(key, run);
  }
  const escaped = (name: string) => name.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  return [...failing.values()].map(({ report, tests }) => {
    const names = [...tests].sort();
    return {
      unit: `${report.name}: ${names.length === 1 ? names[0] : `${names.length} tests, ${names.slice(0, 3).join("; ")}${names.length > 3 ? "; .." : ""}`}`,
      file: report.file,
      env: report.env,
      pattern: `^(?:${names.map(escaped).join("|")})$`,
    };
  });
}

/** The nets' steps no rerun can say, by their names in check.yml, each with
 * its inputs: the metamorphic rewrites, of the change's own cases; the
 * crate corpus and React's port, which run no `bun test`. Where one fails,
 * the newest commit of the batch that touched its inputs is named, as a
 * mutations job's runner (`runnerFiles`). */
export const unrerunnable: Record<string, string[]> = {
  "metamorphic rewrites of the change's cases": ["src/**", "std/**", "test/corpus/*.rs", "test/metamorphic.test.ts", "test/metamorphic/**", "scripts/iteratorSlice.ts"],
  "crate corpus": cratePaths,
  "react-in-rust port": portPaths,
};

/** Of each failed step `unrerunnable` names, its line: the newest commit
 * `touching(paths)` gives, `said(commit)`, or none of the batch. */
export function unattributable(failed: string[], touching: (paths: string[]) => string | undefined, said: (commit: string) => string): string[] {
  return failed
    .filter((step) => step in unrerunnable)
    .map((step) => {
      const by = touching(unrerunnable[step]!);
      return `${step} failed; not attributable by rerun: ${by ? `last change to its inputs was ${said(by)}` : "its inputs unchanged in the batch, so an earlier change's, its machine's or GitHub's"}`;
    });
}

/** Of `commits`, oldest first, at most `most` to run at: each where there
 * are few enough, else spread from the oldest to the newest, both kept. */
export function commitsToRun(commits: string[], most = mostCommits): string[] {
  if (commits.length <= most) return commits;
  const picked = new Set<number>();
  for (let i = 0; i < most; i++) picked.add(Math.round((i * (commits.length - 1)) / (most - 1)));
  return [...picked].sort((a, b) => a - b).map((i) => commits[i]!);
}

/** Of each failing unit, the oldest commit of `runs`, oldest first, from
 * which it fails at each run after: what broke it, where the run before it
 * passed; the first run's, where it failed at every one (it broke at the
 * batch's first commit, or before); or the newest run's, `passes`, where it
 * passed there, a failure the check alone saw, flaky or its machine's. */
export function attribution(runs: { commit: string; failing: string[] }[], units: string[]): { unit: string; commit: string; since: "batch" | "before" | "passes" }[] {
  return units.map((unit) => {
    let at = runs.length;
    while (at > 0 && runs[at - 1]!.failing.includes(unit)) at--;
    if (at === runs.length) return { unit, commit: runs.at(-1)!.commit, since: "passes" as const };
    return at === 0 ? { unit, commit: runs[0]!.commit, since: "before" as const } : { unit, commit: runs[at]!.commit, since: "batch" as const };
  });
}

/** What each kind of mutations job runs by, besides each mutation's own
 * files: a shard, or the survivors' job, that fails with no verdict is said
 * broken by the batch's newest commit that touched one of its own. */
export const runnerFiles = {
  shard: [
    "scripts/mutations.ts",
    "scripts/source-mutations.ts",
    "scripts/mutant-builds.ts",
    "scripts/latent-survivors.ts",
    "scripts/shard.ts",
    "scripts/ci.ts",
    ".github/workflows/rustc-tests.yml",
    ".github/actions/rustc-tests-setup",
  ],
  survivors: ["scripts/survivors-issue.ts", ".github/workflows/rustc-tests.yml"],
};

/** Which kind of mutations job `job` is, by its name. */
export const runnerOf = (job: string): keyof typeof runnerFiles => (/\/ survivors$/.test(job) ? "survivors" : "shard");

/** A failed mutations job, each of its shards' verdicts where it kept them,
 * or none. */
export type FailedJob = { job: string; verdicts?: Record<string, string> };

/** Of each failed mutations job, its lines: each mutation it didn't catch,
 * not `latent`, said changed by `touching(mutation)`'s newest commit, or
 * not by the batch; a job with no verdict, by `touching()`'s, the runner's
 * files'. `said(commit)` is `<sha> (<subject>)`. */
export function mutationLines(failed: FailedJob[], latent: string[], touching: (mutation: string | undefined, job: string) => string[], said: (commit: string) => string): string[] {
  return failed.flatMap(({ job, verdicts }) => {
    const missed = Object.entries(verdicts ?? {}).filter(([name, verdict]) => verdict !== "caught" && !latent.includes(name));
    if (missed.length === 0) {
      const by = touching(undefined, job)[0];
      return [`${job} failed with no mutation's verdict: ${by ? `the runner changed at ${said(by)}` : "its runner unchanged in the batch, so an earlier change's, its machine's or GitHub's"}`];
    }
    return missed.map(([name, verdict]) => {
      const by = touching(name, job)[0];
      const what = verdict.startsWith("SURVIVED") ? "survived" : "was inconclusive";
      return `mutation ${name} ${what}: ${by ? `its site, list or tests changed at ${said(by)}` : "nothing of it changed in the batch, so it's latent, before it"}`;
    });
  });
}

/** The lines of `file` that `find` spans, 1-based, or none. */
export function siteLines(text: string, find: string): [number, number] | undefined {
  const at = text.indexOf(find);
  if (at === -1) return undefined;
  const start = text.slice(0, at).split("\n").length;
  return [start, start + find.split("\n").length - 1];
}

const git = (args: string[]) => Bun.spawnSync(["git", ...args], { cwd: root, stdout: "pipe" }).stdout.toString().trim();

/** The run's jobs, by GitHub's API, each with its steps' names and ends. */
function runJobs(): { name: string; conclusion: string; steps?: { name: string; conclusion: string }[] }[] {
  const run = process.env.GITHUB_RUN_ID;
  if (!run) return [];
  return Bun.spawnSync(["gh", "api", "--paginate", `repos/{owner}/{repo}/actions/runs/${run}/jobs?per_page=100`, "--jq", ".jobs[] | {name, conclusion, steps: [.steps[]? | {name, conclusion}]}"], { cwd: root, stdout: "pipe" })
    .stdout.toString()
    .split("\n")
    .filter(Boolean)
    .map((line) => JSON.parse(line));
}

/** The failed steps of the run's job `job`, by name. */
function failedSteps(job: string): string[] {
  return runJobs()
    .filter(({ name }) => name === job)
    .flatMap(({ steps }) => (steps ?? []).filter(({ conclusion }) => conclusion === "failure").map(({ name }) => name));
}

if (import.meta.main) {
  const [command, ...rest] = process.argv.slice(2);
  mkdirSync(out, { recursive: true });
  if (command === "plan") {
    const [junit, base] = rest;
    const reports = existsSync(junit!) ? readdirSync(junit!, { recursive: true }).map(String).filter((name) => name.endsWith(".xml")) : [];
    // A slice's report is read by what ran it, beside it; the suite's by its files.
    const isSlice = (name: string) => existsSync(join(junit!, `${name}.json`));
    const failed = reports.filter((name) => !isSlice(name)).flatMap((name) => failedIn(readFileSync(join(junit!, name), "utf8")));
    const slices = reports.filter(isSlice).map((name): [SliceReport, string] => [JSON.parse(readFileSync(join(junit!, `${name}.json`), "utf8")), readFileSync(join(junit!, name), "utf8")]);
    const failing: Failing = {
      files: [...new Set(failed.filter(({ file }) => file !== "test/corpus.test.ts" || failed.every((f) => f.file !== file || f.cases.length === 0)).map(({ file }) => file))],
      cases: [...new Set(failed.flatMap(({ file, cases }) => (file === "test/corpus.test.ts" ? cases : [])))],
      runs: sliceReruns(slices),
    };
    writeFileSync(join(out, "failing.json"), JSON.stringify(failing));
    const commits = git(["rev-list", "--reverse", `${base}..HEAD`]).split("\n").filter(Boolean);
    const run = failing.files.length + failing.cases.length + failing.runs.length > 0 && commits.length > 1 ? commitsToRun(commits) : [];
    console.log(`commits=${JSON.stringify(run)}`);
    console.error(`${failing.files.length} files, ${failing.cases.length} cases and ${failing.runs.length} slice runs failed; the batch is ${commits.length} commits, run at ${run.length}`);
    // The nets' failed steps no rerun can say, by the run's jobs' steps.
    const said = (commit: string) => `${commit.slice(0, 9)} (${git(["log", "-1", "--format=%s", commit]).slice(0, 100)})`;
    const touching = (paths: string[]) => git(["log", "-1", "--format=%H", `${base}..HEAD`, "--", ...paths.map((path) => `:(glob)${path}`)]) || undefined;
    const lines = unattributable(failedSteps("nets"), touching, said);
    if (lines.length > 0 && process.env.GITHUB_STEP_SUMMARY) appendFileSync(process.env.GITHUB_STEP_SUMMARY, `### Not attributable by rerun\n\n${lines.map((line) => `- ${line}`).join("\n")}\n`);
    for (const line of lines) console.error(line);
  } else if (command === "report") {
    const [dir] = rest;
    const runs: { commit: string; failing: string[] }[] = readdirSync(dir!, { recursive: true })
      .map(String)
      .filter((name) => name.endsWith("result.json"))
      .map((name) => JSON.parse(readFileSync(join(dir!, name), "utf8")));
    const order = git(["rev-list", "--reverse", "HEAD"]).split("\n");
    runs.sort((a, b) => order.indexOf(a.commit) - order.indexOf(b.commit));
    const failing: Failing = JSON.parse(readFileSync(join(out, "failing.json"), "utf8"));
    const units = [...failing.files, ...failing.cases.map((name) => `test/corpus/${name}`), ...(failing.runs ?? []).map(({ unit }) => unit)];
    const said = runs.length === 0 ? [] : attribution(runs, units);
    const lines = said.map(({ unit, commit, since }) => `${unit} broke at ${commit.slice(0, 9)} (${git(["log", "-1", "--format=%s", commit]).slice(0, 100)})${since === "before" ? ", or before: it fails at the batch's first commit" : since === "passes" ? ": it passes there, so flaky or the check's machine's" : ""}`);
    writeFileSync(join(out, "attribution.json"), JSON.stringify({ head: git(["rev-parse", "HEAD"]), lines }, null, 2));
    if (process.env.GITHUB_STEP_SUMMARY) appendFileSync(process.env.GITHUB_STEP_SUMMARY, `### What broke what\n\n${lines.map((line) => `- ${line}`).join("\n") || "- nothing attributed"}\n`);
    for (const line of lines) console.log(line);
  } else if (command === "mutations") {
    const [dir, base, head = "HEAD"] = rest;
    const found = (name: string) => (existsSync(dir!) ? readdirSync(dir!, { recursive: true }).map(String).filter((file) => file.endsWith(name)) : []);
    const verdictsOf = new Map<string, Record<string, string>>();
    for (const file of found("results.json")) {
      const shard = /mutation-results-(\d+)/.exec(file)?.[1];
      if (shard) verdictsOf.set(shard, (JSON.parse(readFileSync(join(dir!, file), "utf8")) as { results: Record<string, string> }).results);
    }
    const latent = found("latent.json").flatMap((file) => (JSON.parse(readFileSync(join(dir!, file), "utf8")) as { latent: { name: string }[] }).latent.map(({ name }) => name));
    // The run's failed mutations jobs, by GitHub's API.
    const run = process.env.GITHUB_RUN_ID;
    const jobs: { name: string; conclusion: string }[] = run
      ? Bun.spawnSync(["gh", "api", "--paginate", `repos/{owner}/{repo}/actions/runs/${run}/jobs?per_page=100`, "--jq", ".jobs[] | {name, conclusion}"], { cwd: root, stdout: "pipe" })
          .stdout.toString()
          .split("\n")
          .filter(Boolean)
          .map((line) => JSON.parse(line))
      : [];
    const failed: FailedJob[] = jobs
      .filter(({ name, conclusion }) => conclusion === "failure" && /^mutations \//.test(name))
      .map(({ name }) => ({ job: name, verdicts: verdictsOf.get(/\((\d+)\/\d+\)$/.exec(name)?.[1] ?? "") }));
    const byName = new Map<string, Mutation>(lists.flatMap((list) => list.mutations.map((m) => [m.name, m] as const)));
    const listOf = new Map(lists.flatMap((list) => list.mutations.map((m) => [m.name, join("scripts/mutations", list.path)] as const)));
    const touching = (name: string | undefined, job: string): string[] => {
      const range = `${base}..${head}`;
      if (!name) return git(["log", "--format=%H", range, "--", ...runnerFiles[runnerOf(job)]]).split("\n").filter(Boolean);
      const mutation = byName.get(name);
      if (!mutation) return [];
      const tests = (mutation.tests ?? []).filter((arg) => /^test\//.test(arg));
      const plain = git(["log", "--format=%H", range, "--", listOf.get(name)!, ...tests]).split("\n").filter(Boolean);
      const lines = existsSync(join(root, mutation.file)) ? siteLines(readFileSync(join(root, mutation.file), "utf8"), mutation.find) : undefined;
      const site = lines ? git(["log", "--format=%H", "--no-patch", `-L${lines[0]},${lines[1]}:${mutation.file}`, range]).split("\n").filter((line) => /^[0-9a-f]{40}$/.test(line)) : [];
      const order = git(["rev-list", range]).split("\n");
      return [...new Set([...plain, ...site])].sort((a, b) => order.indexOf(a) - order.indexOf(b));
    };
    const said = (commit: string) => `${commit.slice(0, 9)} (${git(["log", "-1", "--format=%s", commit]).slice(0, 100)})`;
    const lines = mutationLines(failed, latent, touching, said);
    writeFileSync(join(out, "attribution-mutations.json"), JSON.stringify({ head: git(["rev-parse", "HEAD"]), lines }, null, 2));
    if (process.env.GITHUB_STEP_SUMMARY) appendFileSync(process.env.GITHUB_STEP_SUMMARY, `### What broke the mutations\n\n${lines.map((line) => `- ${line}`).join("\n") || "- nothing attributed"}\n`);
    for (const line of lines) console.log(line);
  } else {
    console.error("usage: bun scripts/attribute.ts plan <junit dir> <base> | report <results dir> | mutations <artifacts dir> <base>");
    process.exit(2);
  }
}
