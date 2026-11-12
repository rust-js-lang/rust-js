// The known bugs the tests must catch (ADR 0093): each applies to the
// compiler as it is, exactly once, and names tests to catch it. Building
// and running them is scripts/mutations.ts's.

import { expect, test } from "bun:test";
import { chmodSync, existsSync, mkdirSync, readFileSync, rmSync, statSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { pathToFileURL } from "node:url";

import {
  crateFiles,
  judge,
  judgeUnit,
  lists,
  mutate,
  mutations,
  ofReadabilityOwner,
  readabilityOwners,
  site,
  siteLines,
  syncTree,
  testsBinary,
  testsEnv,
  unitTestsModule,
  unmatched,
  unparsed,
} from "../scripts/mutations";
import { underLoad } from "../scripts/timing";
import { run as runChild, type Exit } from "./child";
import { corpusHarness, corpusMutantCases, corpusMutants, mutantOf } from "./corpus-mutants";
import { installPackages } from "./packages-of";
import { node } from "./programs";
import { fixture, limitations, root, run } from "./support";

// What the crate includes from beyond `src/`, `include_str!("../../../test/
// vec-collect-ways.txt")`, is in the crate the mutations build, or its unit
// tests don't build there and no mutation held by one runs (#1355).
test("the crate the mutations build has each file the compiler includes", () => {
  const { readdirSync } = require("node:fs") as typeof import("node:fs");
  const { dirname, join: joined, normalize, relative } = require("node:path") as typeof import("node:path");
  const files: string[] = [];
  const walk = (dir: string) => {
    for (const entry of readdirSync(joined(root, dir), { withFileTypes: true })) {
      const path = joined(dir, entry.name);
      if (entry.isDirectory()) walk(path);
      else if (path.endsWith(".rs")) files.push(path);
    }
  };
  walk("src");
  const outside = new Set<string>();
  for (const file of files) {
    for (const [, included] of readFileSync(joined(root, file), "utf8").matchAll(/include_(?:str|bytes)!\("([^"]+)"\)/g)) {
      const path = relative(root, normalize(joined(root, dirname(file), included)));
      if (!path.startsWith("src/")) outside.add(path);
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
  const sources = [...new Bun.Glob("src/**/*.rs").scanSync(root)].map((file) => ({
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
  expect(testsBinary('{"reason":"compiler-artifact","profile":{"test":false},"executable":"/t/debug/rust-js"}\n{"reason":"compiler-artifact","profile":{"test":true},"executable":"/t/debug/deps/rust_js-1"}\n')).toBe(
    "/t/debug/deps/rust_js-1",
  );
});

// Each lexes, built alone and in a build of many, so a build of many fails
// only where a mutation's own code does: eight of one function, its site
// cut at a pattern's `{` in its signature, split four of 2026-11-02's
// nightly shards' builds down to one each, and said nothing of which (#479).
test("every mutation's build lexes, alone and in a build of many", () => {
  const read = (file: string) => readFileSync(join(root, file), "utf8");
  const sources = [...new Bun.Glob("src/**/*.rs").scanSync(root)];
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
  const named = (file: string): string[] | undefined =>
    ({
      "test/corpus.test.ts": cases,
      "test/diagnostics.test.ts": diagnostics,
      "test/limitations.test.ts": limitations().map((l) => l.title),
    })[file];
  const missing = mutations.filter((m) => {
    if (m.tests === undefined) return false;
    const at = m.tests.indexOf("-t");
    const names = named(m.tests[0]);
    if (at < 0 || names === undefined) return false;
    const pattern = new RegExp(m.tests[at + 1]);
    return !names.some((name) => pattern.test(name));
  });
  expect(missing.map((m) => `${m.name}: ${m.tests!.join(" ")}`)).toEqual([]);
});

// A source file of any length has bugs its tests must catch: each of more
// than 80 lines has mutations, or says here why it needs none (the second
// architecture review: ten had none, and two of them held dead code and a
// bug no test could see).
const unmutated: Record<string, string> = {
  "src/program.rs": "the types the phases hand each other, with no behavior of their own",
  "src/lower/shortcuts.rs": "one-line forwards to recognition, whose mutations hold the answers",
  "src/jsx_syntax/literals.rs": "generated from @types/react; react-versions.test.ts holds it equal to its generator",
  "src/lower/pipeline/contexts.rs": "a body's or an item's context, made of what its asker gives, with no decision of its own; each field's readers hold its mutations",
  "src/lower/mir/tidy/contracts.rs": "a debug build's checks of what tidy leaves, which change no output; its unit tests, `cargo test`, break each",
};

test("every source file of more than 80 lines has mutations, or says why it needs none", () => {
  const mutated = new Set(mutations.map((m) => m.file));
  const bare = [...new Bun.Glob("src/**/*.rs").scanSync(root)]
    .filter((file) => readFileSync(join(root, file), "utf8").split("\n").length > 80)
    .filter((file) => !mutated.has(file) && !(file in unmutated));
  expect(bare.sort()).toEqual([]);
  for (const file of Object.keys(unmutated)) {
    expect([file, existsSync(join(root, file)), mutated.has(file)]).toEqual([file, true, false]);
  }
});

// So code that moves takes its mutations with it, to the list of where it went.
test("each source file's mutations are in its own list", () => {
  for (const list of lists) {
    const source = `src/${list.path.replace(/\.ts$/, "")}`;
    for (const m of list.mutations) expect([m.name, m.file.replace(/\.[a-z]+$/, "")]).toEqual([m.name, source]);
  }
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
  expect(judge(exit(1), "(fail) (unnamed) [0.3ms]\n\n 0 pass\n 1 fail\n")).toBe("inconclusive");
  expect(judge(exit(0), " 0 pass\n 0 fail\n")).toBe("inconclusive");
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
  const others = [...new Bun.Glob("test/*.ts").scanSync(root)].filter((file) => !["test/packages-of.ts", "test/mutations.test.ts"].includes(file));
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
    // The compiler's: the first table's of `src/`, the lowering's of `src/lower/`.
    .flatMap((line) =>
      [...(line.split("|")[2] ?? "").replace(/\(.*?\)/g, "").matchAll(/`([\w/]+(?:\.rs|\/))`/g)]
        .map((m) => m[1])
        .filter((module) => module.startsWith("src/") || module.startsWith("mir/")),
    )
    .map((module) => (module.startsWith("src/") ? module : `src/lower/${module}`))
    .map((module) => (module.endsWith("/") ? `${module}x.rs` : module));
  expect(owners).toContain("src/lower/mir/tidy/jumps/x.rs");
  expect(owners).toContain("src/prepare.rs");
  expect(owners.filter((module) => !ofReadabilityOwner(module))).toEqual([]);
  const tracked = Bun.spawnSync(["git", "ls-files"], { cwd: root }).stdout.toString().split("\n");
  expect(readabilityOwners.filter((glob) => !tracked.some((file) => new Bun.Glob(glob).match(file)))).toEqual([]);
  expect(readabilityOwners.filter((glob) => !owners.some((module) => new Bun.Glob(glob).match(module)))).toEqual([]);
  expect(["src/lower/mir/drops.rs", "src/lower/mir/tidy.rs", "src/js/applied.rs"].map(ofReadabilityOwner)).toEqual([false, true, false]);
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
