// The test map (#877): which of the compiler's functions each test file of
// plain `check:fast`'s suites runs, and each case of the corpus, as it's
// built and as it's built with std's iterator structs (#876), recorded by
// running them with a build of the compiler that counts each function's
// runs. Plain `check:fast` reads it to run only the files and cases a
// change can affect, and the mutation runner which cases reach a
// mutation's site, whose snapshots judge one of how the JS reads (#864).
//
//   bun run test-map           record HEAD's, of a clean tree: about 130 s on 12 cores,
//                              none where any worktree of the machine recorded it
//   bun run test-map --keep    and keep what it recorded, to see why a test runs always
//   bun run test-map pick      what the working tree's change picks, and why
//
// Why a skip is sound (ADR 1101): a test whose compiles ran none of the
// functions a change touches runs the same compiler code after it, so ends
// as it did. That holds only where everything the change touches is a
// function's code: a line of a function's counted code, or a file
// `include_str!` reads at such a line. Anything else, a type, a constant,
// an attribute, an item written in a function, a file outside `src/` (a
// test's helper, std, the runtime's package, the toolchain, a doc a test
// reads), and the map can't say: the check runs what it runs without it
// (`all`). What a test reaches is recorded where nothing is shared with
// another (its own `RUST_JS_TARGET`, std's sysroot and no compile cache),
// so nothing one test made is read by another unrecorded. A test that ran
// no compiler, or one whose compile wasn't counted whole (stopped,
// aborted), or that failed, runs on every change, as does one that reads
// the compiler's sources itself (`readsSources`).
//
// Kept by the commit it's of and the toolchain's, for every worktree of the
// machine, in `~/.cache/rust-js/test-maps/` (#932): a map is the same in
// any checkout of its commit, so a new worktree reads one another recorded,
// and `bun run test-map` of a commit one's recorded for ends at once. The
// three most recently used, about 12 MB each. The counting build is the
// worktree's, `target/test-map/build`, one, as `target/debug` is. A change
// is what the working tree differs from the map's commit by, so a map of an
// earlier commit stays good while what's changed since is functions' code;
// of those of HEAD's ancestors, the nearest is read. One whose commit isn't
// HEAD's ancestor, or of another version or toolchain, is never read.

import { existsSync, mkdirSync, readdirSync, readFileSync, renameSync, rmSync, statSync, symlinkSync, utimesSync, writeFileSync } from "node:fs";
import { availableParallelism } from "node:os";
import { dirname, join, relative, resolve } from "node:path";

import { bytesOf, evict, freeBytes, machineStore } from "../test/bounded";
import { machineVerifiedRuns } from "../test/kept";
import { plainSuiteFiles, shard } from "./shard";

const root = join(import.meta.dir, "..");
/** What records a map, the worktree's: its counting build and a recording's
 * counts. */
export const mapDir = join(root, "target", "test-map");
/** Where maps are kept, the machine's, every worktree's (#932). */
export const mapStore = join(machineStore, "test-maps");
/** The layout's version, which a change to it or to what's recorded bumps. */
export const mapVersion = 3;
/** The most maps kept: each about 12 MB. */
export const keptMaps = 3;

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
};

/** A change to a file: each hunk's lines of the map's commit, `[from,
 * count]`, a count of 0 lines added after `from`, and the lines it writes
 * there; or `new`, a file the commit didn't have. */
export type Hunk = [from: number, count: number, added?: string[]];
export type Change = { file: string; hunks: Hunk[] | "new" };

/** What a change runs: the files and cases it can affect, or all of them,
 * and why. */
export type Picked = { kind: "picked"; files: string[]; cases: string[]; forms: string[]; why: string } | { kind: "all"; why: string };

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

/** The test files the map records, by their suites (scripts/shard.ts). */
export function mappedFiles(): string[] {
  const files = readdirSync(join(root, "test"))
    .filter((name) => name.endsWith(".test.ts"))
    .map((name) => `test/${name}`);
  return plainSuiteFiles(files).flatMap(({ files }) => files);
}

/** Whether `file`, or a module of the checkout's it imports, reads the
 * compiler's sources as text, as the architecture's tests do: such a test
 * can fail on a change it runs no compiled code of, so it runs on every
 * change. A path into `src/` written in it, `join(root, "src", ..)` or
 * `"src/<a file or folder of the compiler's>"`. */
