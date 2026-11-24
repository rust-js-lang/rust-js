// The test selector (scripts/select.ts, ADR 1101, amended): what each kind
// of change runs of the push suite's files and the corpus's cases, by the
// test map where it's the compiler's code, by what imports or reads it where
// it's a script's, a helper's or a doc's, everything where it can't say;
// and what `check:fast`, `bun run push` and CI's suite run of it.

import { expect, test } from "bun:test";
import { readdirSync, readFileSync, writeFileSync } from "node:fs";
import { join, relative } from "node:path";

import { jobSeconds, jobs, ranKey, totalKey } from "../scripts/checkFast";
import { onlyPushedFiles, phases as pushPhases } from "../scripts/push";
import { ownsTimes, slowest, timesLine, whoRuns } from "../scripts/devTimes";
import { failedIn, misses } from "../scripts/selector-audit";
import { baselines, budgeted, capOf, type Costs, hunksAtMap, interaction, mapLine, movedUnits, osBranches, outcome, pushSelection, reachOf, type Reach, sanityRecord, select, type Selection, suiteFiles } from "../scripts/select";
import { pathFunctions, readsOf, readsSources, type TestMap } from "../scripts/testMap";
import { fixture, root } from "./support";
import { compilerRustFiles } from "../scripts/compiler-crates";

const bits = (...functions: number[]) => {
  const bytes = new Uint8Array(1);
  for (const f of functions) bytes[f >> 3] |= 1 << (f & 7);
  return Buffer.from(bytes).toString("base64");
};

// Two functions of a.rs, lines 10-20 and 30-40; one of runtime.rs, which
// reads helper.js at its line 5. Five test files: one and two compile, the
// architecture's reads the sources (null), tool's ran no compiler, and
// std-package's is the runtime's package test.
const map: TestMap = {
  version: 4,
  commit: "0",
  toolchain: "0",
  made: 0,
  functions: [
    { file: "src/a.rs", spans: [[10, 20]] },
    { file: "src/a.rs", spans: [[30, 40]] },
    { file: "crates/rust-js-program/src/runtime.rs", spans: [[1, 9]] },
  ],
  units: {
    "test/one.test.ts": bits(0),
    "test/two.test.ts": bits(1, 2),
    "test/architecture.test.ts": null,
    "test/tool.test.ts": null,
    "test/std-package.test.ts": bits(2),
    "test/corpus.test.ts": bits(0),
    "test/corpus/x.rs": bits(1),
    "test/corpus/y.rs": bits(2),
    "test/iterator-forms.test.ts": bits(1),
    "test/iterator-forms.test.ts test/corpus/x.rs": bits(0),
    "test/iterator-forms.test.ts test/corpus/y.rs": bits(2),
  },
  includes: { "src/helper.txt": [{ file: "crates/rust-js-program/src/runtime.rs", line: 5 }] },
  corpusReaders: ["test/two.test.ts"],
  noCompiler: ["test/tool.test.ts"],
};
const files = ["test/architecture.test.ts", "test/one.test.ts", "test/std-package.test.ts", "test/tool.test.ts", "test/two.test.ts"];
const cases = ["x.rs", "y.rs"];
// What each imports and reads: one imports a helper, which reads a doc;
// tool's reads a script's text; two globs the corpus's JS.
const reach: Reach = new Map([
  ["test/one.test.ts", { modules: new Set(["test/helper.ts", "test/setup.ts"]), reads: new Set(["docs/notes.md"]) }],
  ["test/two.test.ts", { modules: new Set(["test/setup.ts"]), reads: new Set(["test/corpus/*.js"]) }],
  ["test/architecture.test.ts", { modules: new Set(["test/setup.ts"]), reads: new Set(["src"]) }],
  ["test/tool.test.ts", { modules: new Set(["test/setup.ts", "scripts/tool.ts"]), reads: new Set(["scripts/other.ts"]) }],
  ["test/std-package.test.ts", { modules: new Set(["test/setup.ts"]), reads: new Set(["std"]) }],
]);
const context = { files, cases, map, reach };
const of = (...changed: string[]) => select(changed.map((file) => ({ file, hunks: [[12, 1]] as [number, number][] })), context);
const picked = (selection: Selection) => ({ files: selection.files, cases: selection.cases, forms: selection.forms });

test("a change to a function's code runs the tests that ran it, and those the map can't say of", () => {
  expect(picked(of("src/a.rs"))).toEqual({
    files: ["test/architecture.test.ts", "test/one.test.ts", "test/tool.test.ts", "test/corpus.test.ts"],
    cases: [],
    forms: ["x.rs"],
  });
  // A file `include_str!` reads, by the line that reads it.
  expect(picked(of("src/helper.txt"))).toEqual({
    files: ["test/architecture.test.ts", "test/std-package.test.ts", "test/tool.test.ts", "test/two.test.ts"],
    cases: ["y.rs"],
    forms: ["y.rs"],
  });
});

