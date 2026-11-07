// The test map (#877): which of the compiler's functions each test file of
// plain `check:fast`'s suites runs, and each case of the corpus, recorded by
// running them with a build of the compiler that counts each function's
// runs. Plain `check:fast` reads it to run only the files and cases a
// change can affect.
//
//   bun run test-map           record HEAD's, of a clean tree: about 2 min on 12 cores
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
// Kept by the commit it's of in `target/test-map/maps/`, the three most
// recently used, about 7 MB each, and the counting build in
// `target/test-map/build`, one, as `target/debug` is. A change is what the
// working tree differs from that commit by, so a map of an earlier commit
// stays good while what's changed since is functions' code; one whose
// commit isn't HEAD's ancestor, or of another version, is never read.

import { existsSync, mkdirSync, readdirSync, readFileSync, rmSync, statSync, symlinkSync, utimesSync, writeFileSync } from "node:fs";
import { availableParallelism } from "node:os";
import { dirname, join, relative, resolve } from "node:path";

import { bytesOf, evict, freeBytes } from "../test/bounded";
import { plainSuiteFiles, shard } from "./shard";

const root = join(import.meta.dir, "..");
/** Where maps are kept, and what records one. */
export const mapDir = join(root, "target", "test-map");
const maps = join(mapDir, "maps");
/** The layout's version, which a change to it or to what's recorded bumps. */
const VERSION = 1;
/** The most maps kept: each about 6 MB. */
export const keptMaps = 3;

/** Each of the compiler's functions, by its index: the file it's in and the
 * line spans of its counted code; and of each test, the functions it ran,
 * a bit each, base64, or `null`: it runs on every change. */
