// The known bugs the tests must catch (ADR 0093): each applies to the
// compiler as it is, exactly once, and names tests to catch it. Building
// and running them is scripts/mutations.ts's.

import { expect, test } from "bun:test";
import { chmodSync, existsSync, mkdirSync, readdirSync, readFileSync, rmSync, statSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { pathToFileURL } from "node:url";

import { againstWholeCorpus, failingOf, testSetKey, unjudgedBy, duplicateNames, drivesABrowser, ofWholeCorpus, belowFloor, betweenTests, byControl, catchingTest, crateFiles, failures, inconclusiveEnd, judge, judgeTwice, judgeUnit, lists, mutate, mutations, narrowedTo, ofReadabilityOwner, recorded, readabilityOwners, refusedFlags, regenDiffers, removedOfCrate, retiredBy, retiredTrailer, retiringNames, site, siteLines, syncTree, testsEnv, unitTestsModule, unmatched, unparsed, verdictsDiffer } from "../scripts/mutations";
import { builtBinary, freshShare, unapplied, verdictKind } from "../scripts/mutant-builds";
import { casesReaching, noMap } from "../scripts/testMap";
import { underLoad } from "../scripts/timing";
import { run as runChild, runSync, type Exit } from "./child";
import { corpusHarness, corpusMutantCases, corpusMutants, mutantOf } from "./corpus-mutants";
import { installPackages } from "./packages-of";
import { node } from "./programs";
import { limitations } from "./limitations-of";
import { fixture, root, run } from "./support";
import { compilerRustFiles, crateOf, isCompilerSource } from "../scripts/compiler-crates";

// What the crate includes from beyond `src/`, `include_str!("../../../test/
// vec-collect-ways.txt")`, is in the crate the mutations build, or its unit
// tests don't build there and no mutation held by one runs (#1355).
test("the crate the mutations build has each file the compiler includes", () => {
  const { dirname, join: joined, normalize, relative } = require("node:path") as typeof import("node:path");
  const files = compilerRustFiles();
  const outside = new Set<string>();
  for (const file of files) {
    for (const [, included] of readFileSync(joined(root, file), "utf8").matchAll(/include_(?:str|bytes)!\("([^"]+)"\)/g)) {
      const path = relative(root, normalize(joined(root, dirname(file), included)));
      if (!isCompilerSource(path)) outside.add(path);
    }
    // And what a unit test reads where its crate is (`CARGO_MANIFEST_DIR`).
    for (const [, read] of readFileSync(joined(root, file), "utf8").matchAll(/env!\("CARGO_MANIFEST_DIR"\)\)\s*\.join\("([^"]+)"\)/g)) {
      outside.add(relative(root, normalize(joined(root, crateOf(file)!.dir, read))));
    }
  }
  const copied = (path: string) => crateFiles.some((kept) => path === kept || path.startsWith(`${kept}/`));
  expect([...outside].filter((path) => !copied(path))).toEqual([]);
});

test("every mutation applies to the compiler as it is, once", () => {
  expect(mutations.length).toBeGreaterThan(0);
  expect(new Set(mutations.map((m) => m.name)).size).toBe(mutations.length);
  for (const m of mutations) {
    const mutated = mutate(readFileSync(join(root, m.file), "utf8"), m);
    expect([m.name, typeof mutated === "string" ? "applies" : mutated.problem]).toEqual([m.name, "applies"]);
    // Its `bun test` tests or its unit tests, one of the two (#915).
    expect([m.name, m.tests === undefined ? m.unitTests !== undefined && m.snapshots === undefined : m.tests.length > 0 && m.unitTests === undefined]).toEqual([
      m.name,
      true,
    ]);
  }
});