test("a change the map can't say of runs everything, and says whose change it was", () => {
  const outside = (own: boolean) => select([{ file: "src/a.rs", hunks: [[25, 1]] }], { ...context, own: () => own });
  expect(outside(true)).toMatchObject({ all: true, allOwn: true, files, cases, forms: cases });
  expect(outside(true).why).toContain("the change's own");
  expect(outside(false)).toMatchObject({ all: true, allOwn: false });
  expect(outside(false).why).toContain("others' since the map's commit");
  // A new file of the compiler's, and no map at all.
  expect(select([{ file: "src/b.rs", hunks: "new" }], context).all).toBe(true);
  expect(select([{ file: "src/a.rs", hunks: [[12, 1]] }], { ...context, map: "no test map" }).all).toBe(true);
  // A file of another of the compiler's crates (ADR 1181), of a map that
  // counted none of its functions, as one made before it counted the
  // crates; and of one that did, by its functions.
  const program = "crates/rust-js-program/src/js.rs";
  const binaryAlone = { ...map, functions: map.functions.filter((f) => f.file.startsWith("src/")) };
  const uncounted = select([{ file: program, hunks: [[12, 1]] }], { ...context, map: binaryAlone });
  expect(uncounted.all).toBe(true);
  expect(uncounted.why).toContain("counted no function of");
  // a.rs's second function, lines 30-40, as the crate's, lines 10-20.
  const counting = { ...map, functions: [map.functions[0]!, { file: program, spans: [[10, 20]] as [number, number][] }, map.functions[2]!] };
  expect(picked(select([{ file: program, hunks: [[12, 1]] }], { ...context, map: counting }))).toEqual(picked(select([{ file: "src/a.rs", hunks: [[32, 1]] }], context)));
});

// Four agents push to main every 15–30 min: the map's diff holds others'
// commits since its commit. A test whose functions one touched runs; one
// none touched doesn't; only a change outside a function's code runs all.
test("others' changes since the map's commit run the tests whose functions they touched, not everything", () => {
  const selection = select(
    [
      { file: "src/a.rs", hunks: [[12, 1]] },
      { file: "src/a.rs", hunks: [[35, 1]] },
    ],
    { ...context, own: () => false },
  );
  expect(selection.all).toBe(false);
  expect(selection.files).toEqual(["test/architecture.test.ts", "test/one.test.ts", "test/tool.test.ts", "test/two.test.ts", "test/corpus.test.ts"]);
  expect(selection.cases).toEqual(["x.rs"]);
});

// A check is a batch: where pending checks are superseded, the next one's
// selection, of the map's diff to its tree, holds every commit's since the
// map's, so a skipped commit's tests run in it. Each commit's selection is
// a part of the batch's, whatever the batch (ADR 1101, amended).
test("a batch of commits selects every test any one of them selects", () => {
  const commits = [["src/a.rs"], ["test/helper.ts"], ["test/corpus/y.rs"], ["docs/notes.md"], ["src/helper.txt"]];
  const batch = of(...commits.flat());
  for (const changed of commits) {
    const alone = of(...changed);
    expect([changed, alone.files.filter((file) => !batch.files.includes(file)), alone.cases.filter((name) => !batch.cases.includes(name))]).toEqual([changed, [], []]);
  }
});

test("a test file runs itself; the corpus's runs every case; a case runs itself and the corpus's readers", () => {
  expect(picked(of("test/one.test.ts"))).toEqual({ files: ["test/one.test.ts"], cases: [], forms: [] });
  expect(picked(of("test/corpus.test.ts"))).toEqual({ files: ["test/corpus.test.ts"], cases, forms: [] });
  expect(picked(of("test/iterator-forms.test.ts"))).toEqual({ files: [], cases: [], forms: cases });
  expect(picked(of("test/corpus/y.rs"))).toEqual({ files: ["test/two.test.ts"], cases: ["y.rs"], forms: ["y.rs"] });
  // Its JS too, which two globs.
  expect(picked(of("test/corpus/x.js"))).toEqual({ files: ["test/two.test.ts"], cases: ["x.rs"], forms: ["x.rs"] });
});

test("std's package or the runtime runs its package tests, every case, and every file that compiles", () => {
  for (const file of ["std/lib.rs", "crates/rust-js-program/src/runtime/pad.js"]) {
    expect(picked(of(file))).toEqual({
      files: ["test/architecture.test.ts", "test/one.test.ts", "test/std-package.test.ts", "test/two.test.ts"],
      cases,
      forms: cases,
    });
  }
  expect(select([{ file: "std/lib.rs", hunks: [[1, 1]] }], { ...context, map: "no test map" }).all).toBe(true);
});

