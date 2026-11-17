// A change's mutations (ADR 0093): which of them a change touches, by the
// functions, tests and corpus cases it changes, and how they're built many
// to a build. Split from test/ci.test.ts by concept (#1334).

import { expect, test } from "bun:test";
import { cpSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { batches, buildsUnmutated, type Change, changedLines, changedMutations, enclosingFunction, mutations, ofTouched, ownMutations, refusedFlags, schema, setPasses, site, siteLines, type Sited, testBlocks, testResults, testSet, typechecked, unitTestsModule } from "../scripts/mutations";
import { phases } from "../scripts/push";
import { fixture, root } from "./support";

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

// A run builds the compiler as it is itself where a mutant is built beside
// it, of the crate or the corpus's harness, or where none is given: CI's
// mutation shards build none, and a run of the scripts' mutations alone,
// the attribution's, failed "build the read-only compiler" (check
// 43872319768).
test("a run of the scripts' mutations alone builds the compiler where none is given", () => {
  const of = (crate: boolean, harness: boolean, given: boolean) => buildsUnmutated({ crate, harness, given });
  expect([of(false, false, true), of(false, false, false), of(true, false, true), of(false, true, true)]).toEqual([false, true, true, true]);
});

// A push type-checks the mutations of what it changes (scripts/push.ts's
// `mutants`): a `replace` written against a signature the change changed
// passed the push's check of `find` and failed CI's shards as "doesn't
// build", main red on 2026-11-18 (`own-method-call-other-this`).
test("a push type-checks the mutations of each file its change touches, and of each list it touches", () => {
  const of = (name: string, file: string) => ({ name, breaks: "", file, find: "", replace: "", tests: [] });
  const among = [
    { path: "a.ts", mutations: [of("in-a", "src/a.rs"), of("in-a-helper", "src/runtime/a.js")] },
    { path: "b.ts", mutations: [of("in-b", "src/b.rs")] },
    { path: "push.ts", mutations: [of("in-push", "scripts/push.ts")] },
  ];
  const names = (changed: string[]) => ofTouched(changed, among).map((m) => m.name);
  expect(names(["src/a.rs", "docs/x.md"])).toEqual(["in-a"]);
  expect(names(["scripts/mutations/b.ts"])).toEqual(["in-b"]);
  expect(names(["scripts/push.ts", "scripts/mutations/push.ts"])).toEqual([]);
  expect(refusedFlags(["--typecheck=origin/main"])).toEqual([]);
  expect(refusedFlags(["--typecheck"])).toEqual(["--typecheck takes a value: --typecheck=<base>"]);
  expect(phases.indexOf("mutants")).toBeGreaterThan(phases.indexOf("build"));
  expect(phases.indexOf("mutants")).toBeLessThan(phases.indexOf("check"));
  expect(readFileSync(join(root, "scripts/push.ts"), "utf8")).toContain('run("mutants", ["bun", "scripts/mutations.ts", `--typecheck=${base}`]);');
});

test("a change's mutants are type-checked many to a check, and one that doesn't type-check is named alone", async () => {
  const dir = fixture("mutation-typecheck");
  let checks = 0;
  const check = async (files: Map<string, string>) => {
    checks++;
    mkdirSync(join(dir, "src"), { recursive: true });
    writeFileSync(join(dir, "src/main.rs"), files.get("src/main.rs")!);
    const p = Bun.spawnSync(["rustc", "--edition=2021", "-Awarnings", "--emit=metadata", "--out-dir", dir, "src/main.rs"], { cwd: dir });
    return p.exitCode === 0 ? undefined : p.stderr.toString();
  };
  const sited = (of: ReturnType<typeof inProgram>[]): Sited[] => of.map((mutation) => ({ mutation, site: site(mutation, readProgram) }));
  // Those in bodies in one check, and one in a `const fn` alone.
  const twiceOff = inProgram("twice-off", "n * 2", "n * 2 + 1");
  expect([...(await typechecked(sited([sumOff, sumFirst, shownUnit, twiceOff]), check, readProgram))]).toEqual([]);
  expect(checks).toBe(2);
  const summedAsText = inProgram("sum-as-text", "    total\n", '    "total"\n');
  const problems = await typechecked(sited([sumOff, summedAsText, shownUnit]), check, readProgram);
  expect([...problems].map(([m, why]) => [m.name, why.startsWith("doesn't type-check: error[E0308]")])).toEqual([["sum-as-text", true]]);
  // Placed by its error, checked alone, the rest again together: three
  // checks, not a halving's.
  expect(checks).toBe(5);
});