// A mutation's unit tests are there: its filter names a `#[test]` of a
// source file of its module, by its name, or the module of some.
test("every mutation's unit tests are there", () => {
  // Each crate's: a mutation's unit tests are its file's crate's
  // (`unitTarget`, ADR 1181).
  const sources = compilerRustFiles().map((file) => ({
    module: unitTestsModule(file)!,
    tests: [...readFileSync(join(root, file), "utf8").matchAll(/#\[test\]\s*(?:#\[[^\]]*\]\s*)*fn (\w+)/g)].map((m) => m[1]),
  }));
  const named = mutations.filter((m) => m.unitTests !== undefined);
  expect(named.length).toBeGreaterThan(0);
  const missing = named.filter(({ unitTests: filter }) =>
    !sources.some(({ module, tests }) => tests.length > 0 && (module.startsWith(`${filter}::`) || module === filter || (filter!.startsWith(`${module}::`) && tests.some((t) => filter!.endsWith(`::${t}`))))),
  );
  expect(missing.map((m) => `${m.name}: ${m.unitTests}`)).toEqual([]);
});

// A unit test's run, as libtest prints it, says whether its mutant is
// caught: a filter that runs none says nothing of it.
test("a unit-tested mutant is caught where its tests fail, and only then", () => {
  const exit = (code: number | null, stdout: string, more: Partial<Exit> = {}): Exit => ({ code, signal: null, timedOut: false, overflowed: false, stdout, stderr: "", bytes: { stdout: Buffer.alloc(0), stderr: Buffer.alloc(0) }, ...more });
  const ok = "running 1 test\ntest a::tests::b ... ok\n\ntest result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 900 filtered out; finished in 0.00s\n";
  const failed = "running 1 test\ntest a::tests::b ... FAILED\n\ntest result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 900 filtered out; finished in 0.00s\n";
  expect(judgeUnit(exit(101, failed))).toBe("caught");
  expect(judgeUnit(exit(0, ok))).toBe("survived");
  expect(judgeUnit(exit(0, ok.replace("1 passed", "0 passed")))).toBe("inconclusive");
  expect(judgeUnit(exit(null, failed, { timedOut: true, signal: "SIGTERM" }))).toBe("inconclusive");
  expect(judgeUnit(exit(101, "thread 'main' panicked at src/main.rs:1:1\n"))).toBe("inconclusive");
});

// An inconclusive mutant's report says how its tests ended, so a run
// killed under load is told from one that exited 2 (#2112).
test("an inconclusive mutant's report says how its tests ended", () => {
  const exit = (code: number | null, signal: string | null): Exit => ({ code, signal, timedOut: false, overflowed: false, stdout: "", stderr: "", bytes: { stdout: Buffer.alloc(0), stderr: Buffer.alloc(0) } });
  expect(inconclusiveEnd("m", exit(null, "SIGKILL"), "said 2")).toBe("--- m, inconclusive, its tests killed by SIGKILL: its log's end\nsaid 2\n---");
  expect(inconclusiveEnd("m", exit(2, null), "said 2")).toContain("its tests exited 2:");
});

// A mutant's build is held to being of it (scripts/mutant-builds.ts): its
// files in the crate as written, and the crate compiled again, not left as
// cargo had it; and a fresh build's verdict to its incremental build's.
test("a mutant's build is its mutation's, compiled again", () => {
  const artifact = (test: boolean, fresh: boolean, executable: string) =>
    JSON.stringify({ reason: "compiler-artifact", target: { name: "rust-js" }, profile: { test }, fresh, executable });
  const said = [
    JSON.stringify({ reason: "compiler-artifact", target: { name: "oxc_formatter" }, profile: { test: false }, fresh: false, executable: null }),
    artifact(false, true, "/t/debug/rust-js"),
    artifact(true, false, "/t/debug/deps/rust_js-1"),
  ].join("\n");
  expect(builtBinary(said, true)).toEqual({ binary: "/t/debug/deps/rust_js-1", rebuilt: true });
  expect(builtBinary(said, false)).toEqual({ binary: "/t/debug/rust-js", rebuilt: false });
  expect(builtBinary("", false)).toBeUndefined();

  const at = join(root, "target", "mutant-builds-test");
  rmSync(at, { recursive: true, force: true });
  try {
    for (const dir of ["crate", "source"]) mkdirSync(join(at, dir, "src"), { recursive: true });
    writeFileSync(join(at, "source", "src", "a.rs"), "fn a() {}\n");
    writeFileSync(join(at, "crate", "src", "a.rs"), "fn a() { b() }\n");
    const crate = join(at, "crate");
    const source = join(at, "source");
    expect(unapplied(new Map([["src/a.rs", "fn a() { b() }\n"]]), crate, source)).toBeUndefined();
    expect(unapplied(new Map([["src/a.rs", "fn a() { c() }\n"]]), crate, source)).toBe("src/a.rs isn't as the mutation wrote it");
    writeFileSync(join(at, "crate", "src", "a.rs"), "fn a() {}\n");
    expect(unapplied(new Map([["src/a.rs", "fn a() {}\n"]]), crate, source)).toBe("src/a.rs is as the crate has it");
    expect(unapplied(new Map([["src/new.js", "x"]]), crate, source)).toBe("src/new.js isn't as the mutation wrote it");
  } finally {
    rmSync(at, { recursive: true, force: true });
  }

  expect(verdictKind("caught")).toBe("caught");
  expect(verdictKind("SURVIVED: its 3 tests passed with it")).toBe("SURVIVED");
  expect(verdictKind("INCONCLUSIVE: see x.log")).toBe("INCONCLUSIVE");
  expect(verdictKind("doesn't build: error[E0308]")).toBe("doesn't");
  expect(freshShare(["--shard=1/2"])).toBe(0);
  expect(freshShare(["--fresh=0.05"])).toBe(0.05);
  expect(() => freshShare(["--fresh=5"])).toThrow("a share of the builds, 0 to 1");
});

// Each lexes, built alone and in a build of many, so a build of many fails
// only where a mutation's own code does: eight of one function, its site
// cut at a pattern's `{` in its signature, split four of 2026-11-02's
// nightly shards' builds down to one each, and said nothing of which (#479).
test("every mutation's build lexes, alone and in a build of many", () => {
  const read = (file: string) => readFileSync(join(root, file), "utf8");
  const sources = compilerRustFiles();
  expect(sources.filter((file) => unmatched(read(file)) !== undefined)).toEqual([]);
  const files = new Map<string, string>();
  const cached = (file: string) => files.get(file) ?? files.set(file, read(file)).get(file)!;
  const unlexed = mutations.flatMap((m) => {
    const problem = unparsed(m, site(m, cached), cached);
    return problem ? [`${m.name}: ${problem}`] : [];
  });
  expect(unlexed).toEqual([]);
  // 1.6 s alone; 13.6–14.2 s under check:fast's load, all its parts at once
  // (2026-11-06), past bun's 5 s.
}, underLoad(30_000));

test("a mutation's build that wouldn't lex says where", () => {
  const read = (source: string) => () => source;
  const m = { name: "x", breaks: "", file: "a.rs", find: "let a = (1);", replace: "let a = (1;", tests: ["t"] };
  expect(unparsed(m, undefined, read("fn f() {\n    let a = (1);\n}\n"))).toBe("doesn't lex: a.rs with it has an unexpected closing delimiter `}` on line 3");
  // What's in strings, chars and comments is no delimiter.
  expect(unmatched('fn f() { let _ = ("}", \'{\', r#"("#); /* { /* ] */ */ } // )\nfn g<\'a>(_: &\'a ()) {}')).toBeUndefined();
  expect(unmatched("fn f() { (")).toBe("unclosed delimiter `(` on line 1");
});

// The tests a mutation names are there: a `-t` of the corpus names one of
// its cases, of the diagnostics one of theirs, of the limitations one of
// the guide's items. Two removed tests left a shard of main's check each
// running nothing, 2026-10-28, and an item reworded another, 2026-11-04's
// nightly (#606).
test("every mutation's corpus, diagnostics and limitations tests are there", () => {
  const cases = [...new Bun.Glob("*.rs").scanSync(join(root, "test/corpus"))];
  const diagnostics = [
    ...readFileSync(join(root, "test/diagnostics.test.ts"), "utf8").matchAll(/^ {2}\[\s*(["'])(.+?)\1,|test\(\s*(["'])(.+?)\3/gm),
  ].map((m) => m[2] ?? m[4]);
  // Each file's names read once, and each pattern tried once of a file:
  // made again for each mutation, the guide's limitations parsed each
  // time, it took 2 s alone, past bun's 5 s on a loaded machine.
  const named: Record<string, string[]> = {
    "test/corpus.test.ts": cases,
    "test/diagnostics.test.ts": diagnostics,
    "test/limitations.test.ts": limitations().map((l) => l.title),
  };
  const found = new Map<string, boolean>();
  const missing = mutations.filter((m) => {
    if (m.tests === undefined) return false;
    const at = m.tests.indexOf("-t");
    const names = named[m.tests[0]!];
    if (at < 0 || names === undefined) return false;
    const key = `${m.tests[0]}\n${m.tests[at + 1]}`;
    if (!found.has(key)) {
      const pattern = new RegExp(m.tests[at + 1]!);
      found.set(key, names.some((name) => pattern.test(name)));
    }
    return !found.get(key);
  });
  expect(missing.map((m) => `${m.name}: ${m.tests!.join(" ")}`)).toEqual([]);
});

// A source file of any length has bugs its tests must catch: each of more
// than 80 lines has mutations, or says here why it needs none (the second
// architecture review: ten had none, and two of them held dead code and a
// bug no test could see).
const unmutated: Record<string, string> = {
  "crates/rust-js-program/src/program.rs": "the types the phases hand each other, with no behavior of their own",
  "crates/rust-js-lower/src/shortcuts.rs": "one-line forwards to recognition, whose mutations hold the answers",
  "src/jsx_syntax/literals.rs": "generated from @types/react; react-versions.test.ts holds it equal to its generator",
  "crates/rust-js-mir/src/tidy/contracts.rs": "a debug build's checks of what tidy leaves, which change no output; its unit tests, `cargo test`, break each",
};

test("every source file of more than 80 lines has mutations, or says why it needs none", () => {
  const mutated = new Set(mutations.map((m) => m.file));
  const bare = compilerRustFiles()
    .filter((file) => readFileSync(join(root, file), "utf8").split("\n").length > 80)
    .filter((file) => !mutated.has(file) && !(file in unmutated));
  expect(bare.sort()).toEqual([]);
  for (const file of Object.keys(unmutated)) {
    expect([file, existsSync(join(root, file)), mutated.has(file)]).toEqual([file, true, false]);
  }
});

// So code that moves takes its mutations with it, to the list of where it went.
// Each mutation's `-t` names a test its file has: one that names none, of a
// test moved to another file, ran nothing as the compiler is, and CI's
// mutations shard said so only when it ran (the closures' guard tests split
// from lowering-drop-guards.test.ts). Of a file whose tests are each named
// by a literal, read without running it.
test("each mutation's test pattern names a test of its file", () => {
  // Each file's tests named by a literal, and whether others are named
  // where they run, of a template or a value, which aren't read here.
  const named = new Map<string, { names: string[]; dynamic: boolean }>();
  const namesOf = (file: string) => {
    if (!named.has(file)) {
      const source = existsSync(join(root, file)) ? readFileSync(join(root, file), "utf8") : "";
      const literal = /^\s*(?:test|it)(?:\.\w+)?\(\s*"((?:[^"\\]|\\.)*)"/gm;
      const dynamic = /^\s*(?:test|it)(?:\.\w+)?\(\s*[^"\s]/m.test(source) || !file.endsWith(".test.ts");
      // The corpus's tests are its cases, each by its file's name.
      const cases = file === "test/corpus.test.ts" ? readdirSync(join(root, "test/corpus")).filter((name) => name.endsWith(".rs")) : [];
      named.set(file, { names: [...cases, ...[...source.matchAll(literal)].map((m) => JSON.parse(`"${m[1]}"`))], dynamic });
    }
    return named.get(file)!;
  };
  const unmatched = mutations.flatMap((m) => {
    const [file, ...args] = m.tests ?? [];
    const at = args.indexOf("-t");
    if (file === undefined || at < 0) return [];
    const { names, dynamic } = namesOf(file);
    const pattern = new RegExp(args[at + 1]);
    if (names.some((name) => pattern.test(name))) return [];
    if (dynamic) return [];
    return [`${m.name}: ${file} -t ${args[at + 1]}`];
  });
  // Of a file naming others where they run, a pattern that names none of
  // its literal names isn't read here: the runner's controls run each
  // mutation's tests as the compiler is first and refuse one that runs
  // nothing (scripts/mutations.ts), on a push, CI's run and each night's.
  expect(unmatched).toEqual([]);
});

// A test failing as the compiler is leaves its own mutations unjudged, and
// only those: nightly 44963035271's shards 5, 9, 10 and 12 judged none of
// their 950 each, `match-trees.test.ts` failing on main and a Vite watcher
// slow under load (#2304). A mutation sharing the test with other settings,
// or none, runs.
test("a failed control leaves its own mutations unjudged, not the shard's", () => {
  const failing = { tests: ["test/match-trees.test.ts", "-t", "seed 2"] };
  const same = { tests: ["test/match-trees.test.ts", "-t", "seed 2"] };
  const snapshots = { tests: ["test/match-trees.test.ts", "-t", "seed 2"], snapshots: true };
  const other = { tests: ["test/lowering.test.ts"] };
  const unit = { unitTests: "in_place_forms" };
  const failedSets = new Map([[testSetKey(failing), "INCONCLUSIVE: its tests don't pass as the compiler is"]]);
  const failedUnits = new Map([["in_place_forms", "INCONCLUSIVE: its unit tests don't run and pass as the crate is"]]);
  const unjudged = unjudgedBy([failing, same, snapshots, other, unit], failedSets, failedUnits);
  expect([...unjudged.keys()]).toEqual([failing, same, unit]);
  expect(unjudged.get(unit)).toContain("unit tests");
  expect(unjudgedBy([other], failedSets, new Map()).size).toBe(0);
});

// A run of every mutation, the nightly's, files its survivors and fails
// only on what it couldn't judge; a run of some mutations, a push's or the
// ones named, still fails on a survivor (#2304, ADR 0093).
test("a run of every mutation fails on what it couldn't judge, not on its survivors", () => {
  const survived: [string, string][] = [["a", "SURVIVED: its 1 tests passed with it"], ["b", "SURVIVED: its 2 tests passed with it, snapshots not compared"]];
  const inconclusive: [string, string] = ["c", "INCONCLUSIVE: its tests don't pass as the compiler is"];
  expect(failingOf(survived, true)).toEqual([]);
  expect(failingOf([...survived, inconclusive, ["d", "not run"]], true)).toEqual(["c", "d"]);
  expect(failingOf(survived, false)).toEqual(["a", "b"]);
  expect([recorded(["--shard=3/16", "--fresh=0.05", "--timings=target/mutation-timings"]), recorded(["--changed=main"]), recorded(["copy-on-read"])]).toEqual([true, false, false]);
  // What it spares is still filed: each shard's `results.json` is uploaded
  // and the issue's job runs however the shards ended, passing too.
  const workflow = readFileSync(join(root, ".github/workflows/rustc-tests.yml"), "utf8");
  expect(workflow).toMatch(/- if: always\(\)\n\s+uses: actions\/upload-artifact@v\d+\n\s+with:\n\s+name: mutation-results-/);
  expect(workflow).toMatch(/\n  survivors:\n    if: \$\{\{ always\(\) && /);
});

// `bun test` reads one `-t`, the last: a mutation of two ran its first
// files' tests by the second's pattern, and await-value-unread's ran none
// of its first file's (checks 43996182737, 43999199934). Each file's tests
// are named by one pattern, alternatives joined by `|`.
test("a mutation's tests hold one `-t`, the one `bun test` reads", () => {
  const twice = mutations.filter((m) => (m.tests ?? []).filter((arg) => arg === "-t").length > 1).map((m) => m.name);
  expect(twice).toEqual([]);
});

// A mutation names the corpus cases that catch it, `-t`, never the whole
// corpus: a run of it builds and runs 2,800 cases for each mutant, as many
// minutes of a CI shard as the rest of the shard's mutations, where its
// catching cases take seconds (the queue's CI wall time, 2a). `--corpus`
// runs one against the whole corpus, to find them.
test("no mutation names the whole corpus: each names its catching cases by `-t`", () => {
  const whole = mutations.filter((m) => ofWholeCorpus(m.tests ?? [])).map((m) => `${m.name}: find its catching cases with \`bun scripts/mutations.ts --corpus ${m.name}\` and name them with -t`);
  expect(whole).toEqual([]);
});

// A matrix's rows a mutation runs are named by a pattern of their ids,
// `ITERATOR_MATRIX=/ \| copied \| (clone|dup) \| len$/`, never by an
// issue's known rows, `#1983`: the issue's fix takes them from the known
// list, and the mutation's control then finds no case and fails, as
// `copy-matrix-clones-nothing`'s did, main's mutation job red (41d89f84f).
test("a mutation runs a matrix's rows by a pattern of their ids, not by an issue's", () => {
  const byIssue = mutations.filter((m) => /^#/.test(m.env?.ITERATOR_MATRIX ?? "")).map((m) => `${m.name}: ITERATOR_MATRIX=${m.env!.ITERATOR_MATRIX}, name its rows by a pattern of their ids, /<regex>/`);
  expect(byIssue).toEqual([]);
});

// A mutation of the crate is retired with `--retire`'s evidence (ADR 0093,
// amended): twice in a day one was called unreachable after its own rows
// alone ran, and the second's removal failed 8 corpus cases. Each commit
// not yet on `main`, as `bun run push` checks them.
test("a commit that removes a mutation of the crate carries its retirement's evidence", () => {
  const git = (args: string[]) => runSync(["git", ...args], root, 30_000);
  const base = git(["merge-base", "HEAD", "origin/main"]);
  if (base.code !== 0) return;
  const commits = git(["rev-list", `${base.stdout.trim()}..HEAD`]).stdout.split("\n").filter(Boolean);
  const unretired = commits.flatMap((commit) => {
    const touched = git(["diff-tree", "--no-commit-id", "--name-only", "-r", `${commit}^`, commit, "--", "scripts/mutations/"]).stdout.split("\n").filter(Boolean);
    const text = (at: string) => touched.map((file) => git(["show", `${at}:${file}`]).stdout).join("\n");
    const retired = retiredBy(git(["log", "-1", "--format=%B", commit]).stdout);
    return removedOfCrate(text(`${commit}^`), text(commit), (file) => git(["show", `${commit}:${file}`]).stdout)
      .filter((name) => !retired.has(name))
      .map((name) => `${commit.slice(0, 9)} removes ${name}: \`bun scripts/mutations.ts --retire=${name}\` prints the trailer it carries, or the tests that make it a row`);
  });
  expect(unretired).toEqual([]);
  // Two `git show`s of each list a commit touches: a cut's, of every list
  // its moved files have (#2299), takes seconds.
}, underLoad(60_000));

// The rule's parts: a mutation renamed or moved isn't removed, nor one
// whose code went too, nor a script's; and the trailer is `--retire`'s,
// its evidence said.
test("a removed mutation is told from one renamed or moved, and its trailer is --retire's", () => {
  const entry = (name: string, file: string, find: string) => `  {\n    name: "${name}",\n    breaks: "x",\n    file: "${file}",\n    find: "${find}",\n    replace: "",\n    tests: ["test/a.test.ts"],\n  },\n`;
  const before = entry("kept", "src/a.rs", "a") + entry("renamed", "src/b.rs", "b") + entry("gone", "src/c.rs", "c") + entry("script-gone", "scripts/d.ts", "d");
  const after = entry("kept", "src/a.rs", "a") + entry("renamed-now", "src/b.rs", "b");
  const source = () => "a b c d";
  expect(removedOfCrate(before, after, source)).toEqual(["gone"]);
  // One whose code went with it, a refactor's folded into another, can't
  // apply, and isn't retired (#2012's refactor of `given_fact_of`).
  expect(removedOfCrate(before, after, () => "")).toEqual([]);
  // A `find` in single quotes is read too: `cycle-compiled-in-every-build`'s
  // `'    StdPath::IterCycle,\n'`, its code gone with the cycle flip.
  const quoted = `  {\n    name: "single",\n    breaks: "x",\n    file: "src/e.rs",\n    find: '    it("x"),\\n',\n    replace: "",\n    tests: ["test/a.test.ts"],\n  },\n`;
  expect(removedOfCrate(quoted, "", () => "")).toEqual([]);
  expect(removedOfCrate(quoted, "", () => '    it("x"),\n')).toEqual(["single"]);
  expect([...retiredBy(`fix: x\n\n${retiredTrailer}: gone: with it, at 0123abcde, the compiler's 900 unit tests passed, and check:fast's suite\n`)]).toEqual(["gone"]);
  expect([...retiredBy(`fix: x\n\n${retiredTrailer}: gone\n`)]).toEqual([]);
  // Or a run that executed none of its site, a coverage or marker build's.
  expect([...retiredBy(`fix: x\n\n${retiredTrailer}: dead: unreached, at 0123abcde, by a marker build: check:fast --all\n`)]).toEqual(["dead"]);
  expect([...retiredBy(`fix: x\n\n${retiredTrailer}: dead: unreached\n`)]).toEqual([]);
  // A test that fails as the compiler is too, in a mutant's environment,
  // says nothing: it's no row of the mutation's.
  const failed = [{ file: "test/corpus.test.ts", test: "untagged_enums.rs" }, { file: "test/compiles.test.ts", test: "a compile is kept" }];
  expect(byControl(failed, new Set(["test/corpus.test.ts\tuntagged_enums.rs"]), new Set(["test/compiles.test.ts\ta compile is kept"]))).toEqual({ caught: ["test/corpus.test.ts: untagged_enums.rs"], silent: ["test/compiles.test.ts: a compile is kept"], flaked: [] });
  // One that passed run again with the mutant is a flake, no row of it.
  expect(byControl(failed, new Set(), new Set())).toEqual({ caught: [], silent: [], flaked: ["test/corpus.test.ts: untagged_enums.rs", "test/compiles.test.ts: a compile is kept"] });
});

// A retirement's evidence holds React's port too, at its pin, which
// check:fast leaves to CI: a retirement that passed the suite refused 5 of
// the port's Flight entries. Its control, the compiler as it is, must
// regenerate it as committed, or the retirement is refused.
test("--retire regenerates React's port with the mutant, held to its control", () => {
  expect(regenDiffers("### react-in-rust at 0123abcde: 2 file(s) differ from what it committed\n\n- ` M crates/flight/js/server.js`\n- `?? crates/flight/js/new.js`\n")).toEqual(["M crates/flight/js/server.js", "?? crates/flight/js/new.js"]);
  expect(regenDiffers("### react-in-rust at 0123abcde doesn't compile\n\n```\nerror\n```")).toEqual(["its crates didn't compile"]);
  expect(regenDiffers("something else")).toEqual([]);
  const source = readFileSync(join(root, "scripts/mutations.ts"), "utf8");
  const retire = source.match(/\nasync function retire\([\s\S]*?\n}\n/)![0] + source.match(/\nasync function retireSuite\([\s\S]*?\n}\n/)![0]
    + source.match(/\nasync function suiteWith\([\s\S]*?\n}\n/)![0] + source.match(/\nasync function retireVerdict\([\s\S]*?\n}\n/)![0];
  const control = retire.indexOf("portDiffers(portControl)");
  expect(control).toBeGreaterThan(0);
  expect(control).toBeLessThan(retire.indexOf("scripts/checkFast.ts"));
  // Run once for all the mutations retired together, before the first
  // mutant (#2043).
  expect(control).toBeLessThan(retire.indexOf("for (const [at, mutation] of chosen.entries())"));
  expect(retire.indexOf("crateVerdicts(portControl, true)")).toBeLessThan(retire.indexOf("for (const [at, mutation] of chosen.entries())"));
  expect(retire).toContain("port control fails: ");
  expect(retire).toContain("...portCaught, ...crateCaught]");
  expect(retire).toContain("React's port at its pin regenerated as it committed");
});

// A retirement judges nothing below the disk's floor, where the loop's
// stores keep nothing and tests fail with any compiler; and each failure's
// control runs right after the mutant's rerun of it, so the load and the
// disk are one for both. A retirement at the floor blamed source-mutations'
// and native-outcomes' tests on two mutants, their control run an hour
// later with more free (2026-11-18).
test("--retire judges nothing below the disk's floor, and reruns each failure's control beside the mutant's", () => {
  const gib = 2 ** 30;
  expect(belowFloor(50 * gib, 40 * gib)).toBeUndefined();
  expect(belowFloor(40 * gib, 40 * gib)).toBeUndefined();
  expect(belowFloor(39.5 * gib, 40 * gib)).toBe("the disk has 39.5 GiB free, below its floor of 40.0 GiB, where the loop's stores keep nothing and tests fail with any compiler: free some, and run it again");
  const source = readFileSync(join(root, "scripts/mutations.ts"), "utf8");
  const of = (name: string) => source.match(new RegExp(`\\nasync function ${name}\\([\\s\\S]*?\\n}\\n`))![0];
  // Before the controls and before each suite, and after it.
  expect(of("retire").indexOf("belowFloor(freeBytes(root))")).toBeLessThan(of("retire").indexOf("everyUnit(undefined"));
  const suite = of("suiteWith");
  const checkFast = suite.indexOf("scripts/checkFast.ts");
  expect(suite.indexOf("belowFloor(freeBytes(root))")).toBeLessThan(checkFast);
  expect(suite.lastIndexOf("belowFloor(freeBytes(root))")).toBeGreaterThan(checkFast);
  // The reruns follow the suite, in it, and the verdict runs none.
  expect(suite.indexOf("pairedReruns(failed, mutant, unmutated)")).toBeGreaterThan(checkFast);
  expect(of("retireVerdict")).not.toContain("failingWith(");
  // Of each file, the mutant's rerun, then at once the control's, each
  // pair above the floor.
  const paired = of("pairedReruns");
  const loop = paired.slice(paired.indexOf("for (const file of"));
  expect(loop.indexOf("belowFloor(freeBytes(root))")).toBeLessThan(loop.indexOf("failingWith(ofFile, mutant, failures)"));
  expect(loop.indexOf("failingWith(ofFile, mutant, failures)")).toBeLessThan(loop.indexOf("failingWith(ofFile, unmutatedCopy, failedTests)"));
});

// A retirement's suite runs fail-fast first, stopped at its first failure,
// whose paired reruns, a catch, end it, its port and crate corpus unrun;
// where nothing it ran caught it, or a unit test did, its whole suite, or
// none: a retirement's evidence is still the whole suite (#2256).
test("--retire stops a caught mutant at its first catch and runs the whole suite of one nothing catches", () => {
  const source = readFileSync(join(root, "scripts/mutations.ts"), "utf8");
  const of = (name: string) => source.match(new RegExp(`\\nasync function ${name}\\([\\s\\S]*?\\n}\\n`))![0];
  const suite = of("retireSuite");
  expect(suite.indexOf("if (unitCaught.length > 0)")).toBeLessThan(suite.indexOf("suiteWith(mutation, mutant, unmutated, true)"));
  expect(suite).toContain("fast.cut && byControl(fast.failed, fast.again, fast.control).caught.length === 0 ? await suiteWith(mutation, mutant, unmutated, false) : fast");
  expect(of("suiteWith")).toContain(`failFast ? ["--fail-fast"] : []`);
  const verdict = of("retireVerdict");
  expect(verdict.indexOf("const caughtAlready")).toBeLessThan(verdict.indexOf("portDiffers(mutant)"));
  expect(verdict).toContain("caughtAlready ? [] :");
  const checkFast = readFileSync(join(root, "scripts/checkFast.ts"), "utf8");
  expect(checkFast).toContain(`args.includes("--fail-fast")`);
  expect(checkFast).toContain("const failed = results.filter((r) => r.code !== 0 && !r.cut);");
});

// Several retired at once share their controls (#2043): each flag's names,
// comma-separated, in order, once each.
test("--retire names several mutations, each once", () => {
  expect(retiringNames(["--changed=main"])).toBeUndefined();
  expect(retiringNames(["--retire=a"])).toEqual(["a"]);
  expect(retiringNames(["--retire=a,b", "--retire=c,a"])).toEqual(["a", "b", "c"]);
  expect(() => retiringNames(["--retire="])).toThrow("names no mutation");
});

// And crates.io's corpus, every row, which check:fast's suite runs with no
// other compiler: a fallback whose retirement passed the suite and the
// port is reached by itertools. Each crate's verdict is held to the
// compiler's as it is, one that falls and one that rises, and its control
// first, held to the rows' bars, or the retirement is refused.
test("--retire measures the crate corpus with the mutant, held to its control", () => {
  const control = new Map<string, "runs" | "compiles" | "refused" | "blocked">([["itertools", "runs"], ["nom", "refused"], ["semver", "compiles"]]);
  expect(verdictsDiffer(control, new Map(control))).toEqual([]);
  expect(verdictsDiffer(control, new Map([["itertools", "compiles"], ["nom", "compiles"]]))).toEqual([
    "crate corpus: itertools compiles with it, runs as the compiler is",
    "crate corpus: nom compiles with it, refused as the compiler is",
    "crate corpus: semver unmeasured with it, compiles as the compiler is",
  ]);
  const source = readFileSync(join(root, "scripts/mutations.ts"), "utf8");
  const retire = ["retire", "retireSuite", "suiteWith", "retireVerdict"]
    .map((name) => source.match(new RegExp(`\\nasync function ${name}\\([\\s\\S]*?\\n}\\n`))![0])
    .join("");
  const control$ = retire.indexOf("crateVerdicts(portControl, true)");
  expect(control$).toBeGreaterThan(0);
  expect(control$).toBeLessThan(retire.indexOf("scripts/checkFast.ts"));
  expect(retire).toContain("crate corpus control fails: ");
  expect(retire).toContain("crateVerdicts(mutant, false)");
});

test("each mutation list names one existing source file", () => {
  const owners = lists.flatMap(list => list.mutations.length ? [list.mutations[0].file] : []);
  expect(new Set(owners).size).toBe(owners.length);
  for (const list of lists) {
    expect(new Set(list.mutations.map(m => m.file)).size).toBe(list.mutations.length ? 1 : 0);
    for (const m of list.mutations) expect(existsSync(join(root, m.file))).toBe(true);
  }
});

// A mutation is built and judged by its name, so two of one name, in one
// list or two, built one control workspace twice and crashed the run,
// `EEXIST` (#2237): the catalog has each name once.
test("each mutation's name is the catalog's once", () => {
  expect(duplicateNames(lists)).toEqual([]);
  const twice = [
    { path: "a.ts", mutations: [{ name: "x" }, { name: "y" }] },
    { path: "b.ts", mutations: [{ name: "x" }] },
    { path: "c.ts", mutations: [{ name: "z" }, { name: "z" }] },
  ];
  expect(duplicateNames(twice)).toEqual(["x: in scripts/mutations/a.ts and scripts/mutations/b.ts", "z: in scripts/mutations/c.ts and scripts/mutations/c.ts"]);
});

test("a mutation whose code moved, or is there twice, says so", () => {
  const m = { name: "x", breaks: "", file: "a.rs", find: "let a = 1;", replace: "let a = 2;", tests: ["t"] };
  expect(mutate("fn f() { let a = 1; }", m)).toBe("fn f() { let a = 2; }");
  expect(mutate("fn f() {}", m)).toEqual({ problem: "doesn't apply: a.rs has no `let a = 1;`" });
  expect(mutate("let a = 1; let a = 1;", m)).toEqual({ problem: "applies more than once in a.rs" });
});

// A mutant is caught only by a test that failed: a runner that ran out of
// time, was stopped, or failed before any test did, says nothing of it.
// Found in review: every failed process counted as caught.
test("a mutant is caught, survives, or the run says nothing of it", () => {
  const exit = (code: number | null, more: Partial<Exit> = {}): Exit => ({ code, signal: null, timedOut: false, overflowed: false, stdout: "", stderr: "", bytes: { stdout: Buffer.alloc(0), stderr: Buffer.alloc(0) }, ...more });
  const failed = "(fail) wrapping.rs [12.00ms]\n\n 0 pass\n 1 fail\n";
  expect(judge(exit(1), failed)).toBe("caught");
  expect(judge(exit(0), "(pass) wrapping.rs\n\n 1 pass\n 0 fail\n")).toBe("survived");
  expect(judge(exit(null, { timedOut: true, signal: "SIGTERM" }), failed)).toBe("inconclusive");
  expect(judge(exit(null, { signal: "SIGKILL" }), "")).toBe("inconclusive");
  expect(judge(exit(1), "error: Cannot find module './corpus'\n\n 0 pass\n 1 fail\n")).toBe("inconclusive");
  expect(judge(exit(0), " 0 pass\n 0 fail\n")).toBe("inconclusive");
  // A setup that failed, its control having passed, is the mutant's doing:
  // await-value-unread's examples/fetch.rs, compiled in test/compiler.test.ts's
  // `beforeAll`, read what nothing declares, and `--bail` ended the run
  // there (checks 43996182737, 43999199934, inconclusive). As bun test
  // prints a `beforeAll` that threw, and a `describe`'s body.
  expect(judge(exit(1), "(fail) (unnamed) [0.3ms]\nBailed out after 1 failure\n")).toBe("caught");
  const between = (error: string) => `# Unhandled error between tests\n-------------------------------\n${error}\n-------------------------------\n\n 1 pass\n 0 fail\n 1 error\n`;
  expect(judge(exit(1), between("error: describe broke\n      at <anonymous> (d.test.ts:4:35)"))).toBe("caught");
  // One that ran out of time says nothing still.
  expect(judge(exit(1), "(fail) (unnamed) [202.02ms]\n  ^ a beforeEach/afterEach hook timed out for this test.\nBailed out after 1 failure\n")).toBe("inconclusive");
  expect(judge(exit(1), between("error: beforeAll timed out after 5000ms"))).toBe("inconclusive");
  // A long diff bun test cuts short doesn't end its last line, and the
  // `(fail)` after it is printed on that line. Found running `f32_digits`.
  expect(judge(exit(1), "+ 38818(fail) f32_digits.rs [409.30ms]\n\n 0 pass\n 1 fail\n")).toBe("caught");
  // A test that ran out of time failed as bun test prints it, but said
  // nothing of what the JS does. Found in review: it counted as caught.
  const timedOut = "(fail) wrapping.rs [5001.00ms]\n  ^ this test timed out after 5000ms.\n";
  expect(judge(exit(1), `${timedOut}\n 0 pass\n 1 fail\n`)).toBe("inconclusive");
  expect(judge(exit(1), `${timedOut}${failed}`)).toBe("caught");
  // A hook that ran out of time, as bun test prints it: the test never ran.
  const hookTimedOut = "(fail) never reaches assertion [101.76ms]\n  ^ a beforeEach/afterEach hook timed out for this test.\n";
  expect(judge(exit(1), `${hookTimedOut}\n 0 pass\n 1 fail\n`)).toBe("inconclusive");
});

// What caught a mutant is named, its first failing test by its file and
// name, on its result line and in results.json's `caughtBy`, so a sweep of
// survivors reads it rather than running a whole corpus again by hand to
// find it (`mut-iterator-extension-unpeeled`'s twelve minutes): not one
// that ran out of time, which says only that the machine was slow.
test("a caught mutant names the first test that failed with it, by its file", () => {
  const output = [
    "test/class-constructors.test.ts:",
    "(pass) a constructor [3.00ms]",
    "",
    "test/corpus.test.ts:",
    "(pass) wrapping.rs [12.00ms]",
    "(fail) slow.rs [300000.00ms]",
    "  ^ this test timed out after 300000ms.",
    "(fail) alias_reads.rs [640.00ms]",
    "(fail) later.rs [10.00ms]",
  ].join("\n");
  expect(catchingTest(output)).toBe("test/corpus.test.ts > alias_reads.rs");
  expect(catchingTest("test/a.test.ts:\n(fail) d > b [1.00ms]\n")).toBe("test/a.test.ts > d > b");
  expect(catchingTest("test/a.test.ts:\n# Unhandled error between tests\n---\nboom\n---\n")).toBe(`test/a.test.ts > ${betweenTests}`);
  expect(catchingTest("test/a.test.ts:\n(pass) b [1.00ms]\n 1 pass\n 0 fail\n")).toBeUndefined();
  // Not a `(fail)` quoted in a failed test's diff, but the run's own after it.
  expect(catchingTest('test/a.test.ts:\n+     "(fail) slow.rs [3.00ms]",\n(fail) quoting [1.00ms]\n')).toBe("test/a.test.ts > quoting");
});

// A test that fails once with a mutant and passes run again with it is a
// flake, which hid a survivor as a catch: a `--retire` of
// `momentary-alias-read-taken` named tests of native-outcomes and
// source-mutations that passed when run again. A catch fails twice.
test("a mutant is caught by a test that fails with it twice, a flake inconclusive and named", async () => {
  const exit = (code: number | null): Exit => ({ code, signal: null, timedOut: false, overflowed: false, stdout: "", stderr: "", bytes: { stdout: Buffer.alloc(0), stderr: Buffer.alloc(0) } });
  const fail = (name: string) => ({ exit: exit(1), output: `(fail) ${name} [12.00ms]\nBailed out after 1 failure\n` });
  const pass = { exit: exit(0), output: "(pass) wrapping.rs\n\n 1 pass\n 0 fail\n" };
  expect(failures("(fail) d > a.rs [1.00ms]\n(fail) b.rs\n")).toEqual(["d > a.rs", "b.rs"]);
  const tests = ["test/corpus.test.ts", "-t", "^(wrapping|other).rs$"];
  expect(narrowedTo(tests, ["d > wrapping.rs"])).toEqual(["test/corpus.test.ts", "-t", "d.*wrapping\\.rs"]);
  expect(narrowedTo(tests, ["(unnamed)"])).toBe(tests);
  expect(narrowedTo(tests, [betweenTests])).toBe(tests);
  // Each run again: the failed tests alone, then the set whole.
  const runs = (...said: { exit: Exit; output: string }[]) => {
    const asked: string[][] = [];
    return { asked, again: async (tests: string[]) => (asked.push(tests), said.shift()!) };
  };
  const first = fail("wrapping.rs");
  const twice = runs(fail("wrapping.rs"));
  expect((await judgeTwice(first.exit, first.output, tests, twice.again)).verdict).toBe("caught");
  expect(twice.asked).toEqual([["test/corpus.test.ts", "-t", "wrapping\\.rs"]]);
  // One that fails only after the others is caught by the set whole.
  const after = runs(pass, fail("wrapping.rs"));
  expect((await judgeTwice(first.exit, first.output, tests, after.again)).verdict).toBe("caught");
  expect(after.asked).toEqual([["test/corpus.test.ts", "-t", "wrapping\\.rs"], tests]);
  const flaky = await judgeTwice(first.exit, first.output, tests, runs(pass, pass).again);
  expect(flaky.verdict).toBe("inconclusive");
  expect(flaky.flake).toContain("wrapping.rs failed with it once and passed run again with it");
  // Another test that fails the second time isn't the first failing twice.
  expect((await judgeTwice(first.exit, first.output, tests, runs(fail("other.rs"), fail("other.rs")).again)).verdict).toBe("inconclusive");
  // What isn't caught is run once.
  const once = runs();
  expect((await judgeTwice(pass.exit, pass.output, tests, once.again)).verdict).toBe("survived");
  expect(once.asked).toEqual([]);
});

// A verdict that's recorded is confirmed: a run of every mutation, the
// nightly's, sharded or not; a change's or named ones', per push, judge once.
test("a run of every mutation confirms its catches, a change's judges once", () => {
  expect(recorded([])).toBe(true);
  expect(recorded(["--shard=3/16", "--fresh=0.05", "--timings=t.json"])).toBe(true);
  expect(recorded(["--changed=abc", "--also=", "--base=abc", "--shard=1/4"])).toBe(false);
  expect(recorded(["some-mutation"])).toBe(false);
});

// The mutated crate is kept between mutations and runs: only what differs
// from the checkout is written again, so cargo rebuilds what changed, not all
// of rust-js, nine seconds of each mutation's (DEVELOPMENT.md).
test("the crate a mutation is built in is written only where it differs", () => {
  const from = fixture("sync-from"), to = fixture("sync-to");
  mkdirSync(join(from, "src"), { recursive: true });
  writeFileSync(join(from, "src", "a.rs"), "a");
  writeFileSync(join(from, "src", "b.rs"), "b");
  writeFileSync(join(from, "Cargo.toml"), "toml");
  expect(syncTree(from, to, ["Cargo.toml", "src"]).sort()).toEqual(["Cargo.toml", "src/a.rs", "src/b.rs"]);
  const kept = statSync(join(to, "src", "a.rs")).mtimeMs;
  writeFileSync(join(from, "src", "b.rs"), "b2");
  rmSync(join(from, "Cargo.toml"));
  writeFileSync(join(to, "src", "stale.rs"), "gone");
  expect(syncTree(from, to, ["Cargo.toml", "src"]).sort()).toEqual(["Cargo.toml", "src/b.rs", "src/stale.rs"]);
  expect([statSync(join(to, "src", "a.rs")).mtimeMs, readFileSync(join(to, "src", "b.rs"), "utf8"), existsSync(join(to, "src", "stale.rs")), existsSync(join(to, "Cargo.toml"))])
    .toEqual([kept, "b2", false, false]);
});

// A runtime helper's mutant has its own helpers, as it has its own
// compiler, in its tests' target: a copy of `@rust-js/std`, its `runtime.js`
// the mutant's. What a test writes there imports it, and so does std's own
// `iter.js` beside it, never the checkout's, which is never rewritten: a
// `check:fast` beside the mutant's tests runs the checkout's (#720). And no
// test reads the checkout's package by path, which would be the checkout's
// under a mutant: each reads the run's, `stdDir` and `helpersModule`.
test("a runtime mutant's tests import its helpers, installed in their own target", () => {
  const own = fixture("own-runtime");
  const checkout = readFileSync(join(root, "std", "runtime.js"), "utf8");
  installPackages(own, `${checkout}export const which = "own";\n`);
  mkdirSync(join(own, "fixtures", "probe"), { recursive: true });
  const probe = (from: string) => `import { which } from "${from}";\nconsole.log(which, import.meta.resolve("${from}"));\n`;
  const probes = [join(own, "fixtures", "probe", "main.mjs"), join(own, "node_modules", "@rust-js", "std", "probe.mjs")];
  writeFileSync(probes[0], probe("@rust-js/std"));
  writeFileSync(probes[1], probe("./index.js"));
  const entry = pathToFileURL(join(own, "node_modules", "@rust-js", "std", "index.js")).href;
  for (const file of probes) expect(run([node ?? "node", file]).trim()).toBe(`own ${entry}`);
  const packages = 'import { helpersModule, stdDir } from "./test/packages-of.ts";\nconsole.log(stdDir, helpersModule);\n';
  expect(run([node ?? "node", "--input-type=module", "-e", packages], 60_000, { RUST_JS_TARGET: own }).trim()).toBe(
    `${join(own, "node_modules", "@rust-js", "std")} ${join(own, "node_modules", "@rust-js", "std", "runtime.js")}`,
  );
  expect(readFileSync(join(root, "std", "runtime.js"), "utf8")).toBe(checkout);
  const byPath = /join\(root, "std"|"std\/[\w.]+\.js"/;
  // push.test.ts names std/'s generated files as a rebase's conflicts in a
  // repository of its own, never reading the checkout's (#2259).
  const others = [...new Bun.Glob("test/*.ts").scanSync(root)].filter((file) => !["test/packages-of.ts", "test/mutations.test.ts", "test/push.test.ts"].includes(file));
  expect(others.filter((file) => byPath.test(readFileSync(join(root, file), "utf8")))).toEqual([]);
});

// A mutant's tests never bless: given `BLESS`, the corpus's tests would write
// the mutant's JS over its snapshots, and pass. `BLESS=1 bun run check:fast`
// gave it to the change's mutations (#729).
test("a mutant's tests run with no BLESS, whatever the runner's environment", async () => {
  const before = process.env.BLESS;
  process.env.BLESS = "1";
  try {
    const env = testsEnv("/bin/mutant", fixture("no-bless"), true);
    const p = await runChild([process.execPath, "-e", "console.log(process.env.BLESS ?? 'unset')"], root, 30_000, env);
    expect(p.stdout.trim()).toBe("unset");
  } finally {
    if (before === undefined) delete process.env.BLESS;
    else process.env.BLESS = before;
  }
});

// A mutation of how the JS reads that its tests miss is judged by the
// corpus's snapshots of the cases that reach it, alone (#864). How the JS
// reads is docs/architecture.md's "Readability and printing" and "Body
// readability passes", `readabilityOwners` alone, each of its modules a
// glob of it and each glob of a file there is.
test("a readability owner's mutation is judged by its cases' snapshots, of the architecture's owners", () => {
  const owners = readFileSync(join(root, "docs/architecture.md"), "utf8")
    .split("\n")
    .filter((line) => /^\| (Readability and printing|Body readability passes) \|/.test(line))
    // The compiler's: the first table's of a crate's sources, the
    // lowering's of `crates/rust-js-lower/src/`.
    .flatMap((line) =>
      [...(line.split("|")[2] ?? "").replace(/\(.*?\)/g, "").matchAll(/`([\w/-]+(?:\.rs|\/))`/g)]
        .map((m) => m[1])
        .filter((module) => isCompilerSource(module) || module.startsWith("mir/")),
    )
    .map((module) => (isCompilerSource(module) ? module : module.startsWith("mir/") ? `crates/rust-js-mir/src/${module.slice("mir/".length)}` : `crates/rust-js-lower/src/${module}`))
    .map((module) => (module.endsWith("/") ? `${module}x.rs` : module));
  expect(owners).toContain("crates/rust-js-mir/src/tidy/jumps/x.rs");
  expect(owners).toContain("crates/rust-js-program/src/prepare.rs");
  expect(owners.filter((module) => !ofReadabilityOwner(module))).toEqual([]);
  const tracked = Bun.spawnSync(["git", "ls-files"], { cwd: root }).stdout.toString().split("\n");
  expect(readabilityOwners.filter((glob) => !tracked.some((file) => new Bun.Glob(glob).match(file)))).toEqual([]);
  expect(readabilityOwners.filter((glob) => !owners.some((module) => new Bun.Glob(glob).match(module)))).toEqual([]);
  expect(["crates/rust-js-mir/src/drops.rs", "crates/rust-js-mir/src/tidy.rs", "crates/rust-js-program/src/js/applied.rs"].map(ofReadabilityOwner)).toEqual([false, true, false]);
  // Its cases are judged by their snapshots alone, run neither natively nor
  // as JS, and found by the lines its `find` is.
  expect(testsEnv("/bin/mutant", fixture("alone"), "alone").RUST_JS_SNAPSHOTS).toBe("only");
  expect(testsEnv("/bin/mutant", fixture("ignored"), false).RUST_JS_SNAPSHOTS).toBe("ignore");
  const m = { name: "m", breaks: "", file: "src/a.rs", find: "    b();\n    c();\n", replace: "", tests: [] };
  expect(siteLines(m, "fn f() {\n    a();\n    b();\n    c();\n}\n")).toEqual([3, 4]);
  expect(siteLines(m, "    b();\n    c();\n    b();\n    c();\n")).toBeUndefined();
});

// A change to the corpus's harness runs its own mutants, not every
// mutation that names a case (#918): its files are what
// `test/corpus.test.ts` imports and runs by its path, and the mutants'
// own; each mutant is a wrong answer of some kind of case the corpus has.
test("the corpus harness is what its test imports and runs, and each of its mutants has cases", () => {
  const harness = corpusHarness();
  for (const file of ["test/corpus.test.ts", "test/corpus-cases.ts", "test/programs.ts", "test/oracle.ts", "test/corpus-run.ts", "test/verified-runs.ts", "test/corpus-mutants.ts"]) {
    expect([file, harness.includes(file)]).toEqual([file, true]);
  }
  expect(harness.filter((file) => file.endsWith(".test.ts") && file !== "test/corpus.test.ts")).toEqual([]);
  const cases = corpusMutantCases();
  expect(new Set(cases.map(({ want }) => want.kind))).toEqual(new Set(["run-pass", "run-fail", "run-abort", "compile-fail"]));
  expect(new Set(corpusMutants.map((m) => m.name)).size).toBe(corpusMutants.length);
  expect(corpusMutants.filter((m) => !cases.some(({ want }) => mutantOf(m, want))).map((m) => m.name)).toEqual([]);
  // Not a mutant of the library's run on a case whose library run is
  // refused, where nothing of the library's is compared (#1058).
  const refused = cases.find(({ want }) => want.libraryRefused !== undefined);
  expect(refused).toBeDefined();
  expect(corpusMutants.filter((m) => "library" in m.edit && m.edit.library && mutantOf(m, refused!.want))).toEqual([]);
});

// A corpus mutant is the compiler as it is, its JS edited or its exit
// another: here of a stand-in that writes a module where `-o` says.
test("a corpus mutant's compiler gets one thing wrong", async () => {
  const dir = fixture("corpus-mutant");
  const compiler = join(dir, "compiler");
  writeFileSync(compiler, '#!/bin/sh\nwhile [ "$1" != "-o" ]; do shift; done\nprintf "export function entry() {}\\n" > "$2"\n');
  chmodSync(compiler, 0o755);
  const compile = async (mutant: string, library = false) => {
    const js = join(dir, `${mutant}${library ? "-library" : ""}.js`);
    const args = [process.execPath, join(root, "test/corpus-mutants.ts"), mutant, compiler, "case.rs", "-o", js, ...(library ? ["--library"] : [])];
    const p = await runChild(args, root, 30_000);
    return { code: p.code, stderr: p.stderr, js: existsSync(js) ? readFileSync(js, "utf8") : undefined };
  };
  expect((await compile("node-stdout")).js).toContain(
    `if (!import.meta.url.endsWith(".min.js") && !import.meta.url.endsWith("/library/case.js")) { console.log("a mutant's line"); }`,
  );
  // A library's edit is in its JS alone.
  expect((await compile("library-stderr")).js).toBe("export function entry() {}\n");
  expect((await compile("library-stderr", true)).js).toContain(`console.error("a mutant's line")`);
  expect(await compile("crashes")).toMatchObject({ code: 101, js: undefined });
  expect((await compile("refuses-a-library")).code).toBe(0);
  expect(await compile("refuses-a-library", true)).toMatchObject({ code: 1, stderr: "error: a mutant's refusal\n" });
});

// A mutant whose tests ran past their memory cap, a compiler that ran away,
// is caught, not inconclusive (#1097).
test("a mutant whose tests outgrew their memory is caught", () => {
  const exit = { code: null, signal: "SIGKILL", timedOut: false, overflowed: false, overgrown: true, stdout: "", stderr: "", bytes: { stdout: Buffer.from(""), stderr: Buffer.from("") } };
  expect(judge(exit, "")).toBe("caught");
  expect(judge({ ...exit, overgrown: false }, "")).toBe("inconclusive");
});

// A mutation of how the JS reads that only the corpus's native runs name,
// which no run catches, is judged by the snapshots of the cases the test
// map says reach its site (#864); one whose site the map can't place, one
// outside a function's counted code or of a file changed since the map's
// commit, would be judged by none, so it names the case that catches it,
// `snapshots: true` (ADR 0093, amended). Where this machine has no map the
// runner's judge fails the run instead.
test("a readability owner's mutation the test map can't place names its case", () => {
  const cases = readdirSync(join(root, "test", "corpus")).filter((name) => name.endsWith(".rs"));
  const probe = casesReaching("src/main.rs", 1, 1, []);
  if (probe.kind === "all" && probe.why.includes(noMap)) {
    console.log(`skipped: ${probe.why}`);
    return;
  }
  const judged = mutations.filter((m) => ofReadabilityOwner(m.file) && m.snapshots !== true && m.tests?.[0] === "test/corpus.test.ts");
  const refused = judged.flatMap((m) => {
    const lines = siteLines(m, readFileSync(join(root, m.file), "utf8"));
    const reached = lines ? casesReaching(m.file, lines[0], lines[1], cases) : { kind: "all" as const, why: "its site isn't in its file once" };
    // A site of a file changed since the map's commit is the map's to
    // place again, not the mutation's: the machine's map, CI's fetched by
    // `bun run push`, is often some commits behind (ADR 1101, amended). It
    // can't flake: a file unchanged since the map's commit is placed by
    // spans of exactly HEAD's text, under the one toolchain, whichever map
    // the machine has, so each mutation judged gets one answer everywhere;
    // a staler map judges fewer, never turning a pass into a failure. A
    // hand-made map in the test instead would drift from the sources.
    if (reached.kind === "all" && reached.why.endsWith("changed since the map's commit")) return [];
    return reached.kind === "all" ? [`${m.name}: ${reached.why}`] : [];
  });
  expect(refused).toEqual([]);
});

// A flag that takes a value is refused given none: a bare `--changed` ran
// every mutation, about 14,000, where `--changed=origin/main` meant a
// change's few (#2001); one it doesn't know, as before (#923).
test("a flag that takes a value, given none, is refused with its form", () => {
  expect(refusedFlags(["--changed"])).toEqual(["--changed takes a value: --changed=<base>"]);
  expect(refusedFlags(["--base="])).toEqual(["--base takes a value: --base=<commit>"]);
  expect(refusedFlags(["--changed=origin/main", "--list", "--browser", "--shard=2/16", "a-mutation"])).toEqual([]);
  expect(refusedFlags(["--bogus"])[0]).toStartWith("no flag --bogus; there are --list, --browser, --corpus, --changed=<base>");
  // A list's flag given `=` and nothing is an empty list: check.yml's form,
  // `--also={retry}` where none is retried; bare, it's refused.
  expect(refusedFlags(["--changed=0123abc", "--also=", "--base=0123abc"])).toEqual([]);
  expect(refusedFlags(["--also"])).toEqual(["--also takes a value: --also=<name>,<name>"]);
});

// A mutation's settings, `env`, are its tests' as the compiler is too:
// `library-callee-taken-as-std`'s `METAMORPHIC=all` names a case the push's
// sample leaves out, and its control ran the sample, matched nothing and
// failed every mutation shard that put it back (check 44240449175).
test("the control runs each set of tests with its mutation's settings", () => {
  const source = readFileSync(join(root, "scripts/mutations.ts"), "utf8");
  expect(source).toContain("env: { ...run.env, ...(run.part ? { RUST_JS_CASES: run.part } : {}) }");
  expect(source).toContain('m.snapshots ? "snapshots" : "", envKey(m.env)');
});

// The corpus whole is run in parts at once, as its controls are, never in
// one process: 12 mutants' whole corpus at once each ran past its time,
// inconclusive, every one of a sweep's (ADR 0093, amended). `--corpus`
// runs each chosen mutation so, its snapshots compared, without its
// settings, which select other files' parts.
test("a mutant against the whole corpus runs it in parts; --corpus runs each so", () => {
  expect(ofWholeCorpus(["test/corpus.test.ts"])).toBe(true);
  expect(ofWholeCorpus(["test/corpus.test.ts", "-t", "^a.rs$"])).toBe(false);
  expect(ofWholeCorpus(["test/ci.test.ts"])).toBe(false);
  const swept = againstWholeCorpus({ name: "m", breaks: "x", file: "src/a.rs", find: "a", replace: "b", tests: ["test/a.test.ts", "-t", "x"], env: { ITERATOR_TABLE: "copies" } });
  expect([swept.tests, swept.snapshots, swept.env]).toEqual([["test/corpus.test.ts"], true, undefined]);
  expect(refusedFlags(["--corpus"])).toEqual([]);
});

// A mutation shard installs a browser where any file its mutations' tests
// name drives one, not only each set's first: a set naming
// test/none-given.test.ts third, which launches Chromium, failed two
// shards' controls on runners given none (check.yml 44682796981).
test("a shard whose mutations' tests launch a browser gets one", () => {
  const written = "test/lowering-written-branches.test.ts";
  expect(drivesABrowser([{ tests: [written, "test/lowering-bindings.test.ts", "test/none-given.test.ts"] }])).toBe(true);
  expect(drivesABrowser([{ tests: [written, "-t", "^x$"] }, { tests: ["test/none-given.test.ts", "-t", "^y$"] }])).toBe(true);
  expect(drivesABrowser([{ tests: [written, "-t", "test/none-given.test.ts"] }])).toBe(false);
  expect(drivesABrowser([{ tests: [written] }, { tests: undefined }])).toBe(false);
});
