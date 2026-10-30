// What the CI workflow runs where (DEVELOPMENT.md): the suite split into
// shards, the mutations a change touches, and each branch's latest run.

import { expect, test } from "bun:test";
import { cpSync, readdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { checkedSince, latestRuns, MUTATION_SHARDS, mutationJobsPassed, mutationShards, timingsRun } from "../scripts/ci";
import { batches, type Change, changedLines, changedMutations, enclosingFunction, schema, setPasses, site, type Sited, testBlocks, testResults, testSet } from "../scripts/mutations";
import { jobs as localNightly } from "../scripts/nightly";
import { caseParts, caseShardParts, nightly, shard } from "../scripts/shard";
import { fixture, root } from "./support";

const files = ["a", "b", "c", "d", "e", "f", "g"].map((name) => `test/${name}.test.ts`);

test("shards split the files between them, each once, whatever the order given", () => {
  const shards = [1, 2, 3].map((i) => shard(files, i, 3));
  expect(shards.flat().sort()).toEqual(files);
  expect(shards.map((s) => s.length).sort()).toEqual([2, 2, 3]);
  expect(shard([...files].reverse(), 2, 3)).toEqual(shards[1]);
  expect(() => shard(files, 4, 3)).toThrow("a shard is 1 to 3");
});

// The corpus runs in parts, one for each two cores (scripts/test.ts,
// check:fast): each of the parts of all the shards in exactly one.
test("each shard runs its share of a file of cases' parts, each part once", () => {
  expect(caseParts(4)).toBe(2);
  expect(caseParts(12)).toBe(6);
  expect(caseParts(1)).toBe(2);
  expect(caseShardParts(1, 1, 6)).toEqual(["1/6", "2/6", "3/6", "4/6", "5/6", "6/6"]);
  const shards = [1, 2].map((i) => caseShardParts(i, 2, 2));
  expect(shards).toEqual([
    ["1/4", "3/4"],
    ["2/4", "4/4"],
  ]);
  expect(shards.flat().sort()).toEqual(caseShardParts(1, 1, 4));
});

// What only the nightly's suite runs (ADR 1030): each a test file there is,
// and run, by the check the nightly calls, which says `--nightly` to the
// suite and to its bless, where a push's check says neither.
test("the files only the nightly runs are test files, which its check runs", () => {
  const tests = readdirSync(join(root, "test")).map((file) => `test/${file}`);
  for (const file of nightly) expect(tests).toContain(file);
  const check = readFileSync(join(root, ".github/workflows/check.yml"), "utf8");
  const suites = check.split("\n").filter((line) => /bun (run test|scripts\/test\.ts) --shard=/.test(line));
  expect(suites).toHaveLength(2);
  for (const line of suites) expect(line).toContain("${{ inputs.nightly && ' --nightly' || '' }}");
});

// `bun run nightly` runs what the nightly does, here: each of nightly.yml's
// jobs and of the check it calls, check.yml's, by name, but `plan`, which
// picks the mutations a branch's check runs, and `seeds`, the day's number.
test("the nightly, here, has each job the nightly has", () => {
  const ids = (file: string) => {
    const text = readFileSync(join(root, ".github/workflows", file), "utf8");
    return [...text.slice(text.indexOf("\njobs:")).matchAll(/^  ([a-z-]+):$/gm)].map((m) => m[1]);
  };
  const ran = [...ids("check.yml"), ...ids("nightly.yml")].filter((id) => !["plan", "seeds", "check"].includes(id));
  expect(ran.filter((id) => !(id in localNightly))).toEqual([]);
});

const mutation = (name: string, file: string, find = "a", replace = "b", tests = ["test/y.test.ts"]) => ({ name, breaks: "", file, find, replace, tests });

const source = `use std::fmt;

fn first() -> u32 {
    let a = 1;
    a + 1
}

impl Thing {
    pub(super) fn second(&self) -> u32 {
        let b = 2;
        b * 2
    }

    fn short(&self) -> u32 { 3 }
}
`;

const tests = `import { test } from "bun:test";

const shared = 1;

// The first.
test("adds", () => {
  expect(1 + 1).toBe(2);
});

// The second,
// of two lines.
test.skipIf(false)("multiplies", () => {
  expect(2 * 2).toBe(4);
});
`;

// A change's mutations (ADR 0093): those it adds or edits, those in a
// function it changes, and those whose test, or corpus case, it changes, as
// the test is or was: one renamed or moved away is where it was.
test("the changed mutations are of the functions and tests a change touches", () => {
  const files: Record<string, string> = { "src/a.rs": source, "test/x.test.ts": tests };
  const read = (file: string) => files[file];
  const readBase = (file: string) => (file === "test/x.test.ts" ? tests.replace('("multiplies"', '("times"') : files[file]);
  const base = [
    mutation("in-first", "src/a.rs", "a + 1"),
    mutation("in-second", "src/a.rs", "b * 2"),
    mutation("edited", "src/a.rs", "let a = 1;", "let a = 2;"),
    mutation("adds-test", "src/b.rs", "x", "y", ["test/x.test.ts", "-t", "adds"]),
    mutation("multiplies-test", "src/b.rs", "x", "y", ["test/x.test.ts", "-t", "multiplies"]),
    mutation("case", "src/b.rs", "x", "y", ["test/corpus.test.ts", "-t", "array_map"]),
    mutation("times-test", "src/b.rs", "x", "y", ["test/x.test.ts", "-t", "times"]),
    mutation("gone", "src/a.rs", "no longer here"),
  ];
  const now = [...base.slice(0, 2), mutation("edited", "src/a.rs", "let a = 1;", "let a = 3;"), ...base.slice(3), mutation("added", "src/c.rs")];
  const names = (changes: Change[]) => changedMutations(now, base, changes, read, readBase).map((m) => m.name);
  // A line of \`second\`'s: its mutation, the edited and added ones, and one
  // that no longer applies, to say so.
  expect(names([{ file: "src/a.rs", lines: [[10, 10]] }])).toEqual(["in-second", "edited", "gone", "added"]);
  // The second test's comment, then what both tests share.
  expect(names([{ file: "test/x.test.ts", lines: [[11, 11]] }])).toEqual(["edited", "multiplies-test", "added"]);
  expect(names([{ file: "test/x.test.ts", lines: [[3, 3]] }])).toEqual(["edited", "adds-test", "multiplies-test", "times-test", "added"]);
  expect(names([{ file: "test/corpus/array_map.rs", lines: [[1, 1]] }, { file: "docs/x.md", lines: [[1, 1]] }])).toEqual(["edited", "case", "added"]);
  // \`times\` renamed \`multiplies\`: a mutation of either name runs.
  expect(names([{ file: "test/x.test.ts", lines: [[12, 12]] }, { file: "test/x.test.ts", lines: [[12, 12]], base: true }])).toEqual([
    "edited",
    "multiplies-test",
    "times-test",
    "added",
  ]);
  // A corpus case deleted.
  expect(names([{ file: "test/corpus/array_map.rs", lines: [[1, 1]], base: true }])).toEqual(["edited", "case", "added"]);
  expect(changedMutations(now, now, [], read, readBase)).toEqual([]);
});

test("a function is where rustfmt and Prettier lay it out", () => {
  expect(enclosingFunction(source, 5, 5)).toEqual([3, 6]);
  expect(enclosingFunction(source, 11, 11)).toEqual([9, 12]);
  expect(enclosingFunction(source, 14, 14)).toEqual([14, 14]);
  // In none: its lines, and ten each side.
  expect(enclosingFunction(source, 1, 1)).toEqual([1, 11]);
});

// A build of many mutations (ADR 1031): each in a copy of its function's
// body, or of its helper's `include_str!`, chosen by the compiler's file
// name as it runs; one in no body, or in one that can't choose, alone.
const program = `use std::fmt::Display;

fn sum(values: &[u32]) -> u32 {
    let mut total = 0;
    for value in values {
        total += value;
    }
    total
}

fn shown<T: Display>(
    value: T,
    unit: &str,
) -> String {
    format!("{value}{unit}")
}

const fn twice(n: u32) -> u32 {
    n * 2
}

fn main() {
    println!("{} {} {}", sum(&[1, 2, 3]), shown(4, "px"), twice(5));
}
`;
const inProgram = (name: string, find: string, replace: string) => ({ name, breaks: "", file: "src/main.rs", find, replace, tests: [] });
const sumOff = inProgram("sum-off", "total += value;", "total += value + 1;");
const sumFirst = inProgram("sum-first", "    total\n", "    total - values[0]\n");
const shownUnit = inProgram("shown-unit", 'format!("{value}{unit}")', 'format!("{unit}{value}")');
const readProgram = (file: string) => {
  if (file !== "src/main.rs") throw new Error(`no ${file}`);
  return program;
};

test("a mutation goes in a build of many where it's in a function's body that can choose", () => {
  const [a, b, c] = [sumOff, sumFirst, shownUnit].map((mutation) => site(mutation, readProgram));
  expect(a).toEqual(b!);
  expect(a!.file).toBe("src/main.rs");
  expect(program.slice(a!.start, a!.end)).toStartWith("\n    let mut total = 0;");
  expect(program.slice(a!.start, a!.end)).toEndWith("    total\n");
  expect(program.slice(c!.start, c!.end).trim()).toBe('format!("{value}{unit}")');
  // A `const fn` can't ask which it is; a signature is no body.
  expect(site(inProgram("twice-thrice", "n * 2", "n * 3"), readProgram)).toBeUndefined();
  expect(site(inProgram("sum-signature", "fn sum(values: &[u32]) -> u32 {", "fn sum(values: &[u32]) -> u32 {"), readProgram)).toBeUndefined();
  expect(site(inProgram("nowhere", "not in the program", ""), readProgram)).toBeUndefined();
});

test("a build of many has each site once, with all its mutations, and no site inside another", () => {
  const sited: Sited[] = [sumOff, sumFirst, shownUnit].map((mutation) => ({ mutation, site: site(mutation, readProgram) }));
  expect(batches(sited, 32).map((batch) => batch.map((s) => s.mutation.name))).toEqual([["sum-off", "sum-first", "shown-unit"]]);
  expect(batches(sited, 2).map((batch) => batch.map((s) => s.mutation.name))).toEqual([["sum-off", "sum-first"], ["shown-unit"]]);
  const outer: Sited = { mutation: inProgram("outer", "", ""), site: { file: "src/a.rs", start: 0, end: 100 } };
  const inner: Sited = { mutation: inProgram("inner", "", ""), site: { file: "src/a.rs", start: 10, end: 20 } };
  const elsewhere: Sited = { mutation: inProgram("elsewhere", "", ""), site: { file: "src/b.rs", start: 10, end: 20 } };
  expect(batches([outer, inner, elsewhere], 32).map((batch) => batch.map((s) => s.mutation.name))).toEqual([["outer", "elsewhere"], ["inner"]]);
});

test("a build of many runs as each mutation's own build where it's chosen, and as none where none is", () => {
  const sited: Sited[] = [sumOff, sumFirst, shownUnit].map((mutation) => ({ mutation, site: site(mutation, readProgram) }));
  const files = schema(sited, readProgram);
  if (!(files instanceof Map)) throw new Error(files.problem);
  expect([...files.keys()]).toEqual(["src/main.rs"]);
  const dir = fixture("mutation-schema");
  writeFileSync(join(dir, "main.rs"), files.get("src/main.rs")!);
  const built = Bun.spawnSync(["rustc", "--edition=2021", "-Awarnings", join(dir, "main.rs"), "-o", join(dir, "built")], { cwd: root });
  expect(built.stderr.toString()).toBe("");
  const runAs = (name: string, env: Record<string, string> = {}) => {
    cpSync(join(dir, "built"), join(dir, name));
    return Bun.spawnSync([join(dir, name)], { env: { ...process.env, ...env } }).stdout.toString().trim();
  };
  expect(runAs("unmutated")).toBe("6 4px 10");
  expect(runAs("sum-off")).toBe("9 4px 10");
  expect(runAs("sum-first")).toBe("5 4px 10");
  expect(runAs("shown-unit")).toBe("6 px4 10");
  // A test's copy of its own, under another name, is chosen by what it's told.
  expect(runAs("copy", { RUST_JS_MUTANT: "sum-off" })).toBe("9 4px 10");
  // One mutation is a build of its own, as it was.
  const alone = schema([{ mutation: sumOff }], readProgram);
  expect(alone instanceof Map && alone.get("src/main.rs")).toBe(program.replace("total += value;", "total += value + 1;"));
});

// The controls (ADR 1031): each set a mutation names answered from one run
// of each of its files, every test's result by name in its JUnit report.
test("a mutation's set of tests is its files and its pattern", () => {
  expect(testSet(["test/corpus.test.ts", "-t", "^a.rs$"])).toEqual({ files: ["test/corpus.test.ts"], pattern: "^a.rs$" });
  expect(testSet(["test/a.test.ts", "test/b.test.ts"])).toEqual({ files: ["test/a.test.ts", "test/b.test.ts"], pattern: undefined });
});

test("a set passes where each test it names passed, runs nothing where none ran, and fails otherwise", () => {
  const junit = `<testsuites><testsuite name="test/a.test.ts">
    <testcase name="adds &amp; keeps" classname="" time="0" />
    <testcase name="drops" classname="lists" time="0"><failure message="x" /></testcase>
    <testcase name="skipped one" classname="" time="0"><skipped /></testcase>
  </testsuite></testsuites>`;
  const results = testResults(junit);
  expect([...results]).toEqual([["adds & keeps", true], ["lists drops", false]]);
  const ran = new Map([["test/a.test.ts", results]]);
  expect(setPasses({ files: ["test/a.test.ts"], pattern: "adds" }, ran)).toBe(true);
  expect(setPasses({ files: ["test/a.test.ts"], pattern: "drops" }, ran)).toBe(false);
  expect(setPasses({ files: ["test/a.test.ts"] }, ran)).toBe(false);
  expect(setPasses({ files: ["test/a.test.ts"], pattern: "skipped" }, ran)).toBeUndefined();
  expect(setPasses({ files: ["test/b.test.ts"] }, ran)).toBeUndefined();
});

test("a test file's tests are their comments and bodies", () => {
  expect(testBlocks(tests)).toEqual([
    { name: "adds", first: 5, last: 9 },
    { name: "multiplies", first: 10, last: 15 },
  ]);
});

test("a diff's changed lines are where the file is changed now, and was", () => {
  const diff = [
    "diff --git a/src/a.rs b/src/a.rs",
    "--- a/src/a.rs",
    "+++ b/src/a.rs",
    "@@ -3,0 +4,2 @@ fn first() {",
    "+one",
    "+two",
    "@@ -9 +11 @@",
    "-old",
    "+new",
    "@@ -20,2 +21,0 @@",
    "diff --git a/gone.rs b/gone.rs",
    "--- a/gone.rs",
    "+++ /dev/null",
    "@@ -1 +0,0 @@",
  ].join("\n");
  expect(changedLines(diff)).toEqual([
    { file: "src/a.rs", lines: [[3, 4], [9, 9], [20, 21]], base: true },
    { file: "src/a.rs", lines: [[4, 5], [11, 11], [21, 22]] },
    { file: "gone.rs", lines: [[1, 1]], base: true },
  ]);
});

// A check's mutations on as many machines as keeps each near twenty, about
// as long as the suite takes on its two: MIR's run suites of seconds each.
test("a check's mutations are split to about twenty a machine", () => {
  expect([0, 20, 21, 40, 41, 80, 81, 160, 161, 400].map(mutationShards)).toEqual(["1", "1", "2", "2", "4", "4", "8", "8", "16", "16"]);
});

const run = (headBranch: string, databaseId: number, status: string, conclusion: string, headSha = `sha${databaseId}`) =>
  ({ headBranch, databaseId, status, conclusion, createdAt: new Date(databaseId * 1000).toISOString(), displayTitle: headBranch, headSha });

test("each branch's latest run, newest first", () => {
  const runs = [run("a", 1, "completed", "failure"), run("b", 2, "completed", "success"), run("a", 3, "in_progress", "")];
  expect(latestRuns(runs).map((r) => [r.headBranch, r.databaseId])).toEqual([["a", 3], ["b", 2]]);
});

// A check of `main`, which changes are pushed to, runs the mutations of
// what changed since the commit of its last check whose mutations passed:
// one that was cancelled, is running, or whose mutations failed or never
// ran checked none of them, so its changes stay in the next run's.
test("main's mutations are of what changed since its last check whose mutations passed", () => {
  const runs = [run("main", 1, "completed", "success"), run("main", 2, "completed", "failure"), run("main", 3, "completed", "cancelled"), run("main", 4, "in_progress", ""), run("b", 5, "completed", "success")];
  // A failed run is no baseline unless its every mutation job passed.
  expect(checkedSince(runs, "main")).toBe("sha1");
  expect(checkedSince(runs, "main", (r) => r.databaseId === 2)).toBe("sha2");
  expect(checkedSince(runs, "main", (r) => r.databaseId === 3)).toBe("sha1");
  expect(checkedSince(runs, "c")).toBeUndefined();
  const job = (name: string, conclusion: string) => ({ name, conclusion });
  expect(mutationJobsPassed([job("lint", "failure"), job("mutations / mutations (1/2)", "success"), job("mutations / mutations (2/2)", "success")])).toBe(true);
  expect(mutationJobsPassed([job("mutations / mutations (1/2)", "success"), job("mutations / mutations (2/2)", "failure")])).toBe(false);
  expect(mutationJobsPassed([job("mutations / mutations (1/2)", "skipped")])).toBe(false);
  expect(mutationJobsPassed([job("lint", "success")])).toBe(false);
});

// The shards balance by the newest run's timings that are still kept, of
// any branch; none, and they split by file count.
test("the shards balance by the newest kept timings", () => {
  const artifact = (name: string, run: number, createdAt: string, expired = false) => ({ name, run, createdAt, expired });
  const artifacts = [
    artifact("results-1", 5, "2026-10-21T10:00:00Z"),
    artifact("timings-1", 3, "2026-10-21T08:00:00Z"),
    artifact("timings-2", 3, "2026-10-21T08:01:00Z"),
    artifact("timings-1", 4, "2026-10-21T09:00:00Z", true),
    artifact("timings-1", 2, "2026-10-21T07:00:00Z"),
    artifact("timings-extra", 6, "2026-10-21T11:00:00Z"),
  ];
  expect(timingsRun(artifacts)).toBe(3);
  expect(timingsRun(artifacts.filter((a) => a.run !== 3))).toBe(2);
  expect(timingsRun([artifact("bless-test-1", 7, "2026-10-21T12:00:00Z")])).toBeUndefined();
});

// One list of how many machines mutations split across: check.yml's choices
// and rustc-tests.yml's matrix are scripts/ci.ts's, each count its shards.
test("the workflows' mutation shards are ci.ts's", () => {
  const check = readFileSync(join(root, ".github/workflows/check.yml"), "utf8");
  const options = check.match(/mutation_shards:\n\s+description: .*\n\s+type: choice\n\s+options: \[(.*)\]/)?.[1];
  expect(options?.split(", ").map((o) => Number(o.slice(1, -1)))).toEqual(MUTATION_SHARDS);
  const tests = readFileSync(join(root, ".github/workflows/rustc-tests.yml"), "utf8");
  const matrix = [...tests.matchAll(/inputs\.mutation_shards == '(\d+)' && '\[([\d, ]+)\]'/g)].map((m) => [Number(m[1]), m[2]]);
  expect(matrix.map(([n]) => n).sort((a, b) => Number(a) - Number(b))).toEqual(MUTATION_SHARDS.filter((n) => n > 1));
  for (const [n, list] of matrix) expect(list).toBe(Array.from({ length: Number(n) }, (_, i) => i + 1).join(", "));
});

// A check runs to its end: one started after it, by a push or by hand,
// waits, and no other workflow shares its group to cancel it. When a run
// started by hand cancelled the one running, three agents pushing to
// `main` left no check of it finished for hours.
test("a check is never cancelled by another run", () => {
  const dir = join(root, ".github/workflows");
  type Workflow = { concurrency?: { group?: string; "cancel-in-progress"?: unknown } };
  const concurrency = (file: string) => (Bun.YAML.parse(readFileSync(join(dir, file), "utf8")) as Workflow).concurrency ?? {};
  const check = concurrency("check.yml");
  expect(check["cancel-in-progress"]).toBe(false);
  // Started by hand, rustc-tests.yml's group is its own, never the check's `ci-<ref>`.
  expect(concurrency("rustc-tests.yml").group).toContain("format('rustc-{0}', github.ref)");
  // Called by a check, its group, which cancels, is of the caller's branch:
  // one branch's check never cancels another's rustc tests or mutations.
  const called = concurrency("rustc-tests.yml").group ?? "";
  expect(called.slice(called.indexOf("||"))).toContain("github.ref");
  for (const file of readdirSync(dir).filter((f) => f.endsWith(".yml") && f !== "check.yml")) {
    const group = concurrency(file).group ?? "";
    expect(group, file).not.toContain("== 'Check' && format('ci-{0}', github.ref)");
    expect(group.replace(/'ci-\{0\}-/g, ""), file).not.toMatch(/format\('ci-\{0\}', github\.ref\)/);
  }
});

// A workflow GitHub can't read runs nothing, and says so only where it's
// started: each is YAML, its quotes closed.
test("every workflow is YAML", () => {
  const dir = join(root, ".github/workflows");
  for (const file of readdirSync(dir).filter((f) => f.endsWith(".yml"))) {
    expect(() => Bun.YAML.parse(readFileSync(join(dir, file), "utf8")), file).not.toThrow();
  }
});

