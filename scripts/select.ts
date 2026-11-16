// The test selector (ADR 1101, amended): which test files and corpus cases a
// change can affect, the one answer `bun run push`'s check (`check:fast`)
// and CI's suite (check.yml's `plan`) run. The rest pass as they did at the
// test map's commit, where the map recorded each test passing.
//
//   bun scripts/select.ts                  what the working tree's change selects, and why
//   bun scripts/select.ts --json=<file>    and the selection, written as JSON (CI's `plan`)
//
// The change is the diff from the map's commit `M` to the working tree, one
// diff: it holds what others pushed since `M` (`M..B`, B the branch's base)
// and the branch's own change (`B..` the tree). A test runs where that diff
// touches what it reaches: a line of a function the map recorded it running,
// itself, a module it imports, or a file it reads as text. So a test whose
// functions another agent's commit touched runs, a test another agent's
// commit made reach a function of this change runs (that commit touched one
// of its functions), and a test that failed or didn't finish at `M` runs on
// every change (the map records it so). Everything runs only where the map
// can't say: a line of `src/` outside any function's counted code, a type,
// a constant, an item; a new file of `src/`; or no map at all. Each such
// reason says whether it's the branch's own change or others'
// (`Selection.allOwn`), which every push's line and CI's summary count.
//
// The rules, by the changed file's path:
//   src/**                 the map: the tests and cases whose functions it touches
//   std/**, src/runtime/** the runtime: its package tests, the whole corpus, every file the map saw compile
//   test/<x>.test.ts       itself (the corpus's file: every case)
//   test/corpus/<case>.*   the case, and the tests that read the corpus's files
//   .github/**, package files, the selector, the map
//                          everything
//   any other              each test that imports it, or reads it as text,
//                          itself or through a module it imports; none where
//                          none does (a doc nothing reads)
//   a package's or crate's folder (webapi/, react/, vendor/, ...)
//                          everything: tests build them as apps' dependencies,
//                          which no import or read says

