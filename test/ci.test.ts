// What the CI workflow runs where (DEVELOPMENT.md): the suite split into
// shards, the mutations a change touches, and each branch's latest run.

import { expect, test } from "bun:test";
import { cpSync, existsSync, mkdtempSync, readdirSync, readFileSync, rmSync, statSync, utimesSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { checkedSince, latestRuns, MUTATION_SHARDS, mutationJobsPassed, mutationShards, timingsRun } from "../scripts/ci";
import { corpusRows, crashes, dependency, lockedVersion, lockOf, written } from "../scripts/crate-corpus";
import {
  batches,
  type Change,
  changedLines,
  changedMutations,
  enclosingFunction,
  mutations,
  ownMutations,
  schema,
  setPasses,
  site,
  siteLines,
  type Sited,
  testBlocks,
  testResults,
  testSet,
  unitTestsModule,
} from "../scripts/mutations";
import { jobs, leftToCi, readabilityPaths, touchesReadability } from "../scripts/checkFast";
import { changedCases, runs, touchesCrates, touchesPort } from "../scripts/iteratorSlice";
import { loadScale } from "../scripts/timing";
import { jobs as localNightly } from "../scripts/nightly";
import { caseParts, caseShardParts, compilerBuilds, harness, memoryBudget, nightly, parts, peakGB, plainSuiteFiles, pooledParts, shard } from "../scripts/shard";
import { fixture, root } from "./support";

const files = ["a", "b", "c", "d", "e", "f", "g"].map((name) => `test/${name}.test.ts`);

test("shards split the files between them, each once, whatever the order given", () => {
  const shards = [1, 2, 3].map((i) => shard(files, i, 3));
  expect(shards.flat().sort()).toEqual(files);
  expect(shards.map((s) => s.length).sort()).toEqual([2, 2, 3]);
  expect(shard([...files].reverse(), 2, 3)).toEqual(shards[1]);
  expect(() => shard(files, 4, 3)).toThrow("a shard is 1 to 3");
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
  expect(readFileSync(join(root, "scripts/mutations.ts"), "utf8")).toContain(": compilerBuilds(availableParallelism());");
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

// What a change can leave untrue unseen runs where the change is made, not
// a night later (ADR 1030, amended): the guide's limitations, which a
// lowering that lifts one or an example that doesn't compile makes red
// (5096475be's), of a change to the compiler or std; the bindings'
// coverage ratchets, which a binding added makes red, of every change.
test("the guide's limitations and the coverage ratchets run each push and in plain check:fast", () => {
  const recorded: Record<string, "shared" | undefined> = { "test/limitations.test.ts": "shared", "test/next-coverage.test.ts": undefined, "test/react-coverage.test.ts": undefined, "test/webapi-coverage.test.ts": undefined, "test/node-coverage.test.ts": undefined };
  const tests = readdirSync(join(root, "test")).map((file) => `test/${file}`);
  const suites = plainSuiteFiles(tests);
  for (const [file, when] of Object.entries(recorded)) {
    expect(nightly).not.toContain(file);
    expect(suites.filter((suite) => suite.files.includes(file)).map((suite) => suite.when)).toEqual([when]);
  }
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

// The crate corpus runs of a change to what compiles it, plain check:fast's
// and each push's (`plan`'s `crates`), not only `--all`'s and the
// nightly's: 9353847cd refused itertools unseen (#1188).
test("the crate corpus runs where its paths say, of plain check:fast too", () => {
  const named = (scope: { all: boolean; changed: string[]; branch: string[] }) =>
    jobs({ ...scope, added: [], extra: [], cores: 12 }).jobs.some((job) => job.name === "the crate corpus");
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

// `bun run nightly` runs what the nightly does, here: each of nightly.yml's
// jobs and of the check it calls, check.yml's, by name, but `plan`, which
// picks the mutations a branch's check runs, `seeds`, the day's number, and
// `issue`, which files a red night as an issue; and `iterators`,
// `iterator-conformance`, `iterator-forms`, `format-grid`, `number-grid`
// `declarations`, `async-points` and `consumer-net`, the tables' slices,
// the corpus with std's iterator structs and the declarations' callers,
// which the nightly skips, as its `matrix`, `conformance`, `forms`,
// `format`, `numbers`, `async` and `consumers` run all of each, and its
// suite the callers.
test("the nightly, here, has each job the nightly has", () => {
  const ids = (file: string) => {
    const text = readFileSync(join(root, ".github/workflows", file), "utf8");
    return [...text.slice(text.indexOf("\njobs:")).matchAll(/^  ([a-z-]+):$/gm)].map((m) => m[1]);
  };
  const ran = [...ids("check.yml"), ...ids("nightly.yml")].filter((id) => !["plan", "seeds", "check", "issue", "iterators", "iterator-conformance", "iterator-forms", "format-grid", "number-grid", "memory-net", "declarations", "async-points", "consumer-net", "metamorphic-cases", "crate-corpus", "react-in-rust-port"].includes(id));
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

// #923: every mutation whose code a change touches is the change's, of
// every list, as its site's file is now; and one whose `find` alone a
// change edits is its own, though its `name` line is as it was.
test("every mutation whose site a change touches is the change's", () => {
  const sources = new Map<string, string>();
  const read = (file: string) => {
    if (!sources.has(file)) sources.set(file, readFileSync(join(root, file), "utf8"));
    return sources.get(file);
  };
  const sited = mutations.flatMap((m) => {
    const lines = siteLines(m, read(m.file)!);
    return lines ? [{ m, lines }] : [];
  });
  expect(sited.length).toBe(mutations.length);
  // One change of each file, each mutation's site its lines.
  const byFile = Map.groupBy(sited, ({ m }) => m.file);
  const changes: Change[] = [...byFile].map(([file, of]) => ({ file, lines: of.map(({ lines }) => lines) }));
  const chosen = new Set(changedMutations(mutations, mutations, changes, read, read));
  expect(mutations.filter((m) => !chosen.has(m)).map((m) => m.name)).toEqual([]);
  // And one site alone chooses its own.
  for (const { m, lines } of sited.filter((_, i) => i % 500 === 0)) {
    expect(changedMutations(mutations, mutations, [{ file: m.file, lines: [lines] }], read, read)).toContain(m);
  }
  // A test file as it was is read once, not once for each mutation naming
  // it: 5,000 `git show`s were 25 s of choosing.
  let reads = 0;
  const readBase = (file: string) => (reads++, read(file));
  changedMutations(mutations, mutations, [{ file: "test/lowering.test.ts", lines: [[1, 1]], base: true }], read, readBase);
  expect(reads).toBe(1);
  const before = mutation("items-never-cells", "src/a.rs", "match self.is_object(item) {");
  const after = { ...before, find: "    .is_some_and(|item| self.is_cell(item))" };
  expect(ownMutations([after, mutation("kept", "src/a.rs")], [before, mutation("kept", "src/a.rs")]).map((m) => m.name)).toEqual(["items-never-cells"]);
});

// The corpus's tests are its cases: a change to what they share, its
// harness, runs the harness's own mutants (#918), not every mutation that
// names a case, 5,805 of them in 34 min; one to a case runs its mutations.
test("a change to the corpus's harness runs no mutation of a case, a change to a case its own", () => {
  const harness = "import { test } from 'bun:test';\n\nfunction check() {}\n\nfor (const name of cases) {\n  test(name, () => check());\n}\n";
  const read = (file: string) => (file === "test/corpus.test.ts" ? harness : undefined);
  const now = [mutation("of-a-case", "src/b.rs", "x", "y", ["test/corpus.test.ts", "-t", "^array_map\\.rs$"])];
  const names = (changes: Change[]) => changedMutations(now, now, changes, read, read).map((m) => m.name);
  expect(names([{ file: "test/corpus.test.ts", lines: [[3, 3]] }])).toEqual([]);
  expect(names([{ file: "test/corpus.test.ts", lines: [[1, 1]], base: true }])).toEqual([]);
  expect(names([{ file: "test/corpus/array_map.rs", lines: [[1, 1]] }])).toEqual(["of-a-case"]);
});

// One its unit tests catch (#915) runs where a change touches those tests:
// a source file's tests of the filter's module, from its `#[cfg(test)]`,
// or a tests module's file whole.
test("a unit-tested mutation is a change's where the change touches its unit tests", () => {
  const jumps = "fn tidy() {\n    a();\n}\n\n#[cfg(test)]\nmod tests {\n    #[test]\n    fn chain() {}\n}\n";
  const files: Record<string, string> = { "src/lower/jumps.rs": jumps, "src/lower/jumps/tests.rs": "#[test]\nfn more() {}\n", "src/lower.rs": jumps };
  const read = (file: string) => files[file];
  const unit = { name: "unit", breaks: "", file: "src/lower/x.rs", find: "a", replace: "b", unitTests: "lower::jumps::tests::chain" };
  const names = (changes: Change[]) => changedMutations([unit], [unit], changes, read, read).map((m) => m.name);
  expect(names([{ file: "src/lower/jumps.rs", lines: [[8, 8]] }])).toEqual(["unit"]);
  expect(names([{ file: "src/lower/jumps.rs", lines: [[9, 9]], base: true }])).toEqual(["unit"]);
  expect(names([{ file: "src/lower/jumps/tests.rs", lines: [[1, 1]] }])).toEqual(["unit"]);
  // A parent's tests may be its, of an inline `mod jumps`; not the code
  // above its tests, another module's tests, nor a test file's.
  expect(names([{ file: "src/lower.rs", lines: [[8, 8]] }])).toEqual(["unit"]);
  expect(names([{ file: "src/lower/jumps.rs", lines: [[2, 2]] }])).toEqual([]);
  expect(names([{ file: "src/other.rs", lines: [[8, 8]] }, { file: "test/x.test.ts", lines: [[1, 1]] }])).toEqual([]);
  expect([unitTestsModule("src/main.rs"), unitTestsModule("src/lower/mod.rs"), unitTestsModule("src/lower/jumps.rs"), unitTestsModule("test/x.ts")]).toEqual([
    "",
    "lower",
    "lower::jumps",
    undefined,
  ]);
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
  // A struct's pattern in its parameters has a `{` of its own, no body's:
  // cut there, a build of many didn't build (#479).
  const destructured = "struct P {\n    x: u32,\n}\n\nfn x(\n    P {\n        x,\n    }: P,\n) -> u32 {\n    x + 1\n}\n";
  const at = site(inProgram("x-off", "x + 1", "x + 2"), () => destructured);
  expect(destructured.slice(at!.start, at!.end)).toBe("\n    x + 1\n");
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
// (check.yml's `metamorphic-cases`): #1115's case ran as Rust does, but not
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