test("a script's, a helper's or a doc's change runs what imports or reads it, and nothing where nothing does", () => {
  expect(picked(of("test/helper.ts")).files).toEqual(["test/one.test.ts"]);
  expect(picked(of("docs/notes.md")).files).toEqual(["test/one.test.ts"]);
  expect(picked(of("scripts/tool.ts", "scripts/other.ts")).files).toEqual(["test/tool.test.ts"]);
  // The preload, every test's.
  expect(picked(of("test/setup.ts")).files).toEqual(files);
  const none = of("docs/unread.md", "README.md");
  expect([none.all, none.files, none.cases, outcome(none)]).toEqual([false, [], [], "none"]);
});

// A workflow changes how CI runs, not what a test does here, and no local
// check runs a workflow: CI's check of the push does. So a change of one,
// the change's own or another agent's, runs the tests that read it, as
// ci.test.ts reads check.yml (#2480).
test("a workflow's change, the change's own or another's, runs only what reads it", () => {
  for (const file of [".github/workflows/check.yml", ".github/actions/build-rust-js/action.yml"]) {
    for (const own of [true, false]) {
      expect([file, own, select([{ file, hunks: [[1, 1]] }], { ...context, own: () => own })]).toMatchObject([file, own, { all: false, files: [] }]);
      const read: Reach = new Map([...reach, ["test/tool.test.ts", { modules: new Set(["test/setup.ts"]), reads: new Set([".github/workflows", ".github/actions"]) }]]);
      expect([file, own, select([{ file, hunks: [[1, 1]] }], { ...context, reach: read, own: () => own }).files]).toEqual([file, own, ["test/tool.test.ts"]]);
    }
  }
});

// The selector's and the map's own code changes which tests run, not what
// one does: another agent's change of it since the map's commit runs what
// reads it, as a workflow's does; the selector's frequent pushes, a map
// behind, ran every agent's check whole.
test("the selector or the map of the change's own runs everything; another's since the map's commit, only what reads it", () => {
  for (const file of ["scripts/select.ts", "scripts/testMap.ts", "scripts/testMapCompiler.ts"]) {
    const changed = (own: boolean) => select([{ file, hunks: [[1, 1]] }], { ...context, own: () => own });
    expect([file, changed(true)]).toMatchObject([file, { all: true, allOwn: true }]);
    expect([file, changed(false)]).toMatchObject([file, { all: false, files: [] }]);
    const read: Reach = new Map([...reach, ["test/two.test.ts", { modules: new Set([file]), reads: new Set() }]]);
    expect(select([{ file, hunks: [[1, 1]] }], { ...context, reach: read, own: () => false }).files).toEqual(["test/two.test.ts"]);
  }
});

test("the packages' manifests, the toolchain, the selector and the map run everything; so does a package's folder", () => {
  for (const file of ["package.json", "webapi/package.json", "bun.lock", "Cargo.lock", "rust-toolchain.toml", "bunfig.toml", "scripts/select.ts", "scripts/testMap.ts", "webapi/src/lib.rs", "tooling/build.js", "vendor/x/lib.rs"]) {
    expect([file, of(file).all]).toEqual([file, true]);
  }
});

test("a baseline a test reads by a path its source doesn't spell picks its owners", () => {
  const owners = (file: string) => baselines.filter(({ path }) => new Bun.Glob(path).match(file)).flatMap(({ owners }) => owners);
  expect(owners("test/snapshots/std/iterators.js")).toEqual(["test/snapshots.test.ts", "test/snapshots-serde.test.ts", "test/readability.test.ts"]);
  for (const { owners } of baselines) for (const owner of owners) expect(suiteFiles()).toContain(owner);
});

// A sanity pass past a rebase runs the interaction: what both the change
// and the commits the rebase brought can affect. lisa agent's runtime change,
// some 360 of 361 files and every case, ran all of it again after each
// rebase, some 1,120 s on the M1, and lost to a moving main three times
// until her push's bound stopped it (#1976).
test("a sanity pass runs what both the change and the rebase's commits can affect", () => {
  const some = (files: string[], cases: string[]): Selection => ({ all: false, files, cases, forms: cases, why: "" });
  const many = Array.from({ length: 360 }, (_, i) => `test/f${i}.test.ts`);
  const every = Array.from({ length: 2600 }, (_, i) => `c${i}.rs`);
  // The runtime's change against two small commits: theirs, where it meets.
  const small = interaction(some(many, every), some(["test/f3.test.ts", "test/other.test.ts"], ["c1.rs", "x.rs"]));
  expect([small.files, small.cases, small.capped]).toEqual([["test/f3.test.ts"], ["c1.rs"], undefined]);
  expect(small.why).toBe("the interaction: the change's 360 files, 2600 cases and the rebase's commits' 2 files, 2 cases");
  // A side that selects everything gives way to the other.
  const all: Selection = { all: true, files: [], cases: [], forms: [], why: "a workflow" };
  expect(interaction(all, some(["test/a.test.ts"], ["c1.rs"]))).toMatchObject({ all: false, files: ["test/a.test.ts"], cases: ["c1.rs"] });
  expect(interaction(some(["test/a.test.ts"], []), all)).toMatchObject({ all: false, files: ["test/a.test.ts"], cases: [] });
  expect(interaction(all, all).all).toBe(true);
  // The change's own functions go with it, which its pass ranks by.
  expect(interaction({ ...some(["test/a.test.ts"], []), functions: [3] }, all).functions).toEqual([3]);
});