import { existsSync, readdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";

import { importsOf } from "./iteratorSlice";
import { pushSuiteFiles } from "./shard";
import { type Change, caseFiles, hunksOf, mapStore, nearestMap, noMap, pick, readsOf, type TestMap, toolchainOf } from "./testMap";

const root = join(import.meta.dir, "..");

/** What the last `check:fast` selected, its outcome (`outcome`), which
 * `bun run push` says on each run's line. */
export const selectionFile = join(root, "target", "check-fast-selection.json");

/** What a change runs: of the push suite's test files and the corpus's
 * cases, those it can affect, and the corpus with std's iterator structs'
 * cases (`forms`), its net's; `all` where it can't say, every one listed.
 * `why` says why each ran, or why everything did; `allOwn` whether that
 * reason is the branch's own change, not others' since the map's commit. */
export type Selection = { all: boolean; allOwn?: boolean; files: string[]; cases: string[]; forms: string[]; why: string };

/** What a change selects of: the push suite's test files (`suiteFiles`),
 * the corpus's cases, the map or why there's none, and, of each test file,
 * what it reaches besides compiles (`Reach`). `own` says which changed files
 * are the branch's own, the rest others' since the map's commit. */
export type Context = {
  files: string[];
  cases: string[];
  map: TestMap | string;
  reach: Reach;
  own?: (file: string) => boolean;
};

/** Of each test file, the modules it imports, transitively, the preload's
 * too, and what each of those reads as text (`readsOf`, a path, a folder or
 * a glob). */
export type Reach = Map<string, { modules: Set<string>; reads: Set<string> }>;

/** The files whose change runs everything: what every test runs under (the
 * workflows and actions, the packages' manifests and lockfiles, the
 * toolchain, Bun's config and its preload's environment), and the selector
 * and the map themselves, whose rule a change of may be wrong. */
export const everything = [
  ".github/**",
  "package.json",
  "*/package.json",
  "*/*/package.json",
  "bun.lock",
  "bunfig.toml",
  "tsconfig.json",
  ".env.test",
  "Cargo.toml",
  "Cargo.lock",
  "build.rs",
  "rust-toolchain.toml",
  "rustfmt.toml",
  "scripts/select.ts",
  "scripts/testMap.ts",
  "scripts/testMapCompiler.ts",
];

/** The top folders whose files a test reads or imports, so a change there
 * runs only its readers and importers: the rest are packages and crates the
 * tests build as apps' dependencies, which no import or read says, and run
 * everything. And the files at the root, the docs. */
export const readOnly = ["docs", "scripts", "test", "ports", "bench", "guide-site"];

/** The runtime's paths: std's package and the runtime's helpers, which a
 * compiled program runs, in any test that compiles one. */
export const runtimePaths = ["std/**", "src/runtime/**"];

/** The runtime's own package tests. */
const packageTest = /^test\/(std|runtime)-[^/]+\.test\.ts$/;

/** The harness's branches on the machine's OS, each with why it doesn't
 * change what a test compiles: CI's maps are recorded on Linux and read by
 * `bun run push` on macOS, sound only where each test runs the same
 * compiler code on both. The compiler's own sources branch on none but
 * WASI's (test/select.test.ts holds both). A new one is listed here with
 * its reason, or the maps are keyed by OS. */
export const osBranches: Record<string, string> = {
  "test/child.ts": "which processes a run reaps, after its compiles",
  "test/child.test.ts": "the reaping's own tests, which compile nothing",
  "test/programs.ts": "how a native program's memory is held, which runs after the compile",
  "test/create.test.ts": "the distribution's platform, a field of its manifest",
  "test/node-bindings.test.ts": "what the compiled program prints of its platform, as it runs",
  "scripts/strays.ts": "how processes left running are listed",
  "scripts/shard.ts": "how the machine's free memory is read",
  "scripts/package-compiler.ts": "the package's `os` and `cpu`",
  "scripts/package-distribution.ts": "the distribution's platform, a field of its manifest",
  "scripts/qualify.ts": "the host a qualification is of",
  "scripts/serde-bench.ts": "the machine a benchmark says it ran on",
  "tooling/native-launcher.js": "the name of the library path's variable",
};

/** The baselines a test reads by a path its source doesn't spell, each with
 * the tests that read it: what `readsOf` can't see. */
export const baselines: { path: string; owners: string[] }[] = [
  // Each case's own JS, `${name}.js`, beside its source.
  { path: "test/corpus/*.js", owners: ["test/readability.test.ts"] },
  // Each snapshot's files, by the snapshot's name.
  { path: "test/snapshots/**", owners: ["test/snapshots.test.ts", "test/readability.test.ts"] },
];

const globs = (paths: string[]) => paths.map((path) => new Bun.Glob(path));
const everythingGlobs = globs(everything);
const runtimeGlobs = globs(runtimePaths);
const baselineGlobs = baselines.map(({ path, owners }) => ({ glob: new Bun.Glob(path), owners }));

/** What `changes` runs (the file's comment says how). */
export function select(changes: Change[], context: Context): Selection {
  const { files, cases, map, reach } = context;
  const own = context.own ?? (() => true);
  const whole = (why: string, file: string): Selection => ({ all: true, allOwn: own(file), files, cases, forms: cases, why: `${why} (${own(file) ? "the change's own" : "others' since the map's commit"})` });
  const runs = new Set<string>();
  const runCases = new Set<string>();
  const runForms = new Set<string>();
  const why: string[] = [];
  const say = (file: string, ran: string) => {
    if (why.length < 20) why.push(`${file}: ${ran}`);
  };
  // The tests each changed file reaches as a module or a read.
  const readers = (file: string) => {
    const found: string[] = [];
    for (const [test, { modules, reads }] of reach) {
      if (test === file) continue;
      if (modules.has(file) || [...reads].some((read) => file === read || file.startsWith(`${read}/`) || (read.includes("*") && new Bun.Glob(read).match(file)))) found.push(test);
    }
    for (const { glob, owners } of baselineGlobs) if (glob.match(file)) found.push(...owners.filter((owner) => files.includes(owner)));
    return [...new Set(found)];
  };
  // The tests a change runs as what they import or read: the corpus's
  // file, every case; std's iterator structs' (the nightly's, whose cases
  // its net runs), every case of it.
  const ran = (readers: string[]) => {
    for (const reader of readers) {
      if (reader === caseFiles[0]) for (const name of cases) runCases.add(name);
      if (reader === caseFiles[1]) for (const name of cases) runForms.add(name);
      else if (files.includes(reader)) runs.add(reader);
    }
  };
  const srcChanges: Change[] = [];
  let runtime: string | undefined;
  for (const change of changes) {
    const { file } = change;
    if (everythingGlobs.some((glob) => glob.match(file))) return whole(`${file} is what every test runs under`, file);
    // A test file runs itself; the corpus's, every case.
    if (/^test\/[^/]+\.test\.ts$/.test(file)) {
      if (file === caseFiles[0]) {
        for (const name of cases) runCases.add(name);
        runs.add(file);
        say(file, "every corpus case");
      }
      if (file === caseFiles[1]) for (const name of cases) runForms.add(name);
      if (files.includes(file)) runs.add(file);
      ran(readers(file));
      continue;
    }
    // A corpus case runs itself, and the tests that read the corpus's files.
    const named = /^test\/corpus\/([^/]+?)\.[^/]+$/.exec(file)?.[1];
    if (named !== undefined && cases.includes(`${named}.rs`)) {
      runCases.add(`${named}.rs`);
      runForms.add(`${named}.rs`);
      // The map's readers of the corpus's files, each test whose own text
      // says `corpus`, and the baselines' owners; without a map, what
      // reads the corpus's folder, the 50 that list its cases too.
      if (typeof map !== "string") {
        for (const reader of map.corpusReaders) if (files.includes(reader)) runs.add(reader);
        for (const { glob, owners } of baselineGlobs) if (glob.match(file)) ran(owners);
      } else ran(readers(file));
      continue;
    }
    if (runtimeGlobs.some((glob) => glob.match(file))) {
      runtime ??= file;
      ran(readers(file));
      continue;
    }
    // The compiler's sources, and what `include_str!` reads: the map's.
    if (file.startsWith("src/") || (typeof map !== "string" && map.includes[file] !== undefined)) {
      if (typeof map === "string") return whole(`${file} is the compiler's, and ${map}`, file);
      srcChanges.push(change);
      ran(readers(file));
      continue;
    }
    const top = file.includes("/") ? file.split("/")[0] : "";
    if (top !== "" && !readOnly.includes(top)) return whole(`${file} is of ${top}/, which tests build as an app's dependency`, file);
    const found = readers(file);
    ran(found);
    say(file, found.length === 0 ? "nothing reads it" : `${found.length} test${found.length === 1 ? "" : "s"} import or read it`);
  }
  if (runtime !== undefined) {
    // Its package tests, every case, and every file the map saw compile.
    if (typeof map === "string") return whole(`${runtime} is the runtime's, and ${map}`, runtime);
    for (const file of files) if (packageTest.test(file) || !map.noCompiler.includes(file)) runs.add(file);
    for (const name of cases) {
      runCases.add(name);
      runForms.add(name);
    }
    say(runtime, "the runtime: its package tests, every case, every file that compiles");
  }
  if (srcChanges.length > 0 && typeof map !== "string") {
    const picked = pick(map, srcChanges, files, cases);
    if (picked.kind === "all") {
      const at = srcChanges.find((change) => picked.why.startsWith(change.file))?.file ?? srcChanges[0]!.file;
      return whole(picked.why, at);
    }
    for (const file of picked.files) if (files.includes(file)) runs.add(file);
    if (picked.files.includes(caseFiles[0])) runs.add(caseFiles[0]);
    for (const name of picked.cases) runCases.add(name);
    for (const name of picked.forms) runForms.add(name);
    why.push(picked.why);
  }
  const order = (of: Set<string>, all: string[]) => all.filter((item) => of.has(item));
  return {
    all: false,
    files: order(runs, [...new Set([...files, ...caseFiles])]),
    cases: order(runCases, cases),
    forms: order(runForms, cases),
    why: why.join("; ") || "nothing changed",
  };
}

/** The push suite's test files (scripts/shard.ts's `pushSuiteFiles`). */
export function suiteFiles(): string[] {
  return pushSuiteFiles(readdirSync(join(root, "test")));
}

/** The corpus's cases. */
export function corpusCases(): string[] {
  return readdirSync(join(root, "test", "corpus"))
    .filter((name) => name.endsWith(".rs"))
    .sort();
}

/** Of each test file of `files`, what it imports and reads (`Reach`): the
 * preload's (bunfig.toml's, test/setup.ts) is every test's. */
export function reachOf(files: string[]): Reach {
  const preload = importsOf(["test/setup.ts"]);
  const reads = new Map<string, Set<string>>();
  const readsOfModule = (module: string) => {
    let found = reads.get(module);
    if (found === undefined) {
      found = readsOf(module, false);
      reads.set(module, found);
    }
    return found;
  };
  const reach: Reach = new Map();
  for (const file of files) {
    const modules = new Set([...importsOf([file]), ...preload]);
    const read = new Set<string>();
    for (const module of modules) for (const path of readsOfModule(module)) read.add(path);
    modules.delete(file);
    reach.set(file, { modules, reads: read });
  }
  return reach;
}

/** The map's commit's diff to the working tree, untracked files too. */
function changesSince(commit: string): Change[] {
  const git = (args: string[]) => Bun.spawnSync(["git", ...args], { cwd: root, stdout: "pipe" }).stdout.toString();
  const diff = git(["diff", "-U0", "--no-color", "--no-ext-diff", "--no-renames", commit]);
  const untracked = git(["ls-files", "--others", "--exclude-standard"]).split("\n").filter(Boolean);
  return [...hunksOf(diff), ...untracked.map((file): Change => ({ file, hunks: "new" }))];
}

/** The files the branch changes itself since `base`, its working tree's
 * too: the rest of the map's diff is others'. */
function ownFiles(base: string): Set<string> {
  const git = (args: string[]) => Bun.spawnSync(["git", ...args], { cwd: root, stdout: "pipe" }).stdout.toString().split("\n").filter(Boolean);
  const since = git(["merge-base", "HEAD", base])[0];
  return new Set([...(since ? git(["diff", "--name-only", since]) : []), ...git(["ls-files", "--others", "--exclude-standard"])]);
}

/** What this checkout's change selects: by the map of HEAD or its nearest
 * ancestor this machine keeps, the diff from its commit; the branch's own
 * change since `base`'s merge-base, the rest others'. */
export function selectChange(base = "origin/main"): Selection & { map?: string } {
  const files = suiteFiles();
  const cases = corpusCases();
  const map = nearestMap(mapStore, root, toolchainOf());
  const own = ownFiles(base);
  if (map === undefined) {
    // No map: the branch's own change, by the rules that need none, and
    // everything where one does.
    const changes = [...own].map((file): Change => ({ file, hunks: existsSync(join(root, file)) ? [[1, 1]] : "new" }));
    return select(changes, { files, cases, map: `${noMap}: \`bun run test-map\` records one`, reach: reachOf([...files, caseFiles[1]]) });
  }
  const selection = select(changesSince(map.commit), { files, cases, map, reach: reachOf([...files, caseFiles[1]]), own: (file) => own.has(file) });
  return { ...selection, map: map.commit };
}

/** A selection in a line: `all (why)`, `some (N files, M cases)`, `none`;
 * every push's line and CI's summary say it. */
export function outcome(selection: Selection): string {
  if (selection.all) return `all (${selection.why})`;
  const files = selection.files.filter((file) => file !== caseFiles[0]).length;
  if (files === 0 && selection.cases.length === 0) return "none";
  return `some (${files} file${files === 1 ? "" : "s"}, ${selection.cases.length} case${selection.cases.length === 1 ? "" : "s"})`;
}


if (import.meta.main) {
  const arg = (name: string) => process.argv.find((given) => given.startsWith(`--${name}=`))?.slice(name.length + 3);
  const json = arg("json");
  const selection = selectChange(arg("base"));
  console.log(`${outcome(selection)}${selection.map ? `, by the test map of ${selection.map.slice(0, 9)}` : ""}`);
  if (!selection.all) {
    console.log(selection.why);
    console.log(`files: ${selection.files.join(" ")}`);
  }
  if (json) writeFileSync(json, JSON.stringify(selection));
}
