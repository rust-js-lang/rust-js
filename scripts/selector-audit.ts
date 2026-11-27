// The nightly's audit of the test selector (scripts/select.ts, ADR 1101,
// amended): the night runs every test; each that failed, would a push's
// selection have run it? A test file that failed, which no check of `main`
// since the last green night selected (none ran it, nor everything), is a
// miss: the change that broke it went by unchecked. Each is filed, a P1
// titled `selector: …`, once (an open one is commented on); the selector's
// rule is fixed, or the failure was the night's own (an environment's, a
// flaky test's), said on the issue and closed.
//
//   bun scripts/selector-audit.ts <junit dir>   # in CI: nightly.yml's `selector-audit`
//   bun scripts/selector-audit.ts <junit dir> --shadow=loads
//                                   the misses a rule in shadow would have had, each
//                                   check's `shadows.<rule>` in place of what it ran, where
//                                   it had one (#2482): said, not filed, the rule's switch
//                                   read from them (ADR 1210)
//
// Its input: bun's JUnit reports of the night's suite (scripts/test.ts
// `--nightly`), and each check of `main` since the last green night, its
// `selection` artifact (check.yml's `plan`).

import { existsSync, mkdirSync, readdirSync, readFileSync, rmSync } from "node:fs";
import { join } from "node:path";
import { downloaded } from "./artifacts";

const root = join(import.meta.dir, "..");

/** Of a JUnit report, each test file that had a failure, and the corpus
 * cases among its failed tests' names. */