// A sanity pass runs what fits its budget, what reaches most of the
// change's own functions first: its rounds took 23–248 s on the Mac mini,
// the interaction's every case, or both sides' everything capped to the
// change's whole selection again (2026-11-18).
test("a sanity pass runs what reaches most of the change first, within its budget", () => {
  const selection: Selection = { all: false, files: ["test/slow.test.ts", "test/a.test.ts", "test/b.test.ts", "test/corpus.test.ts"], cases: ["x.rs", "y.rs", "z.rs"], forms: ["y.rs"], why: "", functions: [0, 1] };
  const reach = (unit: string) => ({ "test/b.test.ts": 2, "test/corpus/z.rs": 2, "test/corpus/y.rs": 1 } as Record<string, number>)[unit] ?? 0;
  const costs: Costs = { file: (file) => (file === "test/slow.test.ts" ? 100 : 20), case: 10 };
  // 2 cores of 30 s: z (10) then b (20), the cheaper first of those that
  // reach both functions, y (10), then x (10); a has no room left, and
  // slow takes longer alone than the pass.
  const ran = budgeted(selection, reach, costs, 2, 30);
  expect([ran.files, ran.cases, ran.forms]).toEqual([["test/b.test.ts", "test/corpus.test.ts"], ["x.rs", "y.rs", "z.rs"], ["y.rs"]]);
  expect(ran.skipped).toEqual({ files: ["test/slow.test.ts", "test/a.test.ts"], cases: [] });
  expect(outcome(ran)).toBe("capped 1 of 3 files and 3 of 3 cases in 30 s, by reach of the change's 2 functions");
  // What fits whole is the selection as it was; a cap's said first.
  expect(budgeted({ ...selection, files: ["test/a.test.ts"], cases: [] }, reach, costs, 12, 45)).toEqual({ ...selection, files: ["test/a.test.ts"], cases: [] });
  expect(budgeted({ ...selection, capped: "corpus" }, reach, costs, 2, 30).capped).toStartWith("corpus, 1 of 3 files");
  // What it ran and left kept, for a red `main`'s reader.
  expect(sanityRecord(ran, "h", "c")).toMatchObject({ head: "h", checked: "c", ran: { files: ran.files, cases: ran.cases }, skipped: { files: ["test/slow.test.ts", "test/a.test.ts"], cases: [] } });
  // No case left: no corpus file to run them.
  expect(budgeted({ ...selection, cases: ["x.rs"] }, () => 0, { file: () => 30, case: 31 }, 1, 30).files).toEqual(["test/slow.test.ts"]);
});

// A sanity pass the selector can't say of is capped: the change's own
// test files and cases; and of a change to the compiler's sources never
// none, as two changes to one file are where they most meet: lily agent's
// tidy refactor, rebased onto others' edits to the same tidy.rs, was
// capped to its own test files, none, and ran nothing.
test("a sanity pass's cap is never empty for the compiler's code: the map's tests of its files, else the corpus", () => {
  expect(capOf(["test/one.test.ts", "docs/notes.md"], files, cases, map)).toMatchObject({ files: ["test/one.test.ts"], cases: [], capped: "some (1 files, 0 cases)" });
  // Its files' functions, as the map recorded them, approximate.
  const fromMap = capOf(["src/a.rs"], files, cases, map);
  expect([fromMap.files, fromMap.cases, fromMap.capped]).toEqual([["test/one.test.ts", "test/two.test.ts"], ["x.rs"], "some (from the map at 0: 2 files, 1 cases)"]);
  // A file the map has no function of, one others changed too: the corpus.
  for (const given of [map, undefined]) {
    const corpus = capOf(["crates/rust-js-mir/src/tidy.rs"], files, cases, given);
    expect([corpus.files, corpus.cases, corpus.capped]).toEqual([["test/corpus.test.ts"], cases, "corpus"]);
    expect(outcome(corpus)).toBe("capped corpus");
  }
});

test("a selection says itself in a line: all and why, some and how many, or none", () => {
  expect(outcome({ all: true, files, cases, forms: cases, why: "src/a.rs's change at line 25 is outside any function's counted code (the change's own)" })).toBe(
    "all (src/a.rs's change at line 25 is outside any function's counted code (the change's own))",
  );
  expect(outcome(of("src/a.rs"))).toBe("some (3 files, 0 cases)");
  // And the map it's by, and how far behind HEAD.
  expect(outcome({ ...of("src/a.rs"), map: "0123456789abcdef", behind: 6 })).toBe("some (3 files, 0 cases), map 012345678 6 behind");
  expect(outcome(of("test/corpus/y.rs"))).toBe("some (1 file, 1 case)");
});

