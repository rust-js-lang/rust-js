// How long each test the nightly runs takes, kept each night (nightly.yml's
// `test-times`), so the loop's slow spots are found by measurement and a
// test that grows slower is seen the next morning:
//
//   bun scripts/testTimes.ts merge <junit dir> <out dir>   # in CI: the night's times, and its summary against the last night's
//
// Nothing here times anything: the suite's files and tests are what bun's
// own JUnit reporter measured of each run of `bun run test --nightly`
// (scripts/test.ts), and each job's and step's are what GitHub kept of the
// run. The summary is the top 25 files and tests and each job's total, and
// what's slower than the last night's.

import { existsSync, mkdirSync, readdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

/** A test file's time, summed over each run of it (the corpus's parts are
 * runs of one file), and how many tests it ran. */
export type FileTime = { file: string; seconds: number; tests: number };
/** One test's time, by its file and its name. */
export type TestTime = { file: string; name: string; seconds: number };
/** A job's time and each of its steps'. */
export type JobTime = { job: string; seconds: number; steps: { name: string; seconds: number }[] };
/** A night's times. */
export type TestTimes = { commit: string; run: string; files: FileTime[]; tests: TestTime[]; jobs: JobTime[] };

const unescape = (text: string) =>
  text.replace(/&apos;/g, "'").replace(/&quot;/g, '"').replace(/&lt;/g, "<").replace(/&gt;/g, ">").replace(/&amp;/g, "&");
const attribute = (tag: string, name: string) => {
  const found = new RegExp(`\\s${name}="([^"]*)"`).exec(tag);
  return found ? unescape(found[1]!) : undefined;
};

/** The files and tests of one JUnit report, as bun's reporter writes it. */
export function parseJunit(xml: string): { files: FileTime[]; tests: TestTime[] } {
  const files: FileTime[] = [];
  const tests: TestTime[] = [];
  for (const [tag] of xml.matchAll(/<testsuite\s[^>]*>/g)) {
    files.push({ file: attribute(tag, "file") ?? attribute(tag, "name") ?? "", seconds: Number(attribute(tag, "time") ?? 0), tests: Number(attribute(tag, "tests") ?? 0) });
  }
  for (const [tag] of xml.matchAll(/<testcase\s[^>]*>/g)) {
    tests.push({ file: attribute(tag, "file") ?? "", name: attribute(tag, "name") ?? "", seconds: Number(attribute(tag, "time") ?? 0) });
  }
  return { files, tests };
}

/** Each job of a run and its steps, as GitHub's API lists them, the jobs
 * that ran to an end: their times from when each started to when it ended. */
export function jobTimes(jobs: { name: string; started_at: string | null; completed_at: string | null; conclusion: string | null; steps?: { name: string; started_at: string | null; completed_at: string | null }[] }[]): JobTime[] {
  const seconds = (from: string | null, to: string | null) => (from && to ? Math.max(0, (Date.parse(to) - Date.parse(from)) / 1000) : 0);
  return jobs
    .filter((job) => job.completed_at && job.conclusion !== "skipped")
    .map((job) => ({
      job: job.name,
      seconds: seconds(job.started_at, job.completed_at),
      steps: (job.steps ?? []).map((step) => ({ name: step.name, seconds: seconds(step.started_at, step.completed_at) })),
    }));
}

/** The night's times, of its JUnit reports and its jobs: a file run in
 * parts, the corpus, summed; a test by its file and name, the longest of
 * any it ran twice. */
export function merge(reports: string[], jobs: JobTime[], commit: string, run: string): TestTimes {
  const files = new Map<string, FileTime>();
  const tests = new Map<string, TestTime>();
  for (const xml of reports) {
    const parsed = parseJunit(xml);
    for (const f of parsed.files) {
      const was = files.get(f.file);
      files.set(f.file, was ? { file: f.file, seconds: was.seconds + f.seconds, tests: was.tests + f.tests } : f);
    }
    for (const t of parsed.tests) {
      const key = `${t.file}\0${t.name}`;
      if ((tests.get(key)?.seconds ?? -1) < t.seconds) tests.set(key, t);
    }
  }
  const slowest = <T extends { seconds: number }>(list: T[]) => list.sort((a, b) => b.seconds - a.seconds);
  return { commit, run, files: slowest([...files.values()]), tests: slowest([...tests.values()]), jobs: slowest([...jobs]) };
}

/** Whether `now` is slower than `before` enough to say: by more than 30 s,
 * or by more than a quarter of what took 10 s or more. */
export function slower(now: number, before: number): boolean {
  return now - before > 30 || (before >= 10 && now > 1.25 * before);
}

/** What's slower than the last night's: each job, file and test of both. */
export function slowerThan(now: TestTimes, before: TestTimes): { what: string; now: number; before: number }[] {
  const out: { what: string; now: number; before: number }[] = [];
  const compare = <T>(list: T[], old: T[], key: (item: T) => string, seconds: (item: T) => number, label: string) => {
    const was = new Map(old.map((item) => [key(item), seconds(item)]));
    for (const item of list) {
      const b = was.get(key(item));
      if (b !== undefined && slower(seconds(item), b)) out.push({ what: `${label} ${key(item)}`, now: seconds(item), before: b });
    }
  };
  compare(now.jobs, before.jobs, (j) => j.job, (j) => j.seconds, "job");
  compare(now.files, before.files, (f) => f.file, (f) => f.seconds, "file");
  compare(now.tests, before.tests, (t) => `${t.file} > ${t.name}`, (t) => t.seconds, "test");
  return out.sort((a, b) => b.now - b.before - (a.now - a.before));
}

const shown = (seconds: number) => (seconds >= 60 ? `${Math.floor(seconds / 60)} min ${Math.round(seconds % 60)} s` : `${seconds.toFixed(1)} s`);
const cell = (text: string) => text.replace(/\|/g, "\\|").replace(/\n/g, " ");

/** The night's summary: what's slower than the last night's, each job's
 * total, and the top `top` files and tests. */
export function summary(now: TestTimes, before?: TestTimes, top = 25): string {
  const lines = [`## Test times of ${now.commit.slice(0, 9)}`, ""];
  if (before) {
    const worse = slowerThan(now, before);
    lines.push(`### Slower than the last night's (${before.commit.slice(0, 9)}): ${worse.length}`, "");
    if (worse.length > 0) lines.push("| What | Now | Before |", "|---|---:|---:|", ...worse.map((w) => `| ${cell(w.what)} | ${shown(w.now)} | ${shown(w.before)} |`));
    lines.push("");
  } else lines.push("No earlier night's times to compare.", "");
  lines.push("### Each job", "", "| Job | Time |", "|---|---:|", ...now.jobs.map((j) => `| ${cell(j.job)} | ${shown(j.seconds)} |`), "");
  lines.push(`### The ${top} slowest test files`, "", "| File | Tests | Time |", "|---|---:|---:|", ...now.files.slice(0, top).map((f) => `| ${cell(f.file)} | ${f.tests} | ${shown(f.seconds)} |`), "");
  lines.push(`### The ${top} slowest tests`, "", "| File | Test | Time |", "|---|---|---:|", ...now.tests.slice(0, top).map((t) => `| ${cell(t.file)} | ${cell(t.name)} | ${shown(t.seconds)} |`), "");
  return lines.join("\n");
}

function gh(args: string[]): string {
  const p = Bun.spawnSync(["gh", ...args], { stdout: "pipe", stderr: "pipe" });
  if (p.exitCode !== 0) throw new Error(`gh ${args.join(" ")} failed:\n${p.stderr.toString()}`);
  return p.stdout.toString();
}

/** The last completed nightly's times before `run`, `test-times`, if one kept them. */
function lastNight(run: string, dir: string): TestTimes | undefined {
  const runs: { databaseId: number }[] = JSON.parse(gh(["run", "list", "--workflow=nightly.yml", "--status=completed", "--limit=10", "--json=databaseId"]));
  for (const { databaseId } of runs.filter((r) => String(r.databaseId) !== run)) {
    const into = join(dir, String(databaseId));
    try {
      gh(["run", "download", String(databaseId), "--name=test-times", `--dir=${into}`]);
      return JSON.parse(readFileSync(join(into, "test-times.json"), "utf8"));
    } catch {}
  }
  return undefined;
}

if (import.meta.main) {
  const [command, junit, out] = process.argv.slice(2);
  if (command !== "merge" || !junit || !out) {
    console.error("usage: bun scripts/testTimes.ts merge <junit dir> <out dir>");
    process.exit(2);
  }
  const run = process.env.GITHUB_RUN_ID ?? "";
  const reports = existsSync(junit)
    ? readdirSync(junit, { recursive: true, withFileTypes: true }).filter((e) => e.isFile() && e.name.endsWith(".xml")).map((e) => readFileSync(join(e.parentPath, e.name), "utf8"))
    : [];
  const jobs = run ? jobTimes(JSON.parse(gh(["api", `repos/{owner}/{repo}/actions/runs/${run}/jobs?per_page=100`, "--paginate", "--jq", "[.jobs[]]"]).replace(/\]\s*\[/g, ","))) : [];
  const times = merge(reports, jobs, process.env.GITHUB_SHA ?? "", run);
  mkdirSync(out, { recursive: true });
  writeFileSync(join(out, "test-times.json"), JSON.stringify(times));
  let before: TestTimes | undefined;
  try {
    before = run ? lastNight(run, join(out, "before")) : undefined;
  } catch (error) {
    console.error(`the last night's times weren't read: ${error instanceof Error ? error.message : error}`);
  }
  writeFileSync(join(out, "summary.md"), summary(times, before));
  console.log(`${reports.length} reports, ${times.files.length} files, ${times.tests.length} tests, ${times.jobs.length} jobs`);
}
