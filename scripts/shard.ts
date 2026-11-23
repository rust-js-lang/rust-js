// A set of work split across machines that run at once, as CI runs the
// mutations (DEVELOPMENT.md): shard `index` of `count`. (`bun test` splits
// the suite itself, `--shard`, by how long each file took.) And how much
// of a machine's cores and memory the work run at once on it may hold.

import { spawnSync } from "node:child_process";
import { existsSync, mkdirSync, readdirSync, readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { availableParallelism, freemem, platform, totalmem } from "node:os";

/** Shard `index`, from 1, of `count`: the items, sorted, dealt out in turn,
 * so each is in exactly one shard, whatever order they're given in. */
export function shard<T extends string>(items: T[], index: number, count: number): T[] {
  if (!Number.isInteger(index) || !Number.isInteger(count) || index < 1 || index > count) {
    throw new Error(`a shard is 1 to ${count}, not ${index}`);
  }
  return [...items].sort().filter((_, i) => i % count === index - 1);
}

/** Shard `index`, from 1, of `count`, of items that take time: each, the
 * longest first, given to the shard with the least time so far, the first
 * of those that tie, by `seconds` of each, those it doesn't know the median
 * of those it does. Each is in exactly one shard, whatever order they're
 * given in, as every shard given the same times deals the same: the
 * mutations, whose time is mostly a few, so dealt out in turn one shard of
 * the nightly's 16 took 411 min of work and another 230 (nightly
 * 43650311159, docs/dev-loop.md). */
/** The shard with the least load so far, the first of those even. */
function leastLoaded(load: number[]): number {
  let least = 0;
  for (let i = 1; i < load.length; i++) if (load[i]! < load[least]!) least = i;
  return least;
}

export function balanced<T extends string>(items: T[], index: number, count: number, seconds: ReadonlyMap<string, number>): T[] {
  if (!Number.isInteger(index) || !Number.isInteger(count) || index < 1 || index > count) {
    throw new Error(`a shard is 1 to ${count}, not ${index}`);
  }
  const known = items.flatMap((item) => (seconds.has(item) ? [seconds.get(item)!] : [])).sort((a, b) => a - b);
  const usual = known.length === 0 ? 1 : known[Math.floor(known.length / 2)]!;
  const time = (item: T) => seconds.get(item) ?? usual;
  const sorted = [...items].sort((a, b) => time(b) - time(a) || (a < b ? -1 : a > b ? 1 : 0));
  const load = new Array<number>(count).fill(0);
  const mine: T[] = [];
  for (const item of sorted) {
    const least = leastLoaded(load);
    load[least] = load[least]! + time(item);
    if (least === index - 1) mine.push(item);
  }
  return mine.sort();
}

/** Shard `index` of `count` of the mutations, `bun scripts/mutations.ts
 * --shard=i/n --timings=<dir>`: by what each took in the runs whose
 * `timings.json`s are under `dir`, its build's share and its tests
 * (`balanced`), as CI's shards are given the last nightly's (`ci.ts
 * mutation-timings`); with none given, dealt out in turn (`shard`). */
export function timedShard<T extends string>(items: T[], index: number, count: number, dir?: string, groupOf?: (item: string) => string | undefined, controls?: Controls): T[] {
  if (dir === undefined) return shard(items, index, count);
  return groupOf === undefined ? balanced(items, index, count, timingsUnder(dir)) : balancedByGroup(items, index, count, timingsUnder(dir), groupOf, controls);
}

/** What an item adds to its shard beside its own time, by keys the shard's
 * other items share, each run once a shard: a mutation's tests as the
 * compiler is (`controlRuns`), 10 to 19 min of a nightly shard, which the
 * deal left out (#2304). */
export type Controls = (item: string) => readonly (readonly [string, number])[];

/** Of `items`, shard `index` of `count`, by what each took, `seconds`, and
 * of one no run timed, a change's new mutation, an estimate: the median of
 * its group's timed ones, `groupOf`'s, a mutation's file, whose build it
 * shares; else the timed ones' 90th percentile, a new site's build and
 * tests being the costly kind. Those untimed are dealt first, in turn, the
 * longest first, so no shard takes several of them; then the timed, the
 * longest first, each to the shard with the least so far. Of run
 * 43976170163's eight shards, dealt as the median, 10.7 s, its change's new
 * mutations made one 536 s and another 277 (2026-11-17). */
export function balancedByGroup<T extends string>(items: T[], index: number, count: number, seconds: ReadonlyMap<string, number>, groupOf: (item: string) => string | undefined, controls?: Controls): T[] {
  if (!Number.isInteger(index) || !Number.isInteger(count) || index < 1 || index > count) {
    throw new Error(`a shard is 1 to ${count}, not ${index}`);
  }
  return dealtByGroup(items, count, seconds, groupOf, controls).shards[index - 1]!;
}

/** What a mutation no run timed is expected to take where no run timed
 * any: a check's twenty a machine in five minutes or so. Alike, they're
 * dealt in turn. */
export const UNTIMED_SECONDS = 15;

/** A mutation shard's test slots: `run` starts each work once those
 * running leave `weight` of `limit` free, or none runs, in the order they
 * asked. `weight`, the cores the work keeps busy, one of a test file's run,
 * `casePartCores` of a part of a file of cases: counted one each, a sweep
 * of mutants against the whole corpus ran 12 parts at once on 12 cores, a
 * load of 32 (2026-11-19). `holder`, where given, gains the seconds the
 * work held its slot, never those it waited for one: timed from its
 * asking, a nightly's mutation waited minutes behind thousands of others'
 * tests, its median 482 s and 90th percentile 987, where a check's run of
 * the same took 2 to 8 (main's merged times of run 44441216357). */
export function slots(limit: number): <T>(work: () => Promise<T>, holder?: { seconds: number }, weight?: number) => Promise<T> {
  let running = 0;
  const waiting: (() => void)[] = [];
  return async (work, holder, weight = 1) => {
    while (running > 0 && running + weight > limit) await new Promise<void>((resume) => waiting.push(resume));
    running += weight;
    const began = performance.now();
    try {
      return await work();
    } finally {
      if (holder !== undefined) holder.seconds += (performance.now() - began) / 1000;
      running -= weight;
      waiting.splice(0).forEach((resume) => resume());
    }
  };
}

/** A mutation's share of its shard's time, as its `timings.json` keeps
 * it, so a shard's mutations' shares sum to what their part of it takes:
 * its build's own seconds over the shard's build workers, `jobs`, and the
 * seconds its tests held a slot over the shard's test slots, `testSlots`. */
export function shareOfShard(own: { build: number; tests: number }, jobs: number, testSlots: number): { build: number; tests: number } {
  return { build: own.build / jobs, tests: own.tests / testSlots };
}

/** `balancedByGroup`'s deal of `items` to `count` shards: each shard's
 * items, sorted, and what it's expected to take, in seconds, its untimed
 * items by their estimate. `plan` sizes the mutations' machines by it, as
 * each shard deals its own by it. Of `controls`, each item's runs its
 * shard makes once, its load holds them too, and a timed item goes where
 * its time and the runs it adds there are least, so mutations of one test
 * file share its control (#2304). */
export function dealtByGroup<T extends string>(items: T[], count: number, seconds: ReadonlyMap<string, number>, groupOf: (item: string) => string | undefined, controls?: Controls): { shards: T[][]; load: number[] } {
  const all = [...seconds.values()].sort((a, b) => a - b);
  const p90 = all.length === 0 ? UNTIMED_SECONDS : all[Math.min(all.length - 1, Math.floor(all.length * 0.9))]!;
  const byGroup = new Map<string, number[]>();
  for (const [name, time] of seconds) {
    const group = groupOf(name);
    if (group !== undefined) byGroup.set(group, [...(byGroup.get(group) ?? []), time]);
  }
  const estimate = (item: string) => {
    const known = (byGroup.get(groupOf(item) ?? "") ?? []).sort((a, b) => a - b);
    return known.length === 0 ? p90 : known[known.length >> 1]!;
  };
  const byName = (a: T, b: T) => (a < b ? -1 : a > b ? 1 : 0);
  const untimed = items.filter((item) => !seconds.has(item)).sort((a, b) => estimate(b) - estimate(a) || byName(a, b));
  const timed = items.filter((item) => seconds.has(item)).sort((a, b) => seconds.get(b)! - seconds.get(a)! || byName(a, b));
  const load = new Array<number>(count).fill(0);
  const shards: T[][] = Array.from({ length: count }, () => []);
  const held: Set<string>[] = Array.from({ length: count }, () => new Set());
  const added = (at: number, item: string) => (controls?.(item) ?? []).reduce((sum, [key, cost]) => (held[at]!.has(key) ? sum : sum + cost), 0);
  const put = (at: number, item: T, own: number) => {
    load[at] = load[at]! + own + added(at, item);
    for (const [key] of controls?.(item) ?? []) held[at]!.add(key);
    shards[at]!.push(item);
  };
  untimed.forEach((item, k) => put(k % count, item, estimate(item)));
  for (const item of timed) {
    const own = seconds.get(item)!;
    if (controls === undefined) {
      put(leastLoaded(load), item, own);
      continue;
    }
    let best = 0;
    for (let at = 1; at < count; at++) if (load[at]! + added(at, item) < load[best]! + added(best, item)) best = at;
    put(best, item, own);
  }
  return { shards: shards.map((shard) => shard.sort()), load };
}

/** The test files shard `index` of `count` runs of `files`, by their
 * times, `seconds` (`testFileSeconds`): `dealtByGroup`'s, the one deal
 * `plan` expects each shard's time by (`testShards`) and `bun run test`
 * runs, so a shard runs what it was expected to (2e's lesson). */
export function testShardFiles<T extends string>(files: T[], index: number, count: number, seconds: ReadonlyMap<string, number>): T[] {
  if (!Number.isInteger(index) || !Number.isInteger(count) || index < 1 || index > count) {
    throw new Error(`a shard is 1 to ${count}, not ${index}`);
  }
  return dealtByGroup(files, count, seconds, () => "suite").shards[index - 1]!;
}

/** The fewest of `choices`, machines a check may split work across, whose
 * every shard is expected to take `budget` seconds or less, `expected`'s
 * seconds of each shard of that many; else the fewest whose slowest shard
 * is expected soonest, as more machines than those end it no sooner: a
 * shard whose test drives a browser, its install and the compiler's tests
 * as it is over the budget before any mutant, took the most, 16, of one
 * mutation (#2220). The one rule that sizes a check's mutations' machines
 * (`mutationShards`) and its suite's (`testShards`), each by its own
 * shards' expected seconds. */
export function fewestShards(choices: readonly number[], expected: (count: number) => number[], budget: number): number {
  const slowest = choices.map((count) => Math.max(0, ...expected(count)));
  const fits = choices.findIndex((_, i) => slowest[i]! <= budget);
  if (fits !== -1) return choices[fits]!;
  const soonest = Math.min(...slowest);
  return choices[slowest.findIndex((seconds) => seconds <= soonest)]!;
}

/** What a test shard's run measured besides its files' times, which `bun
 * run test` keeps beside them in its timings, `shard` (scripts/test.ts):
 * of a shard of `count`, the rest of the suite's wall seconds, `rest`, and
 * its files' recorded seconds, `recorded`, by which each file's time is its
 * share of that wall (`testFileSeconds`); the pool of the files of cases,
 * its wall from its first part's start to its last's end, and its parts'
 * own seconds, the corpus's with how many cases they ran, the metamorphic
 * sample's; and the browsers' files run after both, their wall and how many. */
export type ShardRun = {
  count: number;
  /** Its machine's processor, as Node names it: GitHub's runners come in
   * tiers of speed (`ShardFacts`' `pace`), which a later plan may tell
   * apart by it. None of a run before 2026-11-19's. */
  cpu?: string;
  rest: number;
  recorded: number;
  pool: { wall: number; corpus: { seconds: number; cases: number }; metamorphic: { seconds: number } };
  browsers: { wall: number; files: number };
};

/** How long each test file took, in seconds, of the runs whose `bun test
 * --update-timings` files `paths` are (`{ version: 1, files: { [file]: ms },
 * shard? }`), the first that timed a file its time: of a run that kept what
 * its shard measured (`ShardRun`), the file's share of its rest's wall, its
 * recorded time times `rest / recorded`, as a mutation's time is its share
 * of its shard (`shareOfShard`). A file's recorded time is its wall beside
 * the others its run ran at once, so it's longer the more ran beside it:
 * of six runs' raw times, mostly a few files' runs, a check of nearly
 * everything took 1.58 times the 2,298 s plan expected (check
 * 44480685461). `measuredOnly`, `plan`'s, passes over a run that kept no
 * such share, an earlier one's; `plan.json`, which `plan` wrote of
 * shares, is read as it is. */
export function testFileSeconds(paths: string[], measuredOnly = false): Map<string, number> {
  const seconds = new Map<string, number>();
  for (const path of paths) {
    if (!existsSync(path)) continue;
    const timings: { files?: Record<string, number>; shard?: ShardRun } = JSON.parse(readFileSync(path, "utf8"));
    if (measuredOnly && timings.shard === undefined) continue;
    const share = timings.shard !== undefined && timings.shard.recorded > 0 ? timings.shard.rest / timings.shard.recorded : 1;
    for (const [file, ms] of Object.entries(timings.files ?? {})) if (!seconds.has(file)) seconds.set(file, (ms / 1000) * share);
  }
  return seconds;
}

/** What the test shards' runs of `paths` measured of the parts of a shard
 * its files' shares don't hold (`ShardRun`), for `plan` to expect each
 * shard's time by: a corpus case's seconds in the pool; the metamorphic
 * sample's whole seconds, a shard's times its count, the mean of the shards
 * that ran it; the pool's wall per second of its parts, as its width,
 * narrower while the rest runs, gives; a browsers' file's wall; and the
 * slowest pace of a run, its pool's wall over what these expect of it,
 * which `plan` expects every shard at, as a check waits on its slowest.
 * GitHub's standard runners come in tiers: the same build of the
 * compiler's three crates took 88 to 94 s, 104 to 125 s or 138 to 152 s in
 * 52 test shards of main's 34 checks of 2026-11-18 and 19, 20 the slowest,
 * so one of two shards is slow 62% of the time. Check 44493804830's two,
 * 678 s each expected at the mean, took 1,102 s on a slow one (its build
 * 149 s, its pool 827 s, pace 1.48) and 688 s on a fast one (88 s, 499 s,
 * 0.89). None where no run kept them. */
export type ShardFacts = { secondsPerCase: number; metamorphic: number; poolWallPerSecond: number; browserSeconds: number; pace: number };
export function shardFacts(paths: string[]): ShardFacts | undefined {
  const runs = paths.flatMap((path) => {
    if (!existsSync(path)) return [];
    const { shard }: { shard?: ShardRun } = JSON.parse(readFileSync(path, "utf8"));
    return shard === undefined ? [] : [shard];
  });
  if (runs.length === 0) return undefined;
  const sum = (of: (run: ShardRun) => number, runs_: ShardRun[] = runs) => runs_.reduce((total, run) => total + of(run), 0);
  const cases = sum((run) => run.pool.corpus.cases);
  const metamorphic = runs.filter((run) => run.pool.metamorphic.seconds > 0);
  const pooled = sum((run) => run.pool.corpus.seconds + run.pool.metamorphic.seconds);
  const pages = sum((run) => run.browsers.files);
  const secondsPerCase = cases === 0 ? 0 : sum((run) => run.pool.corpus.seconds) / cases;
  const metamorphicSeconds = metamorphic.length === 0 ? 0 : sum((run) => run.pool.metamorphic.seconds * run.count, metamorphic) / metamorphic.length;
  const poolWallPerSecond = pooled === 0 ? 0 : sum((run) => run.pool.wall) / pooled;
  // Each run's pool's wall over what the mean expects of its parts.
  const paces = runs.flatMap((run) => {
    const expected = (run.pool.corpus.cases * secondsPerCase + (run.pool.metamorphic.seconds > 0 ? metamorphicSeconds / run.count : 0)) * poolWallPerSecond;
    return expected > 0 ? [run.pool.wall / expected] : [];
  });
  return {
    secondsPerCase,
    metamorphic: metamorphicSeconds,
    poolWallPerSecond,
    browserSeconds: pages === 0 ? 0 : sum((run) => run.browsers.wall) / pages,
    pace: Math.max(1, ...paces),
  };
}

/** What each took of the runs' `timings.json`s under `dir`, in seconds, by
 * its name, of the files in their sorted order, a later one's over an
 * earlier's: none where there's none. */
export function timingsUnder(dir: string): Map<string, number> {
  const seconds = new Map<string, number>();
  if (!existsSync(dir)) return seconds;
  const files = readdirSync(dir, { recursive: true, withFileTypes: true })
    .filter((entry) => entry.isFile() && entry.name === "timings.json")
    .map((entry) => join(entry.parentPath, entry.name))
    .sort();
  for (const file of files) {
    const timings: Record<string, { build: number; tests: number }> = JSON.parse(readFileSync(file, "utf8"));
    for (const [name, time] of Object.entries(timings)) seconds.set(name, time.build + time.tests);
  }
  return seconds;
}

/** `timingsUnder`'s, of each folder a `timings.json` is in under `dir`, a
 * run's shards' downloaded each to its own, `mutation-timings-<n>/`: by
 * shard, so each is judged against its own machine (`normalizedShardTimes`). */
export function timingsByShard(dir: string): Map<string, Map<string, number>> {
  const shards = new Map<string, Map<string, number>>();
  if (!existsSync(dir)) return shards;
  const files = readdirSync(dir, { recursive: true, withFileTypes: true })
    .filter((entry) => entry.isFile() && entry.name === "timings.json")
    .map((entry) => entry.parentPath)
    .sort();
  for (const folder of files) shards.set(folder, timingsUnder(folder));
  return shards;
}

/** How many parts a file of cases runs in at once on `cores`, each a
 * process of its own, `RUST_JS_CASES=i/n` (test/support.ts): one for each
 * two cores. A case is a compile, then its runs, one core at a time, so
 * the cores a part leaves are the other files'. On 12 cores, 3 parts took
 * 85 s, 6 take 70 s, and 10 took 72 s, the rest twice as long and a test
 * of them timing out: it's bound by its CPU time. */
export function caseParts(cores: number): number {
  return Math.max(2, Math.floor(cores / 2));
}

/** The cores a part of a file of cases keeps busy, of `cores`, by
 * `caseParts`' rule: what a run that starts such parts beside other work
 * counts each as (`slots`). */
export function casePartCores(cores: number): number {
  return Math.max(1, Math.floor(cores / caseParts(cores)));
}

/** How many parts a file of cases is cut in where a pool of the `cores`
 * runs them, as `check:fast` does, each part a job it starts once a core
 * is free: one a core. A part keeps one core busy, as measured, not the
 * two `caseParts` gives it, and parts this small leave no part alone at
 * the end: on 12 cores, 6 parts of a minute kept the cores 63% busy, 12 of
 * 32 s beside the other jobs 92% (2026-11-07). */
export function pooledParts(cores: number): number {
  return Math.max(2, cores);
}

/** How many of the suite's parts of a file of cases, the corpus's and the
 * metamorphic sample's, `bun run test` runs at once on `cores` (its
 * pool, scripts/test.ts): one for each two cores while the rest of the
 * suite runs beside them, which keeps the others, and every core once it
 * has ended. Each part a share of `casePoolParts` small enough that none
 * is left alone at the end: with two parts a shard of all at once, CI's
 * shard 1 ran its two corpus parts alone on two of its four cores for 5.5
 * minutes after the rest ended (check 43700659375). */
export function casePoolWidth(cores: number, restRunning: boolean): number {
  return restRunning ? caseParts(cores) : pooledParts(cores);
}

/** How many parts of each file of cases a shard's pool runs on `cores`:
 * the corpus's two for each core, the metamorphic sample's, of some 20
 * cases each of 40 rewrites, one. */
export function casePoolParts(file: keyof typeof caseFiles, cores: number): number {
  return file === "corpus" ? 2 * pooledParts(cores) : pooledParts(cores);
}

/** The files of cases `bun run test` runs through its pool, apart from the
 * rest of the suite's `bun test --parallel`. */
export const caseFiles = { corpus: "test/corpus.test.ts", metamorphic: "test/metamorphic.test.ts" } as const;

/** Each part of the files of cases shard `index` of `count` runs on
 * `cores`, the metamorphic sample's first, as each part of it is longer. */
export function casePoolJobs(index: number, count: number, cores: number): { file: string; part: string }[] {
  return (["metamorphic", "corpus"] as const).flatMap((file) =>
    caseShardParts(index, count, casePoolParts(file, cores)).map((part) => ({ file: caseFiles[file], part })),
  );
}

/** The most memory, in GB, one job of each kind held at once, its process
 * and every one it started, sampled every 200 ms on a quiet Mac mini of 8
 * cores and 16 GB (2026-11-09, #1084), rounded up. A build of the
 * compiler's crate is the one that counts, rustc alone: 3.4 GB building it
 * whole, as a mutation's first build in its target is, or a batch's of many
 * files, 1.6 GB again after one file's change; `cargo test`'s 2.0 and
 * clippy's 1.8. A test file, or a part of the corpus or of a net, each a
 * compile at a time, holds 0.1 to 0.3, the largest 1.8, a crate of the
 * crate corpus 0.5 (2.1 GB, four at once); `check:fast --all` whole, its
 * pool held to the budget below, 5.0 GB. */
export const peakGB = { compilerBuild: 3.5, compilerCheck: 2, crate: 0.6, test: 0.3 };

/** The bytes free for a run as it starts, read once a process: on macOS
 * the kernel's own share, `memory_pressure`'s free percentage of the
 * memory, which counts what it can take back of the files' cache, 85% of
 * 16 GB on a quiet machine where `freemem()`'s pages never used were 27%;
 * elsewhere `freemem()`, Linux's `MemAvailable`. */
let available: number | undefined;
export function availableMemory(): number {
  if (available !== undefined) return available;
  const said = platform() === "darwin" ? spawnSync("memory_pressure", ["-Q"], { encoding: "utf8" }).stdout?.match(/free percentage: (\d+)%/) : null;
  available = said ? (Number(said[1]) / 100) * totalmem() : freemem();
  return available;
}

/** The GB the work a run starts at once may hold together: the machine's
 * memory, `total` bytes, less 6 GB, what the system, the agent and the apps
 * beside it keep, and no more than what's free as it starts, `free` bytes,
 * less 2 GB, where other apps hold more of it, a browser's or another
 * session's. On 16 GB, four builds of the compiler at once with a test on
 * each core, 16.4 GB, set off Claude Code's low-memory guard (#1084). */
export function memoryBudget(total = totalmem(), free = availableMemory()): number {
  return Math.max(peakGB.compilerBuild, Math.min(total / 2 ** 30 - 6, free / 2 ** 30 - 2));
}

/** The bytes one process of the work run at once may hold, a test's or
 * what it keeps running, a native run or a kept compiler (#2301): the
 * memory the machine's runs may hold together (`memoryBudget`) shared by
 * one on each core, as the most that run at once, at least 512 MB. */
export function testShareBytes(budgetGB = memoryBudget(), cores = machine().cores): number {
  return Math.max(512 * 2 ** 20, Math.floor((budgetGB * 2 ** 30) / cores));
}

/** The machine a run sizes itself by, as it starts: its cores,
 * `availableParallelism()`, a container's share of them where it has one,
 * and its memory, in GB, all of it and what's free (`availableMemory`).
 * The one authority on how much of the machine there is: every runner asks
 * `atOnce` how many of a kind of job to start together, or `machine()`
 * itself, never the OS (test/shards.test.ts holds it). */
export type Machine = { cores: number; totalGB: number; freeGB: number };
export function machine(): Machine {
  return { cores: availableParallelism(), totalGB: totalmem() / 2 ** 30, freeGB: availableMemory() / 2 ** 30 };
}

/** Each kind of job a runner starts many of, what it keeps busy and what
 * it holds at most (`peakGB`): a test file or a part of one, a compile at
 * a time, one core; a build of the compiler's crate, two; a crate's
 * native and JS Cargo builds and tests, the Crater's, two and some 2 GB
 * (2.1 GB of four at once, `peakGB`); a crate's `cargo check` and compile,
 * the crate corpus's, two and 0.6 GB. */
export const jobKinds = {
  test: { cores: 1, gb: peakGB.test },
  compilerBuild: { cores: 2, gb: peakGB.compilerBuild },
  crateBuild: { cores: 2, gb: 2 },
  crateCheck: { cores: 2, gb: peakGB.crate },
} as const;

/** How many jobs of `kind` a run starts at once on `on`: as many as its
 * cores keep busy and its memory budget (`memoryBudget`) holds, or `room`
 * GB of it where the run holds some apart, at least one. Fourteen tests
 * on 14 cores and 36 GB, eight on 8 and 16 GB, four on CI's four and 16
 * GB; seven of the compiler's builds on 14 cores, two on CI's. */
export function atOnce(kind: keyof typeof jobKinds, on: Machine = machine(), room = memoryBudget(on.totalGB * 2 ** 30, on.freeGB * 2 ** 30)): number {
  const job = jobKinds[kind];
  return Math.max(1, Math.min(Math.floor(on.cores / job.cores), Math.floor(room / job.gb)));
}

/** How many builds of the compiler's crate a run makes at once on `cores`,
 * each in a target of its own, as mutations' `--jobs` workers do: one for
 * each two cores, as a build keeps about two busy (`atOnce`), up to four,
 * and no more than the memory holds beside a test on each core. Past four
 * a build's own threads fill the cores: on 14 cores and 36 GB, seven builds
 * ran 656 mutations in 533 s where four ran 643 in 540 s, the load at 114
 * where it was 24 (2026-11-11). Two on CI's four cores and 16 GB, two on
 * 8 cores and 16 GB, where the cores alone said four; one there with 9 GB
 * of it free. */
export function compilerBuilds(cores: number, total = totalmem(), free = availableMemory()): number {
  const room = memoryBudget(total, free) - cores * peakGB.test;
  return Math.min(4, atOnce("compilerBuild", { cores, totalGB: total / 2 ** 30, freeGB: free / 2 ** 30 }, room));
}

/** The builds a run makes at once of `asked`, its `--jobs`: as many as it
 * asks, never more than the memory holds (`compilerBuilds`), which an ask
 * lowers and never raises; with whether it was cut. `--jobs=12` on a Mac of
 * 16 GB and 12 cores, where the memory held one, built twelve at once,
 * some 42 GB, and the host stopped the run for its memory (2026-11-20). */
export function compilerBuildsAsked(
  asked: number | undefined,
  cores: number,
  total = totalmem(),
  free = availableMemory(),
): { builds: number; cut: boolean } {
  const held = compilerBuilds(cores, total, free);
  if (asked === undefined || !(asked >= 1)) return { builds: held, cut: false };
  return { builds: Math.min(asked, held), cut: asked > held };
}

/** The parts, `i/n`, of a file of cases that shard `index` of `count`
 * runs, `perShard` of them at once: of `count` times as many in all, each
 * part in exactly one shard. */
export function caseShardParts(index: number, count: number, perShard: number): string[] {
  const parts = count * perShard;
  const all = Array.from({ length: parts }, (_, i) => i + 1);
  return all.filter((part) => (part - 1) % count === index - 1).map((part) => `${part}/${parts}`);
}

/** `--shard=2/4` among `args`: the shard and the count, or nothing. */
export function shardArg(args: string[]): { index: number; count: number } | undefined {
  const arg = args.find((a) => a.startsWith("--shard="));
  if (!arg) return undefined;
  const [index, count] = arg.slice("--shard=".length).split("/").map(Number);
  return { index, count };
}

/** Where the last run's shards' timings of each test file are kept, from
 * `target/`, for `bun run test` to balance this run's by: `bun
 * scripts/ci.ts test-timings` writes there what `plan` read of them, and
 * `bun run test` reads each file in it. */
export const shardTimings = "target/shard-timings";

/** Where the suite's runs keep their JUnit reports (`junitReport`), which
 * check.yml uploads as each test shard's `junit-<shard>`. */
export const junitReports = "target/junit";

/** `bun test`'s arguments for a JUnit report of the run `name` in `dir`,
 * the directory made first: bun writes no report into one that isn't
 * there, saying so and passing, and no test shard kept one until it was
 * made (check 44493804830's 14 `JUnitReportFailed` a shard), so a red
 * check's `attribute` saw no failing test of the suite, and the nightly no
 * suite's times. check.yml's upload fails a shard that kept none. */
export function junitReport(dir: string, name: string): string[] {
  mkdirSync(dir, { recursive: true });
  return ["--reporter=junit", `--reporter-outfile=${join(dir, `${name.replace(/\W+/g, "-")}.xml`)}`];
}

/** The files that run a browser, beside a dev server, Vite's, Next.js's,
 * the playground's and the pilot's, or on a page of its own,
 * webapi-events', webapi-crypto's and jsx-text-browser's: `bun run test` runs them after the rest, on their own,
 * as with the corpus's compiles they starve each other, and a page or a
 * Fast Refresh waits past its test's timeout. `check:fast --all` runs them
 * beside its other jobs, as its pool keeps the load near the cores (ADR
 * 1093, amended). */
export const browsers = ["browser", "deep-chain-next", "jsx-text-browser", "next", "playground", "pilot", "vite", "webapi-crypto", "webapi-events"].map(
  (name) => `test/${name}.test.ts`,
);

/** Whether test file `file`, of the repository at `root`, drives a
 * browser: one of `browsers`, or one that, or a module of the tests' own it
 * imports, `./browser-waits`, imports Playwright, as each file whose row
 * runs in Chromium too does, none-given's. Read of the file, so no list of
 * its own. What needs one installed asks it, CI's mutations (ADR 0093). */
export function drivesBrowser(root: string, file: string): boolean {
  if (browsers.includes(file)) return true;
  const path = join(root, file);
  if (!existsSync(path)) return false;
  const source = readFileSync(path, "utf8");
  const playwright = (text: string) => /from "playwright"|from "@playwright\//.test(text);
  if (playwright(source)) return true;
  return [...source.matchAll(/from "(\.\/[^"]+)"/g)].some(([, local]) => {
    const imported = [`${local}.ts`, local].map((name) => join(dirname(path), name)).find((name) => existsSync(name) && name.endsWith(".ts"));
    return imported !== undefined && playwright(readFileSync(imported, "utf8"));
  });
}

