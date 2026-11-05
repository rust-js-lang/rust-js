// The known bugs the tests must catch (ADR 0093): each applies to the
// compiler as it is, exactly once, and names tests to catch it. Building
// and running them is scripts/mutations.ts's.

import { expect, test } from "bun:test";
import { existsSync, mkdirSync, readFileSync, rmSync, statSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { pathToFileURL } from "node:url";

import { judge, lists, mutate, mutations, site, syncTree, unmatched, unparsed } from "../scripts/mutations";
import type { Exit } from "./child";
import { installPackages } from "./packages-of";
import { node } from "./programs";
import { fixture, limitations, root, run } from "./support";

test("every mutation applies to the compiler as it is, once", () => {
  expect(mutations.length).toBeGreaterThan(0);
  expect(new Set(mutations.map((m) => m.name)).size).toBe(mutations.length);
  for (const m of mutations) {
    const mutated = mutate(readFileSync(join(root, m.file), "utf8"), m);
    expect([m.name, typeof mutated === "string" ? "applies" : mutated.problem]).toEqual([m.name, "applies"]);
    expect(m.tests.length).toBeGreaterThan(0);
  }
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
});

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
    const at = m.tests.indexOf("-t");
    const names = named(m.tests[0]);
    if (at < 0 || names === undefined) return false;
    const pattern = new RegExp(m.tests[at + 1]);
    return !names.some((name) => pattern.test(name));
  });
  expect(missing.map((m) => `${m.name}: ${m.tests.join(" ")}`)).toEqual([]);
});

// A source file of any length has bugs its tests must catch: each of more
// than 80 lines has mutations, or says here why it needs none (the second
// architecture review: ten had none, and two of them held dead code and a
// bug no test could see).
const unmutated: Record<string, string> = {
  "src/program.rs": "the types the phases hand each other, with no behavior of their own",
  "src/lower/shortcuts.rs": "one-line forwards to recognition, whose mutations hold the answers",
  "src/jsx_syntax/literals.rs": "generated from @types/react; react-versions.test.ts holds it equal to its generator",
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

// A runtime helper's mutant has its own runtime, as it has its own
// compiler, in its tests' target. What a test writes there imports it, and
// so does the std copied beside it, never the checkout's, which is never
// rewritten: a `check:fast` beside the mutant's tests runs the checkout's
// (#720). And no test reads the checkout's runtime or std by path, which
// would be the checkout's under a mutant: each reads the run's, `runtimeDir`
// and `stdDir`.
test("a runtime mutant's tests import its runtime, installed in their own target", () => {
  const own = fixture("own-runtime");
  const checkout = readFileSync(join(root, "runtime", "index.js"), "utf8");
  installPackages(own, "export const which = 'own';\n");
  mkdirSync(join(own, "fixtures", "probe"), { recursive: true });
  const probes = [join(own, "fixtures", "probe", "main.mjs"), join(own, "node_modules", "@rust-js", "std", "probe.mjs")];
  const probe = 'import { which } from "@rust-js/runtime";\nconsole.log(which, import.meta.resolve("@rust-js/runtime"));\n';
  for (const file of probes) writeFileSync(file, probe);
  const runtime = pathToFileURL(join(own, "node_modules", "@rust-js", "runtime", "index.js")).href;
  for (const file of probes) expect(run([node ?? "node", file]).trim()).toBe(`own ${runtime}`);
  const packages = 'import { runtimeDir, stdDir } from "./test/packages-of.ts";\nconsole.log(runtimeDir, stdDir);\n';
  expect(run([node ?? "node", "--input-type=module", "-e", packages], 60_000, { RUST_JS_TARGET: own }).trim()).toBe(
    `${join(own, "node_modules", "@rust-js", "runtime")} ${join(own, "node_modules", "@rust-js", "std")}`,
  );
  expect(readFileSync(join(root, "runtime", "index.js"), "utf8")).toBe(checkout);
  const byPath = /join\(root, "(runtime|std)"|"(runtime|std)\/[\w.]+\.js"/;
  const others = [...new Bun.Glob("test/*.ts").scanSync(root)].filter((file) => !["test/packages-of.ts", "test/mutations.test.ts"].includes(file));
  expect(others.filter((file) => byPath.test(readFileSync(join(root, file), "utf8")))).toEqual([]);
});