export type TestMap = {
  version: number;
  commit: string;
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
export type Picked = { kind: "picked"; files: string[]; cases: string[]; why: string } | { kind: "all"; why: string };

const git = (args: string[]) => {
  const p = Bun.spawnSync(["git", ...args], { cwd: root, stdout: "pipe", stderr: "pipe" });
  return { code: p.exitCode, out: p.stdout.toString() };
};

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
      runs.add(file.replace(/\.js$/, ".rs"));
      for (const reader of map.corpusReaders) runs.add(reader);
      continue;
    }
    // The corpus's file checks every case.
    if (file === "test/corpus.test.ts") return { kind: "all", why: `${file} checks every case` };
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
  const pickedFiles = [...files, "test/corpus.test.ts"].filter(affected);
  const pickedCases = cases.map((name) => `test/corpus/${name}`).filter(affected);
  return {
    kind: "picked",
    files: pickedFiles,
    cases: pickedCases.map((file) => file.slice("test/corpus/".length)),
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

/** The maps kept, newest first, each touched as it's read: the least
 * recently used go first past `keptMaps`. */
function keptMapFiles(): string[] {
  if (!existsSync(maps)) return [];
  return readdirSync(maps)
    .filter((name) => name.endsWith(".json"))
    .map((name) => join(maps, name))
    .sort((a, b) => statSync(b).mtimeMs - statSync(a).mtimeMs);
}

/** Hold the maps to `cap` of them, the least recently used going first, or
 * fewer, where the disk's below its floor. */
export function pruneMaps(dir = maps, cap = keptMaps, free = () => freeBytes(dir)) {
  if (!existsSync(dir)) return { removed: 0, size: 0 };
  const kept = readdirSync(dir).map((name) => {
    const path = join(dir, name);
    return { path, bytes: bytesOf(path), used: statSync(path).mtimeMs };
  });
  const newest = [...kept].sort((a, b) => b.used - a.used);
  const past = newest.slice(cap);
  const within = evict(newest.slice(0, cap), { cap: Number.POSITIVE_INFINITY, free });
  for (const { path } of past) rmSync(path, { force: true });
  return { removed: past.length + within.removed, size: within.size };
}

/** What the working tree's change runs, by the newest map of an ancestor of
 * HEAD: its changes since the map's commit, the working tree's and new
 * files' too. */
export function pickChange(files: string[], cases: string[]): Picked {
  for (const path of keptMapFiles()) {
    let map: TestMap;
    try {
      map = JSON.parse(readFileSync(path, "utf8"));
    } catch {
      continue;
    }
    if (map.version !== VERSION) continue;
    if (git(["merge-base", "--is-ancestor", map.commit, "HEAD"]).code !== 0) continue;
    const now = Date.now() / 1000;
    utimesSync(path, now, now);
    const diff = git(["diff", "-U0", "--no-color", "--no-ext-diff", "--no-renames", map.commit]).out;
    const untracked = git(["ls-files", "--others", "--exclude-standard"]).out.split("\n").filter(Boolean);
    const changes = [...hunksOf(diff), ...untracked.map((file): Change => ({ file, hunks: "new" }))];
    const picked = pick(map, changes, files, cases);
    const of = `the test map of ${map.commit.slice(0, 9)}`;
    return picked.kind === "all" ? { kind: "all", why: `${of} can't say: ${picked.why}` } : { ...picked, why: `${of}: ${picked.why}` };
  }
  return { kind: "all", why: "no test map of HEAD or an ancestor of it: `bun run test-map` records one" };
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
      // What's kept by source alone, native programs' builds and verified
      // runs, no compile of rust-js's, shared; all else the unit's own.
      for (const kept of ["native-cache", "corpus-cache"]) {
        mkdirSync(join(root, "target", kept), { recursive: true });
        symlinkSync(join(root, "target", kept), join(own, kept));
      }
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
  if (git(["status", "--porcelain"]).out.trim() !== "") {
    console.error("the test map is of a commit: commit or remove what the working tree changes first");
    process.exit(1);
  }
  const commit = git(["rev-parse", "HEAD"]).out.trim();
  const started = Date.now();
  const tools = llvmTools();
  // The counting build, in a target of its own, so `target/debug`'s isn't
  // built again; its dependencies are sccache's, as `cargo build`'s are.
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
  const built = Bun.spawnSync(["cargo", "rustc", "--locked", "--quiet", "--bin", "rust-js", "--", "-C", "instrument-coverage", "-C", `link-arg=${join(mapDir, "lost.o")}`], {
    cwd: root,
    env: { ...process.env, CARGO_TARGET_DIR: build },
    stdout: "inherit",
    stderr: "inherit",
  });
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
    ...files.map((file) => ({ name: file, cmd: ["bun", "test", file], env: { RUST_JS_TEST_MAP_UNIT: file } })),
  ];
  // The longest first, by what check:fast keeps of each file's time, so no
  // long one runs alone at the end; the corpus's parts first of all.
  const timings = existsSync(join(root, "target", "test-timings.json")) ? JSON.parse(readFileSync(join(root, "target", "test-timings.json"), "utf8")) : {};
  units.sort((a, b) => (a.name.startsWith("test/corpus.test.ts") ? -1 : 0) - (b.name.startsWith("test/corpus.test.ts") ? -1 : 0) || (timings[b.name] ?? 0) - (timings[a.name] ?? 0));
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
          .map((line) => line.split(" ")[0]),
      )
    : new Set<string>();
  const caseNames = readdirSync(join(root, "test", "corpus")).filter((name) => name.endsWith(".rs"));
  const cases = caseNames.map((name) => `test/corpus/${name}`);
  // Of a part that failed, each of its cases, as the corpus shards them.
  const failedCases = new Set<string>();
  let corpusFailed = false;
  for (let i = 1; i <= parts; i++) {
    if (ended.get(`test/corpus.test.ts ${i}/${parts}`) === 0) continue;
    corpusFailed = true;
    for (const name of shard(caseNames, i, parts)) failedCases.add(`test/corpus/${name}`);
  }
  const units_: TestMap["units"] = {};
  const always: string[] = [];
  for (const unit of [...files, "test/corpus.test.ts", ...cases]) {
    const isCase = unit.startsWith("test/corpus/");
    const failed = isCase ? failedCases.has(unit) : unit === "test/corpus.test.ts" ? corpusFailed : ended.get(unit) !== 0;
    const own = bits.get(unit);
    const why = failed
      ? "it failed"
      : own === undefined
        ? "it ran no compiler"
        : unfinished.has(unit)
          ? "a compile of it wasn't counted whole"
          : unknown.has(unit)
            ? "a compile of it ran a function the table hasn't"
            : !isCase && readsSources(unit)
              ? "it reads the compiler's sources"
              : undefined;
    units_[unit] = why === undefined ? Buffer.from(own!).toString("base64") : null;
    if (why !== undefined) always.push(`${unit} (${why})`);
  }
  // The plain suites' files that read the corpus's files: each says where.
  const corpusReaders = files.filter((file) => /\bcorpus\b/.test(readFileSync(join(root, file), "utf8")));
  const map: TestMap = { version: VERSION, commit, made: Date.now(), functions, units: units_, includes: includes(), corpusReaders };
  mkdirSync(maps, { recursive: true });
  writeFileSync(join(maps, `${commit}.json`), JSON.stringify(map));
  pruneMaps();
  // What only this recording needed, its counts and each test's target,
  // kept with `--keep`, to see why a test runs on every change. Its build is
  // kept, one, as `target/debug` is, so the next recording builds only
  // what changed.
  if (!keep) rmSync(recording, { recursive: true, force: true });
  const cased = always.filter((unit) => unit.startsWith("test/corpus/"));
  console.log(`the test map of ${commit.slice(0, 9)}: ${functions.length} functions, ${Object.keys(units_).length} tests; ${always.length} run on every change:`);
  for (const unit of always.filter((unit) => !unit.startsWith("test/corpus/"))) console.log(`  ${unit}`);
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
      console.log(`cases (${picked.cases.length} of ${cases.length})`);
    }
  } else {
    console.error("usage: bun run test-map [record | pick]");
    process.exit(2);
  }
}