/** What only the nightly's suite runs, `bun run test --nightly`, not each
 * push's (ADR 1030): what the nightly runs in full anyway, tidy's stages,
 * the iterator forms, the corpus compiled again (#866), generated programs
 * and the declarations' generated callers (ADR 1091); the release's
 * packaging, which qualification runs too; and React's releases and the
 * harness's own checks. Each is CPU a push's corpus waits on, guards no
 * mutation or a few, and failed none of the 80 red checks before it was
 * moved: a mutation that names one still runs it. Never what a change can
 * leave untrue unseen, as the guide's limitations and the bindings'
 * coverage ratchets, which plain `check:fast` and each push run (ADR 1030,
 * amended). And the iterator matrix, whose slice a check whose change is
 * the iterators' code runs (check.yml's `nets-iterators` job's `iterators` step,
 * scripts/iteratorSlice.ts, ADR 1090). */
export const nightly = [
  "tidy-stages",
  "getter-reads",
  "iterator-forms",
  "determinism",
  "mut-ref-matrix",
  "bounds-matrix",
  "memory-net",
  "none-matrix",
  "object-model",
  "iterator-matrix",
  "declarations-conformance",
  "fuzz",
  "match-trees",
  "packages",
  "create",
  "npm-crates",
  "release",
  "stable",
  "crates-io",
  "registry-crates",
  "compat-versions",
  "react-versions",
  "editor-check",
  "harness",
  "history",
].map((name) => `test/${name}.test.ts`);

