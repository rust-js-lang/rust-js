// How CI's and the machine's work is dealt and sized (scripts/shard.ts,
// DEVELOPMENT.md): the suite's files and the mutations split into shards
// by what each took, how many machines a check's mutations take, and how
// many jobs of each kind run at once on a machine's cores and memory.

import { expect, test } from "bun:test";
import { mkdirSync, mkdtempSync, readdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { gunzipSync } from "node:zlib";
import { join } from "node:path";
import { decodedTestTimings, decodedTimings, MERGED_MUTATION_TIMES, encodedTestTimings, encodedTimings, mergedTimings, mergedTimingsRun, MUTATION_SHARDS, mutationShards, mutationShardPlan, heldJobs, freeJobs, normalizedShardTimes, shardSummary, CONCURRENT_JOBS, SHARD_BUDGET_SECONDS, TEST_SHARD_SETUP_SECONDS, TEST_SHARDS, shardList, testShards, testShardSeconds, timingsRuns, controlPartsOutput } from "../scripts/ci";
import { controlRuns, mutations, testsAsItIsRuns, testsAsItIsSeconds } from "../scripts/mutations";
import { jobs } from "../scripts/checkFast";
import { runs } from "../scripts/iteratorSlice";
import { balancedByGroup, browsers, casePartCores, dealtByGroup, fewestShards, shardFacts, shareOfShard, slots, testFileSeconds, testShardFiles, timingsByShard, wholeCorpusCaseSeconds, wholeCorpusParts, controlParts } from "../scripts/shard";
import { corpusCaseNames, testTimeout } from "../scripts/mutations";
import { atOnce, caseFiles, caseParts, casePoolJobs, casePoolParts, casePoolWidth, caseShardParts, compilerBuilds, compilerBuildsAsked, memoryBudget, nightly, parts, peakGB, pooledParts, shard, timedShard, balanced } from "../scripts/shard";
import { fixture, root } from "./support";

const files = ["a", "b", "c", "d", "e", "f", "g"].map((name) => `test/${name}.test.ts`);

test("shards split the files between them, each once, whatever the order given", () => {
  const shards = [1, 2, 3].map((i) => shard(files, i, 3));
  expect(shards.flat().sort()).toEqual(files);
  expect(shards.map((s) => s.length).sort()).toEqual([2, 2, 3]);
  expect(shard([...files].reverse(), 2, 3)).toEqual(shards[1]);
  expect(() => shard(files, 4, 3)).toThrow("a shard is 1 to 3");
});

// The mutations' shards by what each took (`balanced`): the longest first,
// each to the shard with the least so far, so a few long ones don't make
// one shard's day, as dealt in turn they made the nightly's longest shard
// 411 min of work against its shortest's 230 (docs/dev-loop.md). Each once,
// whatever the order given; one it doesn't know takes the median.
test("mutations shard by their times, each once, the shards near even", () => {
  const names = ["a", "b", "c", "d", "e", "f", "g", "h"];
  const seconds = new Map([["a", 100], ["b", 90], ["c", 10], ["d", 10], ["e", 10], ["f", 10], ["g", 10]]);
  const shards = [1, 2].map((i) => balanced(names, i, 2, seconds));
  expect(shards.flat().sort()).toEqual(names);
  expect(shards).toEqual([["a", "d", "f", "h"], ["b", "c", "e", "g"]]);
  const time = (shard: string[]) => shard.reduce((sum, name) => sum + (seconds.get(name) ?? 10), 0);
  expect(shards.map(time)).toEqual([130, 120]);
  expect(balanced([...names].reverse(), 2, 2, seconds)).toEqual(shards[1]);
  // Two long ones a name apart, which turns deal to one shard.
  const long = new Map([["a", 100], ["b", 1], ["c", 100], ["d", 1]]);
  expect(shard(["a", "b", "c", "d"], 1, 2)).toEqual(["a", "c"]);
  expect([1, 2].map((i) => balanced(["a", "b", "c", "d"], i, 2, long))).toEqual([["a", "b"], ["c", "d"]]);
  // Those no run timed take the usual time, not none, so they're spread.
  expect([1, 2].map((i) => balanced(["p", "q", "r"], i, 2, new Map([["p", 10]])))).toEqual([["p", "r"], ["q"]]);
  expect(() => balanced(names, 3, 2, seconds)).toThrow("a shard is 1 to 2");
});

// `--timings=<dir>`: each run's shards' `timings.json` under it, as `ci.ts
// mutation-timings` downloads them, its build's share and its tests; none
// given, dealt in turn, as before.
test("a mutation shard reads its times of the runs' timings.json under the directory given", () => {
  const dir = fixture("mutation-timings");
  mkdirSync(join(dir, "mutation-timings-1"), { recursive: true });
  mkdirSync(join(dir, "mutation-timings-2"), { recursive: true });
  writeFileSync(join(dir, "mutation-timings-1", "timings.json"), JSON.stringify({ a: { build: 50, tests: 50 }, b: { build: 1, tests: 1 } }));
  writeFileSync(join(dir, "mutation-timings-2", "timings.json"), JSON.stringify({ c: { build: 60, tests: 40 } }));
  const names = ["a", "b", "c", "d"];
  expect([1, 2].map((i) => timedShard(names, i, 2, dir))).toEqual([["a", "d"], ["b", "c"]]);
  expect([1, 2].map((i) => timedShard(names, i, 2))).toEqual([1, 2].map((i) => shard(names, i, 2)));
  expect(timedShard(names, 1, 1, join(dir, "none"))).toEqual(names);
});

// Every shard of a check balances by the same times, `plan`'s: the merged
// times of the newest run of main that kept them, which carry the
// nightly's and each check's since, a later time over an earlier. Handed
// to the shards as the plan's output, so none reads them apart: one that
// couldn't would deal otherwise than the others, a mutation run twice or
// never; none kept, they're dealt in turn.
test("a check's mutation shards balance by the newest merged times of main, handed alike to each", () => {
  const artifact = (name: string, run: number, createdAt: string, branch = "main", expired = false) => ({ name, run, createdAt, branch, expired });
  const artifacts = [
    artifact(MERGED_MUTATION_TIMES, 9, "2026-11-17T03:00:00Z", "main", true),
    artifact(MERGED_MUTATION_TIMES, 8, "2026-11-17T02:00:00Z", "other"),
    artifact("mutation-timings-1", 7, "2026-11-17T01:30:00Z"),
    artifact(MERGED_MUTATION_TIMES, 5, "2026-11-17T01:00:00Z"),
    artifact(MERGED_MUTATION_TIMES, 3, "2026-11-16T01:00:00Z"),
  ];
  expect(mergedTimingsRun(artifacts)).toBe(5);
  expect(mergedTimingsRun(artifacts.slice(0, 3))).toBeUndefined();
  // Never the old name's, whose times were each mutation's from its tests'
  // asking for a slot; and the workflow uploads by the name the merge
  // gives, which is `MERGED_MUTATION_TIMES`, never one of its own.
  expect(mergedTimingsRun([artifact("mutation-timings-merged", 10, "2026-11-18T01:00:00Z")])).toBeUndefined();
  const workflows = readdirSync(join(root, ".github/workflows")).map((file) => readFileSync(join(root, ".github/workflows", file), "utf8"));
  expect(workflows.filter((text) => /mutation-(timings|shares)-merged/.test(text))).toEqual([]);
  expect(workflows.filter((text) => text.includes("name: ${{ steps.merge.outputs.artifact }}"))).toHaveLength(1);
  // Each of the checkout's mutations in turn, a tenth of a second each: a
  // catalog of another count refused, never read askew.
  const names = ["b", "a", "c", "d"];
  const times = new Map([["a", 12.34], ["c", 0.05], ["gone", 7]]);
  expect([...decodedTimings(encodedTimings(times, names), names)]).toEqual([["a", 12.3], ["c", 0.1]]);
  expect(() => decodedTimings(encodedTimings(times, names), [...names, "e"])).toThrow("a catalog of 4, not this checkout's 5");
  const nightly = Array.from({ length: 14000 }, (_, i) => `mutation-of-some-rule-${i}`);
  expect(encodedTimings(new Map(nightly.map((name, i) => [name, (i % 600) / 10])), nightly).length).toBeLessThan(64 * 1024);
  // This run's own times over its plan's: the next plan's.
  expect([...mergedTimings(new Map([["a", 1], ["b", 2]]), new Map([["b", 3], ["c", 4]]))]).toEqual([["a", 1], ["b", 3], ["c", 4]]);
  const check = readFileSync(join(root, ".github/workflows/check.yml"), "utf8");
  expect(check).toContain("mutation_timings: ${{ needs.plan.outputs.mutation_timings }}");
  const tests = readFileSync(join(root, ".github/workflows/rustc-tests.yml"), "utf8");
  const runs = [...tests.matchAll(/bun scripts\/mutations\.ts \$ARGS (.*)--shard=/g)].map((m) => m[1]);
  // The run's shared control, of a `--corpus` dispatch, deals nothing: each
  // shard is dealt by the same times as the browser's ask (#2375).
  const control = "${{ contains(inputs.mutation_args, '--corpus') && '--control=target/mutants/shared' || '' }} ";
  expect(runs).toEqual(["${{ steps.timings.outputs.args }} ", `\${{ steps.timings.outputs.args }} ${control}`]);
  // A run of every mutation, the nightly's, builds 5% of its builds again
  // fresh (ADR 0093's incremental amendment).
  expect(tests).toContain("ARGS: ${{ inputs.mutation_args || '--fresh=0.05' }}");
});

// The test files' times, too, `plan`'s to read once and hand every test
// shard alike: a shard that downloaded them itself failed where GitHub's
// storage turned it away (check of 7f324c2ec), and none downloads them.
test("a test shard is handed its files' times by plan, never downloading them", () => {
  const dir = fixture("test-timings-handed");
  const seconds = new Map([["test/b.test.ts", 2.5], ["test/a.test.ts", 0.0004]]);
  writeFileSync(join(dir, "plan.json"), gunzipSync(Buffer.from(encodedTestTimings(seconds), "base64")));
  expect([...testFileSeconds([join(dir, "plan.json")])].sort()).toEqual([["test/a.test.ts", 0], ["test/b.test.ts", 2.5]]);
  const files = new Map(Array.from({ length: 400 }, (_, i) => [`test/some-concept-of-the-suite-${i}.test.ts`, i * 1.234] as const));
  expect(encodedTestTimings(files).length).toBeLessThan(16 * 1024);
  const jobs = (Bun.YAML.parse(readFileSync(join(root, ".github/workflows/check.yml"), "utf8")) as { jobs: Record<string, { outputs?: Record<string, string>; steps: { run?: string; env?: Record<string, string> }[] }> }).jobs;
  expect(jobs.plan.outputs?.test_timings).toBe("${{ steps.plan.outputs.test_timings }}");
  const handed = jobs.test.steps.find((step) => step.run?.includes("bun scripts/ci.ts test-timings"));
  expect(handed?.env?.TIMES).toBe("${{ needs.plan.outputs.test_timings }}");
  expect(jobs.test.steps.filter((step) => /\bci\.ts timings\b|gh run download/.test(step.run ?? ""))).toEqual([]);
});

// The suite's shards are as many as its selection's expected time needs,
// by the mutations' one rule (`fewestShards`): each shard's setup, then the
// longer of its rest's files, each its share of the wall its run measured,
// and its pool of the files of cases, then its browsers' files
// (`testShardSeconds`). Check 44480685461 (896bcafd0) of 394 files and
// 2,799 cases, planned 3 by its files' raw recorded times alone, ran 516
// to 745 s, the pool each shard's longest leg; its own measures, kept as
// test/fixtures/test-timings-44480685461/, expect 722 s a shard of three
// at their mean, 835 at their slowest pace, past the budget, and plan four.
// Check 44493804830 (d4c76fa8b), planned 2 at the mean, 678 s each, took
// 1,102 s on a slow runner and 688 on a fast one; at its slowest pace, 1.25
// of its own measures, two are expected at 1,032 s, and it plans four.
test("a check's suite takes the machines its selection's expected time needs", () => {
  expect(fewestShards([1, 2, 4], (count) => [100 / count + 10], 40)).toBe(4);
  expect(fewestShards([1, 2, 4], (count) => [1000 / count], 40)).toBe(4);
  expect(fewestShards([1, 2, 4], () => [5], 40)).toBe(1);
  // Every shard within it, not any one.
  expect(fewestShards([1, 2, 4], (count) => (count === 1 ? [50] : count === 2 ? [10, 50] : [10, 10, 10, 10]), 40)).toBe(4);
  // None within it: the fewest whose slowest ends soonest, where more
  // machines end it no sooner, a shard's part no split shortens (#2220).
  expect(fewestShards([1, 2, 4], (count) => (count === 1 ? [100] : Array.from({ length: count }, (_, i) => (i === 0 ? 60 : 5))), 40)).toBe(2);
  const paths = [1, 2, 3].map((k) => join(root, `test/fixtures/test-timings-44480685461/${k}.json`));
  const seconds = testFileSeconds(paths, true);
  const facts = shardFacts(paths)!;
  expect(seconds.size).toBe(384);
  // Each file its share of its shard's rest: the shares sum to the walls.
  expect([...seconds.values()].reduce((a, b) => a + b, 0)).toBeCloseTo(396 + 300.8 + 328.8, 0);
  expect(facts.poolWallPerSecond).toBeCloseTo((666.6 + 468.1 + 595.4) / (1203 + 518 + 951 + 221 + 1245 + 303), 3);
  expect(facts.metamorphic).toBeCloseTo(518 + 221 + 303, 0);
  expect(facts.browserSeconds).toBeCloseTo((78.8 + 48.4 + 9.4) / 9, 3);
  const files = [...seconds.keys()];
  const selection = { all: false, files: [...files, ...browsers, ...Object.values(caseFiles)], cases: Array.from({ length: 2799 }, (_, i) => `c${i}.rs`), forms: [], why: "" };
  const pool = { cases: 2799, metamorphic: true, pages: browsers.length };
  const expected = (count: number) => dealtByGroup(files, count, seconds, () => "suite").load.map((load) => testShardSeconds(load, count, facts, pool));
  // Three, as the check ran: about its runs' mean, its setup beside them,
  // at its slowest shard's pace, the pool's wall over what the mean expects.
  expect(facts.pace).toBeCloseTo(1.156, 3);
  expect(Math.max(...expected(3))).toBeGreaterThan(SHARD_BUDGET_SECONDS);
  expect(Math.max(...expected(3))).toBeCloseTo(facts.pace * (TEST_SHARD_SETUP_SECONDS + (745 + 516 + 605) / 3), -2);
  expect(Math.max(...expected(4))).toBeLessThanOrEqual(SHARD_BUDGET_SECONDS);
  expect(testShards(selection, { seconds, facts })).toBe("4");
  // The pool is the longer leg there: without it, three would do; and at
  // the mean, as if every runner were as fast, two.
  expect(testShards(selection, { seconds, facts: { ...facts, poolWallPerSecond: 0 } })).toBe("3");
  expect(testShards(selection, { seconds, facts: { ...facts, poolWallPerSecond: 0, pace: 1 } })).toBe("2");
  // Check 44493804830's, its slow shard expected about as it ran.
  const slowPaths = [1, 2].map((k) => join(root, `test/fixtures/test-timings-44493804830/${k}.json`));
  const slow = { seconds: testFileSeconds(slowPaths, true), facts: shardFacts(slowPaths)! };
  const slowFiles = [...slow.seconds.keys()];
  const slowExpected = (count: number) =>
    dealtByGroup(slowFiles, count, slow.seconds, () => "suite").load.map((load) => testShardSeconds(load, count, slow.facts, { cases: 2801, metamorphic: true, pages: browsers.length }));
  expect(slow.facts.pace).toBeCloseTo(1.247, 3);
  // Within a tenth of the 1,102 s its slow shard took.
  expect(Math.abs(Math.max(...slowExpected(2)) - 1102) / 1102).toBeLessThan(0.1);
  expect(Math.max(...slowExpected(3))).toBeGreaterThan(SHARD_BUDGET_SECONDS);
  const slowSelection = { ...selection, files: [...slowFiles, ...browsers, ...Object.values(caseFiles)], cases: Array.from({ length: 2801 }, (_, i) => `c${i}.rs`) };
  expect(testShards(slowSelection, slow)).toBe("4");
  // Of everything, every suite file counts, those the last run didn't
  // time, a selection's that ran few, by the timed ones' median, never the
  // timed alone: of 100 files, 10 timed at a minute each, six machines.
  const all = { all: true, files: [], cases: [], forms: [], why: "" };
  const none = { secondsPerCase: 0, metamorphic: 0, poolWallPerSecond: 0, browserSeconds: 0, pace: 1 };
  const suite = Array.from({ length: 100 }, (_, i) => `test/f${i}.test.ts`);
  const few = new Map(suite.slice(0, 10).map((file) => [file, 60] as const));
  expect([testShards(all, { seconds: few, facts: none }, undefined, () => suite, () => 0), testShards(all, { seconds: few, facts: none }, undefined, () => [...few.keys()], () => 0)]).toEqual(["6", "1"]);
  // Of everything, the corpus's every case.
  expect(testShards(all, { seconds: few, facts: { ...none, secondsPerCase: 1, poolWallPerSecond: 1 } }, undefined, () => [...few.keys()], () => 3000)).toBe("5");
  // As many as a check started by hand or the nightly says, given.
  expect(testShards(all, { seconds, facts }, "2")).toBe("2");
  // A small selection, one machine; with no run's measures kept, by count.
  const some = (files: number, cases: number) => ({ all: false, files: Array.from({ length: files }, (_, i) => `test/f${i}.test.ts`), cases: Array.from({ length: cases }, (_, i) => `c${i}.rs`), forms: [], why: "" });
  expect(testShards({ ...some(0, 0), files: ["test/compat.test.ts", "test/adr.test.ts"] }, { seconds, facts })).toBe("1");
  expect([testShards(some(5, 30)), testShards(some(60, 400)), testShards(some(61, 0)), testShards(some(0, 401)), testShards(all)]).toEqual(["1", "1", "2", "2", "2"]);
  // An earlier run's raw times, no shard's measures kept, aren't `plan`'s.
  const legacy = join(root, "test/fixtures/test-timings-44459055794.json");
  expect(testFileSeconds([legacy], true).size).toBe(0);
  expect(shardFacts([legacy])).toBeUndefined();
  // Handed on as the times `plan` encodes, and the matrix its list.
  expect([...decodedTestTimings(encodedTestTimings(new Map([["test/a.test.ts", 1.5]])))]).toEqual([["test/a.test.ts", 1.5]]);
  expect(decodedTestTimings("").size).toBe(0);
  expect(shardList("4")).toBe("[1,2,3,4]");
  const jobs = (Bun.YAML.parse(readFileSync(join(root, ".github/workflows/check.yml"), "utf8")) as { jobs: Record<string, { outputs?: Record<string, string>; strategy?: { matrix?: { shard?: string } }; steps: { run?: string }[] }> }).jobs;
  expect(jobs.plan.outputs?.test_shards).toBe("${{ steps.plan.outputs.test_shards }}");
  expect(jobs.plan.outputs?.test_shard_list).toBe("${{ steps.plan.outputs.test_shard_list }}");
  expect(jobs.test.strategy?.matrix?.shard).toBe("${{ fromJSON(needs.plan.outputs.test_shard_list) }}");
  expect(jobs.plan.steps.find((step) => step.run?.includes("bun scripts/ci.ts plan"))?.run).toContain("--test-shards=");
  expect(JSON.stringify(jobs.test)).not.toContain("inputs.test_shards");
});

// What a shard's run measures is kept beside its files' times, so the
// next `plan` reads each file's share and the pool's: the rest's wall and
// its files' recorded seconds, each part of the pool's, the browsers'.
test("a test shard keeps what its run measured beside its files' times", () => {
  const suite = readFileSync(join(root, "scripts/test.ts"), "utf8");
  expect(suite).toContain("JSON.stringify({ ...kept, shard: measured }");
  expect(suite).toContain("measured.rest = wall(began);");
  expect(suite).toContain("kind.seconds += wall(started);");
  expect(suite).toContain("measured.browsers = {");
  const dir = mkdtempSync(join(tmpdir(), "test-shares-"));
  try {
    const at = (name: string, timings: object) => {
      writeFileSync(join(dir, name), JSON.stringify(timings));
      return join(dir, name);
    };
    const shard = { count: 2, rest: 50, recorded: 200, pool: { wall: 90, corpus: { seconds: 100, cases: 50 }, metamorphic: { seconds: 40 } }, browsers: { wall: 30, files: 3 } };
    const measured = at("measured.json", { version: 1, files: { "test/a.test.ts": 120_000, "test/b.test.ts": 80_000 }, shard });
    const raw = at("raw.json", { version: 1, files: { "test/a.test.ts": 9000, "test/c.test.ts": 4000 } });
    // A run's measured share first; a raw run's as it is, unless measured
    // only, `plan`'s.
    expect([...testFileSeconds([measured, raw])]).toEqual([["test/a.test.ts", 30], ["test/b.test.ts", 20], ["test/c.test.ts", 4]]);
    expect([...testFileSeconds([measured, raw], true)]).toEqual([["test/a.test.ts", 30], ["test/b.test.ts", 20]]);
    expect(shardFacts([measured, raw])).toEqual({ secondsPerCase: 2, metamorphic: 80, poolWallPerSecond: 90 / 140, browserSeconds: 10, pace: 1 });
    // Of two runs of the same parts, one's pool twice the other's wall: the
    // slower's pace, its wall over what their mean expects of it, 180 / 135.
    const slower = at("slower.json", { version: 1, files: {}, shard: { ...shard, pool: { ...shard.pool, wall: 180 } } });
    expect(shardFacts([measured, slower])?.pace).toBeCloseTo(4 / 3, 6);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

// The builds of the compiler a run makes at once, mutations' `--jobs`, are
// as many as the cores and the memory hold: on 8 cores and 16 GB, four at
// once with a test on each core set off Claude Code's low-memory guard
// (#1084), where 12 cores and 32 GB, and CI's 4 cores and 16 GB, keep what
// the cores gave them. check:fast's mutations ask the same, and its pool
// starts no job past the memory's budget.
test("the compiler's builds at once are as many as the cores and the memory hold", () => {
  const gb = 2 ** 30;
  // On a quiet machine, 85% of it free, the memory whole says.
  const quiet = (total: number) => compilerBuilds(total === 16 ? 8 : 12, total * gb, 0.85 * total * gb);
  expect(quiet(16)).toBe(2);
  // Up to four: past it a build's own threads fill the cores (shard.ts).
  expect(quiet(32)).toBe(4);
  expect(quiet(64)).toBe(4);
  expect(compilerBuilds(4, 16 * gb, 14 * gb)).toBe(2);
  expect(compilerBuilds(2, 8 * gb, 7 * gb)).toBe(1);
  expect(memoryBudget(16 * gb, 14 * gb)).toBe(10);
  expect(memoryBudget(4 * gb, 4 * gb)).toBe(peakGB.compilerBuild);
  // Where other apps hold more of it, what's free says: one build on 16 GB
  // with 9 GB free.
  expect(memoryBudget(16 * gb, 9 * gb)).toBe(7);
  expect(compilerBuilds(8, 16 * gb, 9 * gb)).toBe(1);
  // Within the budget, the builds beside a test on each core.
  for (const [cores, total, free] of [[8, 16, 14], [8, 16, 9], [4, 16, 14], [12, 32, 27]]) {
    expect(compilerBuilds(cores, total * gb, free * gb) * peakGB.compilerBuild + cores * peakGB.test).toBeLessThanOrEqual(memoryBudget(total * gb, free * gb));
  }
  const check = jobs({ all: true, changed: [], branch: [], mutations: { names: ["dictionary-uncached"], base: "b" }, extra: [], cores: 8 }).jobs;
  const mutations = check.find((job) => job.name === "the change's mutations")!;
  expect(mutations.cmd).toContain(`--jobs=${compilerBuilds(8)}`);
  expect(mutations.memory).toBe(compilerBuilds(8) * peakGB.compilerBuild);
  expect(check.find((job) => job.name === "clippy")!.memory).toBe(peakGB.compilerCheck);
  // A mutation run's builds of its own share of the cores and the memory
  // alike, `ownPart`'s (#1767).
  const mutationsSource = readFileSync(join(root, "scripts/mutations.ts"), "utf8");
  expect(mutationsSource).toContain("const budget = ownMemory(memoryBudget());");
  expect(mutationsSource).toContain("compilerBuildsAsked(said >= 1 ? said : undefined, ownShare(machine().cores), undefined, undefined, budget);");
  expect(mutationsSource).toContain("Math.max(4, ownMemory(memoryBudget()) / 2)");
  // Beside another run, on 16 GB with 14 free and 8 cores: of half the
  // cores and the whole memory, two builds; of half of each, one.
  expect(compilerBuilds(4, 16 * gb, 14 * gb)).toBe(2);
  expect(compilerBuilds(4, 16 * gb, 14 * gb, memoryBudget(16 * gb, 14 * gb) / 2)).toBe(1);
});

// A run's `--jobs` lowers the builds the memory holds, never raises them:
// `--jobs=12` on 16 GB and 12 cores, where it held one, built twelve at
// once, some 42 GB, and the host stopped the run (2026-11-20).
test("a run's --jobs never builds more at once than the memory holds", () => {
  const gb = 2 ** 30;
  const held = compilerBuilds(12, 16 * gb, 12 * gb);
  expect(held).toBe(1);
  expect(compilerBuildsAsked(12, 12, 16 * gb, 12 * gb)).toEqual({ builds: held, cut: true });
  expect(compilerBuildsAsked(undefined, 12, 16 * gb, 12 * gb)).toEqual({ builds: held, cut: false });
  // Fewer than it holds, as check:fast asks beside its other jobs: as asked.
  expect(compilerBuildsAsked(1, 12, 64 * gb, 60 * gb)).toEqual({ builds: 1, cut: false });
  expect(compilerBuildsAsked(4, 12, 64 * gb, 60 * gb)).toEqual({ builds: 4, cut: false });
  expect(compilerBuildsAsked(9, 12, 64 * gb, 60 * gb)).toEqual({ builds: 4, cut: true });
});

// How many of each kind of job a run starts at once, from the machine as
// it is (scripts/shard.ts' `atOnce`, the one authority): a 16 GB Mac of 8
// cores, the M3 Max's 36 GB of 14, and CI's 16 GB of 4, each 85% free.
test("each kind of job runs as many at once as the machine's cores and memory hold", () => {
  const on = (cores: number, totalGB: number) => ({ cores, totalGB, freeGB: 0.85 * totalGB });
  const sizes = (cores: number, totalGB: number) =>
    Object.fromEntries((["test", "compilerBuild", "crateBuild", "crateCheck", "fileCompile"] as const).map((kind) => [kind, atOnce(kind, on(cores, totalGB))]));
  expect(sizes(8, 16)).toEqual({ test: 8, compilerBuild: 2, crateBuild: 4, crateCheck: 4, fileCompile: 4 });
  expect(sizes(14, 36)).toEqual({ test: 14, compilerBuild: 7, crateBuild: 7, crateCheck: 7, fileCompile: 7 });
  expect(sizes(4, 16)).toEqual({ test: 4, compilerBuild: 2, crateBuild: 2, crateCheck: 2, fileCompile: 2 });
  // Where little is free, the memory says, at least one.
  expect(atOnce("compilerBuild", { cores: 14, totalGB: 36, freeGB: 6 })).toBe(1);
  expect(atOnce("test", { cores: 14, totalGB: 36, freeGB: 3 })).toBe(11);
  expect(atOnce("test", { cores: 1, totalGB: 2, freeGB: 1 })).toBe(1);
});

// No runner counts the machine itself or fixes how many jobs it starts:
// each asks scripts/shard.ts (`machine`, `atOnce`), where a fixed count left
// a larger machine's cores idle, decision coverage's half the cores, the
// mutations' four builds, a net run alone in four parts. CI's workflows
// say their parts, as their machines are fixed.
test("no runner counts the machine's cores or fixes how many jobs run at once itself", () => {
  const files = [...new Bun.Glob("{scripts,test,tooling}/**/*.{ts,js}").scanSync(root)].filter(
    (file) => file !== "scripts/shard.ts" && !file.startsWith("test/corpus/") && !file.startsWith("test/crates/") && !file.includes("/fixtures/"),
  );
  const code = (file: string) => readFileSync(join(root, file), "utf8").split("\n").filter((line) => !/^\s*(\/\/|\*|\/\*)/.test(line)).join("\n");
  const found = files.flatMap((file) => {
    const text = code(file);
    return [
      ...[...text.matchAll(/availableParallelism\(|cpus\(\)\.length/g)].map(() => `${file}: counts the cores itself`),
      ...[...text.matchAll(/--at-once=\d/g)].map(() => `${file}: a fixed --at-once`),
      ...[...text.matchAll(/\bparts\("[^"]+", \d+/g)].map(() => `${file}: a fixed part count`),
    ];
  });
  // This test's own patterns aside.
  expect(found.filter((f) => !f.startsWith("test/shards.test.ts"))).toEqual([]);
});

// A run of the whole corpus, a `--corpus` mutation's control and its
// mutant, is cut so each part, at CI's slowest measured case, is under half
// its limit, which still ends a hang: two parts of some 1,450 cases on a
// 4-core runner took 12 to 20 minutes, and one didn't end in its 20 (#2375).
test("a whole-corpus part's measured time is under half its limit", () => {
  // The slowest measured: 20 min 6 s for some 1,450 cases.
  expect(wholeCorpusCaseSeconds).toBeGreaterThanOrEqual(1206 / 1450);
  const cases = corpusCaseNames().length;
  for (const cores of [4, 8, 12]) {
    const parts = wholeCorpusParts(cores, cases, testTimeout);
    expect(parts).toBeGreaterThanOrEqual(cores);
    expect(Math.ceil(cases / parts) * wholeCorpusCaseSeconds * 1000).toBeLessThan(testTimeout / 2);
  }
  // Twice the cases, twice the parts on a runner it already filled, and
  // one more where they'd each be half exactly, as 6,000 in 10 would.
  expect(wholeCorpusParts(4, 6000, 20 * 60_000)).toBe(11);
  expect(wholeCorpusParts(4, 100, 20 * 60_000)).toBe(4);
});

// A `--corpus` dispatch's control in as many jobs as keep each under half
// its limit: in one, 16 min on every shard's way (#2375). Each job's
// share, the whole corpus as the compiler is and with one that compiles
// nothing, its parts each a core's.
test("a shared control's job is under half its limit, its part of the runs each part's once", () => {
  const cases = corpusCaseNames().length;
  const jobs = controlParts(4, cases, testTimeout);
  expect(jobs).toBeGreaterThan(1);
  expect((2 * cases * wholeCorpusCaseSeconds * 1000) / 4 / jobs).toBeLessThan(testTimeout / 2);
  expect(controlParts(4, 2910, 20 * 60_000)).toBe(3);
  expect(controlParts(4, 100, 20 * 60_000)).toBe(1);
  expect(controlPartsOutput(3)).toBe("parts=[1,2,3]\ncount=3");
  const tests = readFileSync(join(root, ".github/workflows/rustc-tests.yml"), "utf8");
  expect(tests).toContain("run: bun scripts/ci.ts control-parts >> \"$GITHUB_OUTPUT\"");
  expect(tests).toContain("part: ${{ fromJSON(needs.mutation-control-plan.outputs.parts) }}");
  expect(tests).toContain("--control-part=${{ matrix.part }}/${{ needs.mutation-control-plan.outputs.count }} --control-out=target/mutants/control/${{ matrix.part }}.json");
  expect(tests).toContain("pattern: mutation-control-*\n          merge-multiple: true");
});

// The corpus runs in parts, one for each two cores where they all run at
// once (scripts/test.ts), one for each core where check:fast's pool
// starts each as a core is free: each of the parts of all the shards in
// exactly one.
test("each shard runs its share of a file of cases' parts, each part once", () => {
  expect(caseParts(4)).toBe(2);
  expect(caseParts(12)).toBe(6);
  expect(caseParts(1)).toBe(2);
  expect(pooledParts(12)).toBe(12);
  expect(pooledParts(1)).toBe(2);
  expect(jobs({ all: true, changed: [], branch: [], extra: [], cores: 12 }).jobs.filter((job) => job.name.startsWith("corpus ")).map((job) => job.env.RUST_JS_CASES)).toEqual(
    Array.from({ length: 12 }, (_, i) => `${i + 1}/12`),
  );
  expect(caseShardParts(1, 1, 6)).toEqual(["1/6", "2/6", "3/6", "4/6", "5/6", "6/6"]);
  const shards = [1, 2].map((i) => caseShardParts(i, 2, 2));
  expect(shards).toEqual([
    ["1/4", "3/4"],
    ["2/4", "4/4"],
  ]);
  expect(shards.flat().sort()).toEqual(caseShardParts(1, 1, 4));
});

// `bun run test`'s files of cases go through one pool beside the rest of
// the suite: half the cores while the rest runs, every core once it's
// ended, in parts small enough that none is left alone at the end, the
// metamorphic sample's too, never one file of it (check 43700659375).
test("the suite's files of cases are small parts of one pool, wider once the rest has ended", () => {
  expect([casePoolWidth(4, true), casePoolWidth(4, false)]).toEqual([2, 4]);
  expect([casePoolWidth(12, true), casePoolWidth(12, false)]).toEqual([6, 12]);
  expect([casePoolParts("corpus", 4), casePoolParts("metamorphic", 4)]).toEqual([8, 4]);
  // Of both shards on CI's four cores, each part once, the sample's first.
  const shards = [1, 2].map((i) => casePoolJobs(i, 2, 4));
  expect(shards[0]!.slice(0, 5)).toEqual([...["1/8", "3/8", "5/8", "7/8"].map((part) => ({ file: caseFiles.metamorphic, part })), { file: caseFiles.corpus, part: "1/16" }]);
  for (const file of Object.values(caseFiles)) {
    const parts = shards.flat().filter((job) => job.file === file).map((job) => job.part);
    expect([file, new Set(parts).size, parts.length]).toEqual([file, parts.length, Number(parts[0]!.split("/")[1])]);
  }
  const script = readFileSync(join(root, "scripts/test.ts"), "utf8");
  expect(script).toContain("const ignored = [...Object.values(caseFiles), ");
  expect(script).toContain("casePoolWidth(cores, restRunning)");
});

// A check's mutations on the fewest machines of `MUTATION_SHARDS` whose
// every shard, dealt as the shards deal by each's share of its shard, is
// expected to end in `SHARD_BUDGET_SECONDS`: its mutations' shares, its
// setup, the compiler as it is, its mutations' tests as it is, and a
// browser's install where one of its tests drives one. Sized by count, one
// shard of a change's slow mutations took 21 min and another 4; without
// the tests as it is, main's checks' shards took 2 min more than planned
// (#2220).
test("a check's mutations are split by their expected time, each shard under its budget", () => {
  const m = (name: string, tests = ["test/corpus.test.ts", "-t", name]) => ({ name, tests });
  const none = () => false;
  const own = (name: string) => `src/${name}.rs`;
  expect(mutationShards([], new Map(), none, own)).toBe("1");
  // Untimed, each a check's twentieth of five minutes, `UNTIMED_SECONDS`,
  // beside the setup, the compiler as it is and its tests as it is, of no
  // case the corpus has, `TESTS_AS_IT_IS`'s 7 s: 35 of them in the 533 s
  // those leave.
  expect(mutationShards(Array.from({ length: 35 }, (_, i) => m(`u${i}`)), new Map(), none, own)).toBe("1");
  expect(mutationShards(Array.from({ length: 36 }, (_, i) => m(`u${i}`)), new Map(), none, own)).toBe("2");
  // And of cases the corpus has, a third of a second each: 30 mutations'
  // 600 of them named, 210 s, take one more machine (#2228).
  const named = Array.from({ length: 30 }, (_, i) => m(`u${i}`, ["test/corpus.test.ts", "-t", `^c${i}_`]));
  const cases = Array.from({ length: 30 }, (_, i) => Array.from({ length: 20 }, (_, j) => `c${i}_${j}.rs`)).flat();
  expect(mutationShards(named, new Map(), none, own)).toBe("1");
  expect(mutationShards(named, new Map(), none, own, new Set(), cases)).toBe("2");
  // Timed: two of 300 s fit one shard each, never one, whatever their count.
  const slow = [m("a"), m("b")];
  expect(mutationShards(slow, new Map([["a", 300], ["b", 300]]), none, own)).toBe("2");
  expect(mutationShards(slow, new Map([["a", 200], ["b", 300]]), none, own)).toBe("1");
  // A browser's install where a test drives one: the shard it's dealt to,
  // over the budget alone, its fixed parts 720 s before its mutation, so
  // the other's own shard, and no more, where they'd end it no sooner.
  const browsing = [m("a", ["test/browser.test.ts"]), m("b")];
  expect(mutationShards(browsing, new Map([["a", 100], ["b", 100]]), (file) => file === "test/browser.test.ts", own)).toBe("2");
  // One longer than the budget alone takes one machine, not the most,
  // which would end it no sooner (#2220).
  expect(mutationShards([m("a")], new Map([["a", 4000]]), none, own)).toBe("1");
  expect(mutationShards([m("a"), m("b")], new Map([["a", 4000], ["b", 300]]), none, own)).toBe("2");
  // One the last checks found latent: its shard runs the survivors again at
  // the base, the compiler built whole there, 5 min more (#2220).
  expect(mutationShards(slow, new Map([["a", 200], ["b", 200]]), none, own)).toBe("1");
  expect(mutationShards(slow, new Map([["a", 200], ["b", 200]]), none, own, new Set(["a"]))).toBe("2");
});

// What a shard's mutations' tests as the compiler is take, by what they
// run: each test file other than the corpus by mode, whole or of its
// sets' `-t`s, and the corpus's cases they name (#2228).
test("a shard's tests as the compiler is are its distinct test files and the corpus cases they name", () => {
  const cases = ["a.rs", "ab.rs", "b.rs", "c.rs"];
  const runs = testsAsItIsRuns(
    [
      { tests: ["test/corpus.test.ts", "-t", "^a"] },
      { tests: ["test/corpus.test.ts", "-t", "^b"] },
      { tests: ["test/x.test.ts", "-t", "one"] },
      { tests: ["test/x.test.ts", "-t", "two"] },
      { tests: ["test/y.test.ts"] },
      // Another mode's run of the same file is its own.
      { tests: ["test/y.test.ts"], snapshots: true },
      {},
    ],
    cases,
  );
  expect(runs).toEqual({ files: 3, whole: 2, corpusCases: 3 });
  expect(testsAsItIsSeconds(runs)).toBeCloseTo(7 + 3 * 3.7 + 2 * 4 + 3 * 0.35, 5);
});

// The estimate held to 40 shards of main's checks of 2026-11-19 to 21, each
// its runs and what the phase took (test/fixtures/tests-as-it-is.tsv),
// 6 to 371 s: within half a minute at the root mean square, where #2220's
// constant 120 s was 97 s off, and never more than 80 s short, where it was
// 251 s short of a check of 155 mutations a shard (#2228).
test("a shard's tests as the compiler is are expected within their measures", () => {
  const rows = readFileSync(join(root, "test/fixtures/tests-as-it-is.tsv"), "utf8").trim().split("\n").slice(1).map((line) => line.split("\t"));
  expect(rows.length).toBeGreaterThanOrEqual(40);
  const errors = rows.map(([, , files, whole, corpusCases, seconds]) => testsAsItIsSeconds({ files: +files!, whole: +whole!, corpusCases: +corpusCases! }) - +seconds!);
  expect(Math.sqrt(errors.reduce((sum, e) => sum + e * e, 0) / errors.length)).toBeLessThan(30);
  expect(Math.min(...errors)).toBeGreaterThan(-80);
});

// Check 44441824204's five mutations, with the merged times its plan read
// (run 44441216357's): two new ones of scripts/ci.ts, untimed, whose file's
// twelve timed mutations took 3 to 8 s, and three of 2 to 3 s, among a
// catalog of 11,776 whose 90th percentile was 987 s. Estimated by the
// catalog's file, as the shards estimate them, they take one machine; by
// the chosen alone's, none of whose file was timed, each was 987 s, and
// they took 16, 11 of them running nothing.
test("a check's untimed mutations are estimated by their file's mutations in the catalog, as the shards estimate them", () => {
  const chosen = ["mutation-shards-of-setup-alone", "mutation-shards-browser-uncounted", "control-settings-unset", "untimed-mutations-as-the-median", "untimed-mutations-to-the-least"];
  const files = new Map<string, string>([
    ["mutation-shards-of-setup-alone", "scripts/ci.ts"],
    ["mutation-shards-browser-uncounted", "scripts/ci.ts"],
    ["control-settings-unset", "scripts/mutations.ts"],
    ["untimed-mutations-as-the-median", "scripts/shard.ts"],
    ["untimed-mutations-to-the-least", "scripts/shard.ts"],
  ]);
  const seconds = new Map<string, number>([["control-settings-unset", 2.4], ["untimed-mutations-as-the-median", 2.9], ["untimed-mutations-to-the-least", 2.3]]);
  [3, 3.3, 3.3, 3.4, 3.6, 3.7, 3.7, 3.7, 3.7, 3.9, 7.3, 7.7].forEach((time, i) => {
    files.set(`ci-${i}`, "scripts/ci.ts");
    seconds.set(`ci-${i}`, time);
  });
  // The rest of the catalog, its times a nightly's: 90% of them 482 s,
  // the slowest tenth 987.
  for (let i = 0; i < 200; i++) {
    files.set(`src-${i}`, `src/${i}.rs`);
    seconds.set(`src-${i}`, i < 180 ? 482 : 987);
  }
  const none = () => false;
  expect(mutationShards(chosen.map((name) => ({ name, tests: ["test/shards.test.ts"] })), seconds, none, (name) => files.get(name))).toBe("1");
});

// A mutation's time is its share of its shard: of its tests, the seconds
// they held a slot, never those they waited for one, over the slots; of
// its build, its own over the shard's build workers. Timed from asking, a
// nightly's waited minutes, and a check's plan sized by them.
test("a mutation's time is its share of its shard, its tests timed only as they hold a slot", async () => {
  const run = slots(1);
  const sleep = (ms: number) => new Promise((resume) => setTimeout(resume, ms));
  const first = { seconds: 0 };
  const second = { seconds: 0 };
  await Promise.all([run(() => sleep(200), first), run(() => sleep(50), second)]);
  expect(first.seconds).toBeGreaterThanOrEqual(0.19);
  expect(second.seconds).toBeLessThan(0.15);
  expect(shareOfShard({ build: 10, tests: 8 }, 2, 4)).toEqual({ build: 5, tests: 2 });
});

// A part of a file of cases keeps the cores `caseParts` gives it busy, two
// of 12, and holds that many slots: counted one each, a sweep of mutants
// against the whole corpus ran 12 parts at once on 12 cores, a load of 32.
test("a corpus part holds the cores it keeps busy of a run's slots", async () => {
  expect([casePartCores(12), casePartCores(4), casePartCores(2), casePartCores(1)]).toEqual([2, 2, 1, 1]);
  const run = slots(4);
  let busy = 0;
  let most = 0;
  const work = (weight: number) =>
    run(
      async () => {
        busy += weight;
        most = Math.max(most, busy);
        await new Promise((resume) => setTimeout(resume, 20));
        busy -= weight;
      },
      undefined,
      weight,
    );
  await Promise.all([work(2), work(2), work(2), work(1), work(1), work(2)]);
  expect(most).toBe(4);
  // A part waits where three single runs leave one core of four.
  most = 0;
  await Promise.all([work(1), work(1), work(1), work(2)]);
  expect(most).toBeLessThanOrEqual(4);
  // One heavier than the whole runs alone, never waits forever.
  most = 0;
  await Promise.all([work(6), work(1)]);
  expect(most).toBe(6);
});

// The shards balance by the newest runs' timings that are still kept, of
// any branch, each file's the newest that timed it; none, and they split
// by file count.
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
  expect(timingsRuns(artifacts)).toEqual([3, 2]);
  expect(timingsRuns(artifacts, 1)).toEqual([3]);
  expect(timingsRuns(artifacts.filter((a) => a.run !== 3))).toEqual([2]);
  expect(timingsRuns([artifact("bless-test-1", 7, "2026-10-21T12:00:00Z")])).toEqual([]);
});

// A shard runs its mutations' tests as the compiler is before any mutant,
// each test file once, 10 to 19 min of a nightly shard that the deal left
// out (#2304): each mutation's runs (`controlRuns`) are in its shard's load,
// once a shard, and a mutation goes where its time and the runs it adds are
// least, so it joins the shard that already runs its test file.
test("a shard's tests as the compiler is are dealt with its mutations", () => {
  const byName = {
    x1: { name: "x1", tests: ["test/x.test.ts"] },
    y1: { name: "y1", tests: ["test/y.test.ts"] },
    x2: { name: "x2", tests: ["test/x.test.ts"] },
  };
  const seconds = new Map([["x1", 30], ["y1", 29], ["x2", 1]]);
  const controls = (name: string) => controlRuns(byName[name as keyof typeof byName], []);
  expect(controls("x1").map(([, cost]) => cost)).toEqual([3.7, 4]);
  const dealt = dealtByGroup(["x1", "y1", "x2"], 2, seconds, () => "suite", controls);
  // x2 joins x1, whose shard runs test/x.test.ts, though the other has less.
  expect(dealt.shards).toEqual([["x1", "x2"], ["y1"]]);
  // Each shard's load: its mutations' times and its test files' runs, once.
  expect(dealt.load.map((load) => +load.toFixed(1))).toEqual([30 + 1 + 7.7, 29 + 7.7]);
  // Without them, the least-loaded shard took it, as before.
  expect(dealtByGroup(["x1", "y1", "x2"], 2, seconds, () => "suite").shards).toEqual([["x1"], ["x2", "y1"]]);
});

// GitHub's machines differ by a third: nightly 44963035271's shards 1 and
// 13 to 16 took 1.22 to 1.32 times the plan's time of each of their
// mutations, at the median, its others 0.89 to 1.01 (#2304 (b)). The merge
// takes each shard's machine out, by that median, so the next deal reads a
// mutation's time, not its machine's; one that grew against its shard's
// others keeps what it grew by, and a shard of few timed ones is as it ran.
test("each shard's times are merged at the plan's speed, its machine's taken out", () => {
  const names = Array.from({ length: 30 }, (_, i) => `m${i}`);
  const base = new Map(names.map((name) => [name, 2]));
  const slow = new Map(names.slice(0, 15).map((name) => [name, 2.6]));
  const usual = new Map(names.slice(15).map((name) => [name, name === "m20" ? 10 : 2]));
  const few = new Map([["new", 4], ["m0", 3]]);
  const shards = new Map<string, ReadonlyMap<string, number>>([["slow", new Map([...slow, ...names.slice(15, 20).map((n) => [n, 2.6] as [string, number])])], ["usual", usual], ["few", few]]);
  // Of 20 timed ones at least: the slow shard's 1.3 taken out.
  const merged = normalizedShardTimes(base, new Map([["slow", new Map([...slow, ...names.slice(15, 20).map((n) => [n, 2.6] as [string, number])])]]));
  expect(+merged.get("m0")!.toFixed(2)).toBe(2);
  expect(normalizedShardTimes(base, new Map([["usual", usual]])).get("m20")).toBe(10);
  expect(normalizedShardTimes(base, new Map([["few", few]])).get("m0")).toBe(3);
  expect(normalizedShardTimes(base, shards).size).toBe(31);
  // Read by shard, each folder its own.
  const dir = mkdtempSync(join(tmpdir(), "shard-times-"));
  for (const [shard, times] of [["mutation-timings-1", { a: { build: 1, tests: 1 } }], ["mutation-timings-2", { b: { build: 3, tests: 0 } }]] as const) {
    mkdirSync(join(dir, shard), { recursive: true });
    writeFileSync(join(dir, shard, "timings.json"), JSON.stringify(times));
  }
  expect([...timingsByShard(dir).values()].map((times) => [...times])).toEqual([[["a", 2]], [["b", 3]]]);
  rmSync(dir, { recursive: true, force: true });
});

test("a change's untimed mutations are dealt first, in turn, each its file's usual or the timed ones' 90th percentile, so no shard takes several", () => {
  const files: Record<string, string> = { a: "src/x.rs", b: "src/x.rs", n1: "src/x.rs", n2: "src/y.rs", n3: "src/y.rs", n4: "src/z.rs", c: "src/y.rs", d: "src/w.rs" };
  const seconds = new Map([["a", 40], ["b", 60], ["c", 10], ["d", 10]]);
  const shards = [1, 2].map((i) => balancedByGroup(Object.keys(files), i, 2, seconds, (name) => files[name]));
  expect(shards.flat().sort()).toEqual(Object.keys(files).sort());
  // Untimed n1 (src/x.rs's usual, 60), n2, n3 (src/y.rs's, 10), n4 (the
  // 90th percentile, 60): two each, in turn, the longest first.
  expect(shards.map((s) => s.filter((n) => n.startsWith("n")))).toEqual([["n1", "n2"], ["n3", "n4"]].map((s) => s.sort()));
  expect(() => balancedByGroup(["a"], 3, 2, seconds, (name) => files[name])).toThrow("a shard is 1 to 2");
});

// A check's test shards are dealt by what each file took
// (`testShardFiles`), as `plan` expects them, not by `bun test --shard`,
// which gave check 44022102403's shards 101 and 261 of 362 files, 1,127 and
// 1,894 measured seconds, and its second shard ended 4.5 minutes after its
// first. Dealt by those times, each has 1,511.
test("a check's test shards are dealt by what each file took", () => {
  const dir = mkdtempSync(join(tmpdir(), "test-timings-"));
  try {
    const at = (name: string, files: Record<string, number>) => {
      writeFileSync(join(dir, name), JSON.stringify({ version: 1, files }));
      return join(dir, name);
    };
    // This checkout's own time of a file first, then the last CI run's.
    const seconds = testFileSeconds([at("mine.json", { "test/a.test.ts": 4000 }), at("ci.json", { "test/a.test.ts": 9000, "test/b.test.ts": 3000, "test/c.test.ts": 2000, "test/d.test.ts": 1000 }), join(dir, "none.json")]);
    expect([...seconds]).toEqual([["test/a.test.ts", 4], ["test/b.test.ts", 3], ["test/c.test.ts", 2], ["test/d.test.ts", 1]]);
    const files = [...seconds.keys()];
    const load = (index: number) => testShardFiles(files, index, 2, seconds).reduce((sum, file) => sum + seconds.get(file)!, 0);
    expect([load(1), load(2)]).toEqual([5, 5]);
    // The very deal `plan` expects, untimed ones too, each file once.
    const some = [...files, "test/e.test.ts", "test/f.test.ts"];
    const dealt = [1, 2, 3].map((index) => testShardFiles(some, index, 3, seconds));
    expect(dealt).toEqual(dealtByGroup(some, 3, seconds, () => "suite").shards);
    expect(dealt.flat().sort()).toEqual([...some].sort());
    expect(() => testShardFiles(files, 3, 2, seconds)).toThrow("a shard is 1 to 2");
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
  // The suite's files and the browsers' are given to `bun test` dealt so,
  // never with its `--shard`.
  const suite = readFileSync(join(root, "scripts/test.ts"), "utf8");
  expect(suite).not.toContain("...shard,");
  expect(suite).toContain("const pageFiles = dealt(");
  expect(suite).toContain("testShardFiles(files, index, count, seconds)");
});

// A check's mutation shards take at most the account's jobs that other
// checks and its own don't hold, where they'd wait past its 20, and say
// what the cap cost (#2257): fewer, longer shards, each paying its setup
// once, rather than more that queue.
test("a check's mutation shards are as many as the account's jobs free, and say what that cost", () => {
  const runs = [
    { id: 1, jobs: [{ status: "in_progress" }, { status: "queued" }, { status: "completed" }] },
    { id: 2, jobs: [{ status: "in_progress" }] },
    { id: 9, jobs: [{ status: "in_progress" }, { status: "in_progress" }] },
  ];
  expect(heldJobs(runs, 9)).toBe(3);
  expect(freeJobs(3, 10)).toBe(CONCURRENT_JOBS - 13);
  expect(freeJobs(30, 10)).toBe(1);
  const m = (name: string) => ({ name, tests: ["test/corpus.test.ts", "-t", name] });
  const chosen = Array.from({ length: 8 }, (_, i) => m(`s${i}`));
  const seconds = new Map(chosen.map((c) => [c.name, 300]));
  const own = (name: string) => `src/${name}.rs`;
  const free = mutationShardPlan(chosen, seconds, () => false, own);
  expect([free.count, free.wanted]).toEqual([8, 8]);
  const capped = mutationShardPlan(chosen, seconds, () => false, own, new Set(), [], 4);
  expect([capped.count, capped.wanted]).toEqual([4, 8]);
  expect(capped.at).toBeGreaterThan(capped.wantedAt);
  expect(shardSummary(capped, 9, 7)).toContain("Mutation shards: 4 where their time asks 8; 4 of the account's 20 jobs free (9 held by other checks, 7 its own beside them): expected to end in");
  expect(shardSummary(free, 0, 7)).toContain("Mutation shards: 8, as their time asks;");
  expect(shardSummary(free, undefined, 7)).toContain("the jobs other checks hold unread");
});

