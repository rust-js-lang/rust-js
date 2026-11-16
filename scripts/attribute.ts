// Which commit of a batched check broke each test it failed: a check of
// `main` runs once over the commits since the last completed check, and
// where it fails, each failing test file and corpus case runs again at each
// commit of that batch, on its own machine, and the first commit it fails at
// is said to have broken it (check.yml's `attribute`).
//
//   bun scripts/attribute.ts plan <junit dir> <base>   the failing tests (target/attribute/failing.json) and the commits to run them at, `commits=`
//   (check.yml's `attribute` runs them at each commit by `bun test`, each
//   commit's own harness, and writes which still fail, `result.json`)
//   bun scripts/attribute.ts report <results dir>      attribution.json and the summary's lines, "<test> broke at <sha> (<subject>)"

import { appendFileSync, existsSync, mkdirSync, readdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

import { failedIn } from "./selector-audit";

const root = join(import.meta.dir, "..");
const out = join(root, "target", "attribute");

/** The most commits a batch's failures are run again at, each a machine. */
export const mostCommits = 6;

/** The failing tests' units, `test/x.test.ts` or a corpus case's `test/corpus/x.rs`. */
export type Failing = { files: string[]; cases: string[] };

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

const git = (args: string[]) => Bun.spawnSync(["git", ...args], { cwd: root, stdout: "pipe" }).stdout.toString().trim();

if (import.meta.main) {
  const [command, ...rest] = process.argv.slice(2);
  mkdirSync(out, { recursive: true });
  if (command === "plan") {
    const [junit, base] = rest;
    const reports = existsSync(junit!) ? readdirSync(junit!, { recursive: true }).map(String).filter((name) => name.endsWith(".xml")) : [];
    const failed = reports.flatMap((name) => failedIn(readFileSync(join(junit!, name), "utf8")));
    const failing: Failing = {
      files: [...new Set(failed.filter(({ file }) => file !== "test/corpus.test.ts" || failed.every((f) => f.file !== file || f.cases.length === 0)).map(({ file }) => file))],
      cases: [...new Set(failed.flatMap(({ file, cases }) => (file === "test/corpus.test.ts" ? cases : [])))],
    };
    writeFileSync(join(out, "failing.json"), JSON.stringify(failing));
    const commits = git(["rev-list", "--reverse", `${base}..HEAD`]).split("\n").filter(Boolean);
    const run = failing.files.length + failing.cases.length > 0 && commits.length > 1 ? commitsToRun(commits) : [];
    console.log(`commits=${JSON.stringify(run)}`);
    console.error(`${failing.files.length} files and ${failing.cases.length} cases failed; the batch is ${commits.length} commits, run at ${run.length}`);
  } else if (command === "report") {
    const [dir] = rest;
    const runs: { commit: string; failing: string[] }[] = readdirSync(dir!, { recursive: true })
      .map(String)
      .filter((name) => name.endsWith("result.json"))
      .map((name) => JSON.parse(readFileSync(join(dir!, name), "utf8")));
    const order = git(["rev-list", "--reverse", "HEAD"]).split("\n");
    runs.sort((a, b) => order.indexOf(a.commit) - order.indexOf(b.commit));
    const failing: Failing = JSON.parse(readFileSync(join(out, "failing.json"), "utf8"));
    const units = [...failing.files, ...failing.cases.map((name) => `test/corpus/${name}`)];
    const said = runs.length === 0 ? [] : attribution(runs, units);
    const lines = said.map(({ unit, commit, since }) => `${unit} broke at ${commit.slice(0, 9)} (${git(["log", "-1", "--format=%s", commit]).slice(0, 100)})${since === "before" ? ", or before: it fails at the batch's first commit" : since === "passes" ? ": it passes there, so flaky or the check's machine's" : ""}`);
    writeFileSync(join(out, "attribution.json"), JSON.stringify({ head: git(["rev-parse", "HEAD"]), lines }, null, 2));
    if (process.env.GITHUB_STEP_SUMMARY) appendFileSync(process.env.GITHUB_STEP_SUMMARY, `### What broke what\n\n${lines.map((line) => `- ${line}`).join("\n") || "- nothing attributed"}\n`);
    for (const line of lines) console.log(line);
  } else {
    console.error("usage: bun scripts/attribute.ts plan <junit dir> <base> | run | report <results dir>");
    process.exit(2);
  }
}