export function readsSources(file: string, seen = new Set<string>()): boolean {
  const path = resolve(root, file);
  if (seen.has(path) || !existsSync(path)) return false;
  seen.add(path);
  const text = readFileSync(path, "utf8");
  if (/\broot\s*,\s*["'`]src["'`/]|\$\{root\}\/src\b|["'`]\.\.\/src\b/.test(text)) return true;
  for (const [, name] of text.matchAll(/["'`]src\/([\w.-]+)/g)) if (existsSync(join(root, "src", name))) return true;
  for (const [, spec] of text.matchAll(/\bfrom\s+["'](\.{1,2}\/[^"']+)["']/g)) {
    const base = resolve(dirname(path), spec);
    const found = [base, `${base}.ts`, `${base}.js`].find((candidate) => existsSync(candidate) && statSync(candidate).isFile());
    if (found && found.startsWith(root) && !found.includes("node_modules") && readsSources(relative(root, found), seen)) return true;
  }
  return false;
}

/** What a change of `changes` runs, by `map`: of `files`, the test files the
 * plain suites have now, and `cases`, the corpus's. */
export function pick(map: TestMap, changes: Change[], files: string[], cases: string[]): Picked {
  const touched = new Set<number>();
  const runs = new Set<string>();
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
  for (const { file, hunks } of changes) {
    // A case runs itself, and the tests that read the corpus's files.
    if (file.startsWith("test/corpus/") && /\.(rs|js)$/.test(file) && !file.slice("test/corpus/".length).includes("/")) {
      const name = file.slice("test/corpus/".length).replace(/\.js$/, ".rs");
      for (const of of caseFiles) runs.add(caseUnit(of, name)!);
      for (const reader of map.corpusReaders) runs.add(reader);
      continue;
    }
    // The corpus's file checks every case.
    if (file === "test/corpus.test.ts") return { kind: "all", why: `${file} checks every case` };
    // And std's iterator structs' file checks every case of that build.
    if (file === "test/iterator-forms.test.ts") {
      for (const name of cases) runs.add(caseUnit(file, name)!);
      runs.add(file);
      continue;
    }
    // A test file runs itself: of the plain suites', it runs; another's is
    // no other test's, which none imports.
    if (/^test\/[^/]+\.test\.ts$/.test(file)) {
      if (files.includes(file)) runs.add(file);
      continue;
    }
    const sites = map.includes[file];
    if (sites !== undefined && hunks !== "new") {
      for (const { file: at, line } of sites) {
        const found = holding(at, line);
        if (found.length === 0) return { kind: "all", why: `${file} is read at ${at}:${line}, outside any function's counted code` };
        for (const f of found) touched.add(f);
      }
      continue;
    }
    if (!file.startsWith("src/") || !file.endsWith(".rs")) return { kind: "all", why: `${file} isn't the compiler's code` };
    if (hunks === "new") return { kind: "all", why: `${file} is new since the map's commit` };
    for (const [from, count, added = []] of hunks) {
      // An item it writes inside a function, an `impl` say, may be used
      // where the function isn't run.
      const item = added.find((line) => nestedItem.test(line));
      if (item !== undefined) return { kind: "all", why: `${file}'s change at line ${from} writes an item, \`${item.trim()}\`` };
      if (count === 0) {
        // Lines added between two: in a function where each is counted
        // code of one function or of one nested in the other, as a closure
        // or the initializer of a static is, so what's added is in the
        // outer one's body.
        const before = holding(file, from);
        const after = holding(file, from + 1);
        const inside = before.some((a) => after.some((b) => a === b || within(a, b) || within(b, a)));
        if (!inside) return { kind: "all", why: `${file}'s change between lines ${from} and ${from + 1} is outside any function's counted code` };
        for (const f of [...before, ...after]) touched.add(f);
        continue;
      }
      for (let line = from; line < from + count; line++) {
        const found = holding(file, line);
        if (found.length === 0) return { kind: "all", why: `${file}'s change at line ${line} is outside any function's counted code` };
        for (const f of found) touched.add(f);
      }
    }
  }
  const affected = (unit: string) => {
    if (runs.has(unit)) return true;
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
    why: `${touched.size} of the compiler's functions changed`,
  };
}

/** Each hunk of `git diff -U0`'s, by file: its lines of the old side, and
 * the lines it writes. */
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
    const hunk = /^@@ -(\d+)(?:,(\d+))? \+/.exec(line);
    if (hunk && current.hunks !== "new") {
      current.hunks.push([Number(hunk[1]), hunk[2] === undefined ? 1 : Number(hunk[2]), []]);
      inHunks = true;
    } else if (inHunks && line.startsWith("+") && current.hunks !== "new") current.hunks.at(-1)?.[2]?.push(line.slice(1));
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
  const names = readdirSync(dir);
  const kept = names
    .filter((name) => !stale(name))
    .map((name) => {
      const path = join(dir, name);
      return { path, bytes: bytesOf(path), used: statSync(path).mtimeMs };
    });
  const newest = [...kept].sort((a, b) => b.used - a.used);
  const past = [...names.filter(stale).map((name) => join(dir, name)), ...newest.slice(cap).map(({ path }) => path)];
  const within = evict(newest.slice(0, cap), { cap: Number.POSITIVE_INFINITY, free });
  for (const path of past) rmSync(path, { force: true });
  return { removed: past.length + within.removed, size: within.size };
}

