// The test map (#877): which of the compiler's functions each test file of
// the push suite runs, and each case of the corpus, as it's built and as
// it's built with std's iterator structs (#876), recorded by running them
// with a build of the compiler that counts each function's runs. The
// selector reads it to run only the files and cases a change can affect,
// `bun run push`'s check and CI's suite alike (scripts/select.ts, ADR 1101,
// amended), and the mutation runner which cases reach a mutation's site,
// whose snapshots judge one of how the JS reads (#864).
//
//   bun run test-map           record HEAD's, of a clean tree: about 130 s on 12 cores,
//                              none where any worktree of the machine recorded it
//   bun run test-map --keep    and keep what it recorded, to see why a test runs always
//   bun scripts/testMap.ts kept [--head]
//                              whether a map of HEAD or an ancestor (`--head`: of HEAD)
//                              is kept, its exit status: CI's mutation jobs, whose
//                              snapshot judge places each site by it (ADR 0093, amended)
//   bun scripts/testMap.ts build --shards=N [--full]
//                              incremental where an earlier map serves (`planCarry`): the
//                              tests the selector's full rule picks recorded, the rest carried
//   bun scripts/testMap.ts drift
//                              the nightly's: HEAD's full map against what a carry makes
//   bun scripts/testMap.ts units --shard=i/N
//   bun scripts/testMap.ts merge
//                              a recording in N parts on N machines, CI's: the counting
//                              build and the plan, each part's tests, then the one map
//                              of them all, the same map `record` writes (`mergeRecordings`)
//
// Why a skip is sound (ADR 1101): a test whose compiles ran none of the
// functions a change touches runs the same compiler code after it, so ends
// as it did. That holds only where everything the change touches is a
// function's code: a line of a function's counted code, or a file
// `include_str!` reads at such a line. Anything else, a type, a constant,
// an attribute, an item written in a function, a file outside `src/` (a
// test's helper, std, the runtime's package, the toolchain, a doc a test
// reads), and the map can't say: the selector says what runs of those
// (scripts/select.ts), everything where it can't. What a test reaches is
// recorded where nothing is shared with another (its own `RUST_JS_TARGET`,
// std's sysroot and no compile cache),
// so nothing one test made is read by another unrecorded. A test that ran
// no compiler, or one whose compile wasn't counted whole (stopped,
// aborted), or that failed, runs on every change, as does one that reads
// the compiler's sources itself (`readsSources`).
//
// Kept by the commit it's of and the toolchain's, for every worktree of the
// machine, in the main checkout's `.cache/test-maps/` (#932): a map is the same in
// any checkout of its commit, so a new worktree reads one another recorded,
// and `bun run test-map` of a commit one's recorded for ends at once. The
// three most recently used, about 12 MB each. The counting build is the
// worktree's, `target/test-map/build`, one, as `target/debug` is. A change
// is what the working tree differs from the map's commit by, so a map of an
// earlier commit stays good while what's changed since is functions' code;
// of those of HEAD's ancestors, the nearest is read. One whose commit isn't
// HEAD's ancestor, or of another version or toolchain, is never read.

import { appendFileSync, copyFileSync, existsSync, mkdirSync, readdirSync, readFileSync, renameSync, rmSync, statSync, symlinkSync, utimesSync, writeFileSync } from "node:fs";
import { dirname, join, relative, resolve } from "node:path";

import { bytesOf, evict, freeBytes } from "../tooling/bounded.js";
import { floorBytes, machineStore } from "../test/bounded";
import { machineVerifiedRuns } from "../test/kept";
import { downloaded } from "./artifacts";
import { compilerCrates, compilerSourceDirs, crateOf, isCompilerSource } from "./compiler-crates";
import { readJobTimes } from "./jobTimes";
import { type ItemLayer, implBodyAt, inUnitTests, itemLayer, scipIndex, touchedByItem, touchedByNewFile } from "./testMapItems";
import { balanced, caseShardParts, machine, pushSuiteFiles, shard, shardArg } from "./shard";

const root = join(import.meta.dir, "..");
/** What records a map, the worktree's: its counting build and a recording's
 * counts. */
export const mapDir = join(root, "target", "test-map");
/** Where maps are kept, the machine's, every worktree's (#932). */
export const mapStore = join(machineStore, "test-maps");
/** The layout's version, which a change to it or to what's recorded bumps. */
export const mapVersion = 4;
/** The most maps kept: each about 22 MB, one a commit of `main` CI
 * records (ADR 1101, amended). */
export const keptMaps = 8;

/** Each of the compiler's functions, by its index: the file it's in and the
 * line spans of its counted code; and of each test, the functions it ran,
 * a bit each, base64, or `null`: it runs on every change. */
export type TestMap = {
  version: number;
  commit: string;
  /** The toolchain it was recorded with, `rustc -vV`'s commit hash. */
  toolchain: string;
  made: number;
  functions: { file: string; spans: [number, number][] }[];
  units: Record<string, string | null>;
  /** Each file `include_str!` or `include_bytes!` reads, and where. */
  includes: Record<string, { file: string; line: number }[]>;
  /** The test files that read the corpus's files themselves, as
   * readability's ratchet reads its JS and a lowering test a case's
   * source: each runs where a case changes. */
  corpusReaders: string[];
  /** The test files that ran no compiler, each `null` in `units`: one
   * runs where std's package or the runtime changes only where it reads or
   * imports it (scripts/select.ts). */
  noCompiler: string[];
  /** Each function's symbol, by its index: what an incremental recording
   * carries a test's functions over by, a function's index and lines
   * moving between commits. A map without them is never carried from. */
  names?: string[];
  /** The full recording its chain of incremental ones started from: its
   * commit and when it was made, itself where it's full. */
  base?: { commit: string; made: number };
  /** Of an incremental recording, the map it carried from, and how many of
   * its tests it recorded and carried. */
  carried?: { from: string; recorded: number; carried: number };
  /** Which functions reach each item outside any function, and each
   * `impl`'s header (scripts/testMapItems.ts, #2288): a change there selects
   * them where it would select everything. A map without it, every map
   * before it or one whose index failed, selects as before. */
  items?: ItemLayer;
};

/** How far an incremental recording carries from: a map at most this many
 * commits behind, whose chain's full recording is at most `baseDays` old
 * and `baseCommits` behind; past either, a recording is full, so a drift
 * between nightlies can't compound (ADR 1101, amended). */
export const carryCommits = 40;
export const baseCommits = 200;
export const baseDays = 1;
/** Written in the map store where the nightly's drift check failed: each
 * recording is full until a nightly's passes again. */
export const driftFlag = join(mapStore, "drift-failed");

/** Of a test's bits over `from`'s functions, its bits over `to`'s, each
 * function by its symbol: one `to` has that `from` hasn't, new since, no
 * test it carries ran, as only code changed since calls it. */
export function remapBits(bits: Uint8Array, from: string[], to: string[]): Uint8Array {
  const at = new Map(to.map((name, i) => [name, i]));
  const out = new Uint8Array(Math.ceil(to.length / 8));
  for (let f = 0; f < from.length; f++) {
    if (!(bits[f >> 3]! & (1 << (f & 7)))) continue;
    const i = at.get(from[f]!);
    if (i !== undefined) out[i >> 3] |= 1 << (i & 7);
  }
  return out;
}

/** A change to a file: each hunk's lines of the map's commit, `[from,
 * count]`, a count of 0 lines added after `from`, and the lines it writes
 * there; or `new`, a file the commit didn't have. */
export type Hunk = [from: number, count: number, added?: string[], removed?: string[], now?: number, nowCount?: number];

/** A line that's a comment or blank, which changes no test's outcome but
 * a reader of the sources' (`readsSources`), which runs on every change. */
const commentOnly = (line: string) => /^\s*(\/\/.*)?$/.test(line);

/** An item a pure addition may write that nothing written before refers
 * to, so it changes no test's outcome but through the functions changed to
 * use it, which their own hunks select: a `fn`, a type, a constant, a
 * `mod`. Not an `impl`, whose methods may be found where nothing names it,
 * a new inherent method shadowing a trait's; not a `use`, which may shadow
 * a glob's name; not a macro, which may be expanded where it's named. */
const unreferencedItem = /^\s*(?:#\[.*\]\s*)*(?:pub(?:\([^)]*\))?\s+)?(?:(?:unsafe|const|async|extern\s+"[^"]*")\s+)*(?:fn|struct|enum|union|const|static|type|mod)\b/;
const unreferencedOnly = (lines: string[]) =>
  !lines.some((line) => /^\s*(?:#\[.*\]\s*)*(?:pub(?:\([^)]*\))?\s+)?(?:(?:unsafe|default)\s+)*(?:impl|use|trait|macro)\b|macro_rules!/.test(line)) &&
  lines.some((line) => unreferencedItem.test(line));

/** The compiler's sources as the map's commit has them: a file's text, and
 * each line of `src/` that names `word`, as `git grep -w` finds it. */
export type Sources = {
  text: (file: string) => string | undefined;
  naming: (word: string) => { file: string; line: number }[];
  /** Of `names`, those code beyond the crate's own names anywhere, where a
   * method of that name may be a trait's: std's and rustc's, their
   * metadata and std's sources, and each dependency's sources
   * (`namedBeyondTheCrate`). None given: every one, as it can't say. */
  beyond?: (names: string[]) => Set<string>;
};

/** Of `names`, those std's, rustc's or a dependency's code names, a word
 * of it anywhere: the sysroot's crates' metadata, each target's, which
 * holds every item's name, rustc's internal crates' too, std's sources,
 * and the sources of each package Cargo resolves but the crate itself.
 * Where any can't be read, every one of `names`, as it can't say. About
 * 2 s, a single pass of each file's words. */
export function namedBeyondTheCrate(names: string[]): Set<string> {
  const wanted = new Set(names);
  const found = new Set<string>();
  if (wanted.size === 0) return found;
  const sysroot = Bun.spawnSync(["rustc", "--print", "sysroot"], { cwd: root, stdout: "pipe", stderr: "pipe" });
  const metadata = Bun.spawnSync(["cargo", "metadata", "--format-version", "1", "--offline"], { cwd: root, stdout: "pipe", stderr: "pipe" });
  if (sysroot.exitCode !== 0 || metadata.exitCode !== 0) return wanted;
  const sys = sysroot.stdout.toString().trim();
  const scan = (path: string) => {
    for (const m of readFileSync(path).toString("latin1").matchAll(/[A-Za-z_]\w*/g)) if (wanted.has(m[0])) found.add(m[0]);
  };
  const walk = (directory: string): void => {
    for (const entry of readdirSync(directory, { withFileTypes: true })) {
      const path = join(directory, entry.name);
      if (entry.isDirectory()) {
        if (entry.name !== "target" && entry.name !== ".git") walk(path);
      } else if (entry.name.endsWith(".rs")) scan(path);
    }
  };
  try {
    const rustlib = join(sys, "lib", "rustlib");
    for (const target of readdirSync(rustlib)) {
      const lib = join(rustlib, target, "lib");
      if (existsSync(lib) && statSync(lib).isDirectory()) for (const file of readdirSync(lib)) if (file.endsWith(".rmeta")) scan(join(lib, file));
    }
    walk(join(rustlib, "src", "rust", "library"));
    const packages = (JSON.parse(metadata.stdout.toString()) as { packages: { manifest_path: string }[] }).packages;
    for (const { manifest_path } of packages) if (resolve(dirname(manifest_path)) !== resolve(root)) walk(dirname(manifest_path));
  } catch {
    return wanted;
  }
  return found;
}

/** The sources at `commit`, by git. */
export function sourcesAt(commit: string): Sources {
  const git = (args: string[]) => Bun.spawnSync(["git", ...args], { cwd: root, stdout: "pipe", stderr: "pipe" });
  return {
    text: (file) => {
      const p = git(["show", `${commit}:${file}`]);
      return p.exitCode === 0 ? p.stdout.toString() : undefined;
    },
    naming: (word) =>
      git(["grep", "-n", "-w", "-e", word, commit, "--", ...compilerSourceDirs()])
        .stdout.toString()
        .split("\n")
        .flatMap((line) => {
          const m = /^[^:]+:([^:]+):(\d+):/.exec(line);
          return m ? [{ file: m[1]!, line: Number(m[2]) }] : [];
        }),
    beyond: namedBeyondTheCrate,
  };
}

/** A `const` or `static` item's declaration line, its name the second group. */
const constItem = /^\s*(?:pub(?:\([^)]*\))?\s+)?(const|static)\s+(?:mut\s+)?([A-Z_][A-Z0-9_]*)\s*:/;

/** A `use` item's first line. */
const useItem = /^\s*(?:pub(?:\([^)]*\))?\s+)?use\s+()/;

/** Of `lines` (1-based), the `const` or `static` item, or of `item` another
 * kind's, whose lines, from its first to its `;`, hold `from` to `to`: its
 * name and its lines. */
export function constHolding(lines: string[], from: number, to: number, item = constItem): { name: string; start: number; end: number } | undefined {
  for (let start = Math.min(from, lines.length); start >= 1; start--) {
    const m = item.exec(lines[start - 1]!);
    if (!m) continue;
    let depth = 0;
    for (let end = start; end <= lines.length; end++) {
      for (const c of lines[end - 1]!.replace(/\/\/.*$/, "").replace(/"(?:[^"\\]|\\.)*"/g, '""')) {
        if ("([{".includes(c)) depth++;
        else if (")]}".includes(c)) depth--;
        else if (c === ";" && depth === 0) return from >= start && to <= end ? { name: m[2]!, start, end } : undefined;
      }
    }
    return undefined;
  }
  return undefined;
}

/** Whether a new module's `text` is items nothing written before refers
 * to: nothing reaches it but through its `mod` line and the functions
 * changed to use it, whose own hunks select their tests, as a pure
 * addition's items (`unreferencedOnly`). Not where it writes an `impl`
 * of a type it doesn't declare, whose methods may be found where nothing
 * names it; a `trait`; a macro,
 * defined or invoked as an item, which may register what runs anywhere; a
 * `#[test]`; or a `static`, or a `const` whose initializer calls or
 * closes over code. */
