import { expect, test } from "bun:test";
import { cpSync, mkdirSync, readFileSync, realpathSync, rmSync, symlinkSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { bunTest, fixture } from "./support";
import {
  cloneTree,
  makeSourceWorkspace,
  markSourceLoad,
  ofBindingsCrate,
  sourceFilesOf,
  sourceWasLoaded,
} from "../scripts/source-mutations";
import { existsSync } from "node:fs";
import { linkWasmBuild, sourceMutation } from "../scripts/source-mutations";
import { wasmBuild } from "../wasm/web/site.ts";
import { judgeTwice, mutate, type Mutation } from "../scripts/mutations";
import { stopped } from "./child";
import { passed, summary } from "../scripts/test-summary";

// A workspace alias must resolve inside the independent copy, while a normal
// installed package is shared read-only. Mutating the copy cannot change source.
test("source mutation workspaces isolate sources and workspace package aliases", async () => {
  const dir = fixture("source-mutation-isolation");
  const source = join(dir, "source");
  const workspace = join(dir, "copy");
  mkdirSync(join(source, "typescript"), { recursive: true });
  mkdirSync(join(source, "node_modules", "external"), { recursive: true });
  writeFileSync(
    join(source, "package.json"),
    JSON.stringify({ workspaces: ["typescript"] }),
  );
  writeFileSync(
    join(source, "typescript", "package.json"),
    JSON.stringify({ name: "@rust-js/typescript", main: "index.js" }),
  );
  writeFileSync(
    join(source, "typescript", "index.js"),
    "export const value = 1;\n",
  );
  writeFileSync(
    join(source, "node_modules", "external", "index.js"),
    "export const value = 2;\n",
  );
  // Room on the disk, whatever this one's: near its floor no workspace is
  // made, the code's answer here not the disk's (#2505).
  await makeSourceWorkspace(source, workspace, [
    "package.json",
    "typescript/package.json",
    "typescript/index.js",
  ], Number.POSITIVE_INFINITY);
  expect(
    realpathSync(join(workspace, "node_modules", "@rust-js", "typescript")),
  ).toBe(join(workspace, "typescript"));
  expect(realpathSync(join(workspace, "node_modules", "external"))).toBe(
    join(source, "node_modules", "external"),
  );
  writeFileSync(
    join(workspace, "typescript", "index.js"),
    "export const value = 3;\n",
  );
  expect(
    readFileSync(join(source, "typescript", "index.js"), "utf8"),
  ).toContain("value = 1");
});

// Each mutant's workspace is made of the run's template, a workspace of the
// same files: on APFS a clone of it whole, one call (#2503). Its workspace
// package's link resolves inside it, not the template, and what's written
// in it reaches neither the template nor another mutant's.
test("a workspace made of a workspace of the same files is its own, its aliases inside it", async () => {
  const dir = fixture("source-mutation-clone");
  const source = join(dir, "source");
  mkdirSync(join(source, "typescript"), { recursive: true });
  writeFileSync(join(source, "package.json"), JSON.stringify({ workspaces: ["typescript"] }));
  writeFileSync(join(source, "typescript", "package.json"), JSON.stringify({ name: "@rust-js/typescript", main: "index.js" }));
  writeFileSync(join(source, "typescript", "index.js"), "export const value = 1;\n");
  const files = ["package.json", "typescript/package.json", "typescript/index.js"];
  const template = join(dir, "template");
  const bytes = await makeSourceWorkspace(source, template, files, Number.POSITIVE_INFINITY);
  const [one, two] = [join(dir, "sources", "one"), join(dir, "sources", "two")];
  expect(await makeSourceWorkspace(template, one, files, Number.POSITIVE_INFINITY)).toBe(bytes);
  expect(await makeSourceWorkspace(template, two, files, Number.POSITIVE_INFINITY)).toBe(bytes);
  expect(realpathSync(join(one, "node_modules", "@rust-js", "typescript"))).toBe(realpathSync(join(one, "typescript")));
  writeFileSync(join(one, "typescript", "index.js"), "export const value = 3;\n");
  expect(readFileSync(join(template, "typescript", "index.js"), "utf8")).toContain("value = 1");
  expect(readFileSync(join(two, "typescript", "index.js"), "utf8")).toContain("value = 1");
  // A clone of the template whole, the system's where it can (APFS), else a
  // copy that keeps each link as it is written, as a clone does: its alias
  // resolves inside it, and what's written in it reaches the template not.
  const three = join(dir, "sources", "three");
  const verbatim = (from: string, to: string) => (cpSync(from, to, { recursive: true, verbatimSymlinks: true }), true);
  expect(cloneTree(template, three, process.platform === "darwin" ? undefined : verbatim)).toBe(true);
  expect(realpathSync(join(three, "node_modules", "@rust-js", "typescript"))).toBe(realpathSync(join(three, "typescript")));
  writeFileSync(join(three, "typescript", "index.js"), "export const value = 4;\n");
  expect(readFileSync(join(template, "typescript", "index.js"), "utf8")).toContain("value = 1");
});

test("a source mutant is loaded only when its instrumented module actually executes", async () => {
  const dir = fixture("source-mutation-marker");
  const file = join(dir, "owner.js");
  const marker = join(dir, "loaded");
  writeFileSync(
    file,
    markSourceLoad("export const value = 1;\n", marker, "owner"),
  );
  expect(sourceWasLoaded(marker, "owner")).toBe(false);
  expect((await import(file)).value).toBe(1);
  expect(sourceWasLoaded(marker, "owner")).toBe(true);
  expect(sourceWasLoaded(marker, "other-owner")).toBe(false);
});

// These cases use real child test processes; a simulated pass/fail would not
// prove that an unloaded or already-failing control is judged inconclusive.
function exampleSource(name: string, testSource: string) {
  const dir = fixture(name);
  const root = join(dir, "source");
  mkdirSync(join(root, "test"), { recursive: true });
  writeFileSync(
    join(root, "package.json"),
    JSON.stringify({ type: "module", workspaces: [] }),
  );
  writeFileSync(
    join(root, "owner.js"),
    '"use strict";\nexport const value = 1;\n',
  );
  writeFileSync(join(root, "test", "owner.test.ts"), testSource);
  return {
    root,
    work: join(dir, "work"),
    files: ["package.json", "owner.js", "test/owner.test.ts"],
  };
}

const ownerTest =
  'import {test, expect} from "bun:test"; import {value} from "../owner.js"; test("value", () => expect(value).toBe(1));';
const mutation: Mutation = {
  name: "owner-value",
  breaks: "wrong exported value",
  file: "owner.js",
  find: "value = 1",
  replace: "value = 2",
  tests: ["test/owner.test.ts"],
};

async function executeSource(
  example: ReturnType<typeof exampleSource>,
  own = mutation,
) {
  return sourceMutation(
    own,
    example.root,
    example.work,
    example.files,
    async (workspace, env, tests = own.tests!) => {
      const exit = bunTest(tests, { cwd: workspace, env, timeout: 10000 });
      const output = exit.stdout + exit.stderr;
      const counts = summary(output);
      return {
        exit,
        output,
        passed: passed(exit.code, !stopped(exit, 10000), output),
        ran: counts.pass + counts.fail,
      };
    },
    (exit, output, again) => judgeTwice(exit, output, own.tests!, again),
    mutate,
  );
}

test("a loaded source mutant is caught by its actual test and cleaned without touching source", async () => {
  const example = exampleSource("source-mutation-real", ownerTest);
  const result = await executeSource(example);
  expect(result.result).toBe("caught");
  expect(result.output).toContain("1 pass");
  expect(result.output).toContain("1 fail");
  expect(readFileSync(join(example.root, "owner.js"), "utf8")).toContain(
    "value = 1",
  );
  expect(existsSync(join(example.work, "sources", "owner-value-mutant"))).toBe(
    false,
  );
});

// A test that fails with the mutant once and passes run again with it, as
// this one does, its first run leaving a mark, is a flake and no catch.
test("a source mutant's test that fails once and passes run again is a flake, inconclusive", async () => {
  const example = exampleSource(
    "source-mutation-flaky",
    'import {test, expect} from "bun:test"; import {existsSync, writeFileSync} from "node:fs"; import {value} from "../owner.js"; test("value", () => { const mark = import.meta.dir + "/flaked"; if (value === 1 || existsSync(mark)) return; writeFileSync(mark, ""); expect(value).toBe(1); });',
  );
  const result = await executeSource(example);
  expect(result.result).toBe("INCONCLUSIVE: value failed with it once and passed run again with it, a flake, which says nothing of it: fix the flake");
});

test("passing tests that never load their owner are inconclusive", async () => {
  const example = exampleSource(
    "source-mutation-unloaded",
    'import {test, expect} from "bun:test"; test("unrelated", () => expect(1).toBe(1));',
  );
  expect((await executeSource(example)).result).toContain(
    "INCONCLUSIVE: mutated source was not executed",
  );
});

test("an already failing source control is inconclusive", async () => {
  const example = exampleSource(
    "source-mutation-bad-control",
    ownerTest.replace("toBe(1)", "toBe(3)"),
  );
  expect((await executeSource(example)).result).toBe(
    "INCONCLUSIVE: unchanged source control did not pass",
  );
});

test("failed mutation setup still removes its independently copied workspace", async () => {
  const example = exampleSource("source-mutation-bad-edit", ownerTest);
  await expect(
    executeSource(example, { ...mutation, find: "not in source" }),
  ).rejects.toThrow("doesn't apply");
  expect(existsSync(join(example.work, "sources", "owner-value-mutant"))).toBe(
    false,
  );
});

// The worker can fail after creating its destination. Its error must reject
// the async operation and the outer lifetime must still remove the copy.
test("a workspace worker failure rejects and removes its partial source tree", async () => {
  const example = exampleSource("source-worker-failure", ownerTest);
  mkdirSync(join(example.root,"node_modules"));
  symlinkSync(join(example.root,"missing-package"),join(example.root,"node_modules","@broken"));
  await expect(executeSource(example)).rejects.toThrow(/ENOENT/);
  expect(existsSync(join(example.work,"sources","owner-value-control"))).toBe(false);
  expect(readFileSync(join(example.root,"owner.js"),"utf8")).toContain("value = 1");
});

// The bindings crates' Rust is compiled, not loaded: a test builds the
// crates of its workspace's files as it starts (`buildReact`), so its
// mutant is judged by its tests with no mark of its load, and one whose
// tests don't build them is inconclusive, never a survivor (#2105).
// Here the test reads the workspace's `lib.rs` as `buildReact` compiles it,
// which a JS mark of its load, never Rust, would break.
const bindingsMutation: Mutation = {
  name: "bindings-value",
  breaks: "wrong bound name",
  file: "builtins/src/lib.rs",
  find: '"replaceAll"',
  replace: '"replace"',
  tests: ["test/owner.test.ts"],
};
const builtTest =
  'import {test, expect} from "bun:test"; import {readFileSync} from "node:fs"; const buildReact = () => readFileSync(import.meta.dir + "/../builtins/src/lib.rs", "utf8"); test("bound", () => { const rust = buildReact(); expect(rust).not.toContain("appendFileSync"); expect(rust).toContain(\'"replaceAll"\'); });';

function bindingsSource(name: string, testSource: string) {
  const example = exampleSource(name, testSource);
  mkdirSync(join(example.root, "builtins", "src"), { recursive: true });
  writeFileSync(join(example.root, "builtins", "src", "lib.rs"), '#[link_name = "replaceAll"]\npub fn replace_all() {}\n');
  return { ...example, files: [...example.files, "builtins/src/lib.rs"] };
}

test("a mutant of the bindings crates' Rust is judged by the tests that build them, and inconclusive where none does", async () => {
  expect([ofBindingsCrate("builtins/src/lib.rs"), ofBindingsCrate("webapi/src/a.rs"), ofBindingsCrate("builtins/build.sh"), ofBindingsCrate("crates/rust-js-lower/src/lib.rs")]).toEqual([true, true, false, false]);
  const caught = bindingsSource("source-mutation-bindings", builtTest);
  expect((await executeSource(caught, bindingsMutation)).result).toBe("caught");
  const unbuilt = bindingsSource("source-mutation-bindings-unbuilt", builtTest.replaceAll("buildReact", "read"));
  expect((await executeSource(unbuilt, bindingsMutation)).result).toBe(
    "INCONCLUSIVE: test/owner.test.ts doesn't build the bindings crates (`buildReact`), so the mutant of builtins/src/lib.rs is never compiled",
  );
});

// A mutation whose tests read rust-js.wasm (`needs: "wasm"`) has what the
// wasm build made linked into its workspace, which copies only what git
// tracks: the wasm and the sysroot it type-checks against (#2346).
test("a wasm mutation's workspace links what the wasm build made", () => {
  const [from, workspace] = [fixture("wasm-built"), fixture("wasm-workspace")];
  for (const path of wasmBuild) {
    mkdirSync(join(from, path, ".."), { recursive: true });
    if (path.endsWith(".wasm")) writeFileSync(join(from, path), "wasm");
    else mkdirSync(join(from, path), { recursive: true });
  }
  linkWasmBuild(workspace, from);
  expect(wasmBuild.map((path) => realpathSync(join(workspace, path)))).toEqual(wasmBuild.map((path) => realpathSync(join(from, path))));
});

// A source mutant's workspace is the working tree's files, what git tracks
// and what it doesn't track that isn't ignored: a test written for the
// change, not committed yet, was missing from it, so its mutation ran
// nothing, "mutated source was not executed", committed, judged and amended
// three times (#2485). A file deleted from the working tree isn't one.
test("a source workspace's files are the working tree's, its untracked files too, not its ignored nor its deleted", () => {
  const dir = fixture("source-files-of");
  const git = (...args: string[]) => Bun.spawnSync(["git", ...args], { cwd: dir, stdout: "pipe", stderr: "pipe" });
  git("init", "-q");
  writeFileSync(join(dir, ".gitignore"), "built/\n");
  writeFileSync(join(dir, "tracked.ts"), "");
  writeFileSync(join(dir, "gone.ts"), "");
  git("add", ".");
  git("-c", "user.name=t", "-c", "user.email=t@t", "commit", "-qm", "files");
  rmSync(join(dir, "gone.ts"));
  mkdirSync(join(dir, "test"));
  writeFileSync(join(dir, "test", "new.test.ts"), "");
  mkdirSync(join(dir, "built"));
  writeFileSync(join(dir, "built", "out.js"), "");
  // Nor another checkout inside it, a worktree under `.claude/worktrees/`.
  mkdirSync(join(dir, "nested"));
  Bun.spawnSync(["git", "init", "-q"], { cwd: join(dir, "nested") });
  writeFileSync(join(dir, "nested", "theirs.ts"), "");
  expect(sourceFilesOf(dir).sort()).toEqual([".gitignore", "test/new.test.ts", "tracked.ts"]);
});