// What the real suite imports and reads: a script a test reads as text, a
// glob of the corpus's JS, readability's baseline, a helper's own reads.
test("the suite's tests are found by what they import and read", () => {
  const suite = suiteFiles();
  const real = reachOf(suite);
  const readers = (file: string) => select([{ file, hunks: [[1, 1]] }], { files: suite, cases: [], map: "no map", reach: real }).files;
  expect(readers("scripts/crate-corpus.ts")).toContain("test/ci.test.ts");
  expect(readers("docs/readability.txt")).toContain("test/readability.test.ts");
  expect(readers("scripts/push.ts")).toContain("test/select.test.ts");
  // Through a helper: crates.ts reads its pairs.
  expect(readers("test/crates/pairs/x/src/lib.rs")).toContain("test/crates.test.ts");
  // A dynamic import, `await import("./mutations")`.
  expect(real.get("test/ci.test.ts")!.modules.has("scripts/mutations.ts")).toBe(true);
  // Bun's preload, every test's.
  expect(readers("test/setup.ts")).toEqual(suite);
  // A test file isn't its own reader: it runs as itself.
  expect(readers("test/ci.test.ts").filter((file) => file === "test/ci.test.ts")).toEqual(["test/ci.test.ts"]);
});

// A known list's edit runs what reads that list: its net, and push.test's
// check of every list's form. test/support.ts's `knownDir`, `join(root,
// "test", "known", list)`, is a path function, read where it's called with
// a list, `knownText("metamorphic")`, not by each module that imports
// support.ts, which ran all 402 test files on any list's edit (#2284).
test("a known list's edit runs the tests that read that list, not every importer of its reader", () => {
  const suite = suiteFiles();
  const real = reachOf(suite);
  const readers = (file: string) => select([{ file, hunks: [[1, 1]] }], { files: suite, cases: [], map: "no map", reach: real }).files;
  const functions = pathFunctions();
  expect(["knownDir", "knownText", "writeKnown"].map((name) => functions.get(name))).toEqual(["test/known", "test/known", "test/known"]);
  const metamorphic = readers("test/known/metamorphic/2059.txt");
  for (const reader of ["test/metamorphic.test.ts", "test/docs-differences.test.ts", "test/push.test.ts"]) expect(metamorphic).toContain(reader);
  for (const other of ["test/neighbours.test.ts", "test/corpus.test.ts", "test/support.ts"]) expect(metamorphic).not.toContain(other);
  expect(readers("test/known/neighbours/1038.txt")).toContain("test/neighbours.test.ts");
  // The matrix's own test isn't the push suite's; what imports its rows is.
  expect(readers("test/known/iterator-matrix/README.txt")).toContain("test/iterator-rows.test.ts");
  expect(metamorphic.length).toBeLessThan(suite.length / 4);
});

// A path function called with what isn't a literal, `knownText(list)` of a
// helper or a loop's, may be any of its parts: it reads its whole folder, so
// any list's edit selects it, as under-selecting turns main red after the
// push (#2284, the lead's soundness ask).
test("a path function called with what isn't a literal reads its whole folder, so any list's edit selects it", () => {
  const dir = fixture("path-function-variable");
  const helper = join(dir, "helper.ts");
  writeFileSync(helper, `import { knownText, type KnownList } from ${JSON.stringify(join(root, "test", "support"))};
export const each = (list: KnownList) => knownText(list);
export const one = () => knownText("neighbours");
`);
  const reads = readsOf(relative(root, helper), false);
  expect([...reads].sort()).toEqual(["test/known", "test/known/neighbours"]);
  const reach: Reach = new Map([["test/helper.test.ts", { modules: new Set<string>(), reads }]]);
  for (const list of ["metamorphic", "neighbours", "iterator-matrix"]) {
    const picked = select([{ file: `test/known/${list}/1022.txt`, hunks: [[1, 1]] }], { files: ["test/helper.test.ts"], cases: [], map: "no map", reach });
    expect(picked.files).toEqual(["test/helper.test.ts"]);
  }
});

// A test that reads the compiler's sources by their folders, as each
// architecture test does, fails on a change to any of them: the map records
// each `null`, run on every change to the compiler (7121088df, adfe4aeb0).
test("every architecture test reads the compiler's sources, so runs on every change to them", () => {
  const architecture = suiteFiles().filter((file) => file.startsWith("test/architecture"));
  expect(architecture).toContain("test/architecture-generic-facts.test.ts");
  expect(architecture.filter((file) => !readsSources(file))).toEqual([]);
});