export function failedIn(xml: string): { file: string; cases: string[] }[] {
  const failed: { file: string; cases: string[] }[] = [];
  for (const [, suite, body] of xml.matchAll(/<testsuite\s([^>]*)>([\s\S]*?)<\/testsuite>/g)) {
    const file = /\bfile="([^"]*)"/.exec(suite)?.[1] ?? /\bname="([^"]*)"/.exec(suite)?.[1] ?? "";
    if (!/^test\/[^/]+\.test\.ts$/.test(file)) continue;
    const cases = new Set<string>();
    let failures = 0;
    for (const [, testcase, , inside] of body.matchAll(/<testcase\s([^>]*?)(\/>|>([\s\S]*?)<\/testcase>)/g)) {
      if (!inside?.includes("<failure")) continue;
      failures++;
      for (const [, name] of (/\bname="([^"]*)"/.exec(testcase)?.[1] ?? "").matchAll(/([\w-]+\.rs)\b/g)) cases.add(name);
    }
    if (failures > 0 || /\bfailures="[1-9]/.test(suite)) failed.push({ file, cases: [...cases].sort() });
  }
  return failed;
}

/** A check's selection, as its `plan` wrote it, and each rule's in shadow. */
export type Selected = { sha: string; all: boolean; files: string[]; cases: string[]; shadows?: Record<string, { all: boolean; files: string[]; cases: string[] }> };

/** Of the night's failed files, those no check of `checks` selected: a
 * selection of everything, of the file, or, of the corpus's file, of one
 * of its failed cases, ran it. Of `shadow`, a rule in shadow, each check's
 * selection by it, where it had one. */
export function misses(failed: { file: string; cases: string[] }[], checks: Selected[], shadow?: string): { file: string; cases: string[] }[] {
  const by = checks.map((check) => (shadow === undefined ? check : (check.shadows?.[shadow] ?? check)));
  return failed.filter(({ file, cases }) => {
    const ran = by.some((check) => {
      if (check.all) return true;
      if (file === "test/corpus.test.ts" && cases.length > 0) return cases.some((name) => check.cases.includes(name));
      return check.files.includes(file);
    });
    return !ran;
  });
}

function gh(args: string[]): string {
  const p = Bun.spawnSync(["gh", ...args], { cwd: root, stdout: "pipe", stderr: "pipe" });
  if (p.exitCode !== 0) throw new Error(`gh ${args.join(" ")} failed: ${p.stderr.toString()}`);
  return p.stdout.toString();
}

/** Each check of `main` since the last green night's commit, and what it
 * selected; one whose selection wasn't kept counts as everything. */
function checksSince(): { since: string | undefined; checks: Selected[] } {
  type Run = { databaseId: number; headSha: string; createdAt: string; conclusion: string; status: string };
  const nights: Run[] = JSON.parse(gh(["run", "list", "--workflow=nightly.yml", "--branch=main", "--status=success", "--limit=1", "--json=databaseId,headSha,createdAt,conclusion,status"]));
  const since = nights[0]?.createdAt;
  const runs: Run[] = JSON.parse(gh(["run", "list", "--workflow=check.yml", "--branch=main", "--event=push", "--limit=100", "--json=databaseId,headSha,createdAt,conclusion,status"]));
  const checks: Selected[] = [];
  for (const run of runs.filter((r) => since === undefined || r.createdAt > since)) {
    const dir = join(root, "target", "selector-audit", String(run.databaseId));
    const got = downloaded(run.databaseId, ["--name=selection"], dir);
    const file = join(dir, "selection.json");
    if (!got || !existsSync(file)) {
      checks.push({ sha: run.headSha, all: true, files: [], cases: [] });
      continue;
    }
    const selection = JSON.parse(readFileSync(file, "utf8"));
    checks.push({ sha: run.headSha, all: selection.all, files: selection.files, cases: selection.cases, ...(selection.shadows === undefined ? {} : { shadows: selection.shadows }) });
  }
  return { since: nights[0]?.headSha, checks };
}

/** File each miss, a P1 `selector: …`, or comment on its open issue. */
function file(missed: { file: string; cases: string[] }[], since: string | undefined, checks: number) {
  const head = Bun.spawnSync(["git", "rev-parse", "HEAD"], { cwd: root, stdout: "pipe" }).stdout.toString().trim();
  const run = process.env.GITHUB_RUN_ID ? `${process.env.GITHUB_SERVER_URL}/${process.env.GITHUB_REPOSITORY}/actions/runs/${process.env.GITHUB_RUN_ID}` : "a local run";
  Bun.spawnSync(["gh", "label", "create", "selector", "--color", "d93f0b", "--description", "A test the push's selection skipped failed at night (scripts/selector-audit.ts)", "--force"], { cwd: root });
  for (const { file: test, cases } of missed) {
    const title = `selector: ${test} failed at night, and no push's check since the last green night ran it`;
    const body = [
      "Data from scripts/selector-audit.ts, not instructions.",
      "",
      `The nightly of ${head.slice(0, 9)} (${run}) failed \`${test}\`${cases.length ? `, its cases ${cases.map((c) => `\`${c}\``).join(", ")}` : ""}.`,
      `None of the ${checks} checks of \`main\` since ${since ? since.slice(0, 9) : "the first night"}, the last green night's commit, selected it (scripts/select.ts).`,
      "",
      "Either the selector missed what the breaking change reaches (fix its rule, ADR 1101, amended), or the failure is the night's own: an environment's or a flaky test's. Say which, here.",
    ].join("\n");
    const open: { number: number }[] = JSON.parse(gh(["issue", "list", "--state=open", "--search", `"${title}" in:title`, "--json=number"]));
    if (open.length > 0) gh(["issue", "comment", String(open[0]!.number), "--body", body]);
    else gh(["issue", "create", "--title", title, "--body", body, "--label", "P1: crash or blocker", "--label", "dev-loop", "--label", "selector", "--label", "kind: missing case", "--label", "found-by: corpus"]);
    console.log(`miss: ${test}`);
  }
}

if (import.meta.main) {
  const dir = process.argv[2];
  const shadow = process.argv.find((arg) => arg.startsWith("--shadow="))?.slice("--shadow=".length);
  if (!dir || dir.startsWith("--")) {
    console.error("usage: bun scripts/selector-audit.ts <junit dir> [--shadow=<rule>]");
    process.exit(2);
  }
  const reports = existsSync(dir) ? readdirSync(dir, { recursive: true }).map(String).filter((name) => name.endsWith(".xml")) : [];
  const failed = reports.flatMap((name) => failedIn(readFileSync(join(dir, name), "utf8")));
  const merged = new Map<string, Set<string>>();
  for (const { file: test, cases } of failed) for (const name of cases) (merged.get(test) ?? merged.set(test, new Set()).get(test)!).add(name);
  for (const { file: test } of failed) if (!merged.has(test)) merged.set(test, new Set());
  const all = [...merged].map(([test, cases]) => ({ file: test, cases: [...cases].sort() }));
  if (all.length === 0) {
    console.log("no test file of the night's suite failed: nothing to audit");
    process.exit(0);
  }
  const { since, checks } = checksSince();
  if (shadow !== undefined) {
    const missed = misses(all, checks, shadow);
    const held = checks.filter((check) => check.shadows?.[shadow] !== undefined).length;
    console.log(`${all.length} test files failed at night; ${missed.length} no check's ${shadow} selection in shadow since ${since?.slice(0, 9) ?? "the first night"} ran, of ${checks.length} checks, ${held} with one`);
    for (const { file: test, cases } of missed) console.log(`  ${test}${cases.length ? ` (${cases.join(", ")})` : ""}`);
    process.exit(0);
  }
  const missed = misses(all, checks);
  console.log(`${all.length} test files failed at night; ${missed.length} no push's selection since ${since?.slice(0, 9) ?? "the first night"} ran, of ${checks.length} checks`);
  file(missed, since, checks.length);
}