export function unreferencedModule(text: string): boolean {
  const code = text.split("\n").filter((line) => !/^\s*\/\//.test(line));
  // Its own types: an `impl` of one of them, a trait's too, is found only
  // through a value of it, which only code changed to use it makes.
  const own = new Set(code.flatMap((line) => /^\s*(?:pub(?:\([^)]*\))?\s+)?(?:struct|enum|union|type)\s+(\w+)/.exec(line)?.[1] ?? []));
  const ofOwn = (line: string) => {
    const m = /^\s*(?:unsafe\s+)?impl\b(?:\s*<[^>]*>)?\s+(?:[\w:<>, '&]+\s+for\s+)?(?:[\w]+::)*(\w+)/.exec(line);
    return m !== null && own.has(m[1]!);
  };
  return !code.some(
    (line) =>
      (/^\s*(?:#\[.*\]\s*)*(?:pub(?:\([^)]*\))?\s+)?(?:(?:unsafe|default|auto)\s+)*impl\b/.test(line) && !ofOwn(line)) ||
      /^\s*(?:#\[.*\]\s*)*(?:pub(?:\([^)]*\))?\s+)?(?:(?:unsafe|auto)\s+)*(?:trait|static)\b|macro_rules!|#\[test\]|^\s*[\w:]+!\s*[({[]/.test(line) ||
      (/^\s*(?:pub(?:\([^)]*\))?\s+)?const\s+[A-Z_][A-Z0-9_]*\s*:/.test(line) && /[(|]/.test(line.slice(line.indexOf("=")))),
  );
}

/** `line` with an array type's length left out, `[&str; 18]` as `[&str; _]`. */
const withoutLength = (line: string) => line.replace(/;\s*\d+\s*\]/g, "; _]");

/** `line` with its visibility left out, `pub(super) fn f(` as `fn f(`. */
const withoutVisibility = (line: string) => line.replace(/\bpub(?:\([^)]*\))?\s+/g, "");

/** A line that declares an item named `name`: a function, a method of a
 * trait or an impl, a type, a constant, a module or a macro. */
const declaring = (name: string) => new RegExp(`\\b(?:fn|const|static|struct|enum|union|trait|type|mod|macro_rules!)\\s+${name}\\b`);

/** The prelude's functions, which a glob's call falls back to where the
 * crate's own of that name is no longer seen. */
const preludeFunctions = new Set(["drop"]);
/** A file's change since the map's commit: `file` its path there, the
 * map's, and its hunks in its lines there; `now` its path in the tree where
 * git found it renamed, and `similarity` how alike the two are, git's
 * percent (#2337). */
export type Change = { file: string; hunks: Hunk[] | "new"; now?: string; similarity?: number };

/** What a change runs: the files and cases it can affect, or all of them,
 * and why. */
export type Picked =
  | { kind: "picked"; files: string[]; cases: string[]; forms: string[]; functions: number[]; why: string }
  // `mapBehind`: the map can't place it as it is now, its lines changed
  // since the map's commit, its file new or moved too far (`mapLines`),
  // which a map recorded again answers; not a site it never could (#2349).
  | { kind: "all"; why: string; mapBehind?: true };

/** The test files of the corpus's cases, whose each case is a test of its
 * own: the corpus's, and the corpus built with std's iterator structs
 * alone, `ITERATOR_FORMS=corpus`, which a change to the iterators' code
 * runs (scripts/iteratorSlice.ts). */
export const caseFiles = ["test/corpus.test.ts", "test/iterator-forms.test.ts"] as const;

/** The test a compile for case `name` (`x.rs`) is, of the test file `file`
 * runs it: the corpus's `test/corpus/x.rs`, std's iterator structs'
 * `test/iterator-forms.test.ts test/corpus/x.rs`; of another file, none. */
export function caseUnit(file: string, name: string): string | undefined {
  if (file === "test/corpus.test.ts") return `test/corpus/${name}`;
  if (file === "test/iterator-forms.test.ts") return `test/iterator-forms.test.ts test/corpus/${name}`;
  return undefined;
}

const git = (args: string[], cwd = root) => {
  const p = Bun.spawnSync(["git", ...args], { cwd, stdout: "pipe", stderr: "pipe" });
  return { code: p.exitCode, out: p.stdout.toString() };
};

/** The toolchain a checkout builds with, `rustc -vV`'s commit hash, its
 * `rust-toolchain.toml`'s: what a map is of besides its commit. */
export function toolchainOf(checkout = root): string {
  const said = Bun.spawnSync(["rustc", "-vV"], { cwd: checkout, stdout: "pipe", stderr: "pipe" }).stdout.toString();
  return /^commit-hash: (\S+)$/m.exec(said)?.[1] ?? "unknown";
}

/** The file of the map of `commit` with `toolchain` in `store`, of this
 * layout: its name is its key. */
export function mapFile(store: string, commit: string, toolchain: string): string {
  return join(store, `${commit}${keySuffix(toolchain)}`);
}

/** What a map's file is named after its commit: the toolchain's and the
 * layout's. */
const keySuffix = (toolchain: string) => `-${toolchain.slice(0, 12)}-v${mapVersion}.json`;

/** The test files the map records, each a test of its own: the push
 * suite's (scripts/shard.ts), but the corpus's, whose each case is one. */
export function mappedFiles(): string[] {
  return pushSuiteFiles(readdirSync(join(root, "test"))).filter((file) => !(caseFiles as readonly string[]).includes(file));
}

/** The mapped files recorded in parts, as the push suite runs them,
 * `RUST_JS_CASES=i/n`, each part's compiles the file's: the metamorphic
 * sample, some 4 min on one core. */
export const partedFiles = ["test/metamorphic.test.ts"];

/** `path`'s code as readsOf reads it: its lines but its comments', which
 * name what they're about. */
function codeOf(path: string): string {
  return readFileSync(path, "utf8")
    .split("\n")
    .filter((line) => !/^\s*(\/\/|\/?\*)/.test(line))
    .join("\n");
}

/** An exported function whose path is a folder of the root's and its own
 * parameter: `export const knownDir = (list: KnownList): string =>
 * join(root, "test", "known", list)`, its name, its parameter, the folder's
 * parts and the path's last argument. */
const pathFunction = /export const (\w+) = \((\w+)\b[^)]*\)(?:\s*:\s*\w+)?\s*=>\s*join\(\s*root\s*,\s*((?:["'`][^"'`$\n]+["'`]\s*,\s*)+)(\w+)\s*\)/g;

/** An exported function's signature, its name, its first parameter and the
 * rest, `export function knownText(list: KnownList, dir = knownDir(list)) {`:
 * a path function's gives its parameter on, its part, read where it's
 * called, not here. */
const relayedSignature = /export function (\w+)\((\w+)\b([^{]*)\{/g;

let pathFunctionsFound: Map<string, string> | undefined;

/** The checkout's path functions, each with its folder: one whose path is
 * the folder and its parameter (`pathFunction`), `knownDir`'s
 * `test/known`, and one that gives its parameter to another in its
 * signature, `knownText(list, dir = knownDir(list))`. Each reads the folder
 * of the part it's called with, `knownText("metamorphic")`
 * `test/known/metamorphic`, where it's called, not in every module that
 * imports the one that defines it: an edit of one known list ran every test
 * that imports test/support.ts, all 402 (#2284). Of the test and script
 * modules, read once. */
export function pathFunctions(): Map<string, string> {
  if (pathFunctionsFound !== undefined) return pathFunctionsFound;
  const found = new Map<string, string>();
  const texts = ["test", "scripts"].flatMap((folder) =>
    readdirSync(join(root, folder))
      .filter((name) => name.endsWith(".ts"))
      .map((name) => codeOf(join(root, folder, name))),
  );
  for (const text of texts) {
    for (const [, name, param, parts, last] of text.matchAll(pathFunction)) {
      if (last === param) found.set(name, join(...[...parts.matchAll(/["'`]([^"'`$\n]+)["'`]/g)].map(([, part]) => part)));
    }
  }
  for (let more = true; more; ) {
    more = false;
    for (const text of texts) {
      for (const [, name, param, signature] of text.matchAll(relayedSignature)) {
        if (found.has(name)) continue;
        const given = [...found].find(([callee]) => new RegExp(`\\b${callee}\\(\\s*${param}\\s*\\)`).test(signature));
        if (given !== undefined) {
          found.set(name, given[1]);
          more = true;
        }
      }
    }
  }
  pathFunctionsFound = found;
  return found;
}

/** What `file`, a test's module, reads of the checkout as text, as far as
 * its source and the source of each module of the checkout's it imports
 * say (what it imports is its code, not text it reads): each path it names, `"scripts/crate-corpus.ts"`, `join(root, "src",
 * "runtime.rs")`, `${root}/std`, `"../crates/rust-js-lower/src/lib.rs"`, each a file or a
 * folder that's there, said from the checkout's root. A test of a file's
 * text, ci.test.ts's of `scripts/`, the architecture's of `src/`, fails
 * on a change it runs no code of: what it reads picks it. Bun binds a
 * test's `node:fs` imports as it links them, so its reads can't be seen as
 * they're made; and a path a test makes some other way isn't here. */
export function readsOf(file: string, followed = true, seen = new Set<string>()): Set<string> {
  const reads = new Set<string>();
  const path = resolve(root, file);
  if (seen.has(path) || !existsSync(path) || !statSync(path).isFile()) return reads;
  seen.add(path);
  // Its code, not its comments, which name what they're about; a path
  // function's own path read where it's called, not here (`pathFunctions`).
  const text = codeOf(path)
    .replace(pathFunction, (whole, name, param, _parts, last) => (last === param ? `export const ${name} = 0;` : whole))
    .replace(relayedSignature, (whole, name) => (pathFunctions().has(name) ? `export function ${name}() {` : whole));
  // A folder only where it's the root's for sure, `join(root, "src")`: a
  // literal `"src/"` is as often an app's of a fixture's.
  const there = (candidate: string, folder = true) => {
    const at = resolve(root, candidate);
    if (at.startsWith(`${root}/`) && !at.includes("/node_modules/") && existsSync(at) && (folder || statSync(at).isFile())) reads.add(relative(root, at));
  };
  // `join(root, "a", "b")`, `resolve(root, "a/b")`, and `${root}/a/b`: of
  // the checkout's root, but where the file declares a `root` of its own,
  // a fixture's; one it makes of its own folder, `join(import.meta.dir,
  // "..")`, is the checkout's where that's where it leads (adr.test.ts's).
  const parts = (args: string) => [...args.matchAll(/["'`]([^"'`$\n]+)["'`]/g)].map(([, part]) => part);
  const declared = /\b(?:const|let)\s+root\s*=\s*([^;\n]*)/.exec(text)?.[1];
  const ofItsFolder = declared === undefined ? undefined : /^(?:join|resolve)\(\s*import\.meta\.dir\s*((?:,\s*["'`][^"'`$\n]+["'`]\s*)*)\)$/.exec(declared.trim())?.[1];
  const ownRoot = declared !== undefined && !(ofItsFolder !== undefined && resolve(dirname(path), ...parts(ofItsFolder)) === root);
  // And `join(import.meta.dir, "a")`, of its own folder: mutations.ts's
  // lists, `join(import.meta.dir, "mutations")`.
  for (const [, args] of text.matchAll(/\b(?:join|resolve)\(\s*import\.meta\.dir\s*,\s*((?:["'`][^"'`$\n]+["'`]\s*,?\s*)+)\)/g)) {
    there(relative(root, resolve(dirname(path), ...parts(args))));
  }
  if (!ownRoot) for (const [, args] of text.matchAll(/\b(?:join|resolve)\(\s*root\s*,\s*((?:["'`][^"'`$\n]+["'`]\s*,?\s*)+)/g)) {
    there(join(...[...args.matchAll(/["'`]([^"'`$\n]+)["'`]/g)].map(([, part]) => part)));
  }
  if (!ownRoot) for (const [, rest] of text.matchAll(/\$\{root\}\/([\w./-]+)/g)) there(rest);
  // A path from the root, its first part the root's: `"scripts/x.ts"`.
  const tops = new Set(readdirSync(root));
  for (const [, literal] of text.matchAll(/["'`]((?:\.\.\/)?[\w.-]+\/[\w./-]*)["'`]/g)) {
    if (literal.startsWith("../")) there(join(dirname(relative(root, path)), literal), false);
    else if (tops.has(literal.split("/")[0])) there(literal, false);
  }
  // A glob of the checkout's files, `new Bun.Glob("test/corpus/*.js")`, its
  // first part the root's, said as the glob, which the selector matches:
  // readability's ratchet reads the corpus's JS so, which a case's bless
  // changes (#1188's follow-up: main went red three times in a day where a
  // change blessed snapshots and plain check:fast didn't run it). Not one
  // of a fixture's, `"*.rs"`, whose first part is a pattern.
  for (const [, glob] of text.matchAll(/\bGlob\(\s*["'`]([\w.*/-]+)["'`]\s*\)/g)) {
    const first = glob.split("/")[0];
    if (glob.includes("/") && !first.includes("*") && tops.has(first)) reads.add(glob);
  }
  // A path function called with its part, `knownText("metamorphic")`:
  // that path, `test/known/metamorphic`; called with what isn't a literal,
  // `knownText(list)` of a helper or a loop's, its whole folder, as it may
  // be any of its parts (an unselected reader turns main red after the push).
  for (const [name, folder] of pathFunctions()) {
    for (const [, part] of text.matchAll(new RegExp(`(?<!function )\\b${name}\\(\\s*(?:["'\`]([\\w.-]+)["'\`])?`, "g"))) {
      there(part === undefined ? folder : join(folder, part));
    }
  }
  // The compiler's sources, each crate's, where it lists their files
  // (scripts/compiler-crates.ts), as the architecture's tests do.
  if (/(?<!function )\bcompilerRustFiles\(/.test(text)) for (const dir of compilerSourceDirs()) reads.add(dir);
  // What each module of the checkout's it imports reads.
  if (followed) for (const [, spec] of text.matchAll(/\b(?:from|import)\s*\(?\s*["'](\.{1,2}\/[^"']+)["']/g)) {
    const base = resolve(dirname(path), spec);
    const found = [base, `${base}.ts`, `${base}.js`].find((candidate) => existsSync(candidate) && statSync(candidate).isFile());
    if (found && found.startsWith(root) && !found.includes("node_modules")) {
      for (const read of readsOf(relative(root, found), true, seen)) reads.add(read);
    }
  }
  return reads;
}

/** Whether `file` reads the compiler's sources as text, as the
 * architecture's tests do: such a test can fail on a change it runs no
 * compiled code of, so it runs on every change. */
export function readsSources(file: string): boolean {
  const dirs = compilerSourceDirs();
  return [...readsOf(file)].some((read) => dirs.some((dir) => read === dir || read.startsWith(`${dir}/`) || dir.startsWith(`${read}/`)));
}

/** What a change of `changes` to the compiler's code runs, by `map`, the
 * functions it touches: of `files`, the push suite's test files, and
 * `cases`, the corpus's; or `all`, where a change isn't a function's code.
 * What every other path runs is the selector's (scripts/select.ts). */
export function pick(map: TestMap, changes: Change[], files: string[], cases: string[], sources: Sources = sourcesAt(map.commit)): Picked {
  const touched = new Set<number>();
  // Each function's spans, by file, and the lines from its first to its last.
  const spans = new Map<string, { at: number; from: number; to: number }[]>();
  const extent = map.functions.map(({ spans: of }) => [of[0]?.[0] ?? 0, of.at(-1)?.[1] ?? -1] as const);
  map.functions.forEach(({ file, spans: of }, at) => {
    const list = spans.get(file) ?? [];
    for (const [from, to] of of) list.push({ at, from, to });
    spans.set(file, list);
  });
  // The functions whose counted code holds `line` of `file`.
  const holding = (file: string, line: number) => (spans.get(file) ?? []).filter(({ from, to }) => from <= line && line <= to).map(({ at }) => at);
  const within = (outer: number, inner: number) => extent[outer][0] <= extent[inner][0] && extent[inner][1] <= extent[outer][1];
  // The function whose counted code is the first of `file`'s after `line`,
  // within the few lines a signature spans: the one a signature line is of.
  const following = (file: string, line: number) => {
    const after = (spans.get(file) ?? []).filter(({ from }) => from > line && from <= line + 8).sort((a, b) => a.from - b.from);
    return after[0]?.at;
  };
  // Whether lines added between `line` and the next are in a function's
  // code, as the counting below asks of a pure addition.
  const insideAt = (file: string, line: number) => {
    const before = holding(file, line);
    const after = holding(file, line + 1);
    return before.some((a) => after.some((b) => a === b || within(a, b) || within(b, a)));
  };
  // A change of a `const` or `static`'s value, its lines within its item,
  // its type the same or an array's length alone, `HELD_FLIPPED: [&str;
  // 18]` to `19` with an entry added: the functions that name it, which are
  // where its value is read, it being inlined there. A use of it outside
  // any function's code, another constant's, is none of theirs: undefined,
  // and so everything (ADR 1101, amended).
  const constValue = (file: string, from: number, count: number, added: string[], removed: string[] | undefined): number[] | undefined => {
    if (removed === undefined || added.some((line) => nestedItem.test(line) && !constItem.test(line)) || added.some((line) => /\bfn\b|\|[^|]*\|/.test(line))) return undefined;
    const text = sources.text(file);
    if (text === undefined) return undefined;
    const lines = text.split("\n");
    const item = constHolding(lines, Math.max(from, 1), count === 0 ? from + 1 : from + count - 1);
    if (item === undefined || (count === 0 && from < item.start)) return undefined;
    // Its declaration, where the hunk has it: the same but an array's length.
    const declared = item.start >= from && item.start < from + count ? removed[item.start - from] : undefined;
    if (declared !== undefined && !added.some((line) => withoutLength(line) === withoutLength(declared))) return undefined;
    if (added.filter((line) => constItem.test(line)).length > (declared === undefined ? 0 : 1)) return undefined;
    const readers: number[] = [];
    const texts = new Map<string, string[]>();
    const lineOf = (at: string, line: number) => {
      if (!texts.has(at)) texts.set(at, (sources.text(at) ?? "").split("\n"));
      return texts.get(at)![line - 1] ?? "";
    };
    for (const site of sources.naming(item.name)) {
      if (site.file === file && site.line >= item.start && site.line <= item.end) continue;
      // A comment that names it, or a `use` of it, reads nothing.
      if (/^\s*(?:\/\/|(?:pub(?:\([^)]*\))?\s+)?use\b)/.test(lineOf(site.file, site.line))) continue;
      const found = holding(site.file, site.line);
      if (found.length === 0) return undefined;
      readers.push(...found);
    }
    return readers;
  };
  // Names added to a `use`, `imports::{Import, ImportNames}` for
  // `imports::{Import}`, of what no item at the map's commit declared: a
  // name that didn't exist shadows none a glob gave, so it changes nothing
  // but the code changed to use it (4ba38fa64's crates/rust-js-program/src/js.rs:43).
  const newNamesUsed = (file: string, from: number, count: number, added: string[], removed: string[] | undefined): boolean => {
    if (removed === undefined || count === 0 || removed.length !== added.length) return false;
    const text = sources.text(file);
    if (text === undefined || constHolding(text.split("\n"), from, from + count - 1, useItem) === undefined) return false;
    const words = (lines: string[]) => new Set(lines.join(" ").replace(/\/\/.*$/gm, "").match(/\b[A-Za-z_]\w*\b/g) ?? []);
    const before = words(removed);
    const after = words(added);
    if ([...before].some((word) => !after.has(word))) return false;
    const fresh = [...after].filter((word) => !before.has(word));
    const declares = (word: string) => new RegExp(`\\b(?:struct|enum|union|trait|type|fn|const|static|mod|macro_rules!)\\s+${word}\\b`);
    return fresh.length > 0 && fresh.every((word) => sources.naming(word).every((site) => !declares(word).test((sources.text(site.file) ?? "").split("\n")[site.line - 1] ?? "")));
  };
  // A line of a function's signature, from its `fn` to its body's `{`.
  const inSignature = (of: string[], line: number) => {
    for (let at = line; at >= Math.max(1, line - 12); at--) {
      const text = of[at - 1] ?? "";
      if (/\bfn\s+\w+/.test(text)) return true;
      if (at < line && (commentOnly(text) || /[;{}]\s*$/.test(text.replace(/\/\/.*$/, "")))) return false;
    }
    return false;
  };
  // The files a private item of `file` is seen in: its module's, and the
  // modules' in it, `src/a/b.rs` and `src/a/b/**`; the crate's of main.rs.
  const moduleOf = (file: string) => {
    const crate = crateOf(file);
    const under = crate?.rootFile === file ? `${crate.src}/` : file.replace(/(?:\/mod)?\.rs$/, "/");
    return (at: string) => at === file || at.startsWith(under);
  };
  const texts = new Map<string, string[]>();
  const linesOf = (at: string) => {
    if (!texts.has(at)) texts.set(at, (sources.text(at) ?? "").split("\n"));
    return texts.get(at)!;
  };
  // A line's kind, then its item's name (#2288, m1-lisa's design, Tu's
  // heuristic; the nightly and scripts/selector-audit.ts the backstop).
  // Whether each of `lines` is a `mod x;` or an `extern crate` alone: what
  // it brings in changes only where a function using it changed too,
  // which selects its tests, and a new module's file is new since the map,
  // its own rule. Not a `use`, which may change what a type's field names
  // with no function changed: `useChanged`'s and `useNamed`'s, above.
  const useLine = /^\s*(?:pub(?:\([^)]*\))?\s+)?(?:use\s|mod\s+\w+\s*;|extern\s+crate\s)/;
  const modLine = /^\s*(?:pub(?:\([^)]*\))?\s+)?(?:mod\s+\w+\s*;|extern\s+crate\s)/;
  const useOrMod = (lines: string[]) => lines.some((line) => modLine.test(line)) && lines.every((line) => commentOnly(line) || modLine.test(line));
  const itemHeader = /^(\s*)(?:#\[.*\]\s*)*(?:pub(?:\([^)]*\))?\s+)?(?:(?:unsafe|default|auto|async|const|extern\s+"[^"]*")\s+)*(?:enum|struct|union|trait|type|const|static|fn)\s+([A-Za-z_]\w*)/;
  const implHeader = /^(\s*)(?:unsafe\s+)?impl\b(?:\s*<[^>]*>)?\s+(?:[^{]*?\bfor\s+)?(?:[\w]+::)*([A-Za-z_]\w*)/;
  // The item `line` of `file` writes or is in: its own header, else the
  // nearest header above it indented less.
  const itemOf = (file: string, line: number): string | undefined => {
    const lines = linesOf(file);
    const indent = (text: string) => text.match(/^\s*/)![0].length;
    const own = lines[line - 1] ?? "";
    const header = (text: string) => itemHeader.exec(text)?.[2] ?? implHeader.exec(text)?.[2];
    if (header(own) !== undefined) return header(own);
    for (let at = line - 1; at >= 1; at--) {
      const text = lines[at - 1] ?? "";
      if (text.trim() === "" || indent(text) >= indent(own)) continue;
      const name = header(text);
      if (name !== undefined) return name;
    }
    return undefined;
  };
  // Of an item outside any function, the counted functions whose source
  // names it as a word, in any of the compiler's crates; a name too short
  // to search, or one more than 60% of its crate's counted functions name,
  // everything, said by name.
  const crateDirOf = (at: string) => at.match(/^crates\/[^/]+\//)?.[0] ?? at.split("/")[0]!;
  const byName = (file: string, line: number): number[] | string => {
    const name = itemOf(file, line);
    if (name === undefined) return "its item isn't found";
    if (name.length < 3) return `its item, \`${name}\`, is too short a name to search`;
    const found = new Set<number>();
    for (const site of sources.naming(name)) {
      const held = holding(site.file, site.line);
      // Named by another item, a constant's or a static's initializer,
      // even one inside a function, or a type's field outside any: what
      // names that item isn't searched.
      const text = linesOf(site.file)[site.line - 1] ?? "";
      const initializer = /^\s*(?:pub(?:\([^)]*\))?\s+)?(?:const|static)\s+(?:mut\s+)?([A-Za-z_]\w*)/.exec(text)?.[1];
      const other = initializer !== undefined ? initializer !== name : held.length === 0 && !commentOnly(text) && !useLine.test(text) && itemOf(site.file, site.line) !== name;
      if (other) return `\`${name}\` is named by another item, at ${site.file}:${site.line}`;
      for (const f of held) found.add(f);
    }
    const crate = crateDirOf(file);
    const own = map.functions.filter(({ file: at }) => crateDirOf(at) === crate).length;
    const named = [...found].filter((f) => crateDirOf(map.functions[f]!.file) === crate).length;
    if (own > 0 && named > 0.6 * own) return `its item, \`${name}\`, is named by ${named} of its crate's ${own} counted functions`;
    return [...found];
  };
  // A function's signature changed, its parameters' types say, and new
  // items written before it, which nothing named before (as a pure
  // addition's, `unreferencedOnly`): what compiles otherwise is its body
  // and each place that names it, a call or a path, the functions that
  // hold them; one named outside any function's code, everything.
  const signatureChanged = (file: string, from: number, count: number, added: string[], removed: string[] | undefined): number[] | undefined => {
    if (removed === undefined || count === 0 || sources.text(file) === undefined) return undefined;
    const code = (lines: string[]) => lines.filter((line) => !commentOnly(line)).map((line) => line.replace(/\/\/.*$/, ""));
    const before = code(removed);
    const name = before.join(" ").match(/\bfn\s+(\w+)/g);
    if (name === null || name.length !== 1 || !/\{\s*$/.test(before.at(-1) ?? "") || before.slice(0, -1).some((line) => /[;{}]/.test(line))) return undefined;
    const fn = name[0]!.replace(/^fn\s+/, "");
    const after = code(added);
    const at = after.findIndex((line) => new RegExp(`\\bfn\\s+${fn}\\b`).test(line));
    if (at < 0 || !/\{\s*$/.test(after.at(-1) ?? "") || after.slice(at, -1).some((line) => /[;{}]/.test(line))) return undefined;
    const items = after.slice(0, at);
    if (items.length > 0 && !unreferencedOnly(items)) return undefined;
    const own = following(file, from + count - 1);
    if (own === undefined) return undefined;
    // A private one's names are its module's; a `pub` one's, anywhere.
    const seen = /^\s*pub\b/.test(before.find((line) => /\bfn\s/.test(line))!) ? () => true : moduleOf(file);
    const readers = [own];
    for (const site of sources.naming(fn)) {
      if (!seen(site.file)) continue;
      const of = linesOf(site.file);
      if (commentOnly(of[site.line - 1] ?? "") || constHolding(of, site.line, site.line, useItem) !== undefined) continue;
      const found = holding(site.file, site.line);
      if (found.length === 0 && !inSignature(of, site.line)) return undefined;
      readers.push(...found);
    }
    return readers;
  };
  // A private `use` changed, its names added, taken out or moved, and no
  // glob: what it can change is how the code in its module's scope resolves
  // a name, a path's or a method's by a trait it brings, which only a
  // function's code calls (a constant's is const, which calls no trait's
  // method). So every function of the file and of its modules', unless one
  // of its words is named in their code outside any function, a type's
  // field, an `impl`'s header, a constant's value, an item a macro makes,
  // where it's everything. A `pub` one's reach is wherever it's imported,
  // and a glob's names no word says: everything (ADR 1101, amended).
  // A `use` or `pub use` changed, a re-export added say, which no other
  // rule takes: each place its names are used, the function holding it or
  // the functions reaching the item it's in, by the item layer (#2288).
  // None where a use is in neither.
  const useNamed = (file: string, from: number, count: number, added: string[], removed: string[] | undefined): number[] | undefined => {
    if (map.items === undefined || removed === undefined) return undefined;
    const written = [...added, ...removed].filter((line) => !commentOnly(line)).map((line) => line.replace(/\/\/.*$/, ""));
    if (written.length === 0 || !written.every((line) => /^\s*(?:pub(?:\([^)]*\))?\s+)?(?:use\s+)?[\w\s:{},;*]*$/.test(line))) return undefined;
    if (written.some((line) => line.includes("*"))) return undefined;
    const text = sources.text(file);
    if (text === undefined) return undefined;
    const lines = text.split("\n");
    // The lines it replaces, each of a `use`; of an addition, its own lines
    // are what's read, above.
    for (let line = from; line < from + count; line++) if (!commentOnly(lines[line - 1] ?? "") && constHolding(lines, line, line, useItem) === undefined && !/^\s*(?:pub(?:\([^)]*\))?\s+)?use\b/.test(lines[line - 1] ?? "")) return undefined;
    // Of an addition: whole `use`s, or lines inside a `use`'s list.
    const code = added.filter((line) => !commentOnly(line));
    if (count === 0 && !code.every((line) => /^\s*(?:pub(?:\([^)]*\))?\s+)?use\b/.test(line)) && constHolding(lines, from, from + 1, useItem) === undefined) return undefined;
    // The names it brings in: each path's last, or its alias, not the
    // modules on the way, `ExprKind` of `use crate::js::ExprKind;`.
    const reserved = new Set(["use", "pub", "crate", "self", "super", "in", "as", "Self"]);
    const joined = written.join(" ").replace(/\bpub(?:\([^)]*\))?/g, " ");
    const words = new Set(
      [...joined.matchAll(/\b([A-Za-z_]\w*)\s*(?=[,};]|\s+as\b|\s*$)|\bas\s+([A-Za-z_]\w*)/g)].map((m) => m[2] ?? m[1]!).filter((word) => !reserved.has(word)),
    );
    const found = new Set<number>();
    for (const word of words) {
      for (const site of sources.naming(word)) {
        const held = holding(site.file, site.line);
        if (held.length > 0) {
          for (const f of held) found.add(f);
          continue;
        }
        const of = linesOf(site.file);
        if (commentOnly(of[site.line - 1] ?? "") || inUnitTests(map.items, site.file, site.line) || constHolding(of, site.line, site.line, useItem) !== undefined || /^\s*(?:pub(?:\([^)]*\))?\s+)?(?:use\b|mod\s+\w+\s*;)/.test(of[site.line - 1] ?? "")) continue;
        const byItem = touchedByItem(map.items, site.file, site.line);
        if (byItem === undefined) return undefined;
        for (const f of byItem) found.add(f);
      }
    }
    return [...found];
  };
  const useChanged = (file: string, from: number, count: number, added: string[], removed: string[] | undefined): number[] | undefined => {
    if (removed === undefined) return undefined;
    const text = sources.text(file);
    if (text === undefined) return undefined;
    const lines = text.split("\n");
    const privateUse = (at: number) => {
      const item = constHolding(lines, at, at, useItem);
      return item !== undefined && !/^\s*pub\b/.test(lines[item.start - 1]!);
    };
    const written = [...added, ...removed].filter((line) => !commentOnly(line)).map((line) => line.replace(/\/\/.*$/, ""));
    if (written.length === 0 || written.some((line) => /\bpub\b|\bextern\b/.test(line) || !/^\s*(?:use\s+)?[\w\s:{},;]*$/.test(line))) return undefined;
    for (let line = from; line < from + count; line++) if (!commentOnly(lines[line - 1] ?? "") && !privateUse(line)) return undefined;
    const addedCode = added.filter((line) => !commentOnly(line)).join(" ");
    // Lines added between two: inside one `use`'s list, or whole `use`s.
    if (count === 0 && !(privateUse(from) && privateUse(from + 1) && constHolding(lines, from, from + 1, useItem) !== undefined) && !/^(?:\s*use\s+[\w\s:{},]+;)+\s*$/.test(addedCode)) return undefined;
    const keywords = new Set(["use", "crate", "self", "super", "in", "as", "Self"]);
    const words = new Set(written.join(" ").match(/\b[A-Za-z_]\w*\b/g)?.filter((word) => !keywords.has(word)) ?? []);
    const inScope = moduleOf(file);
    const scoped = map.functions.flatMap(({ file: at }, f) => (inScope(at) ? [f] : []));
    for (const word of words) {
      for (const site of sources.naming(word)) {
        if (!inScope(site.file) || holding(site.file, site.line).length > 0) continue;
        const of = linesOf(site.file);
        if (commentOnly(of[site.line - 1] ?? "") || constHolding(of, site.line, site.line, useItem) !== undefined || inSignature(of, site.line)) continue;
        return undefined;
      }
    }
    return scoped;
  };
  // A visibility change that can't change what a name resolves to: of a
  // function declared once in the sources, never by a line the change adds,
  // and not the prelude's; of a method, `self` among its parameters, whose
  // name no code beyond the crate names either, so no trait's method,
  // std's, rustc's or a dependency's, can be found in its place.
  let visibilityOnly = 0;
  const addedLines = changes.flatMap(({ hunks }) => (hunks === "new" ? [] : hunks.flatMap(([, , added = []]) => added)));
  let beyond: Set<string> | undefined;
  const namedBeyond = (name: string) => {
    if (beyond === undefined) {
      const names = [...new Set(changes.flatMap(({ hunks }) => (hunks === "new" ? [] : hunks.flatMap(([, , , removed = []]) => removed.flatMap((line) => [...line.matchAll(/\bfn\s+(\w+)/g)].map((m) => m[1]!))))))];
      beyond = sources.beyond?.(names) ?? new Set(names);
    }
    return beyond.has(name);
  };
  const visibilityAlone = (file: string, from: number, removed: string[]): boolean => {
    const name = removed.join(" ").match(/\bfn\s+(\w+)/)?.[1];
    if (name === undefined || preludeFunctions.has(name)) return false;
    const lines = (sources.text(file) ?? "").split("\n");
    const start = from - 1 + removed.findIndex((line) => /\bfn\s+\w+/.test(line));
    let signature = "";
    for (let at = start; at < Math.min(lines.length, start + 16); at++) {
      signature += ` ${lines[at]!.replace(/\/\/.*$/, "")}`;
      if (/[{;]\s*$/.test(lines[at]!)) break;
    }
    if (!signature.includes(`fn ${name}`)) return false;
    if (/[(,]\s*(?:&\s*(?:'\w+\s+)?)?(?:mut\s+)?self\b/.test(signature) && namedBeyond(name)) return false;
    const declares = declaring(name);
    const declared = sources.naming(name).filter((site) => declares.test(linesOf(site.file)[site.line - 1] ?? ""));
    const own = removed.filter((line) => declares.test(line)).length;
    return declared.length === 1 && own === 1 && addedLines.filter((line) => declares.test(line)).length === 1;
  };
  for (const { file, hunks } of changes) {
    const sites = map.includes[file];
    if (sites !== undefined && hunks !== "new") {
      for (const { file: at, line } of sites) {
        const found = holding(at, line);
        if (found.length === 0) return { kind: "all", why: `${file} is read at ${at}:${line}, outside any function's counted code` };
        for (const f of found) touched.add(f);
      }
      continue;
    }
    if (!isCompilerSource(file) || !file.endsWith(".rs")) return { kind: "all", why: `${file} isn't the compiler's code` };
    if (hunks === "new") {
      // A new module of items nothing referred to: its `mod` line, a pure
      // addition, and the functions changed to use it select its tests.
      const text = existsSync(join(root, file)) ? readFileSync(join(root, file), "utf8") : undefined;
      if (text !== undefined && unreferencedModule(text)) continue;
      // What moved there, by name, and what reaches the items it defines,
      // where the map knows them (#2288).
      const moved = map.items === undefined || text === undefined ? undefined : touchedByNewFile(map.items, text);
      if (moved === undefined || typeof moved === "string") return { kind: "all", why: `${file} is new since the map's commit${typeof moved === "string" ? `: ${moved}` : ""}` };
      for (const f of moved) touched.add(f);
      continue;
    }
    for (const [from, count, added = [], removed] of hunks) {
      // Comments alone, or blank lines, change nothing a test runs.
      if (removed !== undefined && [...added, ...removed].every(commentOnly)) continue;
      // A pure addition of items nothing referred to before, a `mod`, a
      // `fn` or a type, outside any function's code: nothing until a
      // function is changed to use it, whose own hunk selects its tests.
      if (count === 0 && removed !== undefined && unreferencedOnly(added) && !insideAt(file, from)) continue;
      // A function's signature line whose change is only its visibility,
      // or its docs, `pub(super) fn f(` for `fn f(`: that function's change.
      // Of a function whose name nothing else in the sources declares,
      // nor the change adds, nothing: if it compiles, each call and path
      // resolves to it as before, as no other item of that name can be
      // found in its place; of a method, rustc's method probe passing over
      // one it can't see for a trait's method of its name, where no code
      // beyond the crate names it (#2162).
      if (removed !== undefined && count > 0 && removed.length === added.length && removed.length > 0) {
        const same = removed.every((line, i) => withoutVisibility(line) === withoutVisibility(added[i]!) || (commentOnly(line) && commentOnly(added[i]!)));
        const signature = removed.some((line) => /\bfn\s+\w+/.test(line));
        if (signature && same && visibilityAlone(file, from, removed)) {
          visibilityOnly++;
          continue;
        }
        const next = signature && same ? following(file, from + count - 1) : undefined;
        if (next !== undefined) {
          touched.add(next);
          continue;
        }
      }
      if (holding(file, from).length === 0 && newNamesUsed(file, from, count, added, removed)) continue;
      const resolving = holding(file, from).length === 0 ? (useChanged(file, from, count, added, removed) ?? signatureChanged(file, from, count, added, removed) ?? useNamed(file, from, count, added, removed)) : undefined;
      if (resolving !== undefined) {
        for (const f of resolving) touched.add(f);
        continue;
      }
      const readers = holding(file, from).length === 0 ? constValue(file, from, count, added, removed) : undefined;
      if (readers !== undefined) {
        for (const f of readers) touched.add(f);
        continue;
      }
      // An item it writes inside a function, an `impl` say, may be used
      // where the function isn't run.
      const item = added.find((line) => nestedItem.test(line));
      // A constant or a static whose value alone changed, its name the
      // same: the functions holding it, and what reaches it where the item
      // layer knows (#2288).
      const valued = (line: string) => line.match(/^\s*(?:pub(?:\([^)]*\))?\s+)?(?:const|static)\s+(?:mut\s+)?([A-Za-z_]\w*)\s*:/)?.[1];
      const written = added.filter((line) => nestedItem.test(line));
      if (item !== undefined && map.items !== undefined && removed !== undefined && written.every((line) => valued(line) !== undefined && removed.some((was) => valued(was) === valued(line)))) {
        for (const f of holding(file, from)) touched.add(f);
        for (const f of touchedByItem(map.items, file, from) ?? []) touched.add(f);
        continue;
      }
      // An `impl`'s header rewritten, its bounds, its trait or its `Self`:
      // its methods and its trait's referencers, by the layer (#2288).
      const header = map.items?.impls.find((i) => i.file === file && i.header[0] <= from && from + Math.max(count, 1) - 1 <= i.header[1]);
      if (item !== undefined && header !== undefined && written.every((line) => /^\s*(?:unsafe\s+)?impl\b/.test(line)) && removed?.some((line) => /^\s*(?:unsafe\s+)?impl\b/.test(line))) {
        for (const f of header.refs) touched.add(f);
        continue;
      }
      if (item !== undefined) {
        // `use`s and `mod x;` lines, and the crate's unit tests, select no
        // test file; an item, the functions that name it (#2288).
        if (useOrMod([...added, ...(removed ?? [])]) || (map.items !== undefined && inUnitTests(map.items, file, from))) continue;
        const named = byName(file, from);
        if (typeof named === "string") return { kind: "all", why: `${file}'s change at line ${from} writes an item, \`${item.trim()}\`: ${named}` };
        for (const f of named) touched.add(f);
        continue;
      }
      if (count === 0) {
        // Lines added between two: in a function where each is counted
        // code of one function or of one nested in the other, as a closure
        // or the initializer of a static is, so what's added is in the
        // outer one's body.
        const before = holding(file, from);
        const after = holding(file, from + 1);
        const inside = before.some((a) => after.some((b) => a === b || within(a, b) || within(b, a)));
        // Methods added to an inherent `impl`, between two of its own: new
        // functions, reached by the functions changed to call them, but
        // where one shadows a trait's method of its name, each function
        // that names it (#2288).
        const body = map.items === undefined || inside ? undefined : implBodyAt(map.items, file, from);
        const newFns = [...added.join("\n").matchAll(/\bfn\s+([A-Za-z_]\w*)/g)].map((m) => m[1]!);
        if (body !== undefined && !body.trait && newFns.length > 0 && removed !== undefined && removed.length === 0 && !added.some((line) => nestedItem.test(line) && !/\bfn\b/.test(line))) {
          for (const name of newFns) for (const site of sources.naming(name)) for (const f of holding(site.file, site.line)) touched.add(f);
          continue;
        }
        if (!inside) {
          // Of an item outside any function, a variant added between two:
          // the functions that reach it, where the map knows them (#2288).
          const byItem = map.items === undefined ? undefined : (touchedByItem(map.items, file, from) ?? touchedByItem(map.items, file, from + 1));
          if (byItem !== undefined) {
            for (const f of byItem) touched.add(f);
            continue;
          }
          if (useOrMod(added) || (map.items !== undefined && inUnitTests(map.items, file, from))) continue;
          const named = byName(file, from);
          if (typeof named === "string") return { kind: "all", why: `${file}'s change between lines ${from} and ${from + 1} is outside any function's counted code: ${named}` };
          for (const f of named) touched.add(f);
          continue;
        }
        for (const f of [...before, ...after]) touched.add(f);
        continue;
      }
      for (let line = from; line < from + count; line++) {
        const found = holding(file, line);
        if (found.length === 0 && map.items !== undefined) {
          // A doc comment or an attribute reads nothing; a signature line
          // is of the function it opens (#2288).
          const at = linesOf(file)[line - 1] ?? "";
          if (commentOnly(at) || /^\s*#!?\[/.test(at)) continue;
          const opened = inSignature(linesOf(file), line) ? following(file, line) : undefined;
          if (opened !== undefined) {
            touched.add(opened);
            // And its callers, whose calls a signature may change unseen.
            const name = map.items.names[opened];
            if (name) for (const site of sources.naming(name)) for (const f of holding(site.file, site.line)) touched.add(f);
            continue;
          }
        }
        if (found.length === 0) {
          // Of an item outside any function, its variant, field, constant
          // or `impl` header: the functions that reach it (#2288).
          const byItem = map.items === undefined ? undefined : touchedByItem(map.items, file, line);
          if (byItem !== undefined) {
            for (const f of byItem) touched.add(f);
            continue;
          }
          const at = linesOf(file)[line - 1] ?? "";
          if (useOrMod([at, ...added]) || (map.items !== undefined && inUnitTests(map.items, file, line))) continue;
          const named = byName(file, line);
          if (typeof named === "string") return { kind: "all", why: `${file}'s change at line ${line} is outside any function's counted code: ${named}` };
          for (const f of named) touched.add(f);
          continue;
        }
        for (const f of found) touched.add(f);
      }
    }
  }
  const affected = (unit: string) => {
    const bits = map.units[unit];
    if (bits === undefined || bits === null) return true;
    const bytes = Buffer.from(bits, "base64");
    for (const f of touched) if (bytes[f >> 3] & (1 << (f & 7))) return true;
    return false;
  };
  const pickedFiles = [...files, ...caseFiles].filter(affected);
  const pickedCases = cases.filter((name) => affected(caseUnit("test/corpus.test.ts", name)!));
  const pickedForms = cases.filter((name) => affected(caseUnit("test/iterator-forms.test.ts", name)!));
  return {
    kind: "picked",
    files: pickedFiles,
    cases: pickedCases,
    forms: pickedForms,
    functions: [...touched],
    why: `${touched.size} of the compiler's functions changed${visibilityOnly === 0 ? "" : `, ${visibilityOnly} visibility change${visibilityOnly === 1 ? "" : "s"} of names declared once, which select nothing`}`,
  };
}

/** The tests the map recorded running any function of `files`, the
 * compiler's sources: each by its unit's name, the corpus's cases
 * `test/corpus/x.rs`. Those that run on every change, `null`, aren't
 * said: a sanity pass's cap asks what a file's functions reach. */
export function unitsReaching(map: TestMap, files: string[]): string[] {
  const functions = functionsOf(map, files);
  return Object.entries(map.units).flatMap(([unit, bits]) => {
    if (bits === null) return [];
    const bytes = Buffer.from(bits, "base64");
    return functions.some((f) => bytes[f >> 3]! & (1 << (f & 7))) ? [unit] : [];
  });
}

/** The functions of `files`, the compiler's sources, by their indices in
 * the map. */
export function functionsOf(map: TestMap, files: string[]): number[] {
  const of = new Set(files);
  return map.functions.flatMap(({ file }, at) => (of.has(file) ? [at] : []));
}

/** How many of `functions` each unit, a test file or a corpus case
 * (`caseUnit`), ran as the map recorded it: none of one it has no record
 * of, or that ran no compiler. */
export function reachOfFunctions(map: TestMap, functions: number[]): (unit: string) => number {
  return (unit) => {
    const bits = map.units[unit];
    if (bits === undefined || bits === null) return 0;
    const bytes = Buffer.from(bits, "base64");
    return functions.filter((f) => (bytes[f >> 3] ?? 0) & (1 << (f & 7))).length;
  };
}

/** Each hunk of `git diff -U0`'s, by file: its lines of the old side, and
 * the lines it writes. */
/** The diff the map's answers read, of what changed since its commit: its
 * hunks bare, and a moved file as a rename, `-M`, so a move asks the map of
 * the file's path there and of its edit alone (#2337). */
export const changesDiff = ["diff", "-U0", "--no-color", "--no-ext-diff", "-M"];

export function hunksOf(diff: string): Change[] {
  const changes: Change[] = [];
  let current: Change | undefined;
  // Past a file's first hunk, where a `+` line is one it writes, not its
  // header's `+++`.
  let inHunks = false;
  for (const line of diff.split("\n")) {
    const file = /^diff --git a\/(.+) b\//.exec(line);
    if (file) {
      current = { file: file[1], hunks: [] };
      changes.push(current);
      inHunks = false;
      continue;
    }
    if (current === undefined || current.hunks === "new") continue;
    if (!inHunks && line.startsWith("new file mode")) current.hunks = "new";
    // A rename's header, `git diff -M`'s: its path there, the map's, is
    // the `a/` path above, and its path now the tree's.
    const similarity = !inHunks && /^similarity index (\d+)%$/.exec(line);
    if (similarity) current.similarity = Number(similarity[1]);
    if (!inHunks && line.startsWith("rename to ")) current.now = line.slice("rename to ".length);
    const hunk = /^@@ -(\d+)(?:,(\d+))? \+(\d+)(?:,(\d+))? @@/.exec(line);
    if (hunk && current.hunks !== "new") {
      current.hunks.push([Number(hunk[1]), hunk[2] === undefined ? 1 : Number(hunk[2]), [], [], Number(hunk[3]), hunk[4] === undefined ? 1 : Number(hunk[4])]);
      inHunks = true;
    } else if (inHunks && line.startsWith("+") && current.hunks !== "new") current.hunks.at(-1)?.[2]?.push(line.slice(1));
    else if (inHunks && line.startsWith("-") && current.hunks !== "new") current.hunks.at(-1)?.[3]?.push(line.slice(1));
  }
  return changes;
}

/** Hold the maps to `cap` of them, the least recently used going first, or
 * fewer, where the disk's below its floor: each touched as it's read. What
 * a recording was writing when it stopped, a `.tmp`, counts as a map, and
 * goes as the least recently used do. */
export function pruneMaps(dir = mapStore, cap = keptMaps, free = () => freeBytes(dir), toolchain?: string) {
  if (!existsSync(dir)) return { removed: 0, size: 0 };
  // A map of another toolchain or layout than `toolchain`'s, a stale key,
  // which no check of this one reads, goes whatever its use.
  const stale = (name: string) => toolchain !== undefined && name.endsWith(".json") && !name.endsWith(keySuffix(toolchain));
  // The drift check's flag is no map: it stays until a nightly's check passes.
  const names = readdirSync(dir).filter((name) => join(dir, name) !== driftFlag && name !== "drift-failed");
  const kept = names
    .filter((name) => !stale(name))
    .flatMap((name) => {
      // One another run's prune removed since it was listed is no longer kept.
      const path = join(dir, name);
      const seen = statSync(path, { throwIfNoEntry: false });
      return seen === undefined ? [] : [{ path, bytes: bytesOf(path), used: seen.mtimeMs }];
    });
  const newest = [...kept].sort((a, b) => b.used - a.used);
  const past = [...names.filter(stale).map((name) => join(dir, name)), ...newest.slice(cap).map(({ path }) => path)];
  const within = evict(newest.slice(0, cap), { cap: Number.POSITIVE_INFINITY, free, floor: floorBytes });
  for (const path of past) rmSync(path, { force: true });
  return { removed: past.length + within.removed, size: within.size };
}

/** Of the maps in `store`, any worktree's, the one of `checkout`'s HEAD or
 * its nearest ancestor, of this layout and `toolchain`, touched as it's
 * read; or none. A commit `checkout` doesn't have, another clone's, is no
 * ancestor of it. */
/** The file of the map `nearestMap` gives of this checkout, `accept`ed. */
function nearestMapFile(accept: (map: TestMap) => boolean): string | undefined {
  const map = nearestMap(mapStore, root, toolchainOf(), accept);
  return map === undefined ? undefined : mapFile(mapStore, map.commit, map.toolchain);
}

export function nearestMap(store: string, checkout: string, toolchain: string, accept: (map: TestMap) => boolean = () => true): TestMap | undefined {
  if (!existsSync(store)) return undefined;
  // By its file's name, `mapFile`'s, the commits of HEAD's ancestors, the
  // nearest first: only that one's read, where it's whole and of this layout.
  const ancestors = readdirSync(store)
    .flatMap((name) => {
      if (!name.endsWith(keySuffix(toolchain))) return [];
      const commit = name.slice(0, -keySuffix(toolchain).length);
      if (!/^[0-9a-f]{40}$/.test(commit) || git(["merge-base", "--is-ancestor", commit, "HEAD"], checkout).code !== 0) return [];
      return [{ path: join(store, name), behind: Number(git(["rev-list", "--count", `${commit}..HEAD`], checkout).out.trim()) }];
    })
    .sort((a, b) => a.behind - b.behind);
  for (const { path } of ancestors) {
    let map: TestMap;
    try {
      map = JSON.parse(readFileSync(path, "utf8"));
    } catch {
      continue;
    }
    if (map.version !== mapVersion || map.toolchain !== toolchain || !accept(map)) continue;
    const now = Date.now() / 1000;
    utimesSync(path, now, now);
    return map;
  }
  return undefined;
}

/** What names the one map a run reads, its commit: a mutation run's
 * survivors run again at its base (`resultsAt`) read the map the run read,
 * a commit of the push's that's no ancestor of the base, so push and base
 * are judged by one map, placed at the base by the diff from its commit
 * (#2341). Without it, the base's run in CI found no map of the base or an
 * ancestor, and its snapshot judge's failure took every verdict of a shard
 * with it. */
export const namedMapEnv = "RUST_JS_TEST_MAP_OF";

/** The map `headMap` reads in `checkout`: of the commit `named`, where a
 * run names one, of any commit the checkout has, whole and of this
 * layout and `toolchain`; else of HEAD or its nearest ancestor. */
export function mapOf(store: string, checkout: string, toolchain: string, named = process.env[namedMapEnv]): TestMap | undefined {
  if (named === undefined) return nearestMap(store, checkout, toolchain);
  const path = mapFile(store, named, toolchain);
  if (!/^[0-9a-f]{40}$/.test(named) || !existsSync(path) || git(["cat-file", "-e", `${named}^{commit}`], checkout).code !== 0) return undefined;
  let map: TestMap;
  try {
    map = JSON.parse(readFileSync(path, "utf8"));
  } catch {
    return undefined;
  }
  if (map.version !== mapVersion || map.toolchain !== toolchain) return undefined;
  const now = Date.now() / 1000;
  utimesSync(path, now, now);
  return map;
}

/** What each test map CI recorded of a commit of `main` is named: the
 * artifact `test-map-of-<sha>` of test-map.yml's run after that commit's
 * check. */
export const ciMapPrefix = "test-map-of-";

/** Of the artifacts GitHub lists, `test-map-of-<sha>` each of a run of
 * `main` in this repository, not a fork's, unexpired, the one of HEAD's
 * nearest ancestor, `behind` saying how far each commit is, `undefined`
 * where it's no ancestor: the run and the artifact's name. */
export function ciMapOf(
  artifacts: { name: string; expired: boolean; workflow_run: { id: number; head_branch: string; repository_id: number; head_repository_id: number } }[],
  behind: (commit: string) => number | undefined,
): { run: number; name: string; commit: string; behind: number } | undefined {
  return artifacts
    .flatMap((artifact) => {
      const commit = artifact.name.slice(ciMapPrefix.length);
      const run = artifact.workflow_run;
      if (!artifact.name.startsWith(ciMapPrefix) || artifact.expired || !/^[0-9a-f]{40}$/.test(commit)) return [];
      if (run.head_branch !== "main" || run.repository_id !== run.head_repository_id) return [];
      const far = behind(commit);
      return far === undefined ? [] : [{ run: run.id, name: artifact.name, commit, behind: far }];
    })
    .sort((a, b) => a.behind - b.behind)[0];
}

/** Where test-map.yml packs the store's map for `test-map-of-<sha>`
 * (`packMaps`), the one form `fetchMap` takes. */
export const packedMaps = join(mapDir, "packed");

/** A packed map's file name ends so: its JSON, zstd's. */
export const packedSuffix = ".json.zst";

/** zstd's level a map is packed at: 168 MB of JSON 5.4 MB in 0.6 s, where
 * the artifact's own zip made 64 MB of it, which GitHub's storage served a
 * push at some 85 KB/s, 12 min (#2385); level 19 made 3.8 MB in 19 s. */
export const packLevel = 9;

/** How long `fetchMap`'s download may take: about twice a packed map's
 * fetch at that rate, 5.4 MB at 85 KB/s some 65 s; past it, the push
 * selects by the nearest map the machine keeps, or everything where it
 * keeps none (`select.ts`), each said on its line (#2385). */
export const fetchSeconds = 120;

/** Each map of `store` packed into `into`, `<name>.json.zst` (`packLevel`):
 * test-map.yml's, before it uploads `test-map-of-<sha>` (#2385). */
export function packMaps(store = mapStore, into = packedMaps): string[] {
  rmSync(into, { recursive: true, force: true });
  mkdirSync(into, { recursive: true });
  const packed: string[] = [];
  for (const name of readdirSync(store).filter((name) => name.endsWith(".json"))) {
    const json = readFileSync(join(store, name));
    const zst = Bun.zstdCompressSync(json, { level: packLevel });
    writeFileSync(join(into, `${name.slice(0, -".json".length)}${packedSuffix}`), zst);
    packed.push(`${name}: ${json.length} bytes, packed ${zst.length}`);
  }
  return packed;
}

/** Each packed map of `from` unpacked into `store`, its JSON: how many.
 * Nothing else is a map: an artifact of before #2385, its JSON zipped
 * whole, is none, and the push selects by what it keeps. */
export function unpackMaps(from: string, store = mapStore): number {
  mkdirSync(store, { recursive: true });
  let unpacked = 0;
  for (const name of readdirSync(from).filter((name) => name.endsWith(packedSuffix))) {
    const json = Bun.zstdDecompressSync(readFileSync(join(from, name)));
    writeFileSync(join(store, `${name.slice(0, -packedSuffix.length)}.json`), json);
    unpacked++;
  }
  return unpacked;
}

/** How many of test-map.yml's last runs of `main` `fetchMap` looks in,
 * the newest first, each a map of the commit whose check it followed. */
export const fetchedRuns = 12;

/** `bun run push`'s: the map CI recorded of HEAD's nearest ancestor on
 * `main`, downloaded into the machine's store where it's nearer than the
 * nearest kept, a map as any other, read only where it's whole and of this
 * layout and toolchain (`nearestMap`). test-map.yml's runs of `main` are
 * asked, the newest first, each for its own artifacts: the repository's
 * list of every artifact, 100 a page, held a run's `test-map-of-<sha>` past
 * its first page, and none was found (b355dc824's push). What it did, or
 * why it fetched none, in a line, which the push's line says. */
export function fetchMap(): string {
  const gh = (args: string[]) => {
    const p = Bun.spawnSync(["gh", ...args], { cwd: root, stdout: "pipe", stderr: "pipe" });
    return { ok: p.exitCode === 0, out: p.stdout.toString(), err: p.stderr.toString().trim().split("\n")[0] ?? "" };
  };
  const runs = gh([
    "api",
    `repos/{owner}/{repo}/actions/workflows/test-map.yml/runs?branch=main&status=success&per_page=${fetchedRuns}`,
    "--jq",
    "[.workflow_runs[] | {id, head_branch, repository_id: .repository.id, head_repository_id: .head_repository.id}]",
  ]);
  if (!runs.ok) return `no map fetched: test-map.yml's runs weren't listed (${runs.err}; is \`gh\` logged in?)`;
  const behind = (commit: string) => {
    if (git(["merge-base", "--is-ancestor", commit, "HEAD"]).code !== 0) return undefined;
    return Number(git(["rev-list", "--count", `${commit}..HEAD`]).out.trim());
  };
  let found: ReturnType<typeof ciMapOf>;
  const listed: { id: number; head_branch: string; repository_id: number; head_repository_id: number }[] = JSON.parse(runs.out);
  for (const run of listed) {
    const artifacts = gh(["api", `repos/{owner}/{repo}/actions/runs/${run.id}/artifacts`, "--jq", "[.artifacts[] | {name, expired}]"]);
    if (!artifacts.ok) return `no map fetched: run ${run.id}'s artifacts weren't listed (${artifacts.err})`;
    const named: { name: string; expired: boolean }[] = JSON.parse(artifacts.out);
    found = ciMapOf(
      named.map((artifact) => ({ ...artifact, workflow_run: run })),
      behind,
    );
    if (found !== undefined) break;
  }
  if (found === undefined) return `no map fetched: none of test-map.yml's last ${listed.length} runs of main is of an ancestor of HEAD`;
  const kept = nearestMap(mapStore, root, toolchainOf());
  if (kept !== undefined && behind(kept.commit)! <= found.behind) return `the map kept of ${kept.commit.slice(0, 9)} is as near as CI's`;
  const into = join(mapDir, "fetched");
  if (!downloaded(found.run, [`--name=${found.name}`], into, undefined, undefined, fetchSeconds)) {
    return `no map fetched: run ${found.run}'s ${found.name} didn't download, given ${fetchSeconds} s at most`;
  }
  const moved = unpackMaps(into);
  rmSync(into, { recursive: true, force: true });
  pruneMaps(mapStore, keptMaps, () => freeBytes(mapStore), toolchainOf());
  if (moved === 0) return `no map fetched: run ${found.run}'s ${found.name} held no packed map`;
  // Read as any other: of this layout and toolchain, or not at all.
  const read = nearestMap(mapStore, root, toolchainOf());
  if (read?.commit !== found.commit) return `no map fetched: CI's map of ${found.commit.slice(0, 9)} isn't of this layout or toolchain`;
  return `fetched CI's map of ${found.commit.slice(0, 9)}, ${found.behind} commits behind HEAD`;
}

/** The map of HEAD or its nearest ancestor, and the working tree's changes
 * since its commit, new files' too; or why there's none. */
/** What `headMap` says where there's no map, which a caller that needs one
 * tells apart from a site it can't place. */
export const noMap = "no test map of HEAD or an ancestor of it on this machine";

/** Why HEAD's map isn't recorded of a working tree that changes it. */
const unclean = "the test map is of a commit: commit or remove what the working tree changes first";

/** A map of HEAD or an ancestor of it the machine keeps, or, where there's
 * none, HEAD's recorded, of a clean tree: CI's `ensure`, and a local
 * mutation run's snapshot judge, which records its own (ADR 0093, amended).
 * Whether it recorded one, and in how many seconds; `headMap` reads it
 * again after. */
export async function ensureMap(): Promise<{ recorded: boolean; seconds: number; commit: string }> {
  const kept = nearestMap(mapStore, root, toolchainOf());
  if (kept) return { recorded: false, seconds: 0, commit: kept.commit };
  const started = Date.now();
  await record(false);
  headMapRead = undefined;
  const made = nearestMap(mapStore, root, toolchainOf());
  if (made === undefined) throw new Error(`${noMap}, though HEAD's was recorded`);
  return { recorded: true, seconds: (Date.now() - started) / 1000, commit: made.commit };
}

function headMap(): { map: TestMap; changes: Change[] } | string {
  headMapRead ??= readHeadMap();
  return headMapRead;
}

/** `headMap`'s, read once a process: a mutation run asks it of each
 * survivor, and test/mutations.test.ts of every mutation. */
let headMapRead: ReturnType<typeof readHeadMap> | undefined;

/** The commit of the map `headMap` reads, where there's one: what a
 * mutation run names to its run at the base (`namedMapEnv`). */
export function headMapCommit(): string | undefined {
  const found = headMap();
  return typeof found === "string" ? undefined : found.map.commit;
}

function readHeadMap(): { map: TestMap; changes: Change[] } | string {
  const named = process.env[namedMapEnv];
  const map = mapOf(mapStore, root, toolchainOf(), named);
  if (map === undefined) return named === undefined ? `${noMap}: \`bun run test-map\` records one` : `${noMap}: the map of ${named.slice(0, 9)} the run names, \`${namedMapEnv}\`, isn't kept`;
  const diff = git([...changesDiff, map.commit]).out;
  const untracked = git(["ls-files", "--others", "--exclude-standard"]).out.split("\n").filter(Boolean);
  return { map, changes: [...hunksOf(diff), ...untracked.map((file): Change => ({ file, hunks: "new" }))] };
}

/** `picked`, its reason saying which map said it. */
function said(map: TestMap, picked: Picked): Picked {
  const of = `the test map of ${map.commit.slice(0, 9)}`;
  return picked.kind === "all" ? { ...picked, why: `${of} can't say: ${picked.why}` } : { ...picked, why: `${of}: ${picked.why}` };
}

/** The corpus cases whose compiles ran `file`'s lines `first` to `last`,
 * a mutation's site, by the map of HEAD or its nearest ancestor: the
 * mutation runner's snapshot judge's (#864). A derived fact, as of the map's
 * commit: a site whose own lines changed since can't be placed by it, nor
 * one outside a function's counted code, and is said so, `all`, never a
 * stale answer. What else changed since, another file's code or its own
 * file's other lines, leaves the cases the map says reach the site's as
 * they were: the site is placed at its lines there, `mapLines`, under its
 * path there where its file moved (#2337; ADR 0093, amended). One it can't
 * place for that, the map behind it, says so, `mapBehind` (#2349). `found`,
 * the map and what changed since, a test's own where given. */
export function casesReaching(file: string, first: number, last: number, cases: string[], found = headMap()): Picked {
  if (typeof found === "string") return { kind: "all", why: found };
  const { map, changes } = found;
  const change = changes.find((each) => (each.now ?? each.file) === file);
  if (change === undefined) return said(map, pick(map, [{ file, hunks: [[first, last - first + 1]] }], [], cases));
  const at = mapLines(change, first, last);
  if (typeof at === "string") return said(map, { kind: "all", why: `${file} ${at}`, mapBehind: true });
  return said(map, pick(map, [{ file: change.file, hunks: [[at[0], at[1] - at[0] + 1]] }], [], cases));
}

/** Where the lines `first` to `last` of a file as it is now were at the
 * map's commit, by its change since: each unchanged line moved by the
 * hunks before it. Or why it can't be said: the site's own lines changed,
 * the file is new, or it moved far from what it was (below
 * `placedSimilarity`), each of which a guess would answer stale. */
export function mapLines(change: Change, first: number, last: number): [number, number] | string {
  if (change.hunks === "new") return "is new since the map's commit";
  if (change.now !== undefined && (change.similarity ?? 0) < placedSimilarity) return `moved from ${change.file}, ${change.similarity ?? 0}% alike, too unlike to place a line`;
  let delta = 0;
  for (const [from, count, , , now, nowCount] of change.hunks) {
    if (now === undefined || nowCount === undefined) return "changed since the map's commit";
    // Its lines now, `now` to `now + nowCount - 1`; a deletion's, none,
    // between `now` and the next.
    const end = nowCount === 0 ? now : now + nowCount - 1;
    const touches = nowCount === 0 ? first <= now && now < last : now <= last && end >= first;
    if (touches) return `changed at its lines since the map's commit, ${now}–${end}`;
    if (end < first) delta += count - nowCount;
  }
  return [first + delta, last + delta];
}

/** How alike a moved file must be to the one at the map's commit, git's
 * percent, for a line of it to be placed there: a move's edits, paths and
 * visibility, keep #2299's cut's files 95 to 100% alike. */
export const placedSimilarity = 90;

/** The toolchain's `llvm-profdata` and `llvm-cov`, rustup's
 * `llvm-tools` component's. */
function llvmTools(): { profdata: string; cov: string } {
  const sysroot = Bun.spawnSync(["rustc", "--print", "sysroot"], { cwd: root }).stdout.toString().trim();
  const host = /^host: (.+)$/m.exec(Bun.spawnSync(["rustc", "-vV"], { cwd: root }).stdout.toString())?.[1];
  const bin = join(sysroot, "lib", "rustlib", host ?? "", "bin");
  const tools = { profdata: join(bin, "llvm-profdata"), cov: join(bin, "llvm-cov") };
  if (!existsSync(tools.profdata) || !existsSync(tools.cov)) throw new Error("the test map needs LLVM's tools: `rustup component add llvm-tools`");
  return tools;
}

/** Each file `include_str!` or `include_bytes!` reads in the compiler's
 * crates' sources, and the line that reads it. */
function includes(): TestMap["includes"] {
  const found: TestMap["includes"] = {};
  const walk = (dir: string) => {
    for (const entry of readdirSync(dir, { withFileTypes: true })) {
      const path = join(dir, entry.name);
      if (entry.isDirectory()) walk(path);
      else if (entry.name.endsWith(".rs")) {
        readFileSync(path, "utf8")
          .split("\n")
          .forEach((text, i) => {
            for (const [, name] of text.matchAll(/include_(?:str|bytes)!\(\s*"([^"]+)"/g)) {
              const read = relative(root, resolve(dirname(path), name));
              (found[read] ??= []).push({ file: relative(root, path), line: i + 1 });
            }
          });
      }
    }
  };
  for (const dir of compilerSourceDirs()) walk(join(root, dir));
  return found;
}

/** Each function of the compiler's that a build counting its runs has, from
 * one run's counts, which name every function, run or not: its name and the
 * spans of its counted code. */
function functionTable(binary: string, record: string, tools: { profdata: string; cov: string }) {
  const raw = join(record, "table.profraw");
  const ran = Bun.spawnSync([binary, "--version-json"], { env: { ...process.env, LLVM_PROFILE_FILE: raw }, stdout: "pipe" });
  if (ran.exitCode !== 0 || !existsSync(raw)) throw new Error("the counting compiler didn't run");
  const merged = join(record, "table.profdata");
  if (Bun.spawnSync([tools.profdata, "merge", raw, "-o", merged]).exitCode !== 0) throw new Error("llvm-profdata merge failed");
  const exported = Bun.spawnSync([tools.cov, "export", "-format=text", "-skip-expansions", "-instr-profile", merged, binary], { stdout: "pipe", stderr: "pipe" });
  if (exported.exitCode !== 0) throw new Error(`llvm-cov export failed: ${exported.stderr}`);
  type Exported = { name: string; regions: number[][]; filenames: string[] };
  const fns = (JSON.parse(exported.stdout.toString()).data[0].functions as Exported[]).filter((f) => f.filenames.length > 0);
  const functions: TestMap["functions"] = fns.map(({ regions, filenames }) => {
    // A code region's lines, of the function's own file (llvm-cov's kind 0).
    const lines = regions
      .filter((r) => r[7] === 0 && r[5] === 0)
      .map((r) => [r[0], r[2]] as [number, number])
      .sort((a, b) => a[0] - b[0]);
    const file = relative(root, filenames[0]);
    const merged = joinedSpans(lines, sourceLines(file));
    return { file, spans: merged };
  });
  return { names: fns.map((f) => f.name), functions };
}

/** A function's counted lines, `[from, to]` each, in order, joined where
 * what's between is the function's own, what counts none: a `use`, a
 * closing brace, a comment, a pattern. Not where an item is between, nested
 * in it, an `impl`'s or a `const`'s, which may be used outside it: a change
 * there is no function's (`nestedItem`). `text` is the file's lines. */
export function joinedSpans(lines: [number, number][], text: string[]): [number, number][] {
  const merged: [number, number][] = [];
  for (const [from, to] of lines) {
    const last = merged.at(-1);
    const between = last ? text.slice(last[1], from - 1) : [];
    if (last && (from <= last[1] + 1 || !between.some((line) => nestedItem.test(line)))) last[1] = Math.max(last[1], to);
    else merged.push([from, to]);
  }
  return merged;
}

/** A line that starts an item, but a `use` or a `fn`, whose code counts as
 * its own: nested in a function, an `impl` of a type outside it, say, its
 * constants are read where it isn't run. */
export const nestedItem = /^\s*(?:#\[.*\]\s*)*(?:pub(?:\([^)]*\))?\s+)?(?:(?:unsafe|default|auto)\s+)*(?:(?:impl|const|static|struct|enum|union|trait|type|mod|extern|macro)\b|macro_rules!)/;

/** A source file's lines, read once. */
const sources = new Map<string, string[]>();
function sourceLines(file: string): string[] {
  let lines = sources.get(file);
  if (lines === undefined) {
    lines = existsSync(join(root, file)) ? readFileSync(join(root, file), "utf8").split("\n") : [];
    sources.set(file, lines);
  }
  return lines;
}

/** Run `units` at most `cores` at once, each a process given its own
 * target; how each ended. */
async function runUnits(units: { name: string; cmd: string[]; env: Record<string, string> }[], cores: number, record: string, base: Record<string, string>) {
  const ended = new Map<string, number>();
  let next = 0;
  const worker = async () => {
    while (next < units.length) {
      const at = next++;
      const unit = units[at];
      // Named by its place, not the test's name: a test whose JS names
      // where it was written, and checks it for a word its own name has,
      // `switch` of lowering-strict-switches', found it there.
      const own = join(record, "targets", String(at));
      mkdirSync(own, { recursive: true });
      // What's kept by source alone, verified runs, no compile of rust-js's,
      // the machine's, as native programs' builds are in any target (#875,
      // #1047); all else the unit's own.
      mkdirSync(machineVerifiedRuns, { recursive: true });
      symlinkSync(machineVerifiedRuns, join(own, "corpus-cache"));
      const start = Date.now();
      const p = Bun.spawn(unit.cmd, {
        cwd: root,
        env: { ...process.env, ...base, RUST_JS_TARGET: own, RUST_JS_STD_SYSROOT: join(own, "std-sysroot"), ...unit.env },
        stdout: "pipe",
        stderr: "pipe",
      });
      const [out, err] = await Promise.all([new Response(p.stdout).text(), new Response(p.stderr).text()]);
      const code = await p.exited;
      ended.set(unit.name, code);
      rmSync(own, { recursive: true, force: true });
      const seconds = ((Date.now() - start) / 1000).toFixed(0);
      if (code !== 0) console.log(`FAIL ${unit.name} (${seconds} s): it runs on every change\n${(out + err).slice(-1500)}`);
      else console.log(`ok   ${unit.name} ${seconds} s`);
    }
  };
  await Promise.all(Array.from({ length: cores }, worker));
  return ended;
}

/** The C of the object the counting build links, which names where a run
 * given no `LLVM_PROFILE_FILE` writes its counts, `file`: by the runtime's
 * `__llvm_profile_set_filename` as the process starts, never a definition
 * of `__llvm_profile_filename`, which the instrumented crate defines itself
 * on Linux, in a COMDAT, so a second one doesn't link (`duplicate symbol`,
 * check 43786120684). The API's name outranks the environment's, so it's
 * asked only where the environment names none. */
export function lostObject(file: string): string {
  return `#include <stdlib.h>
void __llvm_profile_set_filename(const char *);
__attribute__((constructor)) static void rust_js_lost(void) {
  if (getenv("LLVM_PROFILE_FILE") == NULL) __llvm_profile_set_filename(${JSON.stringify(file)});
}
`;
}

/** What a recording is of, `build`'s, which each part's `units` and `merge`
 * read: the commit and toolchain, the counting build's function table and
 * its functions' names, what the map says of the checkout's sources, and
 * each test to run, the corpus's files in `parts` parts, each dealt to one
 * of `shards` machines (`shard`, from 1). Made once, so every part runs
 * its own share and the merge knows each test it must have heard of. */
export type Plan = {
  version: number;
  commit: string;
  toolchain: string;
  shards: number;
  parts: number;
  names: string[];
  functions: TestMap["functions"];
  files: string[];
  caseNames: string[];
  readsSources: string[];
  corpusReaders: string[];
  includes: TestMap["includes"];
  units: { name: string; cmd: string[]; env: Record<string, string>; shard: number }[];
  /** Of an incremental recording: the map it carries from, its chain's
   * full base, and the tests it records, files and case units; the corpus's
   * cases and std's iterator structs' it runs, a list each its parts read. */
  carry?: { from: string; base: { commit: string; made: number }; recorded: string[]; cases: string[]; forms: string[] };
  /** The item layer of its commit, or none where its index failed. */
  items?: ItemLayer;
};

/** Where `build` writes the plan and each part's recording is, the merge's
 * input: `plan.json`, and `recordings/<shard>/`, each the part's `ended.json`,
 * its tests' bits (`units/`) and its `unknown`. */
const planFile = join(mapDir, "plan.json");
const recordings = join(mapDir, "recordings");
/** The counting build's binary, which each part runs. */
export const countingBinary = join(mapDir, "build", "debug", "rust-js");

/** Each test of a recording on `shards` machines of `perShard` cores:
 * each file of the corpus's cases in `shards * perShard` parts, each
 * machine `perShard` of each (`caseShardParts`), as are `partedFiles`, and the push suite's
 * `files`, dealt by `seconds` of each (`balanced`), in turn where it knows
 * none. Each in exactly one part. */
export function planUnits(files: string[], shards: number, perShard: number, seconds: ReadonlyMap<string, number>): Plan["units"] {
  const units: Plan["units"] = [];
  for (let at = 1; at <= shards; at++) {
    const caseParts = caseShardParts(at, shards, perShard);
    units.push(
      // The corpus in parts, each case a test of its own (testMapCompiler.ts).
      ...caseParts.map((part) => ({
        name: `test/corpus.test.ts ${part}`,
        cmd: ["bun", "test", "test/corpus.test.ts"],
        env: { RUST_JS_CASES: part, RUST_JS_TEST_MAP_UNIT: "test/corpus.test.ts" },
        shard: at,
      })),
      // And the corpus built with std's iterator structs, as an iterator
      // change's check runs it, each case a test of its own too.
      ...caseParts.map((part) => ({
        name: `test/iterator-forms.test.ts ${part}`,
        cmd: ["bun", "test", "test/iterator-forms.test.ts"],
        env: { ITERATOR_FORMS: "corpus", RUST_JS_CASES: part, RUST_JS_TEST_MAP_UNIT: "test/iterator-forms.test.ts" },
        shard: at,
      })),
      // And each file the push suite runs in parts, in as many, its parts'
      // compiles the file's.
      ...partedFiles
        .filter((file) => files.includes(file))
        .flatMap((file) =>
          caseParts.map((part) => ({ name: `${file} ${part}`, cmd: ["bun", "test", file], env: { RUST_JS_CASES: part, RUST_JS_TEST_MAP_UNIT: file }, shard: at })),
        ),
      ...balanced(
        files.filter((file) => !partedFiles.includes(file)),
        at,
        shards,
        seconds,
      ).map((file) => ({ name: file, cmd: ["bun", "test", file], env: { RUST_JS_TEST_MAP_UNIT: file }, shard: at })),
    );
  }
  return units;
}

/** The counting build of HEAD, of a clean tree, and its plan for `shards`
 * machines of this one's cores: the corpus's files in as many parts as
 * there are cores on all of them, the push suite's files dealt by what
 * check:fast keeps of each one's time on this machine (scripts/jobTimes.ts),
 * none where it keeps none, CI's. */
/** The item layer of the checkout's `functions` (scripts/testMapItems.ts),
 * or none, said loudly: a map without it selects as before. */
function recordItems(functions: TestMap["functions"]): ItemLayer | undefined {
  const started = Date.now();
  const channel = /channel\s*=\s*"([^"]+)"/.exec(readFileSync(join(root, "rust-toolchain.toml"), "utf8"))?.[1] ?? "stable";
  const documents = scipIndex(root, join(mapDir, "scip"), channel);
  if (typeof documents === "string") {
    console.log(`⚠ the map has no item layer, its index failed: ${documents}; a change outside any function selects everything, as before`);
    return undefined;
  }
  const text = new Map<string, string[]>();
  const lines = (file: string) => {
    if (!text.has(file)) text.set(file, existsSync(join(root, file)) ? readFileSync(join(root, file), "utf8").split("\n") : []);
    return text.get(file)!;
  };
  const layer = itemLayer(documents, functions, lines);
  console.log(`the item layer: ${layer.items.length} items, ${layer.impls.length} impl headers, ${((Date.now() - started) / 1000).toFixed(0)} s`);
  return layer;
}

async function build(shards: number, carry?: (names: string[], files: string[], caseNames: string[]) => Promise<NonNullable<Plan["carry"]> | string>): Promise<Plan> {
  if (git(["status", "--porcelain"]).out.trim() !== "") throw new Error(unclean);
  const started = Date.now();
  const tools = llvmTools();
  // The counting build, in a target of its own, so `target/debug`'s isn't
  // built again. Its dependencies are sccache's, as `cargo build`'s are:
  // the target given as `--target-dir`, as `CARGO_TARGET_DIR` in rustc's
  // environment is in sccache's key, which missed every crate (#932).
  const build = join(mapDir, "build");
  // Where a run of it writes its counts given no `LLVM_PROFILE_FILE`, as
  // where a test starts it with an environment of its own: there, not in
  // its directory, which the test may remove, so a run the map didn't count
  // is seen, and the recording fails. Named by the runtime's API, as the
  // process starts, where the environment gives none (`lostObject`): the
  // same path on each machine a part runs on, its checkout's.
  mkdirSync(mapDir, { recursive: true });
  writeFileSync(join(mapDir, "lost.c"), lostObject(join(mapDir, "lost", "%p.profraw")));
  if (Bun.spawnSync(["cc", "-c", join(mapDir, "lost.c"), "-o", join(mapDir, "lost.o")], { stdout: "inherit", stderr: "inherit" }).exitCode !== 0) process.exit(1);
  const { CARGO_TARGET_DIR: _, ...env } = process.env;
  // Each of the compiler's crates counts its runs, the binary's and each
  // other's (ADR 1181), by scripts/counting-rustc.sh, and no dependency's;
  // those still built by the machine's cache.
  const counting = join(root, "scripts", "counting-rustc.sh");
  const cache = process.env.RUSTC_WRAPPER ?? Bun.which("sccache") ?? "";
  const counted = compilerCrates().map((crate) => crate.name).join(" ");
  const built = Bun.spawnSync(
    ["cargo", "rustc", "--locked", "--quiet", "--target-dir", build, "--bin", "rust-js", "--", "-C", `link-arg=${join(mapDir, "lost.o")}`],
    { cwd: root, env: { ...env, RUSTC_WRAPPER: counting, RUST_JS_COUNTED_CRATES: counted, RUST_JS_CACHE_WRAPPER: cache }, stdout: "inherit", stderr: "inherit" },
  );
  if (built.exitCode !== 0) process.exit(1);
  console.log(`the counting build: ${((Date.now() - started) / 1000).toFixed(0)} s`);

  rmSync(recordings, { recursive: true, force: true });
  const table = join(mapDir, "table");
  rmSync(table, { recursive: true, force: true });
  mkdirSync(table, { recursive: true });
  const { names, functions } = functionTable(countingBinary, table, tools);
  rmSync(table, { recursive: true, force: true });
  const items = recordItems(functions);

  const perShard = machine().cores;
  const parts = shards * perShard;
  const files = mappedFiles();
  const times = readJobTimes();
  const seconds = new Map(files.map((file) => [file, Math.max(0, ...(times[file]?.seconds ?? []))] as const).filter(([, s]) => s > 0));
  const caseNames = readdirSync(join(root, "test", "corpus")).filter((name) => name.endsWith(".rs"));
  // What it carries, where an earlier map serves; why not, where none does.
  const carried = carry === undefined ? "a full recording, asked" : await carry(names, files, caseNames);
  if (typeof carried === "string") console.log(`a full recording: ${carried}`);
  // The map it carries from, beside the plan, which the merge reads on a
  // machine whose store may not keep it.
  // Of a full one, the nearest earlier map with names, which the nightly's
  // drift check carries from (`driftCheck`).
  rmSync(carriedFrom, { force: true });
  const head = git(["rev-parse", "HEAD"]).out.trim();
  const earlier = typeof carried !== "string" ? mapFile(mapStore, carried.from, toolchainOf()) : nearestMapFile((map) => map.names !== undefined && map.commit !== head);
  if (earlier !== undefined) copyFileSync(earlier, carriedFrom);
  const carrying = typeof carried === "string" ? undefined : carried;
  const units = carrying === undefined ? planUnits(files, shards, perShard, seconds) : carriedUnits(planUnits(files, shards, perShard, seconds), carrying);
  const plan: Plan = {
    version: mapVersion,
    commit: git(["rev-parse", "HEAD"]).out.trim(),
    toolchain: toolchainOf(),
    shards,
    parts,
    names,
    functions,
    files,
    caseNames,
    readsSources: files.filter((file) => readsSources(file)),
    // The push suite's files that read the corpus's files: each says where.
    corpusReaders: files.filter((file) => /\bcorpus\b/.test(readFileSync(join(root, file), "utf8"))),
    includes: includes(),
    units,
    ...(carrying === undefined ? {} : { carry: carrying }),
    ...(items === undefined ? {} : { items }),
  };
  writeFileSync(planFile, JSON.stringify(plan));
  return plan;
}

/** What an incremental recording from `from` records, of the selector's
 * full rule's `selection` of the commits since (CI's check of `main`'s,
 * `fullSelection`, never the push rule): each test file and case it
 * selects; each `from` has none of, new since; and each `null` there, run
 * on every change. */
export function carryOf(from: TestMap, selection: { files: string[]; cases: string[]; forms: string[] }, files: string[], caseNames: string[], base: { commit: string; made: number }): NonNullable<Plan["carry"]> {
  const unrecorded = (unit: string) => typeof from.units[unit] !== "string";
  return {
    from: from.commit,
    base,
    recorded: files.filter((file) => selection.files.includes(file) || unrecorded(file)),
    cases: caseNames.filter((name) => selection.cases.includes(name) || unrecorded(caseUnit(caseFiles[0], name)!)),
    forms: caseNames.filter((name) => selection.forms.includes(name) || unrecorded(caseUnit(caseFiles[1], name)!)),
  };
}

/** Why a recording of HEAD is full, or the carry it records: none where the
 * nightly's drift check failed (`driftFlag`), where no map of an ancestor
 * has names, where the nearest is past `carryCommits`, where its chain's
 * full recording is past `baseDays` or `baseCommits`, or where the
 * selector's full rule says everything. */
export async function planCarry(names: string[], files: string[], caseNames: string[]): Promise<NonNullable<Plan["carry"]> | string> {
  void names;
  if (existsSync(driftFlag)) return "the nightly's drift check failed: each recording full until one passes";
  const head = git(["rev-parse", "HEAD"]).out.trim();
  const from = nearestMap(mapStore, root, toolchainOf(), (map) => map.names !== undefined && map.commit !== head);
  if (from === undefined) return "no map of an ancestor of HEAD has its functions' names";
  const behind = Number(git(["rev-list", "--count", `${from.commit}..HEAD`]).out.trim());
  if (behind > carryCommits) return `the nearest map with names, ${from.commit.slice(0, 9)}, is ${behind} commits behind, past ${carryCommits}`;
  const base = from.base ?? { commit: from.commit, made: from.made };
  const baseBehind = Number(git(["rev-list", "--count", `${base.commit}..HEAD`]).out.trim());
  if (Date.now() - base.made > baseDays * 86_400_000 || baseBehind > baseCommits) return `its chain's full recording, ${base.commit.slice(0, 9)}, is past ${baseDays} day or ${baseCommits} commits`;
  const { fullSelection } = await import("./select");
  const selection = fullSelection(from);
  if (selection.all) return `the selector's full rule selects everything since ${from.commit.slice(0, 9)}: ${selection.why}`;
  return carryOf(from, selection, files, caseNames, base);
}

/** The nightly's drift check: HEAD's full map against the one a carry
 * from the nearest earlier map with names would make of the same tests,
 * each test it carries the earlier map's functions, by symbol, the rest
 * the full map's. Each test the full map counted whose functions differ is
 * drift: written to `driftFlag`, so recordings are full until a nightly's
 * check passes, and said; none, and the flag goes. */
export async function driftCheck(): Promise<string[]> {
  const head = git(["rev-parse", "HEAD"]).out.trim();
  const full = nearestMap(mapStore, root, toolchainOf(), (map) => map.commit === head && map.carried === undefined && map.names !== undefined);
  if (full === undefined) throw new Error("no full map of HEAD with names: the nightly records one first");
  // The map the full recording's plan kept beside it, the nearest earlier
  // one with names; or this machine's.
  const kept = existsSync(carriedFrom) ? (JSON.parse(readFileSync(carriedFrom, "utf8")) as TestMap) : undefined;
  const from = kept?.names !== undefined && kept.commit !== head ? kept : nearestMap(mapStore, root, toolchainOf(), (map) => map.names !== undefined && map.commit !== head);
  if (from === undefined) {
    console.log("no earlier map with names: nothing a carry would make to check");
    return [];
  }
  const { fullSelection } = await import("./select");
  const selection = fullSelection(from);
  if (selection.all) {
    console.log(`the selector's full rule selects everything since ${from.commit.slice(0, 9)}: a carry would record all`);
    return [];
  }
  const caseNames = Object.keys(full.units).flatMap((unit) => /^test\/corpus\/([^ ]+\.rs)$/.exec(unit)?.[1] ?? []);
  const files = Object.keys(full.units).filter((unit) => !unit.includes(" ") && !unit.startsWith("test/corpus/"));
  const carry = carryOf(from, selection, files, caseNames, { commit: from.commit, made: from.made });
  const recorded = new Set([...carry.recorded, ...carry.cases.map((name) => caseUnit(caseFiles[0], name)!), ...carry.forms.map((name) => caseUnit(caseFiles[1], name)!), ...caseFiles]);
  const drift: string[] = [];
  for (const [unit, bits] of Object.entries(full.units)) {
    if (bits === null || recorded.has(unit)) continue;
    const was = from.units[unit];
    if (typeof was !== "string") continue;
    const carried = remapBits(Buffer.from(was, "base64"), from.names!, full.names!);
    const now = Buffer.from(bits, "base64");
    const differ: string[] = [];
    for (let f = 0; f < full.names!.length; f++) if (((carried[f >> 3]! >> (f & 7)) & 1) !== ((now[f >> 3]! >> (f & 7)) & 1)) differ.push(full.names![f]!);
    if (differ.length > 0) drift.push(`${unit}: ${differ.length} functions differ, ${differ.slice(0, 3).join(", ")}`);
  }
  mkdirSync(mapStore, { recursive: true });
  if (drift.length > 0) writeFileSync(driftFlag, `${head}\n${drift.join("\n")}\n`);
  else rmSync(driftFlag, { force: true });
  return drift;
}

/** The map an incremental recording carries from, beside its plan. */
const carriedFrom = join(mapDir, "from.json");

/** Where an incremental recording's parts read the cases they run. */
const carriedCaseLists = { corpus: "target/test-map/carried-cases.txt", forms: "target/test-map/carried-forms.txt" };

/** Of a full plan's `units`, those an incremental one records: each file
 * it names, and the corpus's parts, and std's iterator structs', of the
 * cases it names alone (`RUST_JS_CASES=i/n:<list>`), none where it names
 * none. */
export function carriedUnits(units: Plan["units"], carry: NonNullable<Plan["carry"]>): Plan["units"] {
  const recorded = new Set(carry.recorded);
  return units.flatMap((unit) => {
    const [file] = unit.name.split(" ");
    if (file === caseFiles[0] || file === caseFiles[1]) {
      const list = file === caseFiles[0] ? carry.cases : carry.forms;
      if (list.length === 0) return [];
      return [{ ...unit, env: { ...unit.env, RUST_JS_CASES: `${unit.env.RUST_JS_CASES}:${file === caseFiles[0] ? carriedCaseLists.corpus : carriedCaseLists.forms}` } }];
    }
    return recorded.has(file!) ? [unit] : [];
  });
}

/** The plan `build` wrote, of HEAD's commit, or why there's none. */
function readPlan(): Plan {
  if (!existsSync(planFile)) throw new Error(`no plan of a recording: \`bun scripts/testMap.ts build\` writes ${relative(root, planFile)}`);
  const plan: Plan = JSON.parse(readFileSync(planFile, "utf8"));
  const head = git(["rev-parse", "HEAD"]).out.trim();
  if (plan.version !== mapVersion || plan.commit !== head) throw new Error(`the plan is of ${plan.commit.slice(0, 9)}, layout ${plan.version}, not HEAD's ${head.slice(0, 9)}, layout ${mapVersion}`);
  return plan;
}

/** Run part `shard` of `plan`'s tests with the counting build, its cores'
 * worth at once, into `recordings/<shard>/`: how each test ended
 * (`ended.json`), each compile's bits, and what it couldn't count. */
async function recordPart(plan: Plan, shard: number): Promise<string> {
  if (!Number.isInteger(shard) || shard < 1 || shard > plan.shards) throw new Error(`the plan has ${plan.shards} parts, not ${shard}`);
  if (!existsSync(countingBinary)) throw new Error(`no counting build at ${relative(root, countingBinary)}: \`bun scripts/testMap.ts build\` makes it`);
  const started = Date.now();
  const tools = llvmTools();
  const lost = join(mapDir, "lost");
  rmSync(lost, { recursive: true, force: true });
  mkdirSync(lost, { recursive: true });
  const recording = join(recordings, String(shard));
  rmSync(recording, { recursive: true, force: true });
  for (const dir of ["raw", "units", "targets"]) mkdirSync(join(recording, dir), { recursive: true });
  writeFileSync(join(recording, "names.json"), JSON.stringify(plan.names));
  if (plan.carry !== undefined) {
    writeFileSync(join(root, carriedCaseLists.corpus), `${plan.carry.cases.join("\n")}\n`);
    writeFileSync(join(root, carriedCaseLists.forms), `${plan.carry.forms.join("\n")}\n`);
  }
  const units = plan.units.filter((unit) => unit.shard === shard);
  // The longest first, by what check:fast keeps of each file's time on this
  // machine (scripts/jobTimes.ts), so no long one runs alone at the end; the
  // corpus's parts first of all.
  const times = readJobTimes();
  const took = (name: string) => Math.max(0, ...(times[name]?.seconds ?? []));
  const parted = (name: string) => (caseFiles.some((file) => name.startsWith(`${file} `)) ? -1 : 0);
  units.sort((a, b) => parted(a.name) - parted(b.name) || took(b.name) - took(a.name));
  const ended = await runUnits(units, machine().cores, recording, {
    RUST_JS_COMPILER: join(root, "scripts", "testMapCompiler.ts"),
    RUST_JS_TEST_MAP_COMPILER: countingBinary,
    RUST_JS_TEST_MAP_DIR: recording,
    RUST_JS_TEST_MAP_PROFDATA: tools.profdata,
    RUST_JS_TEST_MAP_ROOT: root,
    RUST_JS_COMPILE_CACHE_MB: "0",
    RUST_JS_LOAD_SCALE: "3",
  });
  // A run the map didn't count: which test's, it can't say.
  const uncounted = readdirSync(lost);
  if (uncounted.length > 0) {
    console.error(`${uncounted.length} runs of the compiler had no LLVM_PROFILE_FILE, so no test's: find the test that starts it with an environment of its own (the map isn't written)`);
    process.exit(1);
  }
  writeFileSync(join(recording, "ended.json"), JSON.stringify(Object.fromEntries(ended)));
  for (const dir of ["raw", "targets", "names.json"]) rmSync(join(recording, dir), { recursive: true, force: true });
  console.log(`part ${shard} of ${plan.shards}: ${units.length} tests, ${((Date.now() - started) / 1000).toFixed(0)} s`);
  return recording;
}

/** The map of `plan`'s recording in `parts`, each a part's recording
 * directory, `made` its time: each test's functions the union of its
 * compiles', in whichever part it ran; one that failed, ran no compiler,
 * wasn't counted whole, ran a function the table hasn't, or reads the
 * compiler's sources, `null`, run on every change. The same map of the
 * same tests however they're parted (test/test-map.test.ts). A test of the
 * plan no part, or two, said ended of is refused: a part was lost. */
export function mergeRecordings(plan: Plan, parts: string[], made: number, from?: TestMap): { map: TestMap; always: string[] } {
  if (plan.carry !== undefined && (from === undefined || from.commit !== plan.carry.from || from.names === undefined)) {
    throw new Error(`the plan carries from ${plan.carry.from.slice(0, 9)}'s map, which isn't kept here, or has no names`);
  }
  const ended = new Map<string, number>();
  const bits = new Map<string, Uint8Array>();
  const unfinished = new Set<string>();
  const unknown = new Set<string>();
  for (const part of parts) {
    const said: Record<string, number> = JSON.parse(readFileSync(join(part, "ended.json"), "utf8"));
    for (const [name, code] of Object.entries(said)) {
      if (ended.has(name)) throw new Error(`${name} ended in two parts of the recording`);
      ended.set(name, code);
    }
    for (const name of readdirSync(join(part, "units"))) {
      const [, slug, kind] = /^(.*)\.\d+-\d+\.(bits|start)$/.exec(name) ?? [];
      if (slug === undefined) continue;
      const unit = slug.replaceAll("__", "/");
      if (kind === "start") {
        unfinished.add(unit);
        continue;
      }
      const own = bits.get(unit) ?? new Uint8Array(Math.ceil(plan.names.length / 8));
      const read = readFileSync(join(part, "units", name));
      for (let i = 0; i < own.length; i++) own[i] |= read[i];
      bits.set(unit, own);
    }
    if (existsSync(join(part, "unknown"))) {
      for (const line of readFileSync(join(part, "unknown"), "utf8").split("\n").filter(Boolean)) unknown.add(line.split("\t")[0]);
    }
  }
  const missing = plan.units.filter((unit) => !ended.has(unit.name)).map((unit) => unit.name);
  if (missing.length > 0) throw new Error(`no part of the recording ran ${missing.length} of its tests: ${missing.slice(0, 5).join(", ")}`);
  // Of a part that failed, each of its cases, as the files shard them, and
  // the file's own tests.
  const failedUnits = new Set<string>();
  for (const file of caseFiles) {
    // Of an incremental recording, its parts ran the cases it lists alone.
    const listed = plan.carry === undefined ? plan.caseNames : plan.caseNames.filter((name) => (file === caseFiles[0] ? plan.carry!.cases : plan.carry!.forms).includes(name));
    for (let i = 1; i <= plan.parts; i++) {
      const name = `${file} ${i}/${plan.parts}`;
      if (!plan.units.some((unit) => unit.name === name) || ended.get(name) === 0) continue;
      failedUnits.add(file);
      for (const name of shard(listed, i, plan.parts)) failedUnits.add(caseUnit(file, name)!);
    }
  }
  // What an incremental recording didn't record, carried over from the map
  // it carries from, by each function's symbol (`remapBits`).
  const recorded = plan.carry === undefined ? undefined : new Set([...plan.carry.recorded, ...plan.carry.cases.flatMap((name) => [caseUnit(caseFiles[0], name)!]), ...plan.carry.forms.flatMap((name) => [caseUnit(caseFiles[1], name)!])]);
  let carriedCount = 0;
  const caseUnits = caseFiles.flatMap((file) => plan.caseNames.map((name) => caseUnit(file, name)!));
  const units: TestMap["units"] = {};
  const always: string[] = [];
  const noCompiler: string[] = [];
  // A file recorded in parts failed where any part did.
  const fileEnded = (file: string) =>
    partedFiles.includes(file) ? plan.units.filter((unit) => unit.name.startsWith(`${file} `)).every((unit) => ended.get(unit.name) === 0) : ended.get(file) === 0;
  for (const unit of [...plan.files, ...caseFiles, ...caseUnits]) {
    const isFile = plan.files.includes(unit);
    if (recorded !== undefined && !recorded.has(unit) && !(caseFiles as readonly string[]).includes(unit)) {
      const was = from!.units[unit];
      if (was === undefined || was === null) throw new Error(`${unit} is neither recorded nor carried: the plan records each test its map has none of`);
      units[unit] = Buffer.from(remapBits(Buffer.from(was, "base64"), from!.names!, plan.names)).toString("base64");
      carriedCount++;
      continue;
    }
    // A case file's own compiles, of an incremental recording, with those
    // the map it carries from saw, so none a case left out ran is lost;
    // carried whole where it ran none of its parts.
    if (recorded !== undefined && (caseFiles as readonly string[]).includes(unit) && typeof from!.units[unit] === "string") {
      const was = remapBits(Buffer.from(from!.units[unit] as string, "base64"), from!.names!, plan.names);
      const own = bits.get(unit) ?? new Uint8Array(was.length);
      for (let i = 0; i < own.length; i++) own[i] |= was[i] ?? 0;
      bits.set(unit, own);
    }
    const failed = isFile ? !fileEnded(unit) : failedUnits.has(unit);
    const own = bits.get(unit);
    const why = failed
      ? "it failed"
      : own === undefined
        ? "it ran no compiler"
        : unfinished.has(unit)
          ? "a compile of it wasn't counted whole"
          : unknown.has(unit)
            ? "a compile of it ran a function the table hasn't"
            : isFile && plan.readsSources.includes(unit)
              ? "it reads the compiler's sources"
              : undefined;
    units[unit] = why === undefined ? Buffer.from(own!).toString("base64") : null;
    if (why !== undefined) always.push(`${unit} (${why})`);
    if (isFile && why === "it ran no compiler") noCompiler.push(unit);
  }
  const { version, commit, toolchain, functions, includes, corpusReaders, names, items } = plan;
  const base = plan.carry?.base ?? { commit, made };
  const carried = plan.carry === undefined ? {} : { carried: { from: plan.carry.from, recorded: Object.keys(units).length - carriedCount, carried: carriedCount } };
  return { map: { version, commit, toolchain, made, functions, units, includes, corpusReaders, noCompiler, names, base, ...carried, ...(items === undefined ? {} : { items }) }, always };
}

/** The map of the plan's recording, every part's, kept in the machine's
 * store; what only the recording needed removed, unless `keep`. */
function merge(plan: Plan, keep: boolean) {
  const parts = existsSync(recordings) ? readdirSync(recordings).map((name) => join(recordings, name)) : [];
  if (parts.length !== plan.shards) throw new Error(`the plan has ${plan.shards} parts, and ${parts.length} were recorded in ${relative(root, recordings)}`);
  const fromPath = plan.carry === undefined ? undefined : existsSync(carriedFrom) ? carriedFrom : mapFile(mapStore, plan.carry.from, plan.toolchain);
  const from = fromPath === undefined || !existsSync(fromPath) ? undefined : (JSON.parse(readFileSync(fromPath, "utf8")) as TestMap);
  const { map, always } = mergeRecordings(plan, parts, Date.now(), from);
  if (map.carried !== undefined) console.log(`carried from ${map.carried.from.slice(0, 9)}: ${map.carried.recorded} tests recorded, ${map.carried.carried} carried; its chain's full recording ${map.base!.commit.slice(0, 9)}`);
  const file = mapFile(mapStore, plan.commit, plan.toolchain);
  // Written whole, then named, so another worktree never reads half of it.
  mkdirSync(mapStore, { recursive: true });
  const writing = `${file}.${process.pid}.tmp`;
  writeFileSync(writing, JSON.stringify(map));
  renameSync(writing, file);
  pruneMaps(mapStore, keptMaps, () => freeBytes(mapStore), plan.toolchain);
  // What only this recording needed, its counts and each test's target,
  // kept with `--keep`, to see why a test runs on every change. Its build is
  // kept, one, as `target/debug` is, so the next recording builds only
  // what changed.
  if (!keep) {
    rmSync(recordings, { recursive: true, force: true });
    rmSync(planFile, { force: true });
  }
  const isCase = (unit: string) => /test\/corpus\/[^ ]+\.rs/.test(unit);
  const cased = always.filter(isCase);
  console.log(`the test map of ${plan.commit.slice(0, 9)}: ${map.functions.length} functions, ${Object.keys(map.units).length} tests; ${always.length} run on every change:`);
  for (const unit of always.filter((unit) => !isCase(unit))) console.log(`  ${unit}`);
  if (cased.length > 0) console.log(`  ${cased.length} cases: ${cased.slice(0, 10).join(", ")}${cased.length > 10 ? ", …" : ""}`);
}

/** Record HEAD's map on this machine: the counting build, then each test,
 * then which functions each ran, one part of one. */
async function record(keep: boolean) {
  const commit = git(["rev-parse", "HEAD"]).out.trim();
  // One any worktree recorded of this commit is this one's (#932); `--keep`
  // records again, to see why a test runs on every change.
  if (!keep && nearestMap(mapStore, root, toolchainOf())?.commit === commit) {
    console.log(`the test map of ${commit.slice(0, 9)} is recorded already, by this worktree or another: ${mapFile(mapStore, commit, toolchainOf())}`);
    return;
  }
  const started = Date.now();
  const plan = await build(1);
  await recordPart(plan, 1);
  merge(plan, keep);
  console.log(`${((Date.now() - started) / 1000).toFixed(0)} s`);
}

if (import.meta.main) {
  const args = process.argv.slice(2);
  const [command] = args;
  if (command === undefined || command === "record" || command === "--keep") await record(args.includes("--keep"));
  else if (command === "kept") {
    // CI's: whether a map of the nearest ancestor its cache kept serves, as
    // one does a worktree's check (`headMap`, the changes since its commit
    // can't be placed by it), or, `--head`, the nightly's, one of HEAD; its
    // exit status. Where none does, the recording is CI's to make, in parts.
    const map = nearestMap(mapStore, root, toolchainOf());
    const head = git(["rev-parse", "HEAD"]).out.trim();
    const serves = map !== undefined && (!args.includes("--head") || map.commit === head);
    console.log(serves ? `the test map of ${map.commit.slice(0, 9)}${map.commit === head ? ", HEAD's," : ", an ancestor of HEAD,"} serves` : `no test map of ${args.includes("--head") ? "HEAD" : "HEAD or an ancestor of it"} is kept`);
    process.exit(serves ? 0 : 1);
  } else if (command === "build") {
    const shards = Number(args.find((arg) => arg.startsWith("--shards="))?.slice("--shards=".length));
    if (!Number.isInteger(shards) || shards < 1) throw new Error("build --shards=N: the machines the recording's parts run on, at least 1");
    const plan = await build(shards, args.includes("--full") ? undefined : planCarry);
    console.log(`the plan of ${plan.commit.slice(0, 9)}: ${plan.units.length} tests in ${plan.shards} parts, the corpus's files in ${plan.parts}${plan.carry ? `, carried from ${plan.carry.from.slice(0, 9)}: ${plan.carry.recorded.length} files, ${plan.carry.cases.length} cases, ${plan.carry.forms.length} forms recorded` : ""}`);
    // CI's parts, a job each: their count and each one's number.
    const parts = Array.from({ length: plan.shards }, (_, i) => i + 1);
    if (process.env.GITHUB_OUTPUT) appendFileSync(process.env.GITHUB_OUTPUT, `shards=${plan.shards}\nparts=${JSON.stringify(parts)}\n`);
  } else if (command === "units") {
    const part = shardArg(args);
    const plan = readPlan();
    if (part === undefined || part.count !== plan.shards) throw new Error(`units --shard=i/${plan.shards}: the plan's parts`);
    await recordPart(plan, part.index);
  } else if (command === "merge") {
    merge(readPlan(), false);
  } else if (command === "drift") {
    const drift = await driftCheck();
    for (const line of drift.slice(0, 50)) console.log(line);
    console.log(drift.length === 0 ? "no drift: each test the carry would have carried ran the functions the full recording saw" : `drift in ${drift.length} tests: recordings are full until a nightly's check passes (${relative(root, driftFlag)})`);
    if (drift.length > 0) process.exit(1);
  } else if (command === "fetch") {
    console.log(fetchMap());
  } else if (command === "pack") {
    for (const line of packMaps()) console.log(line);
  } else {
    console.error("usage: bun run test-map [record | kept [--head] | build --shards=N | units --shard=i/N | merge | fetch | pack] (what a change selects: bun scripts/select.ts)");
    process.exit(2);
  }
}