/** Of the maps in `store`, any worktree's, the one of `checkout`'s HEAD or
 * its nearest ancestor, of this layout and `toolchain`, touched as it's
 * read; or none. A commit `checkout` doesn't have, another clone's, is no
 * ancestor of it. */
export function nearestMap(store: string, checkout: string, toolchain: string): TestMap | undefined {
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
    if (map.version !== mapVersion || map.toolchain !== toolchain) continue;
    const now = Date.now() / 1000;
    utimesSync(path, now, now);
    return map;
  }
  return undefined;
}

/** The map of HEAD or its nearest ancestor, and the working tree's changes
 * since its commit, new files' too; or why there's none. */
function headMap(): { map: TestMap; changes: Change[] } | string {
  const map = nearestMap(mapStore, root, toolchainOf());
  if (map === undefined) return "no test map of HEAD or an ancestor of it on this machine: `bun run test-map` records one";
  const diff = git(["diff", "-U0", "--no-color", "--no-ext-diff", "--no-renames", map.commit]).out;
  const untracked = git(["ls-files", "--others", "--exclude-standard"]).out.split("\n").filter(Boolean);
  return { map, changes: [...hunksOf(diff), ...untracked.map((file): Change => ({ file, hunks: "new" }))] };
}

/** `picked`, its reason saying which map said it. */
function said(map: TestMap, picked: Picked): Picked {
  const of = `the test map of ${map.commit.slice(0, 9)}`;
  return picked.kind === "all" ? { kind: "all", why: `${of} can't say: ${picked.why}` } : { ...picked, why: `${of}: ${picked.why}` };
}

/** What the working tree's change runs, by the newest map of an ancestor of
 * HEAD: its changes since the map's commit, the working tree's and new
 * files' too. */
export function pickChange(files: string[], cases: string[]): Picked {
  const found = headMap();
  if (typeof found === "string") return { kind: "all", why: found };
  return said(found.map, pick(found.map, found.changes, files, cases));
}

/** Lines `first` to `last` of a file as the working tree has it, as lines
 * of the map's commit, by `change`, what the working tree changes of it:
 * each line the change wrote is left out, as the change touches what holds
 * it already; each other line moved by what the change wrote before it. */
export function linesAtCommit(change: Change | undefined, first: number, last: number): Hunk[] {
  if (change?.hunks === "new") return [];
  const lines: Hunk[] = [];
  for (let line = first; line <= last; line++) {
    let offset = 0;
    let at: number | undefined = line;
    for (const [from, count, added = []] of change?.hunks ?? []) {
      // Where what it writes starts in the working tree's file: past `from`
      // where it only adds (`-U0`'s `-from,0`), at it where it replaces.
      const start = (count === 0 ? from + 1 : from) + offset;
      if (line < start) break;
      if (line < start + added.length) {
        at = undefined;
        break;
      }
      offset += added.length - count;
    }
    if (at !== undefined) lines.push([line - offset, 1]);
  }
  return lines;
}

/** The corpus's cases, of `cases`, a change to lines `first` to `last` of
 * `file`, as the working tree has it, can affect: by the newest map of an
 * ancestor of HEAD, with the working tree's changes since its commit, as
 * plain `check:fast` picks them; or `all`, and why, where it can't say.
 * A mutation's site, whose cases' snapshots judge it (scripts/mutations.ts,
 * #864). */
export function casesReaching(file: string, first: number, last: number, cases: string[]): Picked {
  const found = headMap();
  if (typeof found === "string") return { kind: "all", why: found };
  const { map, changes } = found;
  const of = changes.find((change) => change.file === file);
  if (of?.hunks === "new") return said(map, { kind: "all", why: `${file} is new since the map's commit` });
  const site: Change = { file, hunks: linesAtCommit(of, first, last) };
  return said(map, pick(map, [...changes, site], [], cases));
}

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

/** Each file `include_str!` or `include_bytes!` reads in `src/`, and the
 * line that reads it. */
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
  walk(join(root, "src"));
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

/** Record HEAD's map: the counting build, then each test, then which
 * functions each ran. */
