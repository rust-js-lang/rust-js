// What the CI workflow runs where (DEVELOPMENT.md): its jobs, their gates
// and each branch's latest run; how the suite and the mutations are split
// into shards is test/shards.test.ts's, and which of the mutations a
// change touches test/mutation-changes.test.ts's.

import { expect, test } from "bun:test";
import { existsSync, mkdirSync, mkdtempSync, readdirSync, readFileSync, rmSync, statSync, utimesSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, relative } from "node:path";
import { checkedSince, jsonLines, latestRuns, MUTATION_SHARDS, mutationJobsPassed, NET_JOBS, nets, netsSummary, alwaysRun, resultsBase, wasmResults } from "../scripts/ci";
import { readersOf } from "./part-settings";
import { runSync } from "./child";
import { downloaded } from "../scripts/artifacts";
import { corpusRows, crashes, declarationOf, dependency, labelProblems, lockedVersion, lockOf, namesMachine, sampledHeader, sampledIn, sampledOneIn, snapshotProblems, written } from "../scripts/crate-corpus";
import { cargoHome, checkout } from "./bounded";
import { judgedInTheWasmJob, mutations, refusedFlags } from "../scripts/mutations";
import { installNeeded, jobs, leftToCi, manifestsHash, readabilityPaths, type Scope, touchesReadability, variantFamilies } from "../scripts/checkFast";
import { lostObject } from "../scripts/testMap";
import { portRefusal, reactGate } from "../scripts/push";
import { unrerunnable } from "../scripts/attribute";
import { changedCases, PORT_DOES_NOT_COMPILE, ROWS_MOST_SHARDS, ROWS_PARTS, rowsShards, runs, sliceReport, sliceReports, touchesAsync, touchesConsumers, touchesCrates, importsOf, touchesPort, touchesRustc, touchesWasm, wasmInputs } from "../scripts/iteratorSlice";
import * as slices from "../scripts/iteratorSlice";
import { browsers, drivesBrowser, junitReport, junitReports } from "../scripts/shard";
import { loadScale, underLoad } from "../scripts/timing";
import { caseOf, partsOf, runParts, unparted, verdict } from "../scripts/parts";
import { jobs as localNightly } from "../scripts/nightly";
import { harness, nightly, parts, pooledParts, shard } from "../scripts/shard";
import { programFlags, root } from "./support";
import { miriCommand } from "./miri";