test("check:fast runs what the selection selected: its files, its cases in parts, and lint; everything where it's none", () => {
  const plain = { all: false, changed: ["crates/rust-js-lower/src/drops.rs"], branch: [], added: [], extra: [], cores: 12 };
  const given = { caseList: "/cases.txt", formsList: "/forms.txt" };
  const selected = jobs({ ...plain, picked: { ...given, files: new Set(["test/lowering-cells.test.ts", "test/jsx.test.ts"]), cases: ["a.rs", "b.rs"], forms: [] } }).jobs;
  const testJobs = (list: typeof selected) => list.filter((job) => job.cmd[0] === "bun" && job.cmd[1] === "test").map((job) => job.name);
  expect(testJobs(selected).sort()).toEqual(["corpus 1/1", "test/jsx.test.ts", "test/lowering-cells.test.ts"]);
  expect(selected.find((job) => job.name === "corpus 1/1")?.env.RUST_JS_CASES).toBe("1/1:/cases.txt");
  // What the selection sizes is timed against runs of the same selection,
  // the corpus's parts and the files alike; lint against all of its runs.
  const key = ranKey({ files: new Set(["test/lowering-cells.test.ts", "test/jsx.test.ts"]), cases: ["a.rs", "b.rs"], forms: [] });
  expect(["corpus 1/1", "test/jsx.test.ts", "fmt"].map((name) => selected.find((job) => job.name === name)?.ran)).toEqual([key, key, undefined]);
  for (const name of ["fmt", "typecheck", "clippy", "cargo test"]) expect(selected.map((job) => job.name)).toContain(name);
  // A sanity pass's cap: its own test files and cases alone, no lint, nets
  // or builds.
  const capped = jobs({ ...plain, capped: true, picked: { ...given, files: new Set(["test/jsx.test.ts"]), cases: ["a.rs"], forms: [] } }).jobs.map((job) => job.name);
  expect(capped.sort()).toEqual(["corpus 1/1", "test/jsx.test.ts"]);
  const cap = { files: new Set(["test/jsx.test.ts"]), cases: ["a.rs"], forms: [] };
  expect(jobs({ ...plain, capped: true, picked: { ...given, ...cap } }).jobs.map((job) => job.ran)).toEqual([ranKey(cap, true), ranKey(cap, true)]);
  // None selected: lint alone, and of a doc's change neither the compiler's
  // unit tests nor the Vite example's build.
  const docs = jobs({ ...plain, changed: ["docs/guide/README.md"], picked: { ...given, files: new Set(), cases: [], forms: [] } }).jobs.map((job) => job.name);
  expect(docs).toEqual(["fmt", "typecheck", "clippy"]);
  // No selection given: everything, as `--all`.
  const everything = jobs(plain).jobs;
  expect(everything.filter((job) => job.name.startsWith("corpus ")).length).toBe(12);
  expect(testJobs(everything)).toContain("test/snapshots.test.ts");
  expect([...new Set(everything.filter((job) => job.name.startsWith("corpus ") || job.name === "test/snapshots.test.ts").map((job) => job.ran))]).toEqual(["all"]);
  // A whole run is timed against runs of the same jobs, each running what
  // it ran: in any order, and apart from a run with a job more or a
  // selection's other cases.
  expect(totalKey([...everything].reverse())).toBe(totalKey(everything));
  expect(totalKey(everything.slice(1))).not.toBe(totalKey(everything));
  expect(totalKey(selected)).not.toBe(totalKey(everything));
});