async function record(keep: boolean) {
  const commit = git(["rev-parse", "HEAD"]).out.trim();
  const toolchain = toolchainOf();
  const file = mapFile(mapStore, commit, toolchain);
  // One any worktree recorded of this commit is this one's (#932); `--keep`
  // records again, to see why a test runs on every change.
  if (!keep && nearestMap(mapStore, root, toolchain)?.commit === commit) {
    console.log(`the test map of ${commit.slice(0, 9)} is recorded already, by this worktree or another: ${file}`);
    return;
  }
  if (git(["status", "--porcelain"]).out.trim() !== "") {
    console.error("the test map is of a commit: commit or remove what the working tree changes first");
    process.exit(1);
  }
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
  // is seen, and the recording fails (LLVM's runtime reads this name where
  // the environment gives none).
  const lost = join(mapDir, "lost");
  rmSync(lost, { recursive: true, force: true });
  mkdirSync(lost, { recursive: true });
  writeFileSync(join(mapDir, "lost.c"), `char __llvm_profile_filename[] = ${JSON.stringify(join(lost, "%p.profraw"))};\n`);
  if (Bun.spawnSync(["cc", "-c", join(mapDir, "lost.c"), "-o", join(mapDir, "lost.o")], { stdout: "inherit", stderr: "inherit" }).exitCode !== 0) process.exit(1);
  const { CARGO_TARGET_DIR: _, ...env } = process.env;
  const built = Bun.spawnSync(
    ["cargo", "rustc", "--locked", "--quiet", "--target-dir", build, "--bin", "rust-js", "--", "-C", "instrument-coverage", "-C", `link-arg=${join(mapDir, "lost.o")}`],
    { cwd: root, env, stdout: "inherit", stderr: "inherit" },
  );
  if (built.exitCode !== 0) process.exit(1);
  const binary = join(build, "debug", "rust-js");
  console.log(`the counting build: ${((Date.now() - started) / 1000).toFixed(0)} s`);

  const recording = join(mapDir, "recording");
  rmSync(recording, { recursive: true, force: true });
  for (const dir of ["raw", "units", "targets"]) mkdirSync(join(recording, dir), { recursive: true });
  const { names, functions } = functionTable(binary, recording, tools);
  writeFileSync(join(recording, "names.json"), JSON.stringify(names));

  const cores = availableParallelism();
  const files = mappedFiles();
  const parts = cores;
  const units = [
    // The corpus in parts, each case a test of its own (testMapCompiler.ts).
    ...Array.from({ length: parts }, (_, i) => ({
      name: `test/corpus.test.ts ${i + 1}/${parts}`,
      cmd: ["bun", "test", "test/corpus.test.ts"],
      env: { RUST_JS_CASES: `${i + 1}/${parts}`, RUST_JS_LIBRARIES: "differing", RUST_JS_TEST_MAP_UNIT: "test/corpus.test.ts" },
    })),
    // And the corpus built with std's iterator structs, as an iterator
    // change's check runs it, each case a test of its own too.
    ...Array.from({ length: parts }, (_, i) => ({
      name: `test/iterator-forms.test.ts ${i + 1}/${parts}`,
      cmd: ["bun", "test", "test/iterator-forms.test.ts"],
      env: { ITERATOR_FORMS: "corpus", RUST_JS_CASES: `${i + 1}/${parts}`, RUST_JS_TEST_MAP_UNIT: "test/iterator-forms.test.ts" },
    })),
    ...files.map((file) => ({ name: file, cmd: ["bun", "test", file], env: { RUST_JS_TEST_MAP_UNIT: file } })),
  ];
  // The longest first, by what check:fast keeps of each file's time, so no
  // long one runs alone at the end; the corpus's parts first of all.
  const timings = existsSync(join(root, "target", "test-timings.json")) ? JSON.parse(readFileSync(join(root, "target", "test-timings.json"), "utf8")) : {};
  const parted = (name: string) => (caseFiles.some((file) => name.startsWith(`${file} `)) ? -1 : 0);
  units.sort((a, b) => parted(a.name) - parted(b.name) || (timings[b.name] ?? 0) - (timings[a.name] ?? 0));
  const ended = await runUnits(units, cores, recording, {
    RUST_JS_COMPILER: join(root, "scripts", "testMapCompiler.ts"),
    RUST_JS_TEST_MAP_COMPILER: binary,
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

  // Each test's functions, the union of its compiles'.
  const bits = new Map<string, Uint8Array>();
  const unfinished = new Set<string>();
  for (const name of readdirSync(join(recording, "units"))) {
    const [, slug, kind] = /^(.*)\.\d+-\d+\.(bits|start)$/.exec(name) ?? [];
    if (slug === undefined) continue;
    const unit = slug.replaceAll("__", "/");
    if (kind === "start") {
      unfinished.add(unit);
      continue;
    }
    const own = bits.get(unit) ?? new Uint8Array(Math.ceil(names.length / 8));
    const read = readFileSync(join(recording, "units", name));
    for (let i = 0; i < own.length; i++) own[i] |= read[i];
    bits.set(unit, own);
  }
  const unknown = existsSync(join(recording, "unknown"))
    ? new Set(
        readFileSync(join(recording, "unknown"), "utf8")
          .split("\n")
          .filter(Boolean)
          .map((line) => line.split("\t")[0]),
      )
    : new Set<string>();
  const caseNames = readdirSync(join(root, "test", "corpus")).filter((name) => name.endsWith(".rs"));
  // Of a part that failed, each of its cases, as the files shard them, and
  // the file's own tests.
  const failedUnits = new Set<string>();
  for (const file of caseFiles) {
    for (let i = 1; i <= parts; i++) {
      if (ended.get(`${file} ${i}/${parts}`) === 0) continue;
      failedUnits.add(file);
      for (const name of shard(caseNames, i, parts)) failedUnits.add(caseUnit(file, name)!);
    }
  }
  const caseUnits = caseFiles.flatMap((file) => caseNames.map((name) => caseUnit(file, name)!));
  const units_: TestMap["units"] = {};
  const always: string[] = [];
  for (const unit of [...files, ...caseFiles, ...caseUnits]) {
    const isFile = files.includes(unit);
    const failed = isFile ? ended.get(unit) !== 0 : failedUnits.has(unit);
    const own = bits.get(unit);
    const why = failed
      ? "it failed"
      : own === undefined
        ? "it ran no compiler"
        : unfinished.has(unit)
          ? "a compile of it wasn't counted whole"
          : unknown.has(unit)
            ? "a compile of it ran a function the table hasn't"
            : isFile && readsSources(unit)
              ? "it reads the compiler's sources"
              : undefined;
    units_[unit] = why === undefined ? Buffer.from(own!).toString("base64") : null;
    if (why !== undefined) always.push(`${unit} (${why})`);
  }
  // The plain suites' files that read the corpus's files: each says where.
  const corpusReaders = files.filter((file) => /\bcorpus\b/.test(readFileSync(join(root, file), "utf8")));
  const map: TestMap = { version: mapVersion, commit, toolchain, made: Date.now(), functions, units: units_, includes: includes(), corpusReaders };
  // Written whole, then named, so another worktree never reads half of it.
  mkdirSync(mapStore, { recursive: true });
  const writing = `${file}.${process.pid}.tmp`;
  writeFileSync(writing, JSON.stringify(map));
  renameSync(writing, file);
  pruneMaps(mapStore, keptMaps, () => freeBytes(mapStore), toolchain);
  // What only this recording needed, its counts and each test's target,
  // kept with `--keep`, to see why a test runs on every change. Its build is
  // kept, one, as `target/debug` is, so the next recording builds only
  // what changed.
  if (!keep) rmSync(recording, { recursive: true, force: true });
  const isCase = (unit: string) => /test\/corpus\/[^ ]+\.rs/.test(unit);
  const cased = always.filter(isCase);
  console.log(`the test map of ${commit.slice(0, 9)}: ${functions.length} functions, ${Object.keys(units_).length} tests; ${always.length} run on every change:`);
  for (const unit of always.filter((unit) => !isCase(unit))) console.log(`  ${unit}`);
  if (cased.length > 0) console.log(`  ${cased.length} cases: ${cased.slice(0, 10).join(", ")}${cased.length > 10 ? ", …" : ""}`);
  console.log(`${((Date.now() - started) / 1000).toFixed(0)} s`);
}

if (import.meta.main) {
  const [command] = process.argv.slice(2);
  if (command === undefined || command === "record" || command === "--keep") await record(process.argv.includes("--keep"));
  else if (command === "pick") {
    const cases = readdirSync(join(root, "test", "corpus")).filter((name) => name.endsWith(".rs"));
    const picked = pickChange(mappedFiles(), cases);
    console.log(picked.why);
    if (picked.kind === "picked") {
      console.log(`files (${picked.files.length}): ${picked.files.join(" ")}`);
      console.log(`cases (${picked.cases.length} of ${cases.length}), with std's iterator structs (${picked.forms.length})`);
    }
  } else {
    console.error("usage: bun run test-map [record | pick]");
    process.exit(2);
  }
}
