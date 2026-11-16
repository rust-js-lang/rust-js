// What the CI workflow runs where (DEVELOPMENT.md): the suite split into
// shards, the mutations' shards, and each branch's latest run; which of
// the mutations a change touches is test/mutation-changes.test.ts's.

import { expect, test } from "bun:test";
import { existsSync, mkdirSync, mkdtempSync, readdirSync, readFileSync, rmSync, statSync, utimesSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { checkedSince, latestRuns, MUTATION_SHARDS, mutationJobsPassed, mutationShards, mutationTimingsRun, nets, netsSummary, alwaysRun, resultsBase, testShards, timingsRun } from "../scripts/ci";
import { readersOf } from "./part-settings";
import { corpusRows, crashes, dependency, lockedVersion, lockOf, sampledHeader, sampledIn, sampledOneIn, written } from "../scripts/crate-corpus";
import { mutations, site } from "../scripts/mutations";
import { installNeeded, jobs, leftToCi, manifestsHash, readabilityPaths, type Scope, touchesReadability } from "../scripts/checkFast";
import { lostObject } from "../scripts/testMap";
import { changedCases, runs, touchesCrates, importsOf, touchesPort, touchesRustc, touchesWasm, wasmInputs } from "../scripts/iteratorSlice";
import { browsers, drivesBrowser } from "../scripts/shard";
import { loadScale, underLoad } from "../scripts/timing";
import { caseOf, partsOf, runParts, verdict } from "../scripts/parts";
import { jobs as localNightly } from "../scripts/nightly";
import { atOnce, caseFiles, caseParts, casePoolJobs, casePoolParts, casePoolWidth, caseShardParts, compilerBuilds, harness, memoryBudget, nightly, parts, peakGB, pooledParts, shard, timedShard, balanced } from "../scripts/shard";
import { fixture, programFlags, root } from "./support";
import { miriCommand } from "./miri";

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

// Every shard of a check balances by one nightly's times, `plan`'s: the
// newest of the last nightlies that kept them; none, and they're dealt in
// turn. One a shard picked itself might be another's, a newer one ending
// between their starts, and a mutation run twice or never.
test("a check's mutation shards balance by the newest nightly that kept its mutations' times", () => {
  const artifact = (name: string, run: number, expired = false) => ({ name, run, createdAt: "2026-11-15T00:00:00Z", expired });
  const artifacts = [artifact("mutation-timings-1", 9), artifact("mutation-timings-1", 7, true), artifact("mutation-timings-1", 5), artifact("timings-1", 8), artifact("mutation-timings-1", 3)];
  expect(mutationTimingsRun([8, 7, 5, 3], artifacts)).toBe(5);
  expect(mutationTimingsRun([8, 7], artifacts)).toBeUndefined();
  const check = readFileSync(join(root, ".github/workflows/check.yml"), "utf8");
  expect(check).toContain("mutation_timings: ${{ needs.plan.outputs.mutation_timings }}");
  const tests = readFileSync(join(root, ".github/workflows/rustc-tests.yml"), "utf8");
  const runs = [...tests.matchAll(/bun scripts\/mutations\.ts \$ARGS (.*)--shard=/g)].map((m) => m[1]);
  expect(runs).toEqual(["${{ steps.timings.outputs.args }} ", "${{ steps.timings.outputs.args }} "]);
  // A run of every mutation, the nightly's, builds 5% of its builds again
  // fresh (ADR 0093's incremental amendment).
  expect(tests).toContain("ARGS: ${{ inputs.mutation_args || '--fresh=0.05' }}");
});

// The workflows' caches are one entry for each set of inputs, not one a
// commit: six caches of some 450 MB an entry, each a push's, held the
// repository's 10 GB full and evicted one another (docs/dev-loop.md).
// And the native programs' kept runs are restored by any run and saved by
// main's alone, which a branch's check can't write over. The test maps are
// one a commit, held to the newest 8 by test-map-prune.yml (ADR 1101,
// amended), about 180 MB.
test("no cache is keyed by its commit but the test maps, held to 8, and only main's runs save the native outcomes", () => {
  const yamls = [".github/workflows", ".github/actions/build-rust-js", ".github/actions/rustc-tests-setup", ".github/actions/native-outcomes"].flatMap((dir) =>
    readdirSync(join(root, dir)).filter((file) => /\.ya?ml$/.test(file)).map((file) => join(dir, file)),
  );
  const keyed = yamls.flatMap((file) =>
    [...readFileSync(join(root, file), "utf8").matchAll(/^\s*key: .*$/gm)].filter((m) => /github\.sha|env\.SHA/.test(m[0])).map((m) => `${file}: ${m[0].trim()}`),
  );
  expect([...new Set(keyed)]).toEqual([".github/workflows/test-map.yml: key: test-map-${{ runner.os }}-${{ hashFiles('rust-toolchain.toml') }}-${{ env.SHA }}"]);
  expect(readFileSync(join(root, ".github/workflows/test-map-prune.yml"), "utf8")).toContain('prune-cache --keep=8 --prefix="test-map-${{ runner.os }}-"');
  const outcomes = readFileSync(join(root, ".github/actions/native-outcomes/action.yml"), "utf8");
  expect(outcomes).toContain("- if: inputs.step == 'save' && github.ref == 'refs/heads/main'\n      uses: actions/cache/save@");
  expect(outcomes).not.toMatch(/uses: actions\/cache@/);
  // Each job that restores them saves them.
  for (const file of [".github/workflows/check.yml", ".github/workflows/nightly.yml"]) {
    const text = readFileSync(join(root, file), "utf8");
    expect([file, text.match(/step: restore/g)?.length]).toEqual([file, text.match(/step: save/g)?.length]);
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
  expect(found.filter((f) => !f.startsWith("test/ci.test.ts"))).toEqual([]);
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

// A compiler CI keeps is of what it's made of, build.rs's inputs, the one
// list: a key short of one, std/manifest.json, which src/library.rs
// includes, restored a binary of other sources. The suite's shards restore
// the debug one so, as the release one is (check 43700659375: 136 s of
// each shard's longest leg built it again of unchanged sources).
test("a compiler CI keeps is keyed by build.rs's inputs, and the suite's shards run the one kept", () => {
  const script = readFileSync(join(root, "build.rs"), "utf8");
  const inputs = [...script.slice(script.indexOf("let inputs = ["), script.indexOf("];", script.indexOf("let inputs = ["))).matchAll(/join\("([^"]+)"\)/g)].map((m) => (m[1] === "src" ? "src/**" : m[1]!));
  expect(inputs).toEqual(["src/**", "std/manifest.json", "rust-toolchain.toml", "build.rs", "Cargo.toml", "Cargo.lock"]);
  const action = readFileSync(join(root, ".github/actions/build-rust-js/action.yml"), "utf8");
  expect(action).toContain(`key: rust-js-\${{ inputs.profile }}-\${{ runner.os }}-\${{ hashFiles(${inputs.map((input) => `'${input}'`).join(", ")}) }}`);
  const check = Bun.YAML.parse(readFileSync(join(root, ".github/workflows/check.yml"), "utf8")) as { jobs: Record<string, { steps: { uses?: string; with?: Record<string, string>; id?: string; env?: Record<string, string> }[] }> };
  const steps = check.jobs.test!.steps;
  expect(steps.filter((step) => step.uses === "./.github/actions/build-rust-js").map((step) => step.with)).toEqual([{ profile: "debug" }]);
  expect(steps.find((step) => step.id === "test")!.env!.RUST_JS_COMPILER).toBe(["${{ github.workspace }}", "target", "debug", "rust-js"].join("/"));
  expect(readFileSync(join(root, "scripts/test.ts"), "utf8")).toContain("if (!process.env.RUST_JS_COMPILER) {");
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

// What only the nightly's suite runs (ADR 1030): each a test file there is,
// and run, by the check the nightly calls, which says `--nightly` to the
// suite and to its bless, where a push's check gives both its selection.
test("the files only the nightly runs are test files, which its check runs", () => {
  const tests = readdirSync(join(root, "test")).map((file) => `test/${file}`);
  for (const file of nightly) expect(tests).toContain(file);
  const check = readFileSync(join(root, ".github/workflows/check.yml"), "utf8");
  const suites = check.split("\n").filter((line) => /bun (run test|scripts\/test\.ts) --shard=/.test(line));
  expect(suites).toHaveLength(2);
  for (const line of suites) expect(line).toContain("${{ inputs.nightly && ' --nightly' || ' --selection=target/selection.json' }}");
});

// `check:fast --all`, the check before a push, runs what a push's check
// runs (#732): each command of check.yml's jobs is one of its jobs' (`ci`),
// or one `leftToCi` says why it leaves to CI, and each of those is still
// check.yml's. `cargo test` and the pilot's committed JS, run by the check
// alone, went red on `main` unseen (801d40c55, 336984bff).
// Of a change to the iterators' code, the formatting code, the numbers' and the pointers', whose
// jobs check.yml runs where its change is one (scripts/iteratorSlice.ts).
const checkJobs = jobs({ all: true, changed: [], branch: ["src/runtime/steps.js", "src/runtime/pad.js", "src/runtime/div.js", "src/typescript.rs", "src/lower/mir/awaits.rs", "src/lower/unions.rs", "test/corpus/array_map.rs"], added: [], extra: [], cores: 4 });
const ranHere = checkJobs.jobs;

// The readability passes' oracles run of a change to tidy, preparation or
// the oracles alone, `--all`'s too, as a push's CI runs them: of
// readability-passes.yml's paths, the one list (the user, #998).
test("the readability passes run where readability-passes.yml's paths say, of --all too", () => {
  const paths = readabilityPaths();
  expect(paths).toEqual(expect.arrayContaining(["src/prepare/**", "src/lower/mir/tidy/**", "test/getter-reads/**", ".github/workflows/readability-passes.yml"]));
  const named = (scope: { all: boolean; changed: string[]; branch: string[] }) =>
    jobs({ ...scope, added: [], extra: [], cores: 12 }).jobs.some((job) => job.name === "readability passes");
  expect(named({ all: true, changed: ["src/lower/copies.rs"], branch: [] })).toBe(false);
  expect(named({ all: true, changed: [], branch: ["src/lower/mir/tidy/exits.rs"] })).toBe(true);
  expect(named({ all: false, changed: ["src/prepare.rs"], branch: [] })).toBe(true);
  expect(named({ all: false, changed: ["test/getter-reads/counts.rs"], branch: [] })).toBe(true);
  expect(touchesReadability(["src/lower/mir/tidying.rs", "docs/guide/README.md"])).toBe(false);
  expect(() => readabilityPaths("on:\n  push:\n")).toThrow("no `on.push.paths`");
});

// `--all` runs the metamorphic net's push sample in parts, as the corpus's,
// each a process the pool starts where a core is free: one `bun test` of
// it, 204 to 238 s on about one core, ran on alone at a check's end, past a
// consumer's 720 s (#1749). Each part once, `RUST_JS_CASES=i/n`, none of
// the whole file beside them.
test("--all runs the metamorphic sample in parts, each once, and never the whole file beside them", () => {
  const sample = (cores: number) =>
    jobs({ all: true, changed: [], branch: [], added: [], extra: [], cores }).jobs.filter((job) => job.cmd.join(" ") === "bun test test/metamorphic.test.ts");
  for (const cores of [4, 12]) {
    const parts = pooledParts(cores);
    expect(sample(cores).map((job) => job.env.RUST_JS_CASES)).toEqual(Array.from({ length: parts }, (_, i) => `${i + 1}/${parts}`));
  }
});

// The crate corpus runs of a change to what compiles it, plain check:fast's
// and each push's (`plan`'s `crates`), not only `--all`'s and the
// nightly's: 9353847cd refused itertools unseen (#1188).
test("the crate corpus runs where its paths say, of plain check:fast too", () => {
  const none = { files: new Set<string>(), cases: [], caseList: "/c.txt", forms: [], formsList: "/f.txt" };
  const named = (scope: { all: boolean; changed: string[]; branch: string[] }) =>
    jobs({ ...scope, added: [], extra: [], cores: 12, picked: none }).jobs.some((job) => job.name === "the crate corpus");
  expect(named({ all: false, changed: ["src/lower/mir/iter/closing.rs"], branch: [] })).toBe(true);
  expect(named({ all: false, changed: [], branch: ["src/runtime/byte_len.js"] })).toBe(true);
  expect(named({ all: false, changed: ["scripts/crate-corpus/itertools.rs"], branch: [] })).toBe(true);
  expect(named({ all: false, changed: ["docs/guide/README.md", "test/ci.test.ts"], branch: [] })).toBe(false);
  expect(named({ all: true, changed: ["docs/guide/README.md"], branch: [] })).toBe(true);
  expect(touchesCrates(["std/lib.rs"])).toBe(true);
  expect(touchesCrates(["tooling/build.js"])).toBe(false);
  expect(readFileSync(join(root, ".github/workflows/check.yml"), "utf8")).toContain("needs.plan.outputs.crates == 'true'");
  // And React's port, of a change to what compiles it (#1188).
  expect(touchesPort(["tooling/build.js"])).toBe(true);
  expect(touchesPort(["src/lower/copies.rs"])).toBe(true);
  expect(touchesPort(["docs/guide/README.md", "test/corpus/a.rs"])).toBe(false);
  expect(readFileSync(join(root, ".github/workflows/check.yml"), "utf8")).toContain("needs.plan.outputs.port == 'true'");
});

test("each command a push's check runs is check:fast --all's, or left to CI by name", () => {
  type Step = { run?: string; parallel?: Step[] };
  type Job = { uses?: string; steps?: Step[] };
  const workflow = Bun.YAML.parse(readFileSync(join(root, ".github/workflows/check.yml"), "utf8")) as { jobs: Record<string, Job> };
  const steps = (list: Step[]): string[] => list.flatMap((step) => [...(step.run ? [step.run.trim().split("\n")[0]] : []), ...steps(step.parallel ?? [])]);
  const commands = Object.values(workflow.jobs).flatMap((job) => [...(job.uses ? [`uses: ${job.uses}`] : []), ...steps(job.steps ?? [])]);
  // A command, or its start and then its arguments: `bun run test` is
  // `bun run test --shard=..`'s, not `bun run test:rustc`'s.
  const starts = (command: string, start: string) => command === start || command.startsWith(`${start} `);
  const known = [...ranHere.flatMap((job) => (job.ci ? [job.ci] : [])), ...leftToCi.map((left) => left.run)];
  expect(commands.filter((command) => !known.some((start) => starts(command, start)))).toEqual([]);
  expect(leftToCi.filter((left) => !commands.some((command) => starts(command, left.run))).map((left) => left.run)).toEqual([]);
  expect(starts("bun run test:rustc", "bun run test")).toBe(false);
});

// A job's timeout is of the load it starts under, as its usual on a
// machine as loaded as its cores, at most three times it: a fixed 290 s
// stopped every corpus part at a load of 20 or more on 12 cores.
test("check:fast's timeouts grow with the machine's load, to three times", () => {
  expect(loadScale(6, 12)).toBe(1);
  expect(loadScale(18, 12)).toBe(1.5);
  expect(loadScale(67, 12)).toBe(3);
});

// A test's own timeout is as long as the machine's load makes it,
// `underLoad(n)`: a number written alone, `}, 60_000);`, is passed on a
// loaded machine by a test that passes alone. Those past three minutes
// are a hang's bound already.
test("a test's written timeout of three minutes or less grows with the load", () => {
  const fixed = readdirSync(join(root, "test"))
    .filter((name) => name.endsWith(".test.ts"))
    .flatMap((name) =>
      readFileSync(join(root, "test", name), "utf8")
        .split("\n")
        .flatMap((line, i) => {
          const ms = /^\s*\}, ([\d_]+)\);$/.exec(line)?.[1];
          return ms !== undefined && Number(ms.replaceAll("_", "")) >= 1000 && Number(ms.replaceAll("_", "")) <= 180_000 ? [`test/${name}:${i + 1}`] : [];
        }),
    );
  expect(fixed).toEqual([]);
});

test("each test file a push's suite runs, check:fast --all runs", () => {
  // As bun finds them, in test/ (bunfig.toml), and as scripts/test.ts runs
  // them: all but the nightly's.
  const suite = readdirSync(join(root, "test"))
    .filter((name) => name.endsWith(".test.ts"))
    .map((name) => `test/${name}`)
    .filter((file) => !nightly.includes(file));
  expect(suite).toContain("test/pilot.test.ts");
  // A `bun test` job runs each file whose path has one of its filters in it,
  // or, of none, every file, but those it ignores.
  const runs = (cmd: string[], file: string) => {
    if (cmd[0] !== "bun" || cmd[1] !== "test") return false;
    const ignored = cmd.filter((arg) => arg.startsWith("--path-ignore-patterns=")).map((arg) => arg.slice("--path-ignore-patterns=".length));
    const filters = cmd.slice(2).filter((arg) => !arg.startsWith("-"));
    return !ignored.includes(file) && (filters.length === 0 || filters.some((filter) => file.includes(filter)));
  };
  expect(suite.filter((file) => !ranHere.some((job) => runs(job.cmd, file)))).toEqual([]);
});

// A part of a test file a setting selects runs per push or at night, where
// `parts` says (#766): each setting a test file reads is a part's or the
// harness's, and each part's `by` sets it, a push's by what a push runs.
test("each part of a test file a setting selects runs where `parts` says", () => {
  const read = readdirSync(join(root, "test"))
    .filter((name) => /\.tsx?$/.test(name))
    .flatMap((name) => [...readFileSync(join(root, "test", name), "utf8").matchAll(/process\.env\.([A-Z][A-Z0-9_]*)/g)].map((m) => m[1]));
  const named = new Set([...harness, ...parts.map((part) => part.setting)]);
  expect([...new Set(read)].filter((setting) => !named.has(setting)).sort()).toEqual([]);
  // What a file runs, not what its comments say.
  const text = (file: string) =>
    readFileSync(join(root, file), "utf8")
      .split("\n")
      .filter((line) => !/^\s*(\/\/|#|\/?\*)/.test(line))
      .join("\n");
  const check = text(".github/workflows/check.yml");
  const onPush = (file: string) =>
    file.startsWith(".github/workflows/") ? Object.keys((Bun.YAML.parse(text(file)) as { on: Record<string, unknown> }).on).includes("push") : check.includes(`bun ${file}`);
  for (const part of parts) {
    const sets = new RegExp(`${part.setting}["']?\\s*[:=]\\s*["']?${part.value}\\b`);
    expect(sets.test(text(part.by)), `${part.by} sets ${part.setting}=${part.value}`).toBe(true);
    if (part.runs === "push") expect(onPush(part.by), `${part.by} runs per push`).toBe(true);
    else expect(part.why, `why ${part.setting}=${part.value} is the nightly's`).toBeTruthy();
  }
});

// A workflow sets a part's setting only on a step that runs a test file
// reading it (#1210's guard, test/part-settings.ts): nightly 42884008739
// set `FUZZ_START` where its file read it past `partSetting`, and
// `RUST_JS_REQUIRE_MIRI` where its file skipped the read, and each job
// failed as it ended. Each step that sets one names the files it runs, and
// one of them reads it, by `partSetting`, each time it loads, or a module
// of test/ it imports does.
test("a workflow sets a part's setting only where a test file it runs reads it", () => {
  const settings = new Set(parts.map((part) => part.setting));
  // A test file and each module of test/ it imports, and theirs.
  const loads = (file: string, seen = new Set<string>()): Set<string> => {
    if (seen.has(file) || !existsSync(join(root, file))) return seen;
    seen.add(file);
    for (const m of readFileSync(join(root, file), "utf8").matchAll(/from "\.\/([\w.-]+?)(\.tsx?)?"/g)) loads(`test/${m[1]}.ts`, seen);
    return seen;
  };
  const strays: string[] = [];
  for (const file of readdirSync(join(root, ".github/workflows")).filter((name) => name.endsWith(".yml"))) {
    const workflow = Bun.YAML.parse(readFileSync(join(root, ".github/workflows", file), "utf8")) as {
      jobs?: Record<string, { env?: Record<string, unknown>; steps?: { env?: Record<string, unknown>; run?: string }[] }>;
    };
    for (const [id, job] of Object.entries(workflow.jobs ?? {})) {
      for (const step of job.steps ?? []) {
        const run = step.run ?? "";
        const inline = [...run.matchAll(/\b([A-Z][A-Z0-9_]*)=/g)].map((m) => m[1]);
        const set = [...new Set([...Object.keys({ ...job.env, ...step.env }), ...inline])].filter((name) => settings.has(name));
        const files = [...run.matchAll(/\btest\/[\w-]+\.test\.ts\b/g)].map((m) => m[0]);
        for (const name of set) {
          const readers = readersOf(name);
          if (!files.some((test) => [...loads(test)].some((loaded) => readers.includes(loaded)))) strays.push(`${file} ${id}: ${name}, of ${readers.join(" or ")}, where it runs ${files.join(", ") || "no test file"}`);
        }
      }
    }
  }
  expect(strays).toEqual([]);
  // And each reader reads it as it loads, not past a condition: a read in a
  // `&&` or a `?:`'s branch may never run.
  for (const name of settings) {
    for (const reader of readersOf(name)) {
      const text = readFileSync(join(root, reader), "utf8");
      const guarded = new RegExp(`(&&|\\?)\\s*partSetting\\(["']${name}["']\\)`);
      expect(guarded.test(text), `${reader} reads ${name} past a condition`).toBe(false);
    }
  }
});

// `bun run nightly` runs what the nightly does, here: each of nightly.yml's
// jobs and of the check it calls, check.yml's, by name, but `plan`, which
// picks the mutations a branch's check runs, `seeds`, the day's number, and
// `issue`, which files a red night as an issue; and `iterator-forms`, the
// corpus with std's iterator structs, and `nets`, whose steps are the
// tables' slices, the declarations' callers, the twin net, the crate
// corpus's sample and React's port, which the nightly skips, as its
// `matrix`, `conformance`, `forms`, `format`, `numbers`, `async`,
// `consumers`, `std` and `twins` run all of each, and its suite the
// callers; `crater-report`, the Crater's shards said as
// one, which here's `crater` runs whole; and `decision-coverage`, the
// night's decision counts merged, which `bun run decision-coverage` is.
test("the nightly, here, has each job the nightly has", () => {
  const ids = (file: string) => {
    const text = readFileSync(join(root, ".github/workflows", file), "utf8");
    return [...text.slice(text.indexOf("\njobs:")).matchAll(/^  ([a-z-]+):$/gm)].map((m) => m[1]);
  };
  const ran = [...ids("check.yml"), ...ids("nightly.yml")].filter((id) => !["plan", "seeds", "check", "issue", "iterator-forms", "nets", "crater-report", "decision-coverage", "test-times", "selector-audit", "clusters", "attribute-plan", "attribute", "attribute-report", "ci-spend"].includes(id));
  expect(ran.filter((id) => !(id in localNightly))).toEqual([]);
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

// The MIR intervals' rule ablation runs on CI, not the shared Mac:
// rustc-tests.yml's `ablation`, its shards each of ci.ts's counts, each
// running scripts/ablate.ts's share, and one job merging them.
test("rustc-tests.yml ablates the intervals' rules in shards and merges them", () => {
  const tests = readFileSync(join(root, ".github/workflows/rustc-tests.yml"), "utf8");
  const matrix = [...tests.matchAll(/inputs\.ablation_shards == '(\d+)' && '\[([\d, ]+)\]'/g)].map((m) => [Number(m[1]), m[2]] as const);
  expect(matrix.map(([n]) => n).sort((a, b) => a - b)).toEqual(MUTATION_SHARDS.filter((n) => n > 1));
  for (const [n, list] of matrix) expect(list).toBe(Array.from({ length: n }, (_, i) => i + 1).join(", "));
  expect(tests).toContain("bun scripts/ablate.ts $ARGS --shard=${{ matrix.shard }}/${{ inputs.ablation_shards }} --out=ablation-${{ matrix.shard }}.json");
  expect(tests).toContain("bun scripts/ablate.ts --merge shards/*.json --out=ablation.md");
  type Jobs = { jobs: Record<string, { if?: string; needs?: string }> };
  const jobs = (Bun.YAML.parse(tests) as Jobs).jobs;
  expect(jobs.ablation.if).toBe("inputs.ablation");
  expect(jobs["ablation-merge"].needs).toBe("ablation");
  // The full suite's jobs stand aside for it, as for mutations.
  for (const name of ["build", "shard"]) expect(jobs[name].if).toContain("!inputs.ablation");
  // Each shard has mutations to ablate.
  const listed = Bun.spawnSync(["bun", "scripts/ablate.ts", "--list", "--shard=16/16"], { cwd: root });
  expect(listed.stdout.toString()).toMatch(/\b[1-9]\d* of \d+ mutations/);
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

// The nightly's crate corpus is docs/crate-corpus.md's rows, each crate at
// the exact release its row names (ADR 1067): asked for its newest, either
// and smallvec turned red on a release of theirs with no change here.
test("the crate corpus is its table's rows, each at an exact release", () => {
  const rows = corpusRows(readFileSync(join(root, "docs/crate-corpus.md"), "utf8"));
  if (!Array.isArray(rows)) throw new Error(rows.problem);
  expect(rows.length).toBeGreaterThan(20);
  const uuid = rows.find((c) => c.name === "uuid")!;
  expect(dependency(uuid)).toMatch(/^uuid = \{ version = "=\d+\.\d+\.\d+", default-features = false, features = \["std", "v4", "js"\] \}$/);
  expect(dependency(rows.find((c) => c.name === "either")!)).toMatch(/^either = \{ version = "=\d+\.\d+\.\d+" \}$/);
  const table = (crate: string, verdict = "runs") => `| Crate | Verdict | Refusals |\n|---|---|---|\n| ${crate} | ${verdict} | |\n`;
  // A row an open issue marks, its bar and what the crate does now.
  expect(corpusRows(table("itertools 0.14.0", "compiles (refused: #473)"))).toEqual([
    { name: "itertools", version: "0.14.0", bar: "compiles", open: { now: "refused", issue: "#473" } },
  ]);
  expect(corpusRows(table("either 1.15"))).toEqual({
    problem:
      "docs/crate-corpus.md's row `either 1.15` names no exact release: write the crate's name and version, `either 1.19.0`, and its features after them, `chrono 0.4.45 (`alloc`)`",
  });
  expect(corpusRows(table("strum 0.27.2 (`std`, `derive`)"))).toEqual([{ name: "strum", version: "0.27.2", features: ["std", "derive"], bar: "runs" }]);
  // A release's build metadata is part of it, as its lock names it (#1174).
  expect(corpusRows(table("toml 1.2.1+spec-1.1.0", "blocked"))).toEqual([{ name: "toml", version: "1.2.1+spec-1.1.0", bar: "blocked" }]);
});


// And every crate of each one's graph at the release its lock names, which
// Cargo is held to: url's dependencies resolved newer, and its row's "15 of
// its 16 crates compile" measured 13 (ADR 1067, amended; #511).
test("each crate of the corpus has a lock of its graph, its row's release in it, and Cargo is held to it", () => {
  const rows = corpusRows(readFileSync(join(root, "docs/crate-corpus.md"), "utf8"));
  if (!Array.isArray(rows)) throw new Error(rows.problem);
  for (const crate of rows) {
    expect(existsSync(lockOf(crate.name)), `scripts/crate-corpus/${crate.name}.lock`).toBe(true);
    expect(lockedVersion(readFileSync(lockOf(crate.name), "utf8"), crate.name), crate.name).toBe(crate.version);
  }
  // No lock that's no row's.
  const locks = readdirSync(join(root, "scripts/crate-corpus")).filter((file) => file.endsWith(".lock"));
  expect(locks.sort()).toEqual(rows.map((c) => `${c.name}.lock`).sort());
  // Each of Cargo's runs but the one that writes a lock, `--locked`.
  const script = readFileSync(join(root, "scripts/crate-corpus.ts"), "utf8");
  // A run that blocks, one of those several crates make at once, `spawned`,
  // or one stopped past its time with what it started, `runChild`.
  const runs = [
    ...script.matchAll(/(?:spawnSync|spawned)\(\s*"cargo",\s*\[([^\]]*)\]/g),
    ...script.matchAll(/runChild\(\[\s*"cargo",\s*([^\]]*)\]/g),
  ].map(([, args]) => args!);
  expect(runs.length).toBe(4);
  expect(runs.filter((args) => !args.includes('"--locked"'))).toEqual(['`+${toolchain}`, "generate-lockfile"']);
});

// A crate's project is written again each run only where it differs: a file
// written again is a new date, which Cargo takes for a change, and the
// probe was compiled again, by rust-js and natively, on every run (#998).
test("a crate's project file is written only where it differs, its date kept where it doesn't", () => {
  const dir = mkdtempSync(join(tmpdir(), "crate-corpus-written-"));
  try {
    const file = join(dir, "lib.rs");
    written(file, "pub fn report() {}\n");
    const past = new Date(Date.now() - 60_000);
    utimesSync(file, past, past);
    written(file, "pub fn report() {}\n");
    expect(statSync(file).mtimeMs).toBe(past.getTime());
    written(file, "pub fn report() -> u8 { 1 }\n");
    expect(readFileSync(file, "utf8")).toBe("pub fn report() -> u8 { 1 }\n");
    expect(statSync(file).mtimeMs).toBeGreaterThan(past.getTime());
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test("a crate of the corpus's graph rust-js crashed on is said to have crashed, not left out", () => {
  const stderr = [
    "    Checking regex-syntax v0.8.11",
    "",
    "thread 'rustc' (184122857) has overflowed its stack",
    "fatal runtime error: stack overflow, aborting",
    "error: could not compile `regex-syntax` (lib)",
    "",
    "Caused by:",
    "  process didn't exit successfully: `rust-js rustc --crate-name regex_syntax --check-cfg 'cfg(feature, values(\"std\"))'` (signal: 6, SIGABRT: process abort signal)",
    "error: could not compile `aho-corasick` (lib) due to 1 previous error",
  ].join("\n");
  expect(crashes(stderr)).toEqual([{ name: "regex-syntax", how: "signal: 6, SIGABRT: process abort signal" }]);
});

// A change that adds or edits a corpus case runs each metamorphic rewrite of
// it, in check:fast, plain or `--all`, as each push's check does
// (check.yml's `nets` job's `metamorphic` step): #1115's case ran as Rust does, but not
// moved into a library, which nothing ran till the nightly (#1164, #1165).
test("a change adding or editing a corpus case runs each of its rewrites", () => {
  expect(changedCases(["test/corpus/new_case.rs", "src/lower/mir.rs", "test/corpus/new_case.js", "test/corpus/edited.rs", "test/corpus/new_case.rs"])).toEqual([
    "edited.rs",
    "new_case.rs",
  ]);
  const ofCases = (scope: { all: boolean; changed: string[]; branch: string[] }) =>
    jobs({ ...scope, added: [], extra: [], cores: 12 }).jobs.filter((job) => job.name === runs.metamorphic[0].name);
  for (const all of [false, true]) {
    const [rewrites] = ofCases({ all, changed: ["test/corpus/new_case.rs"], branch: [] });
    expect(rewrites.cmd).toEqual(["bun", "scripts/iteratorSlice.ts", "run", "metamorphic", "1"]);
    expect(ofCases({ all, changed: [], branch: ["test/corpus/a.rs", "test/corpus/b.rs"] })[0].cmd.at(-1)).toBe("2");
    expect(ofCases({ all, changed: ["src/lower/mir.rs", "test/corpus/new_case.js"], branch: [] })).toEqual([]);
  }
  expect(readFileSync(join(root, ".github/workflows/check.yml"), "utf8")).toContain("bun scripts/iteratorSlice.ts run metamorphic 4 ${{ needs.plan.outputs.since }}");
});

// A job whose work the change sizes says its size, which the pool expects
// it to take the time of at its last runs' pace (#1364): the rows of the
// iterator tables a change names, the corpus cases whose rewrites it runs,
// the mutations it adds, and the corpus cases with std's iterator structs
// it picks, or all of them. By its last time alone, 0.1 s after a change
// that named no rows, the rows of the next started last and ran alone.
test("a job whose work the change sizes says its size", () => {
  const sized = (scope: Partial<Scope>) =>
    Object.fromEntries(
      jobs({ all: true, changed: [], branch: [], added: [], extra: [], cores: 12, ...scope })
        .jobs.filter((job) => job.size !== undefined)
        .map((job) => [job.name, job.size]),
    );
  const cases = readdirSync(join(root, "test", "corpus")).filter((name) => name.endsWith(".rs")).length;
  expect(sized({ branch: ["src/lower/iterators/clones.rs", "test/corpus/a.rs", "test/corpus/b.rs"], added: ["m", "m", "n"], rows: 9175 })).toEqual({
    "the rows the change names": 9175,
    [runs.forms[0].name]: cases,
    [runs.metamorphic[0].name]: 2,
    "mutations the change adds": 2,
  });
  expect(sized({ branch: ["src/lower/iterators/clones.rs"], rows: 0, forms: { forms: ["a.rs", "b.rs", "c.rs"], formsList: "forms.txt" } })).toEqual({
    "the rows the change names": 0,
    [runs.forms[0].name]: 3,
  });
  expect(sized({ branch: ["src/lower/mir.rs"] })).toEqual({});
});

// A push's check asks every row but the top crates by downloads', and of
// those one in 8, a new set each commit; the nightly asks every row (#1174).
test("a sampled table's rows are marked, and a push's commit picks one in 8 of them, a new set each commit", () => {
  const doc = `| Crate | Verdict | Refusals |\n|---|---|---|\n| either 1.19.0 | runs | |\n\n| ${sampledHeader} | Verdict | Refusals |\n|---|---|---|\n| syn 3.0.10 | blocked | |\n`;
  expect(corpusRows(doc)).toEqual([
    { name: "either", version: "1.19.0", bar: "runs" },
    { name: "syn", version: "3.0.10", bar: "blocked", sampled: true },
  ]);
  const rows = corpusRows(readFileSync(join(root, "docs/crate-corpus.md"), "utf8"));
  if (!Array.isArray(rows)) throw new Error(rows.problem);
  const sampled = rows.filter((c) => c.sampled);
  expect(sampled.length).toBeGreaterThan(70);
  // Every other row, every commit.
  expect(rows.filter((c) => !c.sampled).every((c) => sampledIn(c, "a"))).toBe(true);
  // About one in 8 of a commit's, over many commits, and not the same set.
  const seeds = Array.from({ length: 200 }, (_, i) => `commit${i}`);
  const picked = seeds.map((seed) => sampled.filter((c) => sampledIn(c, seed)).map((c) => c.name).join());
  const share = seeds.reduce((n, seed) => n + sampled.filter((c) => sampledIn(c, seed)).length, 0) / (seeds.length * sampled.length);
  expect(Math.abs(share - 1 / sampledOneIn)).toBeLessThan(0.02);
  expect(new Set(picked).size).toBeGreaterThan(150);
});

// oxc_formatter_core is vendored with #1269's fix (ADR 1121), and
// oxc_formatter with #1807's (ADR 0241): both compilers, the native and the
// browser's, patch oxc's own with the one copy of each, and each copy is of
// the tag every oxc crate is pinned to, so moving that tag fails here until
// the copy is taken again or, once oxc has the fix, removed with its
// `[patch]`.
test("the vendored oxc_formatter_core and oxc_formatter are both compilers' and of their oxc tag", () => {
  const manifest = (path: string) => readFileSync(join(root, path), "utf8");
  const tags = (text: string) => new Set([...text.matchAll(/oxc-project\/oxc", tag = "([^"]+)"/g)].map((m) => m[1]));
  const pinned = tags(manifest("Cargo.toml"));
  expect([...pinned]).toHaveLength(1);
  expect(tags(manifest("vendor/oxc_formatter_core/Cargo.toml"))).toEqual(pinned);
  expect(tags(manifest("vendor/oxc_formatter/Cargo.toml"))).toEqual(pinned);
  expect(tags(manifest("wasm/Cargo.toml"))).toEqual(pinned);
  expect(manifest("Cargo.toml")).toContain('[patch."https://github.com/oxc-project/oxc"]\noxc_formatter_core = { path = "vendor/oxc_formatter_core" }\noxc_formatter = { path = "vendor/oxc_formatter" }');
  expect(manifest("wasm/Cargo.toml")).toContain('[patch."https://github.com/oxc-project/oxc"]\noxc_formatter_core = { path = "../vendor/oxc_formatter_core" }\noxc_formatter = { path = "../vendor/oxc_formatter" }');
});

// The nightly runs a test file in parts by scripts/parts.ts alone (#1306):
// each part's lines as they come, its peak memory and what held it, and a
// part the machine kills said so, with where it was. A shell loop of
// `bun test ... &` printed nothing till every part ended, and nightly
// 42884008739's metamorphic net ended in a bare exit 143.
test("a workflow runs a test file in parts by scripts/parts.ts alone", () => {
  for (const file of readdirSync(join(root, ".github/workflows")).filter((name) => name.endsWith(".yml"))) {
    const text = readFileSync(join(root, ".github/workflows", file), "utf8");
    expect(text, file).not.toMatch(/bun test[^\n]*&\s*$/m);
  }
});

test("a part's spec names its parts, and a part's line says how it ended", () => {
  expect(partsOf("4")).toEqual({ parts: [1, 2, 3, 4], count: 4 });
  expect(partsOf("5-8/24")).toEqual({ parts: [5, 6, 7, 8], count: 24 });
  for (const bad of ["0", "3-2/8", "7-9/8", "4/2", "x"]) expect(() => partsOf(bad), bad).toThrow();
  expect(caseOf("(pass) seed 3 group 14: 16 cases [43707.03ms]")).toBe("seed 3 group 14: 16 cases");
  expect(caseOf("(fail) a, rewritten")).toBe("a, rewritten");
  expect(caseOf(" 53 pass")).toBeUndefined();
  expect(verdict(0, null, "(pass) a\n 1 pass\n 0 fail\n")).toBe("passed");
  expect(verdict(1, null, "(fail) a\n 0 pass\n 1 fail\n")).toBe("failed");
  expect(verdict(1, null, 'Test filter "x" had no matches\n')).toBe("failed");
  expect(verdict(null, "SIGKILL", "(pass) a\n")).toBe("killed (memory?)");
  expect(verdict(143, null, "(pass) a\n")).toBe("killed (memory?)");
});

test("a part the machine kills is said killed, with what it ran and its peak", async () => {
  // Outside test/, where this checkout's `bun test` finds no file.
  const dir = mkdtempSync(join(tmpdir(), "parts-"));
  try {
    writeFileSync(join(dir, "killed.test.ts"), 'import { test } from "bun:test";\ntest("first", () => {});\ntest("second", async () => {\n  await Bun.sleep(1500);\n  process.kill(process.pid, "SIGKILL");\n});\n');
    const said: string[] = [];
    const log = console.log;
    console.log = (line: string) => said.push(line);
    try {
      expect(await runParts("./killed.test.ts", partsOf("1"), 1, [], dir)).toBe(false);
    } finally {
      console.log = log;
    }
    // `bun test` holds a file's lines till it ends where its output is a
    // pipe, but streams them under GitHub Actions, where it prints its
    // `::group::`s: the last case said is none, or the one before the kill.
    const ended = said.find((line) => line.startsWith("── part 1/1 killed (memory?)"));
    expect(ended, said.join("\n")).toMatch(/at most \d+\.\d GB.*; the last case it said ended (none|first)$/);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
}, underLoad(60_000));

// check:fast installs again where the manifests changed since its last
// install (#1817): a rebase over a new workspace, `@rust-js/webapi`, left
// `node_modules` without it. What it's of: the root's manifest, the
// lockfile and each workspace's, and nothing else.
test("check:fast's install is of every manifest and the lockfile", () => {
  const files: Record<string, string> = {
    "package.json": JSON.stringify({ workspaces: ["webapi", "std"] }),
    "bun.lock": "lock",
    "webapi/package.json": '{ "name": "@rust-js/webapi-generator" }',
    "std/package.json": '{ "name": "@rust-js/std" }',
  };
  const hashOf = (changed: Record<string, string>) => manifestsHash((file) => ({ ...files, ...changed })[file] ?? "");
  const now = hashOf({});
  expect(hashOf({})).toBe(now);
  expect(hashOf({ "webapi/package.json": '{ "name": "@rust-js/webapi" }' })).not.toBe(now);
  expect(hashOf({ "bun.lock": "lock 2" })).not.toBe(now);
  expect(hashOf({ "package.json": JSON.stringify({ workspaces: ["webapi"] }) })).not.toBe(now);
  expect(hashOf({ "react/package.json": "{}" })).toBe(now);
});

// And where the hash is the one installed but a workspace the root depends
// on isn't linked, a checkout whose install predates it: m1-lisa's
// check:fast failed `@rust-js/webapi`'s import so (#1817).
test("check:fast installs where a workspace the root depends on isn't linked, its hash as installed", () => {
  const files: Record<string, string> = {
    "package.json": JSON.stringify({ workspaces: ["webapi", "react"], devDependencies: { "@rust-js/webapi": "workspace:*", typescript: "7.1.0-beta" } }),
    "bun.lock": "lock",
    "webapi/package.json": '{ "name": "@rust-js/webapi" }',
    "react/package.json": '{ "name": "@rust-js/react-generator" }',
  };
  const read = (file: string) => files[file] ?? "";
  const stamp = manifestsHash(read);
  expect(installNeeded(read, stamp, () => true)).toBeUndefined();
  expect(installNeeded(read, stamp, (name) => name !== "@rust-js/webapi")).toBe("@rust-js/webapi isn't linked in node_modules");
  // Only a workspace the root depends on is linked: the React generator's isn't.
  expect(installNeeded(read, stamp, (name) => name !== "@rust-js/react-generator")).toBeUndefined();
  expect(installNeeded(read, undefined, () => true)).toBe("the manifests changed since the last install");
});

// A mutation's test that drives a browser, none-given's Chromium row, ran
// in CI's mutations, which had none: each failed as the compiler is, and
// its shard went red. Each shard asks whether what it runs drives one
// (`mutations.ts --browser`, `drivesBrowser`) and installs it then: of each
// file that imports Playwright, and only of those.
test("CI's mutations install the browser a mutation's test drives", () => {
  const workflow = readFileSync(join(root, ".github/workflows/rustc-tests.yml"), "utf8");
  const job = workflow.slice(workflow.indexOf("\n  mutations:\n"));
  expect(job).toContain('--browser)" >> "$GITHUB_OUTPUT"');
  expect(job).toContain("if: steps.browser.outputs.needed == 'browser'\n        name: Install the browser\n        run: bunx --bun playwright install --with-deps chromium");
  expect(job.indexOf("Install the browser")).toBeLessThan(job.indexOf("Put back each known bug"));
  for (const file of browsers) expect([file, drivesBrowser(root, file)]).toEqual([file, true]);
  expect(drivesBrowser(root, "test/none-given.test.ts")).toBe(true);
  expect(drivesBrowser(root, "test/corpus.test.ts")).toBe(false);
  expect(drivesBrowser(root, "test/lowering.test.ts")).toBe(false);
});

// A check of `main` follows the last whose every mutation shard completed
// and kept what it found, of its own commit: what changed since, and what it
// ran and didn't catch, again. One whose results are missing, of another
// commit, or of a shard that didn't complete, is passed over: a mutation is
// caught only where it was, so one survivor no longer reruns every
// mutation since the last green check.
test("main's mutations follow the last check whose shards kept their results, and retry what it didn't catch", () => {
  const found = (commit: string, results: Record<string, string>) => ({ commit, results });
  const red = { run: run("main", 4, "completed", "failure"), shards: 2, completed: 2, results: [found("sha4", { a: "caught", b: "SURVIVED: its 1 tests passed with it" }), found("sha4", { c: "caught", d: "not run" })] };
  expect(resultsBase([red])).toEqual({ since: "sha4", retry: ["b", "d"] });
  const green = { run: run("main", 3, "completed", "success"), shards: 1, completed: 1, results: [found("sha3", { a: "caught" })] };
  // Passed over: a shard cancelled, a shard's results missing, another commit's.
  expect(resultsBase([{ ...red, completed: 1 }, green])).toEqual({ since: "sha3", retry: [] });
  expect(resultsBase([{ ...red, results: red.results.slice(1) }, green])).toEqual({ since: "sha3", retry: [] });
  expect(resultsBase([{ ...red, results: [found("sha9", {}), red.results[1]] }, green])).toEqual({ since: "sha3", retry: [] });
  expect(resultsBase([{ ...red, shards: 0, completed: 0, results: [] }])).toBeUndefined();
  // Not one it judged latent, surviving at its base too, which a push that
  // changes its site still runs: the check's `--changed`, the runner's own
  // set, beside `--also`, the retry list.
  expect(resultsBase([{ ...red, latent: ["b"] }])).toEqual({ since: "sha4", retry: ["d"] });
  expect(readFileSync(join(root, ".github/workflows/check.yml"), "utf8")).toContain("format('--changed={0} --also={1} --base={0}', needs.plan.outputs.since, needs.plan.outputs.retry)");
});

// Each push builds every corpus case as a library too (#1920): built only
// where `library-differs` said, d2e47ad9f's change to what a library
// copies passed its push, and every mutation shard of its check failed on
// ten cases whose library JS it changed. Nothing selects fewer again.
test("each push's corpus builds every case as a library, as the nightly's does", () => {
  const files = [...new Bun.Glob("{scripts,test}/**/*.ts").scanSync(root)].filter((file) => file !== "test/ci.test.ts" && !file.startsWith("test/corpus/") && !file.startsWith("scripts/mutations/"));
  expect(files.filter((file) => readFileSync(join(root, file), "utf8").includes("RUST_JS_LIBRARIES"))).toEqual([]);
  expect(readFileSync(join(root, ".github/workflows/check.yml"), "utf8")).not.toContain("RUST_JS_LIBRARIES");
});

// The `wasm` job and rustc's tests run of a change to what they build and
// run, as `plan` says, and the mutations of a change that touches one: a
// doc's change or a corpus case's ran all three to the answers they had,
// 13 min of WASM and rustc's eight jobs, waiting for machines behind the
// rest (dev-loop change 3). Each path list is the closure of what its job
// runs: the WASM's inputs are wasm/prebuilt.sh's, and each file the job's
// tests or script import is in it.
test("the WASM build, rustc's tests and the mutations run where what they run changed", () => {
  const check = readFileSync(join(root, ".github/workflows/check.yml"), "utf8");
  expect(check).toContain("if: ${{ inputs.nightly || needs.plan.outputs.wasm == 'true' }}");
  expect(check).toContain("if: ${{ inputs.nightly || needs.plan.outputs.rustc == 'true' }}");
  expect(check).toContain("if: ${{ inputs.nightly || needs.plan.outputs.mutations == 'true' }}");
  expect(wasmInputs("INPUTS=(src Cargo.toml wasm/patches)\n")).toEqual(["src/**", "Cargo.toml", "wasm/patches/**"]);
  for (const file of ["src/lower/copies.rs", "wasm/patches/a.patch", "rust-toolchain.toml", "std/lib.rs", "test/snapshots/x.js", "examples/fib.rs"]) {
    expect([file, touchesWasm([file])]).toEqual([file, true]);
  }
  for (const file of ["src/lower/copies.rs", "std/lib.rs", "test/rustc-known-failures.txt", "scripts/rustc-suite.ts", ".github/workflows/rustc-tests.yml"]) {
    expect([file, touchesRustc([file])]).toEqual([file, true]);
  }
  for (const file of ["docs/guide/README.md", "test/corpus/a.rs", "test/ci.test.ts", "TESTS.md"]) {
    expect([file, touchesWasm([file]), touchesRustc([file])]).toEqual([file, false, false]);
  }
  // A job's own code is what its tests or script import, transitively.
  expect(importsOf(["test/playground.test.ts"])).toEqual(expect.arrayContaining(["test/playground.test.ts", "test/support.ts", "test/sourcemap.ts", "test/packages-of.ts", "tooling/bounded.js"]));
  for (const file of importsOf(["test/snapshots.test.ts", "test/playground.test.ts"])) expect([file, touchesWasm([file])]).toEqual([file, true]);
  for (const file of importsOf(["scripts/rustc-suite.ts"])) expect([file, touchesRustc([file])]).toEqual([file, true]);
});

// The test map's counting build is made by testMap.ts alone, its flags
// there and in no workflow's environment, and links an object that names
// where an uncounted run writes by the runtime's API, defining no symbol the
// instrumented crate defines: one that defined `__llvm_profile_filename`
// linked on macOS, where the crate's is weak, and not on Linux, where it's
// in a COMDAT (check 43786120684).
test("the test map's counting build has its flags from testMap.ts alone, and links on Linux", () => {
  const job = readFileSync(join(root, ".github/workflows/test-map.yml"), "utf8");
  expect(job).toContain("bun scripts/testMap.ts");
  for (const flag of ["RUSTFLAGS", "instrument-coverage", "CARGO_ENCODED_RUSTFLAGS", "profile"]) expect([flag, job.includes(flag)]).toEqual([flag, false]);
  const lost = lostObject("/t/lost/%p.profraw");
  expect(lost).toContain('__llvm_profile_set_filename("/t/lost/%p.profraw")');
  expect(lost).not.toMatch(/char\s+__llvm_profile_filename/);
});

// The `wasm` job's build takes rustc's crates as the cache built them: its
// tree made as old as one time, only a target of the same tree restored, by
// a key of what decides it, and the tree's hash held to the target's
// (wasm/build.sh). Built again each run, they were 4 min 51 s of its 12.
test("the WASM build takes rustc's crates as built where the cache holds the same tree", () => {
  const check = readFileSync(join(root, ".github/workflows/check.yml"), "utf8");
  const tree = "${{ hashFiles('rust-toolchain.toml', 'scripts/toolchain.ts', 'wasm/patches/*', 'wasm/build.sh') }}";
  expect(check).toContain(`key: wasm-check-\${{ runner.os }}-${tree}-`);
  expect(check).toContain(`restore-keys: wasm-check-\${{ runner.os }}-${tree}-\n`);
  expect(check).toContain("RUST_JS_RUSTC_MTIME: '200001010000'");
  const build = readFileSync(join(root, "wasm/build.sh"), "utf8");
  expect(build.indexOf("RUST_JS_RUSTC_MTIME")).toBeGreaterThan(build.indexOf("patches/*.patch"));
  expect(build.indexOf("RUST_JS_RUSTC_MTIME")).toBeLessThan(build.indexOf('cargo "+$TOOLCHAIN" build'));
  expect(build).toContain("built from other rustc sources");
});

// The nets of a few minutes are steps of one job, `nets`, each in the
// background where `plan` says its change runs it: each a job of its own
// spent most of its minutes setting up. A red `nets` says which net failed
// by its step's name, and its summary names each (dev-loop change 3).
test("the short nets are the nets job's steps, each where its change runs it, each failure named", () => {
  const listed = nets();
  expect(listed.map((net) => net.id)).toEqual(["iterators", "conformance", "copies", "format", "numbers", "memory", "declarations", "async", "consumers", "std", "twins", "metamorphic", "crates", "port"]);
  type Step = { id?: string; if?: string; background?: boolean; "wait-all"?: null };
  const workflow = Bun.YAML.parse(readFileSync(join(root, ".github/workflows/check.yml"), "utf8")) as { jobs: Record<string, { if?: string; steps: Step[] }> };
  const job = workflow.jobs.nets!;
  for (const step of job.steps.filter((s) => s.background)) {
    const output = /^needs\.plan\.outputs\.(\w+) == 'true'$/.exec(step.if ?? "")?.[1];
    expect([step.id, output !== undefined && job.if!.includes(`needs.plan.outputs.${output} == 'true'`)]).toEqual([step.id, true]);
  }
  const waits = job.steps.findIndex((s) => "wait-all" in s);
  expect(waits).toBe(job.steps.findLastIndex((s) => s.background) + 1);
  const summary = netsSummary({ iterators: { outcome: "failure" }, std: { outcome: "success" }, port: { outcome: "skipped" } }, listed);
  expect(summary).toContain("### 1 net failed: iterator matrix slice");
  expect(summary).toContain("- std net: passed");
  expect(summary).not.toContain("react-in-rust port");
  expect(netsSummary({ std: { outcome: "success" } }, listed)).toContain("### Every net that ran passed");
});

// The nightly keeps each job's, file's and test's time (scripts/testTimes.ts),
// the suite's by bun's JUnit reports, after every job that runs tests: a
// job added to the night and not to its `needs` is timed by none.
test("the nightly's test times follow each of its jobs, the suite's reports among them", () => {
  const nightly = Bun.YAML.parse(readFileSync(join(root, ".github/workflows/nightly.yml"), "utf8")) as { jobs: Record<string, { needs?: string[]; if?: string }> };
  const times = nightly.jobs["test-times"]!;
  expect(times.if).toBe("always()");
  const others = Object.keys(nightly.jobs).filter((id) => !["test-times", "issue", "decision-coverage", "seeds", "crater-report", "selector-audit", "clusters", "ci-spend"].includes(id));
  expect(others.filter((id) => !times.needs!.includes(id))).toEqual([]);
  expect(readFileSync(join(root, ".github/workflows/check.yml"), "utf8")).toContain("name: junit-${{ matrix.shard }}");
  expect(readFileSync(join(root, "scripts/test.ts"), "utf8")).toContain('"--reporter=junit"');
});

// Miri judges the program native Rust runs, its arithmetic wrapping: at
// its default a wrap panics first, and undefined behaviour after it was
// taken as defined (#1970). `programFlags` is the one source of the flags.
test("Miri and every native build of a program take its flags from programFlags alone", () => {
  expect(miriCommand("case.rs", "2021", "sysroot")).toEqual(expect.arrayContaining(programFlags("2021")));
  const harness = [...readdirSync(join(root, "test")).map((f) => `test/${f}`), ...readdirSync(join(root, "scripts")).map((f) => `scripts/${f}`)]
    .filter((f) => f.endsWith(".ts") && f !== "test/support.ts" && f !== "test/ci.test.ts");
  const own = harness.filter((f) => readFileSync(join(root, f), "utf8").split("\n").some((l) => /"-Coverflow-checks/.test(l)));
  expect(own).toEqual([]);
});

// A run whose verdict, map or report lasts is never cancelled as it runs:
// with four agents pushing, checks of main end every few minutes, and each
// cancelling the one running left none to finish (test-map.yml's 5 of 7).
// Of those waiting, GitHub keeps only the newest. Cancelling is for jobs
// that take seconds and leave nothing undone, each named here.
test("no workflow cancels a run in progress but the seconds-long idempotent ones", () => {
  const idempotent = ["test-map-prune.yml"];
  const dir = join(root, ".github/workflows");
  for (const file of readdirSync(dir).filter((name) => name.endsWith(".yml"))) {
    const cancels = (Bun.YAML.parse(readFileSync(join(dir, file), "utf8")) as { concurrency?: { "cancel-in-progress"?: unknown } }).concurrency?.["cancel-in-progress"];
    expect([file, cancels === undefined || cancels === false || idempotent.includes(file)]).toEqual([file, true]);
  }
  // The nightly is in a group of its own, never cancelled by a run of
  // anything else, nor one of its own: cancelling it loses the night (#1989).
  const nightly = (Bun.YAML.parse(readFileSync(join(dir, "nightly.yml"), "utf8")) as { concurrency?: { group?: string; "cancel-in-progress"?: unknown } }).concurrency;
  expect(nightly).toEqual({ group: "nightly", "cancel-in-progress": false });
  expect(readFileSync(join(root, "DEVELOPMENT.md"), "utf8")).toContain("**Never cancel a nightly**");
});

// The suite's shards are as many as its selection needs: one of a small
// selection, so the free plan's 20 jobs at once keep room for the map's
// recording and the nets; two of a larger one or everything.
test("a check's suite takes the machines its selection needs", () => {
  const some = (files: number, cases: number) => ({ all: false, files: Array.from({ length: files }, (_, i) => `test/f${i}.test.ts`), cases: Array.from({ length: cases }, (_, i) => `c${i}.rs`), forms: [], why: "" });
  expect([testShards(some(5, 30)), testShards(some(60, 400)), testShards(some(61, 0)), testShards(some(0, 401)), testShards({ ...some(0, 0), all: true })]).toEqual(["1", "1", "2", "2", "2"]);
  const check = readFileSync(join(root, ".github/workflows/check.yml"), "utf8");
  expect(check).toContain("test_shards: ${{ steps.plan.outputs.test_shards }}");
  expect(check).toContain("${{ inputs.test_shards || needs.plan.outputs.test_shards || '2' }}");
});

// Every minute counts (Tu, 2026-11-16): each job of a push's check runs
// only where its change can affect it, by a gate, its `if` reading
// `plan`'s outputs or another job's result, or is on `alwaysRun` with its
// reason. A new job with neither fails here, and its author says why.
test("each job of a push's check is gated, or always runs for a reason it names", () => {
  const check = Bun.YAML.parse(readFileSync(join(root, ".github/workflows/check.yml"), "utf8")) as { jobs: Record<string, { if?: string }> };
  const gated = (id: string) => /\bneeds\.[\w-]+\.(outputs\.[\w-]+|result)\b/.test(check.jobs[id]!.if ?? "");
  expect(Object.keys(check.jobs).filter((id) => !gated(id) && !(id in alwaysRun))).toEqual([]);
  // The list names jobs there are, ungated, each with a reason.
  expect(Object.entries(alwaysRun).filter(([id, why]) => !(id in check.jobs) || gated(id) || why.length < 20)).toEqual([]);
  // Another workflow a push starts runs only of the paths it reads.
  for (const file of readdirSync(join(root, ".github/workflows")).filter((name) => name !== "check.yml")) {
    const on = (Bun.YAML.parse(readFileSync(join(root, ".github/workflows", file), "utf8")) as { on: Record<string, { paths?: string[] } | null> }).on;
    if ("push" in on) expect([file, (on.push?.paths ?? []).length > 0]).toEqual([file, true]);
  }
});