/** The push suite's test files of `names`, the test folder's entries,
 * `test/<name>.test.ts` each: every one but the nightly's alone
 * (`nightly`), as `bun run test` runs them. What the test map records and
 * the selector selects of (scripts/select.ts). The folder is read by its
 * callers, not here, which every test imports. */
export function pushSuiteFiles(names: string[]): string[] {
  return names
    .filter((name) => name.endsWith(".test.ts"))
    .map((name) => `test/${name}`)
    .filter((file) => !nightly.includes(file))
    .sort();
}

/** A part of a test file that a setting of its environment selects, and
 * where it runs: `push`, by `by`, a workflow each push to `main` starts or
 * a script one runs, which sets it; or `nightly`, by the nightly alone,
 * with why. The corpus with std's iterator structs ran only at night, and a
 * regression only it showed reached `main` (#766). */
export type Part = { setting: string; value: string; runs: "push" | "nightly"; by: string; why?: string };

/** Each part a setting selects, of every setting a test file reads but
 * `harness`'s: test/ci.test.ts fails on a setting that's neither, and on a
 * part whose `by` doesn't set it. */
export const parts: Part[] = [
  { setting: "ITERATOR_TABLE", value: "matrix", runs: "push", by: "scripts/iteratorSlice.ts" },
  { setting: "ITERATOR_TABLE", value: "conformance", runs: "push", by: "scripts/iteratorSlice.ts" },
  // Each adapter through each step std's writes, by each path (#2112).
  { setting: "ITERATOR_TABLE", value: "paths", runs: "push", by: "scripts/iteratorSlice.ts" },
  { setting: "ITERATOR_FORMS", value: "corpus", runs: "push", by: "scripts/iteratorSlice.ts" },
  // Of each change, what scripts/iteratorRows.ts picks: `/<regex>/`, the
  // rows it names, `pairs`, a sample of each adapter with each consumer, or
  // both (#818).
  { setting: "ITERATOR_MATRIX", value: "p.setting", runs: "push", by: "scripts/iteratorSlice.ts" },
  // Of a change to the formatting code, the formatting grid's slice (#941);
  // the nightly's `format` runs every batch.
  { setting: "ITERATOR_TABLE", value: "format", runs: "push", by: "scripts/iteratorSlice.ts" },
  // Of a change to the numbers' code, the numbers grid's slice (#940).
  { setting: "ITERATOR_TABLE", value: "numbers", runs: "push", by: "scripts/iteratorSlice.ts" },
  // Of a change to how a future awaits and drops, the async table (#942).
  { setting: "ITERATOR_TABLE", value: "async", runs: "push", by: "scripts/iteratorSlice.ts" },
  // Of a change to the iterators' code or the drops', the consumer net.
  { setting: "ITERATOR_TABLE", value: "consumers", runs: "push", by: "scripts/iteratorSlice.ts" },
  // Of a change to std's collections', slices', strings' or cells' code, the std net (#1171).
  { setting: "ITERATOR_TABLE", value: "std", runs: "push", by: "scripts/iteratorSlice.ts" },
  // Of a change to the iterators' code, the runtime's among it, the twin
  // net: each table against std's own bodies, no native run (#1205).
  { setting: "ITERATOR_ORACLE", value: "std", runs: "push", by: "scripts/iteratorSlice.ts" },
  {
    setting: "ITERATOR_TABLE",
    value: "points",
    runs: "nightly",
    by: ".github/workflows/nightly.yml",
    why: "every case at each of its panic points, 105 batches: a push's gate is #818's to choose",
  },
  {
    setting: "ITERATOR_TABLE",
    value: "unwind",
    runs: "nightly",
    by: ".github/workflows/nightly.yml",
    why: "every operation beyond iterators at each of its panic points, 58 batches, about 5 CPU-minutes: a push's gate is #818's to choose",
  },
  { setting: "TIDY_STAGES", value: "programs", runs: "push", by: ".github/workflows/readability-passes.yml" },
  // Of a change to tidy or preparation, their variants alone; of one to
  // the facts MIR's intervals keep, theirs (#2112 B).
  { setting: "TIDY_VARIANTS", value: "readability", runs: "push", by: ".github/workflows/readability-passes.yml" },
  { setting: "TIDY_VARIANTS", value: "intervals", runs: "push", by: "scripts/iteratorSlice.ts" },
  { setting: "FUZZ_START", value: "1", runs: "push", by: ".github/workflows/readability-passes.yml" },
  { setting: "FUZZ_SEEDS", value: "6", runs: "push", by: ".github/workflows/readability-passes.yml" },
  { setting: "FUZZ_MIRI", value: "1", runs: "nightly", by: ".github/workflows/rustc-tests.yml", why: "each generated program run under Miri first, which needs the nightly toolchain's Miri, which the batch's job installs (#1085)" },
  { setting: "RUST_JS_REQUIRE_WASM", value: "1", runs: "push", by: ".github/workflows/check.yml" },
  // A run of the mutations that need the wasm, the wasm job's (#2346).
  { setting: "RUST_JS_REQUIRE_WASM", value: "1", runs: "push", by: "scripts/mutations.ts" },
  { setting: "RUST_JS_REQUIRE_MIRI", value: "1", runs: "nightly", by: ".github/workflows/rustc-tests.yml", why: "the reducer's tests that need Miri, which the fuzz job installs and no push's job has (#1104)" },
  { setting: "ITERATOR_MATRIX", value: "all", runs: "nightly", by: ".github/workflows/nightly.yml", why: "every case, about 4 CPU-hours: a push runs its slice" },
  { setting: "TIDY_STAGES", value: "corpus", runs: "nightly", by: ".github/workflows/nightly.yml", why: "every corpus case after each of tidy's stages: a push of tidy runs nine" },
  {
    setting: "RUST_JS_WORKER_CASES",
    value: "all",
    runs: "nightly",
    by: ".github/workflows/nightly.yml",
    why: "every case that runs, its builds compiled alone and as the worker's jobs in both orders (ADR 1182): a push compiles a sample",
  },
  {
    setting: "DETERMINISM",
    value: "corpus",
    runs: "nightly",
    by: ".github/workflows/nightly.yml",
    why: "every corpus case compiled twice more, about 25 CPU-minutes: the corpus's own snapshots compare each push's one compile",
  },
  { setting: "MUT_REF_MATRIX", value: "all", runs: "nightly", by: ".github/workflows/nightly.yml", why: "all 210 programs: check:fast runs twelve where a `&mut` changes" },
  { setting: "METAMORPHIC", value: "all", runs: "nightly", by: ".github/workflows/nightly.yml", why: "every corpus case's every rewrite, some 12,000 programs of each build: a push runs one case in 128 (#939, #1008)" },
  { setting: "MEMORY_NET", value: "all", runs: "nightly", by: ".github/workflows/nightly.yml", why: "400 seeds from the day's, each under Miri: a push runs the slice's 24 where the pointers' code changes (#1172)" },
  { setting: "NONE_MATRIX", value: "all", runs: "nightly", by: ".github/workflows/nightly.yml", why: "all 580 programs: check:fast runs six where a `None`'s form changes" },
  { setting: "OBJECT_MODEL", value: "all", runs: "nightly", by: ".github/workflows/nightly.yml", why: "every corpus case's library run with its values proxied, some 1,900: check:fast runs one case in 64 where the object model's owners or the readability passes change (#891)" },
  { setting: "NEIGHBOURS", value: "all", runs: "nightly", by: ".github/workflows/nightly.yml", why: "every group of the day's seed, some 120 crates of the corpus's cases: a push runs seed 0's first two and each listed pair (#853)" },
  { setting: "RUSTLANTIS_SEEDS", value: "1000", runs: "nightly", by: ".github/workflows/nightly.yml", why: "1,000 of Rustlantis's custom MIR programs from the day's seed, its generator built first: a push runs the net's list's own tests (ADR 1125)" },
  { setting: "RUST_JS_DISTRIBUTION", value: "dist", runs: "nightly", by: "scripts/qualify.ts", why: "a release's packaged binary, which only qualification builds" },
];

