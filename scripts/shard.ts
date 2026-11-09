// A set of work split across machines that run at once, as CI runs the
// mutations (DEVELOPMENT.md): shard `index` of `count`. (`bun test` splits
// the suite itself, `--shard`, by how long each file took.) And how much
// of a machine's cores and memory the work run at once on it may hold.

import { spawnSync } from "node:child_process";
import { freemem, platform, totalmem } from "node:os";

/** Shard `index`, from 1, of `count`: the items, sorted, dealt out in turn,
 * so each is in exactly one shard, whatever order they're given in. */
export function shard<T extends string>(items: T[], index: number, count: number): T[] {
  if (!Number.isInteger(index) || !Number.isInteger(count) || index < 1 || index > count) {
    throw new Error(`a shard is 1 to ${count}, not ${index}`);
  }
  return [...items].sort().filter((_, i) => i % count === index - 1);
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

/** How many parts a file of cases is cut in where a pool of the `cores`
 * runs them, as `check:fast` does, each part a job it starts once a core
 * is free: one a core. A part keeps one core busy, as measured, not the
 * two `caseParts` gives it, and parts this small leave no part alone at
 * the end: on 12 cores, 6 parts of a minute kept the cores 63% busy, 12 of
 * 32 s beside the other jobs 92% (2026-11-07). */
export function pooledParts(cores: number): number {
  return Math.max(2, cores);
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

/** How many builds of the compiler's crate a run makes at once on `cores`,
 * each in a target of its own, as mutations' `--jobs` workers do: one for
 * each two cores, as a build keeps about two busy, up to four (DEVELOPMENT.md's
 * timings of sixteen mutations), and no more
 * than the memory holds beside a test on each core. Four on 12 cores and
 * 32 GB, two on CI's four cores and 16 GB, and two on 8 cores and 16 GB,
 * where the cores alone said four; one there with 9 GB of it free. */
export function compilerBuilds(cores: number, total = totalmem(), free = availableMemory()): number {
  const room = memoryBudget(total, free) - cores * peakGB.test;
  return Math.max(1, Math.min(4, Math.floor(cores / 2), Math.floor(room / peakGB.compilerBuild)));
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
 * `target/`, for `bun test --shard` to balance this run's by: `bun
 * scripts/ci.ts timings` downloads them there, and `bun run test` gives
 * each file in it. */
export const shardTimings = "target/shard-timings";

/** The files that run a browser, beside a dev server, Vite's, Next.js's,
 * the playground's and the pilot's, or on a page of its own,
 * webapi-events': `bun run test` runs them after the rest, on their own,
 * as with the corpus's compiles they starve each other, and a page or a
 * Fast Refresh waits past its test's timeout. `check:fast --all` runs them
 * beside its other jobs, as its pool keeps the load near the cores (ADR
 * 1093, amended). */
export const browsers = ["browser", "next", "playground", "pilot", "vite", "webapi-events"].map(
  (name) => `test/${name}.test.ts`,
);

/** What only the nightly's suite runs, `bun run test --nightly`, not each
 * push's (ADR 1030): what the nightly runs in full anyway, tidy's stages,
 * the iterator forms, the corpus compiled again (#866), generated programs
 * and the declarations' generated callers (ADR 1091); the release's
 * packaging, which
 * qualification runs too; and the ratchets, React's releases and the
 * harness's own checks. Each is CPU a push's corpus waits on, guards no
 * mutation or a few, and failed none of the 80 red checks before it was
 * moved: a mutation that names one still runs it. And the iterator
 * matrix, whose slice a check whose change is the iterators' code runs
 * on its own (check.yml's `iterators`, scripts/iteratorSlice.ts, ADR 1090). */
export const nightly = [
  "tidy-stages",
  "getter-reads",
  "iterator-forms",
  "determinism",
  "mut-ref-matrix",
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
  "react-versions",
  "next-coverage",
  "react-coverage",
  "webapi-coverage",
  "node-coverage",
  "editor-check",
  "harness",
  "history",
  "limitations",
].map((name) => `test/${name}.test.ts`);

/** The suites plain `check:fast` runs of a change, each file a job of its
 * pool's (scripts/checkFast.ts), by the starts of their files' paths: what
 * checks the JS bodies are lowered to, and, where the change is the
 * compiler's or the runtime's (`shared`), what pins the JS written, the
 * names it gives and what it refuses, the guide's examples compiled, std's
 * package, the runtime package's helpers and the readability ratchet, each
 * of which a lowering or a runtime change has turned `main` red with (#212,
 * #222, #253); JSX's where it's JSX's. The test map records what each of
 * their files reaches of the compiler, and the corpus's each case, so that a
 * change runs only those it can affect (scripts/testMap.ts, #877). */
export const plainSuites: { name: string; starts: string[]; when?: "shared" | "jsx" }[] = [
  { name: "architecture, mutation lists", starts: ["architecture.test.ts", "architecture-iterators.test.ts", "architecture-nones.test.ts", "architecture-cleanups.test.ts", "architecture-writes.test.ts", "mutations.test.ts"] },
  // Each a file of a concept's (ADR 0572), `test/lowering` all of lowering's.
  { name: "lowering", starts: ["lowering"] },
  { name: "compiler", starts: ["compiler.test.ts"] },
  {
    name: "bindings",
    starts: ["bindings.test.ts", "bindings-require.test.ts", "js-builtins.test.ts", "js-errors.test.ts", "node-bindings.test.ts", "webapi-bindings.test.ts"],
  },
  { name: "JSX", starts: ["jsx"], when: "jsx" },
  { name: "guide, packages, readability", starts: ["docs.test.ts", "std-package.test.ts", "runtime-package.test.ts", "readability.test.ts"], when: "shared" },
  { name: "React's ports", starts: ["port-"], when: "shared" },
  { name: "snapshots", starts: ["snapshots.test.ts"], when: "shared" },
  { name: "diagnostics", starts: ["diagnostics.test.ts"], when: "shared" },
];

/** Each plain suite's test files, of `files`, `test/<name>.test.ts` each. */
export function plainSuiteFiles(files: string[]): { name: string; files: string[]; when?: "shared" | "jsx" }[] {
  return plainSuites.map(({ name, starts, when }) => ({
    name,
    when,
    files: files.filter((file) => file.endsWith(".test.ts") && starts.some((start) => file.startsWith(`test/${start}`))).sort(),
  }));
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
  { setting: "FUZZ_START", value: "1", runs: "push", by: ".github/workflows/readability-passes.yml" },
  { setting: "FUZZ_SEEDS", value: "6", runs: "push", by: ".github/workflows/readability-passes.yml" },
  { setting: "FUZZ_MIRI", value: "1", runs: "nightly", by: ".github/workflows/rustc-tests.yml", why: "each generated program run under Miri first, which needs the nightly toolchain's Miri, which the batch's job installs (#1085)" },
  { setting: "RUST_JS_REQUIRE_WASM", value: "1", runs: "push", by: ".github/workflows/check.yml" },
  { setting: "RUST_JS_REQUIRE_MIRI", value: "1", runs: "nightly", by: ".github/workflows/rustc-tests.yml", why: "the reducer's tests that need Miri, which the fuzz job installs and no push's job has (#1104)" },
  { setting: "ITERATOR_MATRIX", value: "all", runs: "nightly", by: ".github/workflows/nightly.yml", why: "every case, about 4 CPU-hours: a push runs its slice" },
  { setting: "TIDY_STAGES", value: "corpus", runs: "nightly", by: ".github/workflows/nightly.yml", why: "every corpus case after each of tidy's stages: a push of tidy runs nine" },
  {
    setting: "DETERMINISM",
    value: "corpus",
    runs: "nightly",
    by: ".github/workflows/nightly.yml",
    why: "every corpus case compiled twice more, about 25 CPU-minutes: the corpus's own snapshots compare each push's one compile",
  },
  { setting: "MUT_REF_MATRIX", value: "all", runs: "nightly", by: ".github/workflows/nightly.yml", why: "all 210 programs: check:fast runs twelve where a `&mut` changes" },
  { setting: "METAMORPHIC", value: "all", runs: "nightly", by: ".github/workflows/nightly.yml", why: "every corpus case's every rewrite, some 12,000 programs of each build: a push runs one case in 128 (#939, #1008)" },
  { setting: "NONE_MATRIX", value: "all", runs: "nightly", by: ".github/workflows/nightly.yml", why: "all 580 programs: check:fast runs six where a `None`'s form changes" },
  { setting: "OBJECT_MODEL", value: "all", runs: "nightly", by: ".github/workflows/nightly.yml", why: "every corpus case's library run with its values proxied, some 1,900: check:fast runs one case in 64 where the object model's owners or the readability passes change (#891)" },
  { setting: "NEIGHBOURS", value: "all", runs: "nightly", by: ".github/workflows/nightly.yml", why: "every group of the day's seed, some 120 crates of the corpus's cases: a push runs seed 0's first two and each listed pair (#853)" },
  { setting: "RUST_JS_DISTRIBUTION", value: "dist", runs: "nightly", by: "scripts/qualify.ts", why: "a release's packaged binary, which only qualification builds" },
];

/** The settings test files read that select no part: the harness's own,
 * what a test sets for a child it runs, the printer of declarations its
 * compiles name (test/setup.ts), Miri's sysroot, which a run asks for once
 * (test/miri.ts), and a report's. */
export const harness = [
  "BLESS",
  "BUN_TEST_WORKER_ID",
  "CHILD_LEFT_OUT",
  "FUZZ_REPORT",
  "ITERATOR_MATRIX_REPORT",
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
  "RUST_JS_APPS_SEED",
  "RUST_JS_CASES",
  "RUST_JS_COMPILE_CACHE_MB",
  "RUST_JS_COMPILE_TIMEOUT",
  "RUST_JS_COMPILER",
  "RUST_JS_KEEP_FIXTURES",
  "RUST_JS_LIBRARIES",
  "RUST_JS_MUTANT",
  "RUST_JS_SNAPSHOTS",
  "RUST_JS_TARGET",
  "RUST_JS_TEST_VALUE",
  "RUST_JS_TYPESCRIPT",
];
