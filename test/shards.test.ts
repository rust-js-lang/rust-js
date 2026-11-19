// How CI's and the machine's work is dealt and sized (scripts/shard.ts,
// DEVELOPMENT.md): the suite's files and the mutations split into shards
// by what each took, how many machines a check's mutations take, and how
// many jobs of each kind run at once on a machine's cores and memory.

import { expect, test } from "bun:test";
import { mkdirSync, mkdtempSync, readdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { gunzipSync } from "node:zlib";
import { join } from "node:path";
import { decodedTestTimings, decodedTimings, MERGED_MUTATION_TIMES, encodedTestTimings, encodedTimings, mergedTimings, mergedTimingsRun, MUTATION_SHARDS, mutationShards, SHARD_BUDGET_SECONDS, TEST_SHARD_SETUP_SECONDS, TEST_SHARDS, TEST_WALL_PER_RECORDED, testShardList, testShards, timingsRun } from "../scripts/ci";
import { mutations } from "../scripts/mutations";
import { jobs } from "../scripts/checkFast";
import { runs } from "../scripts/iteratorSlice";
import { balancedByGroup, fewestShards, shareOfShard, slots, testFileSeconds } from "../scripts/shard";
import { atOnce, caseFiles, caseParts, casePoolJobs, casePoolParts, casePoolWidth, caseShardParts, compilerBuilds, memoryBudget, nightly, parts, peakGB, pooledParts, shard, timedShard, balanced } from "../scripts/shard";
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
  expect(runs).toEqual(["${{ steps.timings.outputs.args }} ", "${{ steps.timings.outputs.args }} "]);
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
// by the mutations' one rule (`fewestShards`): each shard's setup and its
// files' recorded seconds at the wall each second of them took, measured
// on check 44459055794's two shards of everything (a6b13d280): its files'
// recorded 1,826 and 1,875 s, its runs' walls 873 and 927 s. That
// selection, its very times, plans the shards that bring each under the
// budget: not one, not the most.
test("a check's suite takes the machines its selection's expected time needs", () => {
  expect(fewestShards([1, 2, 4], (count) => [100 / count + 10], 40)).toBe(4);
  expect(fewestShards([1, 2, 4], (count) => [1000 / count], 40)).toBe(4);
  expect(fewestShards([1, 2, 4], () => [5], 40)).toBe(1);
  // Every shard within it, not any one.
  expect(fewestShards([1, 2, 4], (count) => (count === 1 ? [50] : count === 2 ? [10, 50] : [10, 10, 10, 10]), 40)).toBe(4);
  expect(873 / 1826).toBeCloseTo(TEST_WALL_PER_RECORDED, 1);
  expect(927 / 1875).toBeCloseTo(TEST_WALL_PER_RECORDED, 1);
  const measured = testFileSeconds([join(root, "test/fixtures/test-timings-44459055794.json")]);
  expect(measured.size).toBe(384);
  const all = { all: true, files: [], cases: [], forms: [], why: "" };
  const planned = Number(testShards(all, measured, undefined, () => [...measured.keys()]));
  expect(planned).toBeGreaterThan(1);
  expect(planned).toBeLessThan(TEST_SHARDS.at(-1)!);
  expect(planned).toBe(4);
  // Of everything, every suite file counts, those the last run didn't
  // time, a selection's that ran few, by the timed ones' median, never the
  // timed alone: of 100 files, 10 timed at a minute each, six machines.
  const suite = Array.from({ length: 100 }, (_, i) => `test/f${i}.test.ts`);
  const few = new Map(suite.slice(0, 10).map((file) => [file, 60] as const));
  expect([testShards(all, few, undefined, () => suite), testShards(all, few, undefined, () => [...few.keys()])]).toEqual(["6", "1"]);
  // As many as a check started by hand or the nightly says, given.
  expect(testShards(all, measured, "2")).toBe("2");
  // Each shard of it within the budget, and one fewer would not be.
  const total = [...measured.values()].reduce((a, b) => a + b, 0);
  expect(TEST_SHARD_SETUP_SECONDS + (TEST_WALL_PER_RECORDED * total) / planned).toBeLessThanOrEqual(SHARD_BUDGET_SECONDS);
  expect(TEST_SHARD_SETUP_SECONDS + (TEST_WALL_PER_RECORDED * total) / (planned - 1)).toBeGreaterThan(SHARD_BUDGET_SECONDS);
  // A small selection, one machine; with no run's times kept, by count.
  const some = (files: number, cases: number) => ({ all: false, files: Array.from({ length: files }, (_, i) => `test/f${i}.test.ts`), cases: Array.from({ length: cases }, (_, i) => `c${i}.rs`), forms: [], why: "" });
  expect(testShards({ ...some(0, 0), files: ["test/compat.test.ts", "test/adr.test.ts"] }, measured)).toBe("1");
  expect([testShards(some(5, 30)), testShards(some(60, 400)), testShards(some(61, 0)), testShards(some(0, 401)), testShards(all)]).toEqual(["1", "1", "2", "2", "2"]);
  // Handed on as the times `plan` encodes, and the matrix its list.
  expect([...decodedTestTimings(encodedTestTimings(new Map([["test/a.test.ts", 1.5]])))]).toEqual([["test/a.test.ts", 1.5]]);
  expect(decodedTestTimings("").size).toBe(0);
  expect(testShardList("4")).toBe("[1,2,3,4]");
  const jobs = (Bun.YAML.parse(readFileSync(join(root, ".github/workflows/check.yml"), "utf8")) as { jobs: Record<string, { outputs?: Record<string, string>; strategy?: { matrix?: { shard?: string } }; steps: { run?: string }[] }> }).jobs;
  expect(jobs.plan.outputs?.test_shards).toBe("${{ steps.plan.outputs.test_shards }}");
  expect(jobs.plan.outputs?.test_shard_list).toBe("${{ steps.plan.outputs.test_shard_list }}");
  expect(jobs.test.strategy?.matrix?.shard).toBe("${{ fromJSON(needs.plan.outputs.test_shard_list) }}");
  expect(jobs.plan.steps.find((step) => step.run?.includes("bun scripts/ci.ts plan"))?.run).toContain("--test-shards=");
  expect(JSON.stringify(jobs.test)).not.toContain("inputs.test_shards");
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
  const check = jobs({ all: true, changed: [], branch: [], added: ["dictionary-uncached"], extra: [], cores: 8 }).jobs;
  const mutations = check.find((job) => job.name === "mutations the change adds")!;
  expect(mutations.cmd).toContain(`--jobs=${compilerBuilds(8)}`);
  expect(mutations.memory).toBe(compilerBuilds(8) * peakGB.compilerBuild);
  expect(check.find((job) => job.name === "clippy")!.memory).toBe(peakGB.compilerCheck);
  expect(readFileSync(join(root, "scripts/mutations.ts"), "utf8")).toContain(": compilerBuilds(ownShare(machine().cores));");
});

// How many of each kind of job a run starts at once, from the machine as
// it is (scripts/shard.ts' `atOnce`, the one authority): a 16 GB Mac of 8
// cores, the M3 Max's 36 GB of 14, and CI's 16 GB of 4, each 85% free.
test("each kind of job runs as many at once as the machine's cores and memory hold", () => {
  const on = (cores: number, totalGB: number) => ({ cores, totalGB, freeGB: 0.85 * totalGB });
  const sizes = (cores: number, totalGB: number) =>
    Object.fromEntries((["test", "compilerBuild", "crateBuild", "crateCheck"] as const).map((kind) => [kind, atOnce(kind, on(cores, totalGB))]));
  expect(sizes(8, 16)).toEqual({ test: 8, compilerBuild: 2, crateBuild: 4, crateCheck: 4 });
  expect(sizes(14, 36)).toEqual({ test: 14, compilerBuild: 7, crateBuild: 7, crateCheck: 7 });
  expect(sizes(4, 16)).toEqual({ test: 4, compilerBuild: 2, crateBuild: 2, crateCheck: 2 });
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
  expect(jobs({ all: true, changed: [], branch: [], added: [], extra: [], cores: 12 }).jobs.filter((job) => job.name.startsWith("corpus ")).map((job) => job.env.RUST_JS_CASES)).toEqual(
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
// setup, the compiler as it is, and a browser's install where one of its
// tests drives one. Sized by count, one shard of a change's slow mutations
// took 21 min and another 4.
test("a check's mutations are split by their expected time, each shard under its budget", () => {
  const m = (name: string, tests = ["test/corpus.test.ts", "-t", name]) => ({ name, tests });
  const none = () => false;
  const own = (name: string) => `src/${name}.rs`;
  expect(mutationShards([], new Map(), none, own)).toBe("1");
  // Untimed, each a check's twentieth of five minutes, `UNTIMED_SECONDS`,
  // beside the setup and the compiler as it is.
  expect(mutationShards(Array.from({ length: 36 }, (_, i) => m(`u${i}`)), new Map(), none, own)).toBe("1");
  expect(mutationShards(Array.from({ length: 37 }, (_, i) => m(`u${i}`)), new Map(), none, own)).toBe("2");
  // Timed: two of 300 s fit one shard each, never one, whatever their count.
  const slow = [m("a"), m("b")];
  expect(mutationShards(slow, new Map([["a", 300], ["b", 300]]), none, own)).toBe("2");
  expect(mutationShards(slow, new Map([["a", 200], ["b", 300]]), none, own)).toBe("1");
  // A browser's install where a test drives one: the shard it's dealt to.
  const browsing = [m("a", ["test/browser.test.ts"]), m("b")];
  expect(mutationShards(browsing, new Map([["a", 100], ["b", 100]]), (file) => file === "test/browser.test.ts", own)).toBe("2");
  // One longer than the budget alone takes the most.
  expect(mutationShards([m("a")], new Map([["a", 4000]]), none, own)).toBe(String(MUTATION_SHARDS.at(-1)));
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

// A check's test shards are dealt by what each file took (`balanced`), not
// by `bun test --shard`, which gave check 44022102403's shards 101 and 261
// of 362 files, 1,127 and 1,894 measured seconds, and its second shard
// ended 4.5 minutes after its first. Dealt by those times, each has 1,511.
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
    const load = (index: number) => balanced(files, index, 2, seconds).reduce((sum, file) => sum + seconds.get(file)!, 0);
    expect([load(1), load(2)]).toEqual([5, 5]);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
  // The suite's files and the browsers' are given to `bun test` dealt so,
  // never with its `--shard`.
  const suite = readFileSync(join(root, "scripts/test.ts"), "utf8");
  expect(suite).not.toContain("...shard,");
  expect(suite).toContain("const pageFiles = dealt(");
});