test("bun run push says its selection on its line, and runs the check and the port only where they can differ", () => {
  expect(pushPhases).toEqual(["rebase", "gate", "fast", "build", "std", "mutants", "check", "sanity", "react", "files", "push"]);
  expect(onlyPushedFiles(["a.rs", "b.rs"], ["a.rs"])).toEqual(["b.rs"]);
  const line = timesLine({ at: new Date("2026-11-16T08:00:00Z"), machine: "m", agent: "a", kind: "push", sha: "abc", total: 61.4, phases: [["check", 300]], selection: "some (3 files, 12 cases)", load: [1, 2, 3], ok: true, note: "" });
  expect(line).toBe("2026-11-16T08:00:00Z | m | a | push | abc | total 61s | check 300s | select some (3 files, 12 cases) | load 1.00/2.00/3.00 | ok | ");
  // And the other runs that held the cores as it began and ended, which
  // an overlapped run's time says it shared (2026-11-21's corpus, 1,190 s
  // overlapped where 658 s quiet).
  const shared = timesLine({ at: new Date("2026-11-16T08:00:00Z"), machine: "m", agent: "a", kind: "corpus", sha: "abc", total: 1190, phases: [], selection: "2849 cases", load: [2, 2, 2], others: [1, 0], ok: true, note: "" });
  expect(shared).toContain("| load 2.00/2.00/2.00, other runs 1/0 | ok |");
  expect(JSON.parse(readFileSync(join(root, "package.json"), "utf8")).scripts.push).toBe("bun scripts/push.ts");
  for (const doc of ["DEVELOPMENT.md", "CONTRIBUTING.md"]) expect(readFileSync(join(root, doc), "utf8")).toContain("`bun run push`");
  const push = readFileSync(join(root, "scripts/push.ts"), "utf8");
  expect(push).not.toContain('"check:fast", "--all"');
  expect(push).toContain("touchesPort(changed)");
  // Past a rebase, the change's own sanity pass, as often as main moves.
  expect(push).toContain('const asked = name === "sanity" ? [`--sanity=${checked}`] : [];');
  expect(push).not.toMatch(/attempts|main moved \$\{/);
  // What it pushes is held to the commit it checked over, not to where a
  // fetch since moved `origin/main`, whose commits aren't the branch's.
  expect(push).toContain('lines(git("diff", "--name-only", base).out)');
  expect(push).toContain('`${base}..HEAD`');
  // Its line on every exit, a signal's too, and who ran it, each machine's.
  expect(push).toContain('for (const signal of ["SIGINT", "SIGTERM", "SIGHUP"] as const)');
  expect(whoRuns({ RUST_JS_MACHINE: "m1-lisa", RUST_JS_AGENT: "ioX" })).toEqual({ machine: "m1-lisa", agent: "ioX" });
  // Not under the host's name, apart from the machine's other lines: none
  // written, saying which isn't set (#2322).
  expect(whoRuns({})).toEqual({ unset: ["RUST_JS_MACHINE", "RUST_JS_AGENT"] });
  expect(whoRuns({ RUST_JS_MACHINE: "m6" })).toEqual({ unset: ["RUST_JS_AGENT"] });
  // Each run of the loop writes its own line through the one writer, but
  // one that's a step of another's, whose environment says so: a push's
  // check:fast, a retirement's port regen, check:fast's corpus parts.
  const env: Record<string, string | undefined> = {};
  expect(ownsTimes("push", env, () => 2)).toBe(true);
  expect(ownsTimes("check:fast", env, () => 5)).toBe(false);
  expect(env.RUST_JS_TIMED_BY).toBe("push");
  expect(env.RUST_JS_TIMED_OTHERS).toBe("2");
  expect(slowest([["a", 1], ["b", 9], ["c", 5]], 2)).toEqual([["b", 9], ["c", 5]]);
  // A job's seconds, its run's `Ran` holding its start and end in seconds
  // already: corpus 5/12's 190 s, not `0s` (#2322).
  expect(slowest(jobSeconds([{ job: { name: "corpus 5/12" }, start: 10, end: 200 }, { job: { name: "compat" }, start: 0, end: 297 }]))).toEqual([["compat", 297], ["corpus 5/12", 190]]);
  for (const file of ["scripts/push.ts", "scripts/checkFast.ts", "scripts/mutations.ts", "scripts/react-in-rust.ts", "test/corpus.test.ts"]) {
    expect(readFileSync(join(root, file), "utf8")).toContain("ownsTimes(");
  }
  // A map not fetched says why on the line, not only that there's none.
  expect(push).toContain('if (fetched.startsWith("no map fetched")) selection += ` [${fetched}]`;');
});

// CI's maps are recorded on Linux and read on macOS by `bun run push`: sound
// only where each test runs the same compiler code on both. The compiler's
// sources branch on no OS but WASI, which no test's compiler is; the
// harness's branches are each listed with why they change no compile.
test("a test runs the same compiler code on Linux and macOS, so one OS's map serves the other", () => {
  const branches = compilerRustFiles().flatMap((file) =>
    readFileSync(join(root, file), "utf8")
      .split("\n")
      .flatMap((line, i) => (/\bcfg!?\s*\(.*\b(target_os|target_family|target_vendor|target_env|unix|windows)\b|consts::OS\b/.test(line) && !/target_os = "wasi"/.test(line) && !/^\s*\/\//.test(line) ? [`${file}:${i + 1}: ${line.trim()}`] : [])),
  );
  expect(branches).toEqual([]);
  const harness = ["test", "scripts", "tooling"].flatMap((dir) =>
    readdirSync(join(root, dir))
      .filter((name) => /\.(ts|js)$/.test(name))
      .map((name) => `${dir}/${name}`)
      .filter((file) => /process\.platform|\bplatform\(\)|os\.platform|process\.arch\b/.test(readFileSync(join(root, file), "utf8"))),
  );
  expect(harness.sort()).toEqual(Object.keys(osBranches).sort());
});

// The nightly's audit: a test file the night failed that no push's check
// since the last green night selected is a miss, filed as a P1 `selector`.
test("a test the night failed that no push's selection ran is a selector's miss", () => {
  const xml = [
    '<testsuites><testsuite name="test/a.test.ts" file="test/a.test.ts" failures="1">',
    '<testcase name="x" file="test/a.test.ts" />',
    '<testcase name="y" file="test/a.test.ts"><failure type="AssertionError" /></testcase>',
    "</testsuite>",
    '<testsuite name="test/corpus.test.ts" file="test/corpus.test.ts" failures="1">',
    '<testcase name="test/corpus/two_fold.rs runs as native Rust" file="test/corpus.test.ts"><failure /></testcase>',
    '<testcase name="test/corpus/one.rs runs as native Rust" file="test/corpus.test.ts" />',
    "</testsuite>",
    '<testsuite name="test/b.test.ts" file="test/b.test.ts" failures="0"><testcase name="z" /></testsuite>',
    "</testsuites>",
  ].join("\n");
  const failed = failedIn(xml);
  expect(failed).toEqual([
    { file: "test/a.test.ts", cases: [] },
    { file: "test/corpus.test.ts", cases: ["two_fold.rs"] },
  ]);
  const check = (files: string[], cases: string[] = [], all = false) => ({ sha: "s", all, files, cases });
  expect(misses(failed, [check(["test/a.test.ts"], ["one.rs"])])).toEqual([{ file: "test/corpus.test.ts", cases: ["two_fold.rs"] }]);
  expect(misses(failed, [check([], ["two_fold.rs"]), check(["test/a.test.ts"])])).toEqual([]);
  expect(misses(failed, [check([], [], true)])).toEqual([]);
  expect(misses(failed, [])).toEqual(failed);
});

// A push's check runs what its own change can affect (ADR 1101, amended):
// others' commits since the map's were checked by their pushes and by CI,
// whose check keeps the full rule. lisa agent's test-only push selected all,
// "crates/rust-js-program/src/js/writes.rs's change at line 89 is outside any function's counted
// code (others' since the map's commit)": an item of theirs. Their function
// changes still count where this change touches the compiler's code, as a
// test their change made reach one of its functions runs.
test("a push's check runs its own change's tests, others' function changes only beside its compiler code, never their all", () => {
  const side = (file: string, hunks: [number, number][], own: boolean) => select([{ file, hunks }], { ...context, own: () => own });
  // Her case: her own test file, and another's item since the map.
  const hers = side("test/two.test.ts", [[3, 1]], true);
  const item = side("src/a.rs", [[25, 1]], false);
  expect(item.all).toBe(true);
  const pushed = pushSelection(hers, item, false);
  expect([pushed.all, pushed.files, pushed.cases]).toEqual([false, ["test/two.test.ts"], []]);
  expect(pushed.why).toContain("not run here, which CI's check runs");
  // Her own compiler change beside another's function change: both sides'.
  const mine = side("src/a.rs", [[32, 1]], true);
  const theirs = side("src/a.rs", [[12, 1]], false);
  const both = pushSelection(mine, theirs, true);
  expect(both.all).toBe(false);
  expect(both.files).toEqual(expect.arrayContaining([...mine.files, ...theirs.files]));
  expect(both.forms).toEqual(expect.arrayContaining(["x.rs"]));
  // Beside another's item: her own alone.
  expect(pushSelection(mine, item, true)).toMatchObject({ all: false, files: mine.files, cases: mine.cases });
  // Her own item: everything.
  expect(pushSelection(side("src/a.rs", [[25, 1]], true), theirs, true).all).toBe(true);
});

test("a change's line in a file changed since the map is placed at the map's line its text has, and only text that changed falls back", () => {
  // From the map's file to the change's base, as `git diff -U0` numbers
  // them: the map's lines 3 and 4 one line, 3; two lines added after the
  // map's 10, the base's 10 and 11; the map's 20 removed, after the base's 20.
  const between: [number, number, number, number][] = [[3, 2, 3, 1], [10, 0, 10, 2], [20, 1, 20, 0]];
  expect([1, 2, 3, 4, 5, 9, 10, 11, 12, 20, 21, 30].map((line) => mapLine(between, line))).toEqual([1, 2, undefined, 5, 6, 10, undefined, undefined, 11, 19, 21, 30]);
  // A hunk in lines neither changed, placed; one in a changed line, none.
  expect(hunksAtMap({ file: "src/a.rs", hunks: [[12, 2, ["x", "y"], ["a", "b"]], [21, 0, ["z"], []]] }, between)).toEqual({ file: "src/a.rs", hunks: [[11, 2, ["x", "y"], ["a", "b"]], [21, 0, ["z"], []]] });
  expect(hunksAtMap({ file: "src/a.rs", hunks: [[3, 1, ["x"], ["a"]]] }, between)).toBeUndefined();
  expect(hunksAtMap({ file: "src/a.rs", hunks: [[10, 0, ["x"], []]] }, between)).toBeUndefined();
  // The fallback runs what runs on every change too.
  const map = { functions: [{ file: "src/a.rs", spans: [[1, 9]] }, { file: "src/b.rs", spans: [[1, 9]] }], units: { "test/a.test.ts": Buffer.from([1]).toString("base64"), "test/b.test.ts": Buffer.from([2]).toString("base64"), "test/always.test.ts": null } } as never;
  expect(movedUnits(map, ["src/a.rs"]).sort()).toEqual(["test/a.test.ts", "test/always.test.ts"]);
});