// A compiler CI keeps is of what it's made of, build.rs's inputs, the one
// list, the compiler's crates' too (ADR 1181): a key short of one, std/manifest.json, which crates/rust-js-program/src/library.rs
// includes (and std/compiled-std.json, the catalogue it reads, #2305), restored a binary of other sources. The suite's shards restore
// the debug one so, as the release one is (check 43700659375: 136 s of
// each shard's longest leg built it again of unchanged sources).
test("a compiler CI keeps is keyed by build.rs's inputs, and the suite's shards run the one kept", () => {
  const script = readFileSync(join(root, "build.rs"), "utf8");
  const inputs = [...script.slice(script.indexOf("let inputs = ["), script.indexOf("];", script.indexOf("let inputs = ["))).matchAll(/join\("([^"]+)"\)/g)].map((m) => (statSync(join(root, m[1]!)).isDirectory() ? `${m[1]}/**` : m[1]!));
  expect(inputs).toEqual(["src/**", "crates/**", "std/manifest.json", "std/compiled-std.json", "rust-toolchain.toml", "build.rs", "rustc-rpath.rs", "Cargo.toml", "Cargo.lock"]);
  const action = readFileSync(join(root, ".github/actions/build-rust-js/action.yml"), "utf8");
  expect(action).toContain(`key: rust-js-\${{ inputs.profile }}-\${{ runner.os }}-\${{ hashFiles(${inputs.map((input) => `'${input}'`).join(", ")}) }}`);
  const check = Bun.YAML.parse(readFileSync(join(root, ".github/workflows/check.yml"), "utf8")) as { jobs: Record<string, { steps: { uses?: string; with?: Record<string, string>; id?: string; env?: Record<string, string> }[] }> };
  const steps = check.jobs.test!.steps;
  expect(steps.filter((step) => step.uses === "./.github/actions/build-rust-js").map((step) => step.with)).toEqual([{ profile: "debug" }]);
  expect(steps.find((step) => step.id === "test")!.env!.RUST_JS_COMPILER).toBe(["${{ github.workspace }}", "target", "debug", "rust-js"].join("/"));
  expect(readFileSync(join(root, "scripts/test.ts"), "utf8")).toContain("if (!process.env.RUST_JS_COMPILER) {");
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

// std/'s freshness runs on each push's check, in its lint job, after the
// compiler's built: a merge on GitHub takes no push gate, and std/ left as
// it was tested a runtime change against the old code (#1971).
test("each push's lint job checks std/ is what its sources make, after the compiler's built", () => {
  const lint = (Bun.YAML.parse(readFileSync(join(root, ".github/workflows/check.yml"), "utf8")) as { jobs: Record<string, { steps: { run?: string }[] }> }).jobs.lint;
  const runs = lint.steps.map((step) => step.run ?? "");
  const fresh = runs.indexOf("bun scripts/std-fresh.ts");
  expect(fresh).toBeGreaterThan(runs.indexOf("cargo build --locked"));
  expect(runs.indexOf("cargo build --locked")).toBeGreaterThanOrEqual(0);
  // The one check, which `bun run push`'s std phase runs too.
  expect(readFileSync(join(root, "scripts/push.ts"), "utf8")).toContain('run("std", ["bun", "scripts/std-fresh.ts"]);');
  // And CI's map is fetched beside the round, waited for at its selection
  // (#2385).
  const push = readFileSync(join(root, "scripts/push.ts"), "utf8");
  const fetch = push.indexOf('beside(["bun", "scripts/testMap.ts", "fetch"], mapFetchLog)');
  expect([fetch > 0, fetch < push.indexOf('run("build",'), push.indexOf("await fetching!.done") > push.indexOf('run("build",')]).toEqual([true, true, true]);
});

// `check:fast --all`, the check before a push, runs what a push's check
// runs (#732): each command of check.yml's jobs is one of its jobs' (`ci`),
// or one `leftToCi` says why it leaves to CI, and each of those is still
// check.yml's. `cargo test` and the pilot's committed JS, run by the check
// alone, went red on `main` unseen (801d40c55, 336984bff).
// Of a change to the iterators' code, the formatting code, the numbers' and the pointers', whose
// jobs check.yml runs where its change is one (scripts/iteratorSlice.ts).
const checkJobs = jobs({ all: true, changed: [], branch: ["crates/rust-js-program/src/runtime/steps.js", "crates/rust-js-program/src/runtime/pad.js", "crates/rust-js-program/src/runtime/div.js", "crates/rust-js-emit/src/typescript.rs", "crates/rust-js-lower/src/analysis/awaits.rs", "crates/rust-js-lower/src/unions.rs", "crates/rust-js-lower/src/analysis/intervals/state.rs", "crates/rust-js-lower/src/drops/ref_cells.rs", "test/corpus/array_map.rs"], extra: [], cores: 4 });
const ranHere = checkJobs.jobs;

// The readability passes' oracles run of a change to tidy, preparation or
// the oracles alone, `--all`'s too, as a push's CI runs them: of
// readability-passes.yml's paths, the one list (the user, #998).
test("the readability passes run where readability-passes.yml's paths say, of --all too", () => {
  const paths = readabilityPaths();
  expect(paths).toEqual(expect.arrayContaining(["crates/rust-js-program/src/prepare/**", "crates/rust-js-mir/src/tidy/**", "crates/rust-js-lower/src/mut_refs/**", "crates/rust-js-lower/src/noted/**", "test/getter-reads/**", ".github/workflows/readability-passes.yml"]));
  const named = (scope: { all: boolean; changed: string[]; branch: string[] }) =>
    jobs({ ...scope, extra: [], cores: 12 }).jobs.some((job) => job.name === "readability passes");
  expect(named({ all: true, changed: ["crates/rust-js-lower/src/copies.rs"], branch: [] })).toBe(false);
  expect(named({ all: true, changed: [], branch: ["crates/rust-js-mir/src/tidy/exits.rs"] })).toBe(true);
  expect(named({ all: false, changed: ["crates/rust-js-program/src/prepare.rs"], branch: [] })).toBe(true);
  expect(named({ all: false, changed: ["test/getter-reads/counts.rs"], branch: [] })).toBe(true);
  expect(touchesReadability(["crates/rust-js-mir/src/tidying.rs", "docs/guide/README.md"])).toBe(false);
  expect(() => readabilityPaths("on:\n  push:\n")).toThrow("no `on.push.paths`");
});

// A net's paths name a Rust module whole: a folder's glob, `drops/**`,
// with the module's own file, `drops.rs`, which holds its code too. The
// drops' paths had the folder alone, so 845c1a3e9's change to
// `check_drops_given` in drops.rs ran neither the consumer net nor the
// async table in its push, and a push that selected everything found the
// consumer net's known list stale (#2575).
test("each net's paths take a module's own file with its folder", () => {
  const catalogs = Object.entries(slices).filter((entry): entry is [string, string[]] => entry[0].endsWith("Paths") && Array.isArray(entry[1]));
  expect(catalogs.map(([name]) => name)).toEqual(expect.arrayContaining(["iteratorPaths", "consumerPaths", "asyncPaths", "cellsPaths"]));
  const halves = catalogs.flatMap(([name, paths]) =>
    paths
      .filter((path) => path.endsWith("/**") && existsSync(join(root, `${path.slice(0, -3)}.rs`)))
      .filter((path) => !paths.some((other) => new Bun.Glob(other).match(`${path.slice(0, -3)}.rs`)))
      .map((path) => `${name}: ${path} without ${path.slice(0, -3)}.rs`),
  );
  expect(halves).toEqual([]);
  expect([touchesConsumers(["crates/rust-js-lower/src/drops.rs"]), touchesAsync(["crates/rust-js-lower/src/drops.rs"])]).toEqual([true, true]);
});

// Each family of tidy-stages' variants runs of its own paths: the
// intervals' of a change to the facts MIR's intervals keep or to what
// writes past them, never of tidy's, nor tidy's of theirs (#2112 B).
test("the intervals' variant runs where intervalsPaths say, and the readability passes don't", () => {
  const named = (changed: string[]) => jobs({ all: true, changed, branch: [], extra: [], cores: 12 }).jobs.map((job) => job.name);
  const intervals = named(["crates/rust-js-lower/src/analysis/intervals/state.rs"]);
  expect(intervals).toContain("the intervals' variant");
  expect(intervals).not.toContain("readability passes");
  expect(named(["crates/rust-js-lower/src/pointers/keeps.rs"])).toContain("the intervals' variant");
  const tidy = named(["crates/rust-js-mir/src/tidy/exits.rs"]);
  expect(tidy).toContain("readability passes");
  expect(tidy).not.toContain("the intervals' variant");
  expect(variantFamilies.intervals(["crates/rust-js-lower/src/copies.rs"])).toBe(false);
  // The cells' of a change to what decides a borrow's form (#2491).
  const cells = named(["crates/rust-js-lower/src/drops/ref_cells.rs"]);
  expect(cells).toContain("the cells' variant");
  expect(cells).not.toContain("the intervals' variant");
  expect(named(["crates/rust-js-mir/src/local_facts.rs"])).toContain("the cells' variant");
  expect(variantFamilies.cells(["crates/rust-js-lower/src/analysis/intervals/state.rs"])).toBe(false);
});

// `--all` runs the metamorphic net's push sample in parts, as the corpus's,
// each a process the pool starts where a core is free: one `bun test` of
// it, 204 to 238 s on about one core, ran on alone at a check's end, past a
// consumer's 720 s (#1749). Each part once, `RUST_JS_CASES=i/n`, none of
// the whole file beside them.
test("--all runs the metamorphic sample in parts, each once, and never the whole file beside them", () => {
  const sample = (cores: number) =>
    jobs({ all: true, changed: [], branch: [], extra: [], cores }).jobs.filter((job) => job.cmd.join(" ") === "bun test test/metamorphic.test.ts");
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
    jobs({ ...scope, extra: [], cores: 12, picked: none }).jobs.some((job) => job.name === "the crate corpus");
  expect(named({ all: false, changed: ["crates/rust-js-lower/src/iterators/mir_steps/closing.rs"], branch: [] })).toBe(true);
  expect(named({ all: false, changed: [], branch: ["crates/rust-js-program/src/runtime/byte_len.js"] })).toBe(true);
  expect(named({ all: false, changed: ["scripts/crate-corpus/itertools.rs"], branch: [] })).toBe(true);
  expect(named({ all: false, changed: ["docs/guide/README.md", "test/ci.test.ts"], branch: [] })).toBe(false);
  expect(named({ all: true, changed: ["docs/guide/README.md"], branch: [] })).toBe(true);
  expect(touchesCrates(["std/lib.rs"])).toBe(true);
  expect(touchesCrates(["tooling/build.js"])).toBe(false);
  expect(readFileSync(join(root, ".github/workflows/check.yml"), "utf8")).toContain("needs.plan.outputs.crates == 'true'");
  // And React's port, of a change to what compiles it (#1188).
  expect(touchesPort(["tooling/build.js"])).toBe(true);
  expect(touchesPort(["crates/rust-js-lower/src/copies.rs"])).toBe(true);
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
// corpus with std's iterator structs, `iterator-rows`, the rows a push
// names, and the jobs of nets, whose steps are the
// tables' slices, the declarations' callers, the twin net, the crate
// corpus's sample and React's port, which the nightly skips, as its
// `matrix`, `conformance`, `forms`, `format`, `numbers`, `async`,
// `consumers`, `std` and `twins` run all of each, and its suite the
// callers; and `decision-coverage`, the
// night's decision counts merged, which `bun run decision-coverage` is.
// The small Crater has one definition, crater.yml, started by hand for the
// crates it's asked of and called by the Nightly for every one: a fix of the
// Crater was proved only by the next night, the Nightly's copy of its steps
// the only way to run it on CI (#2460).
test("the small Crater is crater.yml's alone, the Nightly calling it", () => {
  type Workflow = { on: Record<string, { inputs?: Record<string, unknown> } | null>; jobs: Record<string, { uses?: string; with?: Record<string, string>; steps?: { run?: string }[] }> };
  const read = (file: string) => Bun.YAML.parse(readFileSync(join(root, ".github/workflows", file), "utf8")) as Workflow;
  const crater = read("crater.yml");
  expect(Object.keys(crater.on).sort()).toEqual(["workflow_call", "workflow_dispatch"]);
  for (const on of ["workflow_call", "workflow_dispatch"]) expect(Object.keys(crater.on[on]?.inputs ?? {}), on).toContain("crates");
  const runs = (workflow: Workflow) => Object.values(workflow.jobs).flatMap((job) => job.steps ?? []).filter((step) => step.run?.includes("scripts/crater.ts"));
  expect(runs(crater).length).toBe(2);
  const nightly = read("nightly.yml");
  expect(runs(nightly)).toEqual([]);
  expect(Object.entries(nightly.jobs).filter(([, job]) => job.uses === "./.github/workflows/crater.yml").map(([id, job]) => [id, job.with?.shards])).toEqual([["crater", "[1, 2, 3, 4]"]]);
});

test("the nightly, here, has each job the nightly has", () => {
  const ids = (file: string) => {
    const text = readFileSync(join(root, ".github/workflows", file), "utf8");
    return [...text.slice(text.indexOf("\njobs:")).matchAll(/^  ([a-z-]+):$/gm)].map((m) => m[1]);
  };
  const ran = [...ids("check.yml"), ...ids("nightly.yml")].filter((id) => !["plan", "seeds", "check", "check-mutations", "issue", "iterator-forms", "iterator-rows", ...NET_JOBS, "decision-coverage", "test-times", "selector-audit", "clusters", "attribute-plan", "attribute", "attribute-report", "attribute-mutations", "ci-spend"].includes(id));
  expect(ran.filter((id) => !(id in localNightly))).toEqual([]);
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
// `main` left no check of it finished for hours. And one started by hand
// waits apart from the pushes': GitHub keeps only the newest of a group's
// waiting runs, and a measuring check in the pushes' group dropped two
// pushes' checks waiting behind it (44846113211).
test("a check is never cancelled by another run", () => {
  const dir = join(root, ".github/workflows");
  type Workflow = { concurrency?: { group?: string; "cancel-in-progress"?: unknown } };
  const concurrency = (file: string) => (Bun.YAML.parse(readFileSync(join(dir, file), "utf8")) as Workflow).concurrency ?? {};
  const check = concurrency("check.yml");
  expect(check["cancel-in-progress"]).toBe(false);
  expect(check.group).toContain("format('ci-{0}-{1}', github.ref, github.event_name)");
  // Started by hand, rustc-tests.yml's group is its own, never the check's `ci-<ref>`.
  expect(concurrency("rustc-tests.yml").group).toContain("format('rustc-{0}', github.ref)");
  // Called by a check, its group, which cancels, is of the caller's branch:
  // one branch's check never cancels another's rustc tests or mutations.
  const called = concurrency("rustc-tests.yml").group ?? "";
  expect(called.slice(called.indexOf("||"))).toContain("github.ref");
  for (const file of readdirSync(dir).filter((f) => f.endsWith(".yml") && f !== "check.yml")) {
    const group = concurrency(file).group ?? "";
    expect(group, file).not.toContain("== 'Check' && format('ci-{0}-{1}', github.ref, github.event_name)");
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

// The crate corpus's JS is under the label rule as the corpus's and std's
// are: a label MIR made that isn't listed fails, and a listed one that's
// gone of a crate this run measured, not of one it didn't choose (#2215).
test("the crate corpus's labels MIR made are listed, each listed one still there", () => {
  const listed = new Set(["num-traits@0.2.19/lib.js: f", "itertools's probe/lib.js: g", "uuid@1.29.0/error.js: h"]);
  const measured = (crate: string) => crate !== "uuid";
  expect(labelProblems(new Set(["num-traits@0.2.19/lib.js: f", "itertools's probe/lib.js: g"]), listed, measured)).toEqual([]);
  const problems = labelProblems(new Set(["num-traits@0.2.19/lib.js: new"]), listed, measured);
  expect(problems).toHaveLength(3);
  expect(problems[0]).toStartWith("num-traits@0.2.19/lib.js: new: a label MIR made");
  expect(problems.slice(1).every((p) => p.endsWith("and gone: take it off the list"))).toBe(true);
});

// A real crate's functions no corpus case makes are held to a snapshot,
// each cut from its file whole, as it reads (#1979), and JS that names the
// machine's `$CARGO_HOME` or rust-js's checkout is found (#2435, #2438): a listed one that differs fails the check, `BLESS=1` writes it,
// and one gone from its file, or a snapshot no entry lists, fails too.
test("the crate corpus's snapshotted functions are cut whole and held", () => {
  const js = ["/** Put back. */", "export function putBack(a) {", "  return {", `    at: "${cargoHome}/registry/src/index/x-1.0.0/src/lib.rs:1:1",`, "  };", "}", "", "export function putBackDebug(a) {}", "", "export const PutBack = {", "  of(a) {", "    return a;", "  },", "};"].join("\n");
  expect(declarationOf(js, "putBack")).toBe(js.split("\n").slice(0, 6).join("\n"));
  expect(declarationOf(js, "putBackDebug")).toBe("export function putBackDebug(a) {}");
  expect(declarationOf(js, "PutBack")).toBe(js.split("\n").slice(9).join("\n"));
  expect(declarationOf(js, "put")).toBeUndefined();
  expect(namesMachine(js)).toBe(true);
  expect(namesMachine(`"${checkout}/tooling/compat/js-sys/src/lib.rs:1:1"`)).toBe(true);
  expect(namesMachine('"/cargo/registry/src/index/x-1.0.0/src/lib.rs:1:1"')).toBe(false);
  expect(namesMachine('"/rust-js/0.0.4/tooling/compat/js-sys/src/lib.rs:1:1"')).toBe(false);
  const dir = mkdtempSync(join(tmpdir(), "crate-snapshots-"));
  try {
    const [built, kept] = [join(dir, "x"), join(dir, "snapshots")];
    mkdirSync(built);
    writeFileSync(join(built, "lib.js"), js);
    const listed = ["x@1.0.0/lib.js: putBack", "x@1.0.0/lib.js: PutBack"];
    const check = (bless = false, of = listed) => snapshotProblems((crate) => (crate === "x@1.0.0" ? built : undefined), (crate) => crate === "x", { bless, listed: of, dir: kept });
    expect(check()).toEqual([expect.stringContaining("x@1.0.0/lib.js: its JS isn't scripts/crate-corpus/snapshots/x@1.0.0/lib.js, which isn't written")]);
    expect(check(true)).toEqual([]);
    const snapshot = readFileSync(join(kept, "x@1.0.0/lib.js"), "utf8");
    expect(snapshot).toContain(`"${cargoHome}/registry/src/index/x-1.0.0/src/lib.rs:1:1"`);
    expect(snapshot).not.toContain("putBackDebug");
    expect(check()).toEqual([]);
    writeFileSync(join(built, "lib.js"), js.replace("return a;", "return a + 1;"));
    expect(check()).toEqual([expect.stringContaining('x@1.0.0/lib.js, first at its line 13: "    return a;", now "    return a + 1;"')]);
    expect(check(false, [...listed, "x@1.0.0/lib.js: gone"])).toContainEqual(expect.stringContaining("declares no `gone`"));
    writeFileSync(join(kept, "x@1.0.0/other.js"), "");
    expect(check(true)).toEqual([]);
    writeFileSync(join(kept, "x@1.0.0/other.js"), "");
    expect(check()).toEqual([expect.stringContaining("snapshots/x@1.0.0/other.js: no entry of `snapshotted` lists it")]);
    expect(check(false, ["x@1.0.0/gone.js: putBack"])).toContainEqual(expect.stringContaining("x@1.0.0/gone.js: snapshotted, and x@1.0.0's JS has no gone.js"));
    expect(snapshotProblems(() => undefined, () => false, { listed, dir: kept })).toEqual([]);
    expect(snapshotProblems(() => undefined, (crate) => crate === "x", { listed, dir: kept })).toContainEqual(expect.stringContaining("x@1.0.0 wasn't compiled"));
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

// A change that adds or edits a corpus case runs each metamorphic rewrite of
// it, in check:fast, plain or `--all`, as each push's check does
// (check.yml's `nets` job's `metamorphic` step): #1115's case ran as Rust does, but not
// moved into a library, which nothing ran till the nightly (#1164, #1165).
test("a change adding or editing a corpus case runs each of its rewrites", () => {
  expect(changedCases(["test/corpus/new_case.rs", "crates/rust-js-mir/src/lib.rs", "test/corpus/new_case.js", "test/corpus/edited.rs", "test/corpus/new_case.rs"])).toEqual([
    "edited.rs",
    "new_case.rs",
  ]);
  const ofCases = (scope: { all: boolean; changed: string[]; branch: string[] }) =>
    jobs({ ...scope, extra: [], cores: 12 }).jobs.filter((job) => job.name === runs.metamorphic[0].name);
  for (const all of [false, true]) {
    const [rewrites] = ofCases({ all, changed: ["test/corpus/new_case.rs"], branch: [] });
    expect(rewrites.cmd).toEqual(["bun", "scripts/iteratorSlice.ts", "run", "metamorphic", "1"]);
    expect(ofCases({ all, changed: [], branch: ["test/corpus/a.rs", "test/corpus/b.rs"] })[0].cmd.at(-1)).toBe("2");
    expect(ofCases({ all, changed: ["crates/rust-js-mir/src/lib.rs", "test/corpus/new_case.js"], branch: [] })).toEqual([]);
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
      jobs({ all: true, changed: [], branch: [], extra: [], cores: 12, ...scope })
        .jobs.filter((job) => job.size !== undefined)
        .map((job) => [job.name, job.size]),
    );
  const cases = readdirSync(join(root, "test", "corpus")).filter((name) => name.endsWith(".rs")).length;
  expect(sized({ branch: ["crates/rust-js-lower/src/iterators/clones.rs", "test/corpus/a.rs", "test/corpus/b.rs"], mutations: { names: ["m", "m", "n"], base: "b" }, rows: 9175 })).toEqual({
    "the rows the change names": 9175,
    [runs.forms[0].name]: cases,
    [runs.metamorphic[0].name]: 2,
    "the change's mutations": 2,
  });
  expect(sized({ branch: ["crates/rust-js-lower/src/iterators/clones.rs"], rows: 0, forms: { forms: ["a.rs", "b.rs", "c.rs"], formsList: "forms.txt" } })).toEqual({
    "the rows the change names": 0,
    [runs.forms[0].name]: 3,
  });
  expect(sized({ branch: ["crates/rust-js-mir/src/lib.rs"] })).toEqual({});
});

// check:fast judges the change's mutations CI's check judges, those not on
// record for its tree, each survivor at the commit the change left `main`
// at latent there too, as CI's shards judge it (#2485, scripts/judged.ts).
test("check:fast judges the change's mutations as CI's check does, a survivor at its base latent", () => {
  const judging = jobs({ all: true, changed: [], branch: [], mutations: { names: ["m", "n"], base: "abc" }, extra: [], cores: 12 }).jobs.find((job) => job.name === "the change's mutations");
  expect(judging?.cmd.slice(1).filter((arg) => !arg.startsWith("--jobs="))).toEqual(["scripts/mutations.ts", "--base=abc", "m", "n"]);
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
  // The playground's build names no oxc crate of its own: it takes them
  // through printing's crate, at the workspace's tag (ADR 1181).
  expect(tags(manifest("wasm/Cargo.toml"))).toEqual(new Set());
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

// A file in parts is one of cases each part takes its share of: a file of
// none, or a second file among the flags, ran whole in every part, and the
// suite's files in 10 parts filled the disk with 20 GiB of fixtures. Each
// file a workflow runs in parts is one. (The sources below are built, so
// this file reads as one of no cases, which it is.)
test("parts.ts runs one file of cases in parts, and refuses another, naming check:fast", () => {
  const cases = ["RUST", "JS", "CASES"].join("_");
  const sharded = `const [part, parts] = (process.env.${cases} ?? "1/1").split("/").map(Number);`;
  expect(unparted("test/a.test.ts", sharded, ["--timeout=60000"])).toBeUndefined();
  expect(unparted("test/a.test.ts", `for (const name of ${"caseShard"}(cases)) test(name, () => {});`, [])).toBeUndefined();
  expect(unparted("test/ci.test.ts", readFileSync(join(root, "test/ci.test.ts"), "utf8"), [])).toContain("each part would run it whole: run the suite with `bun run check:fast`");
  expect(unparted("test/a.test.ts", sharded, ["test/b.test.ts", "--timeout=1"])).toContain("test/b.test.ts: one test file runs in parts");
  const workflows = readdirSync(join(root, ".github/workflows")).filter((name) => name.endsWith(".yml"));
  const runs = workflows.flatMap((name) => [...readFileSync(join(root, ".github/workflows", name), "utf8").matchAll(/bun scripts\/parts\.ts .*?(test\/\S+\.test\.tsx?)([^\n]*)/g)]);
  expect(runs.length).toBeGreaterThan(0);
  for (const [, file, flags] of runs) expect(unparted(file!, readFileSync(join(root, file!), "utf8"), flags!.trim().split(/\s+/).filter(Boolean)), file).toBeUndefined();
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
// on isn't linked, a checkout whose install predates it: lisa agent's
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
// Chromium alone, never with Playwright's system packages: the runner's
// image has every library it asks, and `--with-deps` only added nine font
// packages through apt, whose Azure mirror timed out on each until a check's
// test shard hung 7.7 min (check 44480685461's shard 2). The browser tests
// measure text and boxes against each other in one page, ours beside
// React's, a panel beside its trigger, never against fixed pixels, so the
// image's own fonts answer as those did.
test("no workflow installs the browser's system packages", () => {
  const dir = join(root, ".github/workflows");
  const installs = readdirSync(dir).flatMap((name) =>
    readFileSync(join(dir, name), "utf8")
      .split("\n")
      .filter((line) => line.includes("playwright install") && !line.trimStart().startsWith("#"))
      .map((line) => [name, line.trim()]),
  );
  expect(installs.length).toBeGreaterThan(4);
  expect(installs.filter(([, line]) => !line.endsWith("playwright install chromium"))).toEqual([]);
});

test("CI's mutations install the browser a mutation's test drives", () => {
  const workflow = readFileSync(join(root, ".github/workflows/rustc-tests.yml"), "utf8");
  const job = workflow.slice(workflow.indexOf("\n  mutations:\n"));
  expect(job).toContain('--browser)" >> "$GITHUB_OUTPUT"');
  expect(job).toContain("if: steps.browser.outputs.needed == 'browser'\n        name: Install the browser\n        run: bunx --bun playwright install chromium");
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
  // What the shards left to the wasm job: caught where its run, of the
  // same commit, caught it; else, its job not run or another commit's,
  // retried (#2346).
  const left = { ...red, results: [found("sha4", { a: "caught", w: judgedInTheWasmJob }), found("sha4", { c: "caught" })] };
  expect(resultsBase([left])).toEqual({ since: "sha4", retry: ["w"] });
  expect(resultsBase([{ ...left, wasm: found("sha4", { w: "caught" }) }])).toEqual({ since: "sha4", retry: [] });
  expect(resultsBase([{ ...left, wasm: found("sha9", { w: "caught" }) }])).toEqual({ since: "sha4", retry: ["w"] });
  expect(resultsBase([{ ...left, wasm: found("sha4", { w: "SURVIVED: its tests passed" }) }])).toEqual({ since: "sha4", retry: ["w"] });
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
  expect(check).toContain("if: ${{ inputs.part != 'mutations' && (inputs.nightly || needs.plan.outputs.wasm == 'true') }}");
  expect(check).toContain("if: ${{ inputs.part != 'mutations' && (inputs.nightly || needs.plan.outputs.rustc == 'true') }}");
  expect(check).toContain("if: ${{ inputs.part != 'suite' && (inputs.nightly || needs.plan.outputs.mutations == 'true') }}");
  // And the mutations that need the WASM, in its job, of the shards' own
  // selection, their results beside the shards' (#2346).
  const wasmJob = check.slice(check.indexOf("\n  wasm:\n"), check.indexOf("\n  rustc:\n"));
  expect(check).toContain("wasm_mutations: ${{ steps.plan.outputs.wasm_mutations }}");
  expect(wasmJob).toContain("needs.plan.outputs.wasm_mutations == 'true'");
  expect(wasmJob).toContain("fetch-depth: 0");
  expect(wasmJob).toContain("bun scripts/mutations.ts --wasm $ARGS");
  // The Nightly's in its suite part's call, whose wasm job runs anyway: its
  // mutations part's would hold one more runner past its budget.
  expect(wasmJob).toContain("(inputs.nightly && inputs.part != 'mutations') || (!inputs.nightly && inputs.part != 'suite' && needs.plan.outputs.wasm_mutations == 'true')");
  expect(wasmJob).toContain("format('--changed={0} --also={1} --base={0}', needs.plan.outputs.since, needs.plan.outputs.retry)");
  expect(wasmJob).toContain(`name: ${wasmResults}\n`);
  expect(wasmJob.indexOf("bun run wasm")).toBeLessThan(wasmJob.indexOf("--wasm $ARGS"));
  expect(wasmInputs("INPUTS=(src Cargo.toml wasm/patches)\n")).toEqual(["src/**", "Cargo.toml", "wasm/patches/**"]);
  for (const file of ["crates/rust-js-lower/src/copies.rs", "wasm/patches/a.patch", "rust-toolchain.toml", "std/lib.rs", "test/snapshots/x.js", "examples/fib.rs"]) {
    expect([file, touchesWasm([file])]).toEqual([file, true]);
  }
  for (const file of ["crates/rust-js-lower/src/copies.rs", "std/lib.rs", "test/rustc-known-failures.txt", "scripts/rustc-suite.ts", ".github/workflows/rustc-tests.yml"]) {
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

// The nets of a few minutes are steps of three jobs, `NET_JOBS`: the
// iterator nets, the program nets and the rest, each in the background
// where `plan` says its change runs it: each a job of its own spent most of
// its minutes setting up, and one job of them all was the check's longest
// leg, its iterator and program nets its longest steps (docs/dev-loop.md).
// A red job says which net failed by its step's name, and its summary
// names each (dev-loop change 3).
test("the short nets are the steps of the jobs of nets, each where its change runs it, each failure named", () => {
  const listed = nets();
  expect(listed.map((net) => net.id)).toEqual(["iterators", "conformance", "paths", "copies", "consumers", "twins", "format", "numbers", "memory", "intervals", "cells", "declarations", "async", "std", "metamorphic", "crates", "port"]);
  type Step = { id?: string; if?: string; background?: boolean; "wait-all"?: null };
  const workflow = Bun.YAML.parse(readFileSync(join(root, ".github/workflows/check.yml"), "utf8")) as { jobs: Record<string, { if?: string; steps: Step[] }> };
  expect(Object.keys(workflow.jobs).filter((name) => name.startsWith("nets"))).toEqual([...NET_JOBS]);
  for (const name of NET_JOBS) {
    const job = workflow.jobs[name]!;
    const outputs: string[] = [];
    for (const step of job.steps.filter((s) => s.background)) {
      const output = /^needs\.plan\.outputs\.(\w+) == 'true'$/.exec(step.if ?? "")?.[1];
      outputs.push(output ?? "");
      expect([name, step.id, output !== undefined && job.if!.includes(`needs.plan.outputs.${output} == 'true'`)]).toEqual([name, step.id, true]);
    }
    // Its gate is its steps' and no other's: none starts a machine to run nothing.
    const gated = [...job.if!.matchAll(/needs\.plan\.outputs\.(\w+) == 'true'/g)].map((m) => m[1]!);
    expect([name, [...new Set(gated)].sort()]).toEqual([name, [...new Set(outputs)].sort()]);
    const waits = job.steps.findIndex((s) => "wait-all" in s);
    expect([name, waits]).toEqual([name, job.steps.findLastIndex((s) => s.background) + 1]);
  }
  const summary = netsSummary({ iterators: { outcome: "failure" }, std: { outcome: "success" }, port: { outcome: "skipped" } }, listed);
  expect(summary).toContain("### 1 net failed: iterator matrix slice");
  expect(summary).toContain("- std net: passed");
  expect(summary).not.toContain("react-in-rust port");
  expect(netsSummary({ std: { outcome: "success" } }, listed)).toContain("### Every net that ran passed");
});

// The rows a push names run on machines of their own, as many as `plan`
// says its pick needs, and none of a push that names none: a step of
// `nets-iterators` beside its four slices, they were the check's longest
// leg (#2374). `plan` and the job pick them alike, since the same commit,
// in the same parts.
test("the rows a push names are a job of their own, on the machines plan's pick of them needs", () => {
  type Step = { name?: string; run?: string; env?: Record<string, string> };
  type Job = { needs?: string[]; if?: string; outputs?: Record<string, string>; strategy?: { matrix?: { shard?: string } }; steps: Step[] };
  const check = Bun.YAML.parse(readFileSync(join(root, ".github/workflows/check.yml"), "utf8")) as { jobs: Record<string, Job> };
  const rows = check.jobs["iterator-rows"]!;
  expect(rows.needs).toEqual(["plan", "build"]);
  expect(rows.if).toBe("${{ !inputs.nightly && needs.plan.outputs.rows == 'true' }}");
  expect(rows.strategy?.matrix?.shard).toBe("${{ fromJSON(needs.plan.outputs.rows_shard_list) }}");
  const step = rows.steps.find((s) => s.run?.includes("iteratorSlice.ts run rows"))!;
  expect(step.run).toBe(`bun scripts/iteratorSlice.ts run rows ${ROWS_PARTS} --shard=\${{ matrix.shard }}/\${{ needs.plan.outputs.rows_shards }}`);
  const plan = check.jobs.plan!;
  for (const output of ["rows", "rows_shards", "rows_shard_list"]) expect(plan.outputs?.[output]).toBe(`\${{ steps.plan.outputs.${output} }}`);
  const base = "${{ github.event.before }}";
  expect([step.env?.RUST_JS_ROWS_BASE, plan.steps.find((s) => s.run?.startsWith("bun scripts/ci.ts plan"))?.env?.RUST_JS_ROWS_BASE]).toEqual([base, base]);
  // No job of nets runs them too.
  for (const name of NET_JOBS) expect([name, check.jobs[name]!.steps.some((s) => s.run?.includes("run rows"))]).toEqual([name, false]);
  // None of no pick; one where each table's rows fit a machine's budget;
  // more of a removal's past it, at most ROWS_MOST_SHARDS.
  const budget = () => 1_000;
  const picked = (table: "matrix" | "conformance", count: number) => ({ table, setting: "", rows: count, why: [] });
  expect(rowsShards([], budget)).toBe(0);
  expect(rowsShards([picked("matrix", 600), picked("matrix", 400), picked("conformance", 900)], budget)).toBe(1);
  expect(rowsShards([picked("matrix", 600), picked("matrix", 401)], budget)).toBe(2);
  expect(rowsShards([picked("matrix", 10), picked("conformance", 2_500)], budget)).toBe(3);
  expect(rowsShards([picked("matrix", 90_113)], budget)).toBe(ROWS_MOST_SHARDS);
});

// The nightly keeps each job's, file's and test's time (scripts/testTimes.ts),
// the suite's by bun's JUnit reports, after every job that runs tests: a
// job added to the night and not to its `needs` is timed by none.
test("the nightly's test times follow each of its jobs, the suite's reports among them", () => {
  const nightly = Bun.YAML.parse(readFileSync(join(root, ".github/workflows/nightly.yml"), "utf8")) as { jobs: Record<string, { needs?: string[]; if?: string }> };
  const times = nightly.jobs["test-times"]!;
  expect(times.if).toBe("always()");
  const others = Object.keys(nightly.jobs).filter((id) => !["test-times", "issue", "decision-coverage", "seeds", "selector-audit", "clusters", "ci-spend"].includes(id));
  expect(others.filter((id) => !times.needs!.includes(id))).toEqual([]);
  expect(readFileSync(join(root, ".github/workflows/check.yml"), "utf8")).toContain("name: junit-${{ matrix.shard }}");
  expect(readFileSync(join(root, "scripts/test.ts"), "utf8")).toContain("junitReport(junitDir, name)");
});

// bun writes no JUnit report into a directory that isn't there, and says so
// and passes: no test shard kept one until scripts/test.ts made it, so a red
// check's `attribute` saw no failing test of the suite, and the nightly no
// suite's times. Its upload fails a shard whose suite ran and kept none.
test("each test shard whose suite ran keeps its JUnit reports, or fails", () => {
  type Step = { id?: string; uses?: string; if?: string; with?: Record<string, string> };
  const check = Bun.YAML.parse(readFileSync(join(root, ".github/workflows/check.yml"), "utf8")) as { jobs: Record<string, { steps?: Step[] }> };
  const steps = check.jobs.test!.steps!;
  expect(steps.some((step) => step.id === "test")).toBe(true);
  const upload = steps.find((step) => step.uses?.startsWith("actions/upload-artifact") && step.with?.name === "junit-${{ matrix.shard }}");
  expect([upload?.if, upload?.with?.path, upload?.with?.["if-no-files-found"]]).toEqual(["always() && steps.test.outcome != 'skipped'", junitReports, "error"]);
  // Each report's directory made as it's asked for.
  const dir = mkdtempSync(join(tmpdir(), "junit-"));
  try {
    const reports = join(dir, junitReports);
    expect(junitReport(reports, "corpus 1/16")).toEqual(["--reporter=junit", `--reporter-outfile=${join(reports, "corpus-1-16.xml")}`]);
    expect(existsSync(reports)).toBe(true);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
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
  const idempotent = ["cache-prune.yml"];
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

// The mutation shards' snapshot judge places a site by the test map, the
// store test-map.yml served or recorded in its run, restored from the
// Actions cache by the key it gives (`outputs.key`), not an artifact each
// shard downloads: test-map.yml called by rustc-tests.yml.
// A called workflow's event is its caller's, `push` of each
// check of main, and test-map.yml named its artifact by it, so a call
// uploaded `test-map-of-<sha>` and all 16 shards failed to download
// `test-map` (check 43889550882). Each job that downloads `test-map` needs a
// job of its run that calls test-map.yml, which tells a call from a push by
// what only a call gives, `inputs.of`, never the event's name.
test("each job that restores the test map by its run's key needs the job that gives it", () => {
  type Step = { uses?: string; with?: Record<string, string> };
  type Job = { uses?: string; needs?: string | string[]; steps?: Step[]; with?: Record<string, string> };
  const workflows = readdirSync(join(root, ".github/workflows")).filter((file) => file.endsWith(".yml"));
  const downloads: string[] = [];
  for (const file of workflows) {
    const { jobs: all } = Bun.YAML.parse(readFileSync(join(root, ".github/workflows", file), "utf8")) as { jobs: Record<string, Job> };
    for (const [id, job] of Object.entries(all)) {
      expect((job.steps ?? []).some((step) => step.uses?.startsWith("actions/download-artifact") && step.with?.name === "test-map"), `${file}'s ${id} downloads the test-map artifact, which no run uploads: restore it from the cache by \`needs.test-map.outputs.key\``).toBe(false);
      if (!(job.steps ?? []).some((step) => step.uses?.startsWith("actions/cache/restore") && String(step.with?.key).includes("needs.test-map.outputs.key"))) continue;
      downloads.push(`${file} ${id}`);
      const needs = [job.needs ?? []].flat();
      expect(needs.some((need) => all[need]?.uses === "./.github/workflows/test-map.yml" && all[need]?.with?.of !== undefined), `${file}'s ${id} restores the test map by its key: it needs a job that calls test-map.yml with \`of\``).toBe(true);
    }
  }
  expect(downloads.length).toBeGreaterThan(0);
  const made = readFileSync(join(root, ".github/workflows/test-map.yml"), "utf8");
  expect(made).not.toContain("github.event_name");
  // A push's map, by its commit, which `bun run push` fetches; a call's
  // none, its store in the cache.
  expect(made.match(/name: test-map-of-\$\{\{ env\.SHA \}\}/g)?.length).toBe(2);
  // Each upload, and the pack before it (#2385).
  expect(made.match(/if: [^\n]*!inputs\.of/g)?.length).toBe(4);
  expect(made).toContain("value: ${{ jobs.test-map.outputs.key }}");
});

// CI's check of main selects by the full rule, the whole diff from its
// map's commit, others' `all` inherited, so what a push's own rule leaves,
// an item of another's read by a function neither changed, is run there;
// `bun run push` and check:fast by the push's (ADR 1101, amended).
test("CI's plan selects by the full rule, the push by its own", () => {
  const read = (file: string) => readFileSync(join(root, file), "utf8");
  expect(read("scripts/ci.ts")).toContain("selection = selectChange(since);");
  expect(read("scripts/ci.ts")).not.toContain('"push")');
  expect(read("scripts/checkFast.ts")).toContain('selectChange("origin/main", sanity, "push")');
  expect(read("scripts/push.ts")).toContain('["bun", "scripts/select.ts", "--push", ...asked]');
});

test("the mutation shards' arguments, as check.yml writes them, are ones the runner takes, a retry or none", () => {
  // `--also={1}` is `--also=` where no mutation is retried: #2001's refusal
  // of an empty value made every shard red (c502b22f6).
  const yml = readFileSync(join(import.meta.dir, "../.github/workflows/check.yml"), "utf8");
  const form = /format\('(--changed=\{0\} --also=\{1\} --base=\{0\})', needs\.plan\.outputs\.since, needs\.plan\.outputs\.retry\)/.exec(yml)?.[1];
  expect(form).toBeDefined();
  for (const retry of ["", "a-mutation,another"]) {
    const args = form!.replaceAll("{0}", "0123456789abcdef").replaceAll("{1}", retry).split(" ");
    expect(refusedFlags(args)).toEqual([]);
  }
});

test("each job that runs a slice uploads its JUnit reports, and attribution reads them: a red forms or nets job names a commit (#2003)", () => {
  type Step = { name?: string; run?: string; uses?: string; if?: string; with?: Record<string, string> };
  const check = Bun.YAML.parse(readFileSync(join(root, ".github/workflows/check.yml"), "utf8")) as { jobs: Record<string, { needs?: string[]; if?: string; steps?: Step[] }> };
  const slicing = Object.entries(check.jobs).filter(([, job]) => (job.steps ?? []).some((step) => /iteratorSlice\.ts run (?!metamorphic)/.test(step.run ?? "")));
  expect(slicing.map(([name]) => name).sort()).toEqual(["iterator-forms", "iterator-rows", ...NET_JOBS].sort());
  const plan = check.jobs["attribute-plan"]!;
  for (const [name, job] of slicing) {
    const upload = job.steps!.find((step) => step.uses?.startsWith("actions/upload-artifact") && step.with?.name?.startsWith("junit-"));
    expect([name, upload?.if, upload?.with?.path]).toEqual([name, "always()", relative(root, dirname(sliceReports))]);
    expect([name, plan.needs?.includes(name), plan.if?.includes(`needs.${name}.result == 'failure'`)]).toEqual([name, true, true]);
  }
  // And each net no rerun can say is a step of `nets` by that name.
  const nets = check.jobs.nets!.steps!.map((step) => step.name);
  for (const step of Object.keys(unrerunnable)) expect([step, nets.includes(step)]).toEqual([step, true]);
});

test("a push's React gate runs again past a rebase only where the commits it brought touch what compiles the port", () => {
  // Its own change touches the compiler; a round before passed.
  expect(reactGate(["crates/rust-js-lower/src/lib.rs"], undefined)).toBe("run");
  expect(reactGate(["crates/rust-js-lower/src/lib.rs"], ["docs/guide/README.md", "test/corpus/x.rs"])).toBe("reused");
  expect(reactGate(["crates/rust-js-lower/src/lib.rs"], ["crates/rust-js-program/src/js.rs"])).toBe("run");
  // A change that doesn't touch the port runs none.
  expect(reactGate(["docs/guide/README.md"], ["crates/rust-js-program/src/js.rs"])).toBe("none");
});


// A slice's part writes its JUnit report where its job uploads it, and
// beside it what ran it, its run's name, file and settings: a failed
// check's `attribute` reads that to run each failing test again (#2003).
test("a slice's report says what ran it, beside where its JUnit is written", () => {
  const into = mkdtempSync(join(tmpdir(), "slice-report-"));
  try {
    const ran = { of: "matrix", name: "the matrix's slice", file: "test/iterator-matrix.test.ts", env: { ITERATOR_TABLE: "matrix" } };
    const report = sliceReport(ran, 2, 4, into);
    expect(relative(into, report)).toBe("matrix-the-matrix-s-slice-2-of-4.xml");
    expect(JSON.parse(readFileSync(`${report}.json`, "utf8"))).toEqual(ran);
  } finally {
    rmSync(into, { recursive: true, force: true });
  }
});


// `gh api --paginate` runs its `--jq` on each page: one that makes an
// array makes one a page, two arrays where there are two pages, which
// isn't one JSON value. The plan read the mutations' kept times so, and
// past a hundred artifacts read none, every check's mutations dealt in turn
// (check 44072223912's shard 16, 18 minutes against the others' 8). Each
// value a line is read whatever the pages.
test("what gh prints a value a line, over its pages, is read whole", () => {
  const pages = ['{"name":"mutation-timings-1","run":1}', '{"name":"mutation-timings-1","run":2}', "", '{"name":"mutation-timings-1","run":3}', ""].join("\n");
  expect(jsonLines<{ run: number }>(pages).map((a) => a.run)).toEqual([1, 2, 3]);
  const ci = readFileSync(join(root, "scripts/ci.ts"), "utf8");
  expect(ci).not.toMatch(/"--paginate",[^\n]*"--jq", "\[/);
});

// A download GitHub's storage turns away, `503 Egress is over the account
// limit`, is asked again into an empty directory: into what it left half
// extracted, the retry failed `openat timings.json: file exists`, and the
// mutation shard with it (main's check of 6d9fcfb45). One that matches no
// artifact is asked once.
test("an artifact's download is asked again into an empty directory", () => {
  const dir = mkdtempSync(join(tmpdir(), "downloaded-"));
  const seen: string[][] = [];
  const flaky = (args: string[]) => {
    const into = args.find((arg) => arg.startsWith("--dir="))!.slice("--dir=".length);
    seen.push(readdirSync(into));
    writeFileSync(join(into, "timings.json"), "{");
    return seen.length < 3 ? { ok: false, said: "HTTP 503: 503 Egress is over the account limit." } : { ok: true, said: "" };
  };
  expect(downloaded(1, ["--name=x"], dir, flaky, () => {})).toBe(true);
  expect(seen).toEqual([[], [], []]);
  let asked = 0;
  expect(downloaded(1, ["--name=x"], dir, () => (asked++, { ok: false, said: "no valid artifacts found to download" }), () => {})).toBe(false);
  expect(asked).toBe(1);
  // Given its time, one that took it is asked once, as again would be as
  // slow (#2385).
  let timed = 0;
  const slow = (_: string[], seconds?: number) => (timed++, { ok: false, said: `no answer in ${seconds} s`, timedOut: true });
  expect(downloaded(1, ["--name=x"], dir, slow, () => {}, 5)).toBe(false);
  expect(timed).toBe(1);
  // Every script fetches a run's artifacts so.
  const scripts = readdirSync(join(root, "scripts")).filter((name) => name.endsWith(".ts") && name !== "artifacts.ts");
  expect(scripts.filter((name) => /"run", "download"/.test(readFileSync(join(root, "scripts", name), "utf8")))).toEqual([]);
  rmSync(dir, { recursive: true, force: true });
});

// A test file that compiles against the js, webapi or react crate's
// metadata in `target` builds it first, `buildReact`, as the run's first
// file to ask for it may be its own: test/on-load-names.test.ts didn't,
// and passed where `target` held them already, but failed on CI, "extern
// location for js does not exist" (#2070's file, main red).
test("each test file that compiles against the bindings' crates builds them first", () => {
  const dir = join(root, "test");
  const crates = /join\(target, "lib(js|webapi|react)\.rmeta"\)|import \{[^}]*\bjs\b[^}]*\} from "\.\/crates"/;
  const unbuilt = readdirSync(dir)
    .filter((file) => file.endsWith(".ts") && file !== "support.ts" && file !== "crates.ts")
    .filter((file) => {
      const text = readFileSync(join(dir, file), "utf8");
      return crates.test(text) && !/\bbuildReact\b/.test(text);
    });
  expect(unbuilt).toEqual([]);
});

// A React port crate that doesn't compile writes no JS, so the port's diff is
// empty: regen exits with its own code, and the push says so, never "it
// differs", which a reader of an empty diff took for no change (#2106's
// measurement).
test("a React port that doesn't compile is told from one that differs", () => {
  expect(PORT_DOES_NOT_COMPILE).not.toBe(1);
  expect(portRefusal(PORT_DOES_NOT_COMPILE, "log")).toContain("doesn't compile");
  expect(portRefusal(1, "log")).toContain("differs at its pin");
  const regen = readFileSync(join(root, "scripts/react-in-rust.ts"), "utf8");
  expect(regen).toContain("process.exit(PORT_DOES_NOT_COMPILE);");
  expect(regen.indexOf("process.exit(PORT_DOES_NOT_COMPILE);", regen.indexOf("if (failed.length) {"))).toBeLessThan(regen.indexOf("if (changed.length) process.exit(1);"));
});

// A step that captures one of our scripts' stdout, `x=$(bun scripts/a.ts
// mode)`, reads it as one value: a second line, a status said on stdout,
// corrupts it, as #2346's wasm lines did `needed=` in `$GITHUB_OUTPUT`
// (f10b9502f). Each such capture in .github, run as written with what CI
// fills in (`${{ .. }}`, `$VAR`) left off, prints one line.
test("each capture of a script's stdout in .github is one line", () => {
  const ymls = [...new Bun.Glob(".github/**/*.{yml,yaml}").scanSync({ cwd: root })];
  const captured = ymls.flatMap((file) =>
    [...readFileSync(join(root, file), "utf8").matchAll(/\$\((bun scripts\/[^)]*)\)/g)].map((m) => ({
      file,
      args: m[1]!.replace(/\$\{\{[^}]*\}\}/g, "\0").split(/\s+/).filter((word) => word !== "" && !word.includes("\0") && !word.startsWith("$")),
    })),
  );
  expect(new Set(captured.map((c) => c.args[1])).size).toBeGreaterThanOrEqual(2);
  for (const { file, args } of new Map(captured.map((c) => [c.args.join(" "), c])).values()) {
    const asked = runSync([process.execPath, ...args.slice(1)], root, 120_000);
    expect([file, args.join(" "), asked.code, asked.stdout.split("\n").length]).toEqual([file, args.join(" "), 0, 2]);
  }
});
