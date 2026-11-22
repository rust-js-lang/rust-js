// The test selector (ADR 1101, amended): which test files and corpus cases a
// change can affect, the one answer `bun run push`'s check (`check:fast`)
// and CI's suite (check.yml's `plan`) run. The rest pass as they did at the
// test map's commit, where the map recorded each test passing.
//
//   bun scripts/select.ts                  what the working tree's change selects, and why
//   bun scripts/select.ts --json=<file>    and the selection, written as JSON (CI's `plan`)
//   bun scripts/select.ts --push           by the push's rule (`pushSelection`), as `bun run push` checks
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
//   src/**, crates/*/src/** the map: the tests and cases whose functions it
//                          touches, the compiler's crates' (scripts/compiler-crates.ts);
//                          everything of a crate the map counted none of (`counted`)
//   std/**, crates/rust-js-program/src/runtime/** the runtime: its package tests, the whole corpus, every file the map saw compile
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

import { existsSync, mkdirSync, readdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";

import { machineStore } from "../test/bounded";
import { importsOf } from "./iteratorSlice";
import { expected, type JobTimes, readJobTimes } from "./jobTimes";
import { machine, pushSuiteFiles } from "./shard";
import { type Change, caseFiles, caseUnit, functionsOf, type Hunk, hunksOf, mapStore, nearestMap, noMap, pick, reachOfFunctions, readsOf, type TestMap, toolchainOf, unitsReaching } from "./testMap";
import { crateOf, isCompilerSource } from "./compiler-crates";

const root = join(import.meta.dir, "..");

/** Whether `map` counted the functions of `file`'s crate: the binary's, and
 * each other of the compiler's crates' (ADR 1181) where it records one of
 * theirs. A map whose counting build counted the binary alone records none
 * of another crate's, and a change to it, outside every function it knows,
 * would select what the map can't say. */
export function counted(map: TestMap, file: string): boolean {
  const crate = crateOf(file);
  return crate === undefined || crate.dir === "" || map.functions.some((f) => f.file.startsWith(`${crate.src}/`));
}

/** What the last `check:fast` selected, its outcome (`outcome`), which
 * `bun run push` says on each run's line. */
export const selectionFile = join(root, "target", "check-fast-selection.json");

/** What a change runs: of the push suite's test files and the corpus's
 * cases, those it can affect, and the corpus with std's iterator structs'
 * cases (`forms`), its net's; `all` where it can't say, every one listed.
 * `why` says why each ran, or why everything did; `allOwn` whether that
 * reason is the branch's own change, not others' since the map's commit. */
export type Selection = {
  all: boolean;
  allOwn?: boolean;
  files: string[];
  cases: string[];
  forms: string[];
  why: string;
  map?: string;
  behind?: number;
  capped?: string;
  /** Of a change to the compiler's code, the functions it changes, by
   * their indices in the map: what a sanity pass ranks its tests by. */
  functions?: number[];
  /** Of a sanity pass's budget, what it left out (`budgeted`). */
  skipped?: { files: string[]; cases: string[] };
  /** What the map's item layer would have selected, where it has one, run
   * in shadow beside this one, which runs (#2288): logged on the push's
   * line, with whether it held every test that failed, until the switch. */
  shadow?: Selection;
};

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
 * packages' manifests and lockfiles, the toolchain, Bun's config and its
 * preload's environment). */
export const everything = [
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
];

/** What changes which tests run, or how CI runs them, not what a test
 * does: of the change itself, everything, as its rule may be wrong; of
 * others' commits since the map's, only what reads it, as each was checked
 * by its own push and CI. Each with what it is: the workflows and actions,
 * and the selector and the map themselves, whose others' changes, a
 * selector's pushes, ran every agent's check whole a map behind them. */
export const ownRunsAll: Record<string, string> = {
  ".github/**": "how CI runs every test",
  "scripts/select.ts": "the selector, which picks every test",
  "scripts/testMap.ts": "the test map, which the selector picks every test by",
  "scripts/testMapCompiler.ts": "the test map's counting build, which the selector picks every test by",
};

/** The top folders whose files a test reads or imports, so a change there
 * runs only its readers and importers: the rest are packages and crates the
 * tests build as apps' dependencies, which no import or read says, and run
 * everything. And the files at the root, the docs. */
export const readOnly = ["docs", "scripts", "test", "ports", "bench", "guide-site"];

/** The runtime's paths: std's package and the runtime's helpers, which a
 * compiled program runs, in any test that compiles one. */
export const runtimePaths = ["std/**", "crates/rust-js-program/src/runtime/**"];

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
const ownRunsAllGlobs = Object.entries(ownRunsAll).map(([path, what]) => ({ glob: new Bun.Glob(path), what }));
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
  let functions: number[] | undefined;
  for (const change of changes) {
    const { file } = change;
    if (everythingGlobs.some((glob) => glob.match(file))) return whole(`${file} is what every test runs under`, file);
    const runsAll = ownRunsAllGlobs.find(({ glob }) => glob.match(file));
    if (runsAll) {
      if (own(file)) return whole(`${file} is ${runsAll.what}`, file);
      const found = readers(file);
      ran(found);
      say(file, `${runsAll.what}, others': ${found.length} test${found.length === 1 ? "" : "s"} read it`);
      continue;
    }
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
    if (isCompilerSource(file) || (typeof map !== "string" && map.includes[file] !== undefined)) {
      if (typeof map === "string") return whole(`${file} is the compiler's, and ${map}`, file);
      if (!counted(map, file)) return whole(`${file} is of a crate the map at ${map.commit} counted no function of`, file);
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
    functions = picked.functions;
  }
  const order = (of: Set<string>, all: string[]) => all.filter((item) => of.has(item));
  return {
    all: false,
    files: order(runs, [...new Set([...files, ...caseFiles])]),
    cases: order(runCases, cases),
    forms: order(runForms, cases),
    why: why.join("; ") || "nothing changed",
    ...(functions === undefined ? {} : { functions }),
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
/** The changes from `from` to `to`, or to the working tree with what it
 * doesn't track. */
export function changesBetween(from: string, to?: string): Change[] {
  if (to === undefined) return changesSince(from);
  const diff = Bun.spawnSync(["git", "diff", "-U0", "--no-color", "--no-ext-diff", "--no-renames", from, to], { cwd: root, stdout: "pipe" }).stdout.toString();
  return hunksOf(diff);
}

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

/** Which rule a selection is of: `full`, CI's check of `main` (check.yml's
 * `plan`), the whole diff from the map's commit, others' commits since it
 * too, their `all` inherited; `push`, `bun run push`'s check, the
 * branch's own change and what others' function changes reach
 * (`pushSelection`, ADR 1101 amended). */
export type Rule = "full" | "push";

/** What this checkout's change selects: by the map of HEAD or its nearest
 * ancestor this machine keeps, the diff from its commit; the branch's own
 * change since `base`'s merge-base, the rest others'. Of `rule` `push`,
 * `pushSelection`'s. Of `sanity`, a pass past a rebase that brought
 * commits, given the commit last checked before it: the interaction alone
 * (`interaction`), what both the branch's own change and the commits the
 * rebase brought can affect. */
export function selectChange(base = "origin/main", sanity?: string, rule: Rule = "full"): Selection & { map?: string } {
  const map = nearestMap(mapStore, root, toolchainOf());
  // The selection that runs is the map's without its item layer; the
  // layer's, beside it, in shadow until the switch (#2288).
  if (map?.items === undefined) return selectOf(map, base, sanity, rule);
  const ran = selectOf({ ...map, items: undefined }, base, sanity, rule);
  return { ...ran, shadow: selectOf(map, base, sanity, rule) };
}

function selectOf(map: TestMap | undefined, base: string, sanity: string | undefined, rule: Rule): Selection & { map?: string } {
  const files = suiteFiles();
  const cases = corpusCases();
  const own = ownFiles(base);
  const reach = reachOf([...files, caseFiles[1]]);
  if (map === undefined) {
    // No map of a sanity pass: the cap, unranked.
    if (sanity !== undefined) return capOf([...own], files, cases);
    // No map: the branch's own change, by the rules that need none, and
    // everything where one does.
    const changes = [...own].map((file): Change => ({ file, hunks: existsSync(join(root, file)) ? [[1, 1]] : "new" }));
    return select(changes, { files, cases, map: `${noMap}: \`bun run test-map\` records one`, reach });
  }
  const behind = commitsBehind(map.commit);
  if (sanity !== undefined) {
    const now = mergeBase(base);
    const before = now === undefined ? undefined : Bun.spawnSync(["git", "merge-base", sanity, now], { cwd: root, stdout: "pipe" }).stdout.toString().trim() || undefined;
    if (now === undefined || before === undefined) return { all: true, files: [], cases: [], forms: [], why: "the sanity pass can't find the commits the rebase brought", map: map.commit, behind };
    // The incoming commits are others', each checked by its own push.
    const side = (from: string, to: string | undefined, mine: boolean) => sideSelection(changesBetween(from, to), from, { files, cases, map, reach, own: () => mine });
    const both = interaction(side(now, undefined, true), side(before, now, false));
    // Where it can't say, the cap; and of either, what fits the budget,
    // by the change's own functions: those it changes, or of its files.
    const changed = Bun.spawnSync(["git", "diff", "--name-only", now], { cwd: root, stdout: "pipe" }).stdout.toString().split("\n").filter(Boolean);
    const said = both.all ? capOf(changed, files, cases, map) : both;
    const functions = said.functions ?? functionsOf(map, changed.filter((file) => isCompilerSource(file)));
    return { ...budgeted({ ...said, functions }, reachOfFunctions(map, functions), costsOf(readJobTimes(), cases.length), machine().cores), map: map.commit, behind };
  }
  const since = rule === "push" ? mergeBase(base) : undefined;
  if (since !== undefined) {
    const mine = sideSelection(changesBetween(since), since, { files, cases, map, reach, own: () => true });
    const theirs = sideSelection(changesBetween(map.commit, since), map.commit, { files, cases, map, reach, own: () => false });
    return { ...pushSelection(mine, theirs, [...own].some((file) => isCompilerSource(file))), map: map.commit, behind };
  }
  const selection = fullSelection(map, (file) => own.has(file), { files, cases, reach });
  return { ...selection, map: map.commit, behind };
}

/** The full rule's selection of what changed since `map`'s commit, the
 * working tree's too: CI's check of `main`'s (`selectChange`'s `full`), and
 * what an incremental test map records (scripts/testMap.ts's `planCarry`),
 * so the two can't diverge (ADR 1101, amended). */
export function fullSelection(map: TestMap, own: (file: string) => boolean = () => true, given?: { files: string[]; cases: string[]; reach: Reach }): Selection {
  const files = given?.files ?? suiteFiles();
  const cases = given?.cases ?? corpusCases();
  const reach = given?.reach ?? reachOf([...files, caseFiles[1]]);
  return select(changesSince(map.commit), { files, cases, map, reach, own });
}

/** What a push's check runs (ADR 1101, amended): what the branch's own
 * change, `mine`, can affect; and where it touches the compiler's code,
 * `compiler`, what others' commits since the map's commit, `theirs`, reach
 * by their functions' changes too: a test their change made reach a
 * function of this one runs, as the map at its commit can't say it does.
 * Never their `all`, an item, a type or a new file of theirs, which their
 * own push and CI checked: one push's item no longer runs every other
 * agent's whole suite. CI's check keeps the full rule (`Rule`), which
 * holds what this leaves: an item of theirs, `const ENABLE`, read by a
 * function neither side changed, that now reaches a function of this
 * change. */
export function pushSelection(mine: Selection, theirs: Selection, compiler: boolean): Selection {
  if (mine.all) return mine;
  if (!compiler || theirs.all) {
    const left = theirs.all ? `; others' since the map's commit not run here, which CI's check runs: ${theirs.why}` : "";
    return { ...mine, why: `the change's own: ${mine.why}${left}` };
  }
  return {
    all: false,
    files: [...new Set([...mine.files, ...theirs.files])],
    cases: [...new Set([...mine.cases, ...theirs.cases])],
    forms: [...new Set([...mine.forms, ...theirs.forms])],
    why: `the change's own: ${mine.why}; and what others' function changes reach: ${theirs.why}`,
  };
}

/** Of a file the map's commit and `from` differ on, the line of the map's
 * each of `from`'s is, where it's one neither changed: `between` is the
 * diff's hunks from the map's to `from`'s, `[mapStart, mapCount,
 * fromStart, fromCount]` each, as `git diff -U0` numbers them. A line in
 * a hunk of theirs, text that changed since the map, is none. */
export function mapLine(between: [number, number, number, number][], line: number): number | undefined {
  let shift = 0;
  for (const [mapStart, mapCount, fromStart, fromCount] of between) {
    // A pure removal or addition `git diff` numbers after the line it's
    // after; a change, from its first line.
    const first = fromCount === 0 ? fromStart + 1 : fromStart;
    if (line < first) break;
    if (line < first + fromCount) return undefined;
    shift = mapStart + (mapCount === 0 ? 1 : mapCount) - (fromStart + (fromCount === 0 ? 1 : fromCount));
  }
  return line + shift;
}

/** `change`'s hunks, numbered by `from`'s file, numbered by the map's
 * (`mapLine`): each line of each hunk one neither the map's commit nor
 * `from` changed since; or none, where a hunk lies in text that changed
 * since the map. */
export function hunksAtMap(change: Change, between: [number, number, number, number][]): Change | undefined {
  if (change.hunks === "new") return undefined;
  const hunks: Hunk[] = [];
  for (const [start, count, added, removed] of change.hunks) {
    if (count === 0) {
      // Lines added after `start`: where it and the line after are the map's, in turn.
      const at = mapLine(between, start);
      const next = mapLine(between, start + 1);
      if (at === undefined || (next !== undefined && next !== at + 1)) return undefined;
      hunks.push([at, 0, added, removed]);
      continue;
    }
    const at = mapLine(between, start);
    for (let line = start; line < start + count; line++) if (mapLine(between, line) !== (at === undefined ? undefined : at + line - start)) return undefined;
    if (at === undefined) return undefined;
    hunks.push([at, count, added, removed]);
  }
  return { file: change.file, hunks };
}

/** What a change to `moved`, files whose text changed since the map where
 * the change lies, runs: every test the map saw run any of their
 * functions, and each it says runs on every change, `null`'s, as `pick`
 * runs them; without those, the fallback ran 81 test files fewer than
 * d0ce7d1ab's placed change, 285 where it's 366 (2026-11-17). */
export function movedUnits(map: TestMap, moved: string[]): string[] {
  const always = Object.entries(map.units).flatMap(([unit, bits]) => (bits === null ? [unit] : []));
  return [...new Set([...unitsReaching(map, moved), ...always])];
}

/** The hunks `git diff -U0 <a> <b> -- <file>` gives. */
function diffHunks(a: string, b: string, file: string): [number, number, number, number][] {
  const out = Bun.spawnSync(["git", "diff", "-U0", "--no-color", "--no-ext-diff", "--no-renames", a, b, "--", file], { cwd: root, stdout: "pipe" }).stdout.toString();
  return [...out.matchAll(/^@@ -(\d+)(?:,(\d+))? \+(\d+)(?:,(\d+))? @@/gm)].map((m) => [Number(m[1]), m[2] === undefined ? 1 : Number(m[2]), Number(m[3]), m[4] === undefined ? 1 : Number(m[4])]);
}

/** What one side of a rebase selects, its changes since `from` by the map:
 * a line of a compiler's file the map's commit and `from` differ on is
 * placed at the map's line its text has there where neither changed it
 * since (`hunksAtMap`), as git places a moved line; one in text that
 * changed since the map isn't the map's, so of such a file, every test the
 * map saw run any of its functions (`unitsReaching`). Of 35 of ioCM's
 * pushes and checks of 2026-11-17, 7 took that fallback, and 2905d5cdc's
 * own change, crates/rust-js-emit/src/output.rs, selected 291 files where the map at
 * 7fe40bde3 placed it in one function, 7. */
export function sideSelection(changes: Change[], from: string, context: { files: string[]; cases: string[]; map: TestMap; reach: Reach; own: (file: string) => boolean }): Selection {
  const differing = changes.filter((change) => isCompilerSource(change.file) && change.hunks !== "new" && differs(context.map.commit, from, change.file));
  const placed = new Map(differing.flatMap((change) => {
    const at = hunksAtMap(change, diffHunks(context.map.commit, from, change.file));
    return at === undefined ? [] : [[change.file, at] as const];
  }));
  const moved = differing.map(({ file }) => file).filter((file) => !placed.has(file));
  const selection = select(changes.filter((change) => !moved.includes(change.file)).map((change) => placed.get(change.file) ?? change), context);
  if (moved.length === 0 || selection.all) return selection;
  const reached = new Set(movedUnits(context.map, moved));
  const byCase = context.cases.filter((name) => reached.has(`test/corpus/${name}`));
  return {
    ...selection,
    files: [...new Set([...selection.files, ...context.files.filter((file) => reached.has(file))])],
    cases: [...new Set([...selection.cases, ...byCase])],
    forms: [...new Set([...selection.forms, ...byCase])],
    functions: [...new Set([...(selection.functions ?? []), ...functionsOf(context.map, moved)])],
    why: `${selection.why}; ${moved.join(", ")} by every test of its functions`,
  };
}

/** A sanity pass past a rebase: what both the branch's own change, `own`,
 * and the commits the rebase brought, `incoming`, can affect. A test only
 * the branch's change affects ran with it before the rebase, and its
 * outcome changes only if the incoming commits affect it too; a test only
 * the incoming commits affect was checked by their pushers, and changes
 * only if the branch's change affects it too (ADR 1101, amended). Where a
 * side selects everything, the other side's; where both do, everything,
 * for the cap (`capOf`). What it runs of it is `budgeted`'s. */
export function interaction(own: Selection, incoming: Selection): Selection {
  if (own.all && incoming.all) return { ...own, why: `both the change and the commits the rebase brought select everything (${own.why}; ${incoming.why})` };
  const say = (selection: Selection) => (selection.all ? "everything" : `${selection.files.length} files, ${selection.cases.length} cases`);
  const why = `the interaction: the change's ${say(own)} and the rebase's commits' ${say(incoming)}`;
  const functions = own.functions === undefined ? {} : { functions: own.functions };
  if (own.all) return { ...incoming, all: false, why, ...functions };
  if (incoming.all) return { ...own, all: false, why };
  return {
    all: false,
    files: own.files.filter((file) => incoming.files.includes(file)),
    cases: own.cases.filter((name) => incoming.cases.includes(name)),
    forms: own.forms.filter((name) => incoming.forms.includes(name)),
    why,
    ...functions,
  };
}

/** A sanity pass's budget: the seconds its tests take on the machine's
 * cores at once, past check:fast's own start of some 25 s, so the pass is
 * about a minute (Tu's rule: quick, then push; CI's check covers the
 * combination). Its rounds took 23–248 s on the Mac mini, the interaction's
 * every corpus case, or both sides' everything capped to the change's own
 * whole selection again, 220–228 s (2026-11-18); a longer pass is a moving
 * `main`'s next round too. */
export const sanitySeconds = 45;

/** What a sanity pass's tests are said to cost, in seconds of a core: a
 * test file its usual, as check:fast's jobs keep it (`readJobTimes`), and
 * a corpus case the corpus's whole run's usual over its cases. One not yet
 * measured, its file's or the corpus's, is said to cost `unmeasured`. */
export type Costs = { file: (file: string) => number; case: number };
export const unmeasured = { file: 10, case: 0.3 };

/** `Costs` by check:fast's jobs as the machine kept them, of `cases` the
 * corpus's cases. */
export function costsOf(times: Record<string, JobTimes>, cases: number): Costs {
  const usual = (name: string) => expected(times[name]).usual;
  const parts = Object.keys(times).filter((name) => /^corpus \d+\/(\d+)$/.test(name));
  const whole = parts.filter((name) => name.endsWith(`/${machine().cores}`)).map(usual).filter(Number.isFinite);
  const perCase = whole.length > 0 && cases > 0 ? (whole.reduce((sum, seconds) => sum + seconds, 0) * (machine().cores / whole.length)) / cases : unmeasured.case;
  return { file: (file) => (Number.isFinite(usual(file)) ? usual(file) : unmeasured.file), case: perCase };
}

/** Of a sanity pass's selection, what it runs within `seconds` on `cores`
 * (`sanitySeconds`): its test files and corpus cases each by how many of
 * the change's own functions it ran, `reach`, most first, and the cheaper
 * first between equals, so a change no map can rank runs the most it can,
 * then the selection's order; each taken where it fits what's left, and
 * none that alone takes longer than `seconds`. What it leaves is `skipped`, which
 * `capped` counts and `recordSanity` keeps, so a red `main` an interaction
 * made can be held to whether its pass ran the failing test (ADR 1101,
 * amended). */
export function budgeted(selection: Selection, reach: (unit: string) => number, costs: Costs, cores: number, seconds = sanitySeconds): Selection {
  if (selection.all) return selection;
  const units = [
    ...selection.files.filter((file) => !(caseFiles as readonly string[]).includes(file)).map((file) => ({ file, unit: file, cost: costs.file(file) })),
    ...selection.cases.map((name) => ({ name, unit: caseUnit(caseFiles[0], name)!, cost: costs.case })),
  ];
  const ranked = units.map((unit, at) => ({ ...unit, at, reach: reach(unit.unit) })).sort((a, b) => b.reach - a.reach || a.cost - b.cost || a.at - b.at);
  let left = seconds * cores;
  const ran = new Set<string>();
  for (const unit of ranked) {
    if (unit.cost > seconds || unit.cost > left) continue;
    left -= unit.cost;
    ran.add(unit.unit);
  }
  const files = selection.files.filter((file) => (caseFiles as readonly string[]).includes(file) || ran.has(file));
  const cases = selection.cases.filter((name) => ran.has(caseUnit(caseFiles[0], name)!));
  const skipped = { files: files.length === selection.files.length ? [] : selection.files.filter((file) => !files.includes(file)), cases: selection.cases.filter((name) => !cases.includes(name)) };
  const kept = cases.length > 0 ? files : files.filter((file) => file !== caseFiles[0]);
  if (skipped.files.length === 0 && skipped.cases.length === 0) return selection;
  const said = (of: string[], from: string[], what: string) => (from.length === 0 ? [] : [`${of.length} of ${from.length} ${what}`]);
  const tested = (of: string[]) => of.filter((file) => !(caseFiles as readonly string[]).includes(file));
  const by = selection.functions?.length ? `by reach of the change's ${selection.functions.length} functions` : "cheapest first";
  return {
    ...selection,
    files: kept,
    cases,
    forms: selection.forms.filter((name) => cases.includes(name)),
    capped: `${selection.capped ? `${selection.capped}, ` : ""}${[...said(tested(kept), tested(selection.files), "files"), ...said(cases, selection.cases, "cases")].join(" and ")} in ${seconds} s, ${by}`,
    skipped,
  };
}

/** What `base` and HEAD have in common. */
function mergeBase(base: string): string | undefined {
  return Bun.spawnSync(["git", "merge-base", "HEAD", base], { cwd: root, stdout: "pipe" }).stdout.toString().trim() || undefined;
}

/** How many commits HEAD is past the map's. */
function commitsBehind(commit: string): number | undefined {
  const out = Bun.spawnSync(["git", "rev-list", "--count", `${commit}..HEAD`], { cwd: root, stdout: "pipe" }).stdout.toString().trim();
  return out === "" ? undefined : Number(out);
}

/** Whether `file` differs between two commits. */
function differs(from: string, to: string, file: string): boolean {
  return Bun.spawnSync(["git", "diff", "--quiet", from, to, "--", file], { cwd: root }).exitCode !== 0;
}

/** Of a sanity pass whose own change the selector can't say of, the cap
 * (Tu's rule: quick, then push; CI's check covers the combination): the
 * change's own test files and corpus cases; and of a change to the
 * compiler's sources, never none, as two changes to one file are where
 * they most meet (m6-lily's tidy refactor's cap ran nothing): the tests
 * and cases the map, as of its commit, recorded running any function of
 * the files it changes, an approximation; or, where that's none, the whole
 * corpus. `capped` says which. */
export function capOf(changed: string[], files: string[], cases: string[], map?: TestMap): Selection {
  const ownFiles = files.filter((file) => changed.includes(file));
  const ownCases = cases.filter((name) => changed.some((file) => file.startsWith(`test/corpus/${name.replace(/\.rs$/, "")}.`)));
  const sources = changed.filter((file) => isCompilerSource(file));
  const result = (picked: string[], picks: string[], capped: string): Selection => ({
    all: false,
    files: picked,
    cases: picks,
    forms: picks,
    why: "the sanity pass's cap",
    capped,
  });
  if (sources.length === 0) return result(ownFiles, ownCases, `some (${ownFiles.length} files, ${ownCases.length} cases)`);
  const reached = map ? unitsReaching(map, sources) : [];
  const mapFiles = files.filter((file) => reached.includes(file) || ownFiles.includes(file));
  const mapCases = cases.filter((name) => reached.includes(`test/corpus/${name}`) || ownCases.includes(name));
  if (map && reached.length > 0) return { ...result(mapFiles, mapCases, `some (from the map at ${map.commit.slice(0, 9)}: ${mapFiles.length} files, ${mapCases.length} cases)`), functions: functionsOf(map, sources) };
  return result([...new Set([...ownFiles, "test/corpus.test.ts"])], cases, "corpus");
}

/** Where the machine keeps what each sanity pass ran and left
 * (`recordSanity`), a file a pass, by the commit it checked; and the most
 * it keeps, as `scripts/disk.ts` bounds it, the oldest first. */
export const sanityRunsStore = join(machineStore, "sanity-runs");
export const sanityRunsCap = 16 * 2 ** 20;

/** A sanity pass's `selection` kept, by HEAD's commit, the one it checks
 * and pushes, past `checked`, the commit checked before the rebase: what
 * it ran and what its budget left (`skipped`). Where `main` goes red from
 * an interaction of two pushes, the failing test is in the pushed
 * commit's `ran`, or its `skipped`: whether the budget let it through
 * (ADR 1101, amended; docs/dev-loop.md says when to revisit it). */
export function recordSanity(selection: Selection, checked: string): void {
  const head = Bun.spawnSync(["git", "rev-parse", "HEAD"], { cwd: root, stdout: "pipe" }).stdout.toString().trim();
  if (head === "") return;
  mkdirSync(sanityRunsStore, { recursive: true });
  writeFileSync(join(sanityRunsStore, `${head}.json`), `${JSON.stringify(sanityRecord(selection, head, checked), null, 1)}\n`);
}

/** What `recordSanity` keeps of a pass. */
export function sanityRecord(selection: Selection, head: string, checked: string) {
  return { head, checked, map: selection.map ?? null, outcome: outcome(selection), all: selection.all, ran: { files: selection.files, cases: selection.cases }, skipped: selection.skipped ?? { files: [], cases: [] } };
}

/** A selection in a line: `all (why)`, `some (N files, M cases)`, `none`;
 * every push's line and CI's summary say it. */
/** Of a selection in shadow, `shadow`, the tests that failed in the run of
 * the one that ran, `failed` (each job's file and the tests it names), that
 * it leaves out: a corpus case or an iterator-forms case it doesn't hold, or
 * a test file it doesn't, by name. None of one that selects everything. */
export function shadowMissed(shadow: { all: boolean; files: string[]; cases: string[]; forms: string[] }, failed: { file: string; tests: string[] }[]): string[] {
  if (shadow.all) return [];
  const missed: string[] = [];
  for (const { file, tests } of failed) {
    if (file === caseFiles[0]) {
      for (const test of tests) if (!shadow.cases.includes(test)) missed.push(`${file}: ${test}`);
    } else if (file === caseFiles[1]) {
      for (const test of tests) if (!shadow.forms.includes(test.replace(/ with .*$/, ""))) missed.push(`${file}: ${test}`);
    } else if (file !== "" && !shadow.files.includes(file)) missed.push(file);
  }
  return missed;
}

export function outcome(selection: Selection): string {
  const behind = selection.map ? `, map ${selection.map.slice(0, 9)} ${selection.behind ?? "?"} behind` : "";
  return `${outcomeOf(selection)}${behind}`;
}

function outcomeOf(selection: Selection): string {
  if (selection.capped) return `capped ${selection.capped}`;
  if (selection.all) return `all (${selection.why})`;
  const files = selection.files.filter((file) => file !== caseFiles[0]).length;
  if (files === 0 && selection.cases.length === 0) return "none";
  return `some (${files} file${files === 1 ? "" : "s"}, ${selection.cases.length} case${selection.cases.length === 1 ? "" : "s"})`;
}


if (import.meta.main) {
  const arg = (name: string) => process.argv.find((given) => given.startsWith(`--${name}=`))?.slice(name.length + 3);
  const json = arg("json");
  const selection = selectChange(arg("base"), arg("sanity"), process.argv.includes("--push") ? "push" : "full");
  console.log(`${outcome(selection)}${selection.map ? `, by the test map of ${selection.map.slice(0, 9)}` : ""}`);
  if (!selection.all) {
    console.log(selection.why);
    console.log(`files: ${selection.files.join(" ")}`);
  }
  if (json) writeFileSync(json, JSON.stringify(selection));
}