/** The settings test files read that select no part: the harness's own,
 * what a test sets for a child it runs, the printer of declarations its
 * compiles name (test/setup.ts), the temp root it gives every test process
 * (test/bounded.ts), Miri's sysroot, which a run asks for once
 * (test/miri.ts), a report's, decision coverage's counts, which the
 * compile cache never keeps a compile of (ADR 1124), and the share of kept
 * native runs built and run again, the nightly's, which its jobs' native
 * outcomes action sets (test/native-outcomes.ts). */
export const harness = [
  "BLESS",
  // Where Cargo keeps its registry, a native run's files named under it
  // as rust-js names them (test/bounded.ts's `cargoHome`, #2325).
  "CARGO_HOME",
  "BUN_TEST_WORKER_ID",
  "CHILD_LEFT_OUT",
  "FUZZ_REDUCE_BUDGET",
  "FUZZ_REPORT",
  "ITERATOR_MATRIX_REPORT",
  "MEMORY_NET_START",
  "METAMORPHIC_REWRITES",
  "METAMORPHIC_TALLY",
  "MIRI_SYSROOT",
  "NET_REDUCE",
  "NET_REDUCE_BUDGET",
  "NEIGHBOURS_SEED",
  "OBJECT_MODEL_TALLY",
  "PATH",
  "PILOT_API",
  "RUSTC_BOOTSTRAP",
  "RUSTLANTIS_START",
  "RUST_JS_APPS_SEED",
  "RUST_JS_CASES",
  "RUST_JS_COMPILE_CACHE_MB",
  "RUST_JS_COMPILE_TIMEOUT",
  "RUST_JS_COMPILER",
  "RUST_JS_DECISIONS",
  "RUST_JS_DEV_TIMES",
  "RUST_JS_KEEP_FIXTURES",
  "RUST_JS_MUTANT",
  "RUST_JS_NATIVE_RECHECK",
  "RUST_JS_OUTCOME_VALUE",
  "RUST_JS_PHASE_TIMES",
  "RUST_JS_SNAPSHOTS",
  "RUST_JS_SYSTEM_TMPDIR",
  "RUST_JS_TARGET",
  "RUST_JS_TEST_VALUE",
  "RUST_JS_TYPESCRIPT",
  "TMPDIR",
];
