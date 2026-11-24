// A compile kept by its exact inputs is given again only where every one
// of them is the same (test/compiles.ts, ADR 1099): a changed source, a
// file it read, an option, a variable of its environment, a crate where it
// searches, or another compiler compiles again. And what's kept is
// bounded: another build's dropped, the least recently used past the cap.

import { beforeAll, expect, test } from "bun:test";
import { chmodSync, cpSync, existsSync, mkdirSync, readdirSync, readFileSync, rmSync, statSync, utimesSync, writeFileSync } from "node:fs";
import { join, resolve } from "node:path";

import type { Exit } from "./child";
import { bytesOf, freeBytes } from "../tooling/bounded.js";
import { floorBytes } from "./bounded";
import { capBytes, compileCache, currentBuild, keptCompile, parseRecord, prune, unread } from "./compiles";
import { target } from "./packages-of";
import { buildCompiler, compiler, fixture, root } from "./support";
import { compilerRustFiles } from "../scripts/compiler-crates";

beforeAll(buildCompiler, 600_000);

// These tests are of what's kept, and a compile asked to count its
// decisions (ADR 1124), as every one of the nightly's is, is never kept:
// they count none, but the one that says it isn't.
delete process.env.RUST_JS_DECISIONS;
// Nor a mutant's, whose runner keeps none of its compiles: these test what
// the cache does, so in a mutant's environment too, a `--retire`'s, they
// ask it as a build does, or each counts a compile more than it keeps.
delete process.env.RUST_JS_MUTANT;

/** `cmd` run as it is, each run counted; and an environment of this run's
 * own, so what an earlier run kept, in the same fixture, isn't found. */
function counting() {
  const ran: string[][] = [];
  const own = { COMPILES_TEST_RUN: crypto.randomUUID() };
  const compile = (cmd: string[]): Exit => {
    ran.push(cmd);
    const p = Bun.spawnSync(cmd, { cwd: root, stdout: "pipe", stderr: "pipe" });
    return { code: p.exitCode, signal: null, timedOut: false, overflowed: false, stdout: p.stdout.toString(), stderr: p.stderr.toString(), bytes: { stdout: p.stdout, stderr: p.stderr } };
  };
  return { ran, compile, own };
}

/** A disk with room, so what a test keeps is kept however full this
 * machine's is (#963): a test of the real disk's floor is its own. */
const room = () => Number.POSITIVE_INFINITY;

const lib = `pub const TEXT: &str = include_str!("text.txt");\npub fn text() -> &'static str {\n    TEXT\n}\n`;

test("a compile is given again only where all it's made of is the same", () => {
  const dir = fixture("kept-inputs");
  writeFileSync(join(dir, "lib.rs"), lib);
  writeFileSync(join(dir, "text.txt"), "first");
  const js = join(dir, "lib.js");
  const { ran, compile, own } = counting();
  // `flags` rustc's, after `--`; `before` rust-js's own.
  const kept = (flags: string[] = [], env: Record<string, string> = {}, before: string[] = []) =>
    keptCompile([compiler, join(dir, "lib.rs"), "-o", js, ...before, "--", "-Awarnings", ...flags], root, { ...own, ...env }, compile, room);

  expect(kept().code).toBe(0);
  expect(ran.length).toBe(1);
  const first = readFileSync(js, "utf8");
  expect(first).toContain('"first"');
  // The same again: what it wrote, written again, and nothing compiled.
  rmSync(js);
  expect(kept().code).toBe(0);
  expect(ran.length).toBe(1);
  expect(readFileSync(js, "utf8")).toBe(first);
  // A file it read changed, which only its record names.
  writeFileSync(join(dir, "text.txt"), "second");
  kept();
  expect(ran.length).toBe(2);
  expect(readFileSync(js, "utf8")).toContain('"second"');
  // An option.
  kept(["--cfg", "other"]);
  expect(ran.length).toBe(3);
  // One of rust-js's own, before `--`: the corpus with std's iterator
  // structs is compiled so, its JS another (#2501).
  kept(["--cfg", "other"], {}, ["--std-iterators"]);
  expect(ran.length).toBe(4);
  kept(["--cfg", "other"], {}, ["--std-iterators"]);
  expect(ran.length).toBe(4);
  // A variable of its environment, an `env!`'s or not.
  kept([], { COMPILES_TEST_VALUE: "a" });
  expect(ran.length).toBe(5);
  kept([], { COMPILES_TEST_VALUE: "a" });
  expect(ran.length).toBe(5);
  // A crate where it searches, which another of the same name would be.
  const search = join(dir, "search");
  mkdirSync(search);
  kept(["-L", search]);
  expect(ran.length).toBe(6);
  writeFileSync(join(search, "libother.rmeta"), "");
  kept(["-L", search]);
  expect(ran.length).toBe(7);
  // The source itself, back as it was first: that compile again.
  writeFileSync(join(dir, "text.txt"), "first");
  kept();
  expect(ran.length).toBe(7);
  expect(readFileSync(js, "utf8")).toBe(first);
  // A compile asked to keep none.
  kept([], { RUST_JS_COMPILE_CACHE_MB: "0" });
  expect(ran.length).toBe(8);
});

// What a compile writes beside its JS where it's asked, the facts a reader
// found, the passes that fired and the recognisers that took a call, is of
// what it wrote, which its record names: given again, it's written again,
// where it was missing, ENOENT, the second time a test ran (#1526).
test("what a compile reports where it's asked is written again where it's given again", () => {
  const dir = fixture("kept-reports");
  writeFileSync(join(dir, "lib.rs"), lib);
  writeFileSync(join(dir, "text.txt"), "first");
  const reports = ["found.txt", "fired.txt", "recognized.txt", "tidy.txt"].map((file) => join(dir, file));
  const [found, fired, recognized, tidy] = reports;
  const { ran, compile, own } = counting();
  const kept = () =>
    keptCompile([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--knowns-found", found, "--prepare-fired", fired, "--recognizers-found", recognized, "--tidy-fired", tidy, "--", "-Awarnings"], root, own, compile, room);
  expect(kept().code).toBe(0);
  const first = reports.map((file) => readFileSync(file, "utf8"));
  for (const file of reports) rmSync(file);
  expect(kept().code).toBe(0);
  expect(ran.length).toBe(1);
  expect(reports.map((file) => readFileSync(file, "utf8"))).toEqual(first);
});

// Which its record says it wrote, as a compile the tests' cache keeps
// reads it (#1526).
test("a compile's record names what it reports where it's asked as written", () => {
  const dir = fixture("recorded-reports");
  writeFileSync(join(dir, "lib.rs"), lib);
  writeFileSync(join(dir, "text.txt"), "first");
  const reports = ["found.txt", "fired.txt", "recognized.txt", "tidy.txt"].map((file) => join(dir, file));
  const [found, fired, recognized, tidy] = reports;
  const record = join(dir, "lib.d");
  const p = Bun.spawnSync([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--knowns-found", found, "--prepare-fired", fired, "--recognizers-found", recognized, "--tidy-fired", tidy, "--", `--emit=dep-info=${record}`, "-Awarnings"], { cwd: root, stderr: "pipe" });
  expect(p.stderr.toString()).toBe("");
  expect(parseRecord(readFileSync(record, "utf8")).wrote).toEqual(expect.arrayContaining(reports));
});

test("another compiler's compiles, a failed compile and one that read what it wrote aren't kept", () => {
  const dir = fixture("kept-not");
  writeFileSync(join(dir, "lib.rs"), lib);
  writeFileSync(join(dir, "text.txt"), "text");
  const { ran, compile, own } = counting();
  // A compiler not the tests', the same build run by a script.
  const script = join(dir, "rust-js");
  writeFileSync(script, `#!/bin/sh\nexec ${JSON.stringify(compiler)} "$@"\n`);
  chmodSync(script, 0o755);
  for (let i = 0; i < 2; i++) keptCompile([script, join(dir, "lib.rs"), "-o", join(dir, "a.js")], root, own, compile, room);
  expect(ran.length).toBe(2);
  // A compile that fails.
  writeFileSync(join(dir, "bad.rs"), "pub fn f() -> u8 { 300 }\n");
  for (let i = 0; i < 2; i++) expect(keptCompile([compiler, join(dir, "bad.rs"), "-o", join(dir, "bad.js")], root, own, compile, room).code).not.toBe(0);
  expect(ran.length).toBe(4);
  // Its manifest written by a compile of another source, which it reads
  // for what's stale: compiled, and not kept.
  const manifest = join(dir, "m.json");
  const withManifest = () => keptCompile([compiler, join(dir, "lib.rs"), "-o", join(dir, "m.js"), "--manifest", manifest], root, own, compile, room);
  withManifest();
  expect(ran.length).toBe(5);
  writeFileSync(join(dir, "lib.rs"), `${lib}pub mod more {}\n`);
  withManifest();
  expect(ran.length).toBe(6);
  writeFileSync(join(dir, "lib.rs"), lib);
  withManifest();
  withManifest();
  expect(ran.length).toBe(7);
});

// A compile asked to count its decisions (ADR 1124) appends its counts
// to a file no record names: given again, nothing would be counted, so
// it's compiled each time; and its build is still named, as a Crater of
// the nightly's, which counts, names it.
test("a compile that counts its decisions isn't kept, and its build is named", () => {
  const dir = fixture("kept-counting");
  writeFileSync(join(dir, "lib.rs"), lib);
  writeFileSync(join(dir, "text.txt"), "counted");
  const { ran, compile, own } = counting();
  const counts = { ...own, RUST_JS_DECISIONS: join(dir, "counts.tsv") };
  for (let i = 0; i < 2; i++) keptCompile([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "-Awarnings"], root, counts, compile, room);
  expect(ran.length).toBe(2);
  process.env.RUST_JS_DECISIONS = counts.RUST_JS_DECISIONS;
  try {
    expect(currentBuild(compiler)).toMatch(/^[0-9a-f]{24}$/);
  } finally {
    delete process.env.RUST_JS_DECISIONS;
  }
});

// The real disk's floor (#963): a compile is kept where the disk the cache
// is on has room above it, and not where it hasn't, each as this machine's
// disk is now, so the test holds on a full disk and an empty one alike.
test("a compile is kept only where the real disk is above its floor", () => {
  const dir = fixture("kept-floor");
  writeFileSync(join(dir, "lib.rs"), lib);
  writeFileSync(join(dir, "text.txt"), "floor");
  const { ran, compile, own } = counting();
  const kept = () => keptCompile([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "-Awarnings"], root, own, compile);
  const above = freeBytes(target) >= floorBytes + 2 ** 30;
  const below = freeBytes(target) < floorBytes;
  kept();
  kept();
  // Kept, given again; or not kept, compiled again. Within a GB of the
  // floor, where the compile's own bytes may cross it, either.
  if (above) expect(ran.length).toBe(1);
  if (below) expect(ran.length).toBe(2);
  expect(ran.length).toBeGreaterThanOrEqual(1);
});

test("a file's record of its build says what it wrote and all it read, and changes nothing it writes", () => {
  const dir = fixture("record");
  mkdirSync(join(dir, "out"));
  writeFileSync(join(dir, "lib.rs"), `${lib}pub const SAID: Option<&str> = option_env!("COMPILES_RECORD_TEST");\npub mod shape;\n`);
  writeFileSync(join(dir, "shape.rs"), "pub fn sides() -> u32 {\n    4\n}\n");
  writeFileSync(join(dir, "text.txt"), "text");
  const record = join(dir, "lib.d");
  const args = [join(dir, "lib.rs"), "-o", join(dir, "out", "lib.js"), "--manifest", join(dir, "out", "m.json")];
  const recorded = Bun.spawnSync([compiler, ...args, "--", `--emit=dep-info=${record}`], { cwd: root, stderr: "pipe" });
  expect(recorded.stderr.toString()).toBe("");
  const written = Object.fromEntries(["lib.js", "lib.js.map", "shape.js", "shape.js.map", "m.json"].map((f) => [f, readFileSync(join(dir, "out", f), "utf8")]));
  const { wrote, read, env } = parseRecord(readFileSync(record, "utf8"));
  expect(wrote.sort()).toEqual(["lib.js", "lib.js.map", "m.json", "shape.js", "shape.js.map"].map((f) => join(dir, "out", f)).sort());
  for (const file of ["lib.rs", "shape.rs", "text.txt"]) expect(read).toContain(join(dir, file));
  // The crates it loaded, std's among them, the compiler, its settings,
  // and the variable an `option_env!` read; but no manifest before it.
  expect(read.some((file) => /\/libstd-[0-9a-f]+\.rmeta$/.test(file))).toBe(true);
  expect(read).toContain(compiler);
  expect(read).toContain(join(root, "Cargo.toml"));
  expect(read).not.toContain(join(dir, "out", "m.json"));
  expect(env).toEqual(["# env-dep:COMPILES_RECORD_TEST"]);
  // Again, its manifest there: read for what's stale.
  Bun.spawnSync([compiler, ...args, "--", `--emit=dep-info=${record}`], { cwd: root });
  expect(parseRecord(readFileSync(record, "utf8")).read).toContain(join(dir, "out", "m.json"));
  // Without it, the same files, byte for byte.
  rmSync(join(dir, "out"), { recursive: true });
  expect(Bun.spawnSync([compiler, ...args], { cwd: root }).exitCode).toBe(0);
  for (const [file, text] of Object.entries(written)) expect([file, readFileSync(join(dir, "out", file), "utf8")]).toEqual([file, text]);
  // Declarations printed by the printer `RUST_JS_TYPESCRIPT` names: its
  // module, its package's manifest, and the TypeScript it finds, which pins
  // the release it runs, are what it read (ADR 1099, amended).
  const printer = Bun.resolveSync("@rust-js/typescript", root);
  const declaredBy = Bun.spawnSync([compiler, ...args, "--declarations", "--", `--emit=dep-info=${record}`], {
    cwd: root,
    stderr: "pipe",
    env: { ...process.env, RUST_JS_TYPESCRIPT: printer },
  });
  expect(declaredBy.stderr.toString()).toBe("");
  expect(parseRecord(readFileSync(record, "utf8")).read).toEqual(
    expect.arrayContaining([printer, join(root, "typescript", "package.json"), join(root, "typescript", "node_modules", "typescript", "package.json")]),
  );
  // What it can't say all of is refused: declarations a crate's own
  // TypeScript, which Node finds, prints, and a crate's hooks, programs of
  // its own.
  const { RUST_JS_TYPESCRIPT: _, ...unnamed } = process.env;
  const declared = Bun.spawnSync([compiler, ...args, "--declarations", "--", `--emit=dep-info=${record}`], { cwd: root, stderr: "pipe", env: unnamed });
  expect(declared.stderr.toString()).toContain("--emit=dep-info can't say all a build reads that prints declarations");
  writeFileSync(join(dir, "Cargo.toml"), `[package]\nname = "hooked"\nversion = "0.0.0"\nedition = "2024"\n\n[package.metadata.rust-js.hooks]\ntransform = [["cat"]]\n`);
  const hooked = Bun.spawnSync([compiler, ...args, "--", `--emit=dep-info=${record}`], { cwd: root, stderr: "pipe" });
  expect(hooked.stderr.toString()).toContain("--emit=dep-info can't say all a build reads whose crate has hooks");
});

// A kept compile is named by its size as it lies on the disk, every file's
// blocks, `entry.json`'s too, as every store of the loop's is counted
// (`bytesOf`): its files' lengths alone counted 2.0 GiB of a store that
// took 5.1, which `prune` then held to its cap only on paper.
test("a kept compile is counted by its blocks on the disk, so a store pruned to its cap is at or under it", () => {
  const dir = fixture("kept-blocks");
  writeFileSync(join(dir, "lib.rs"), lib);
  writeFileSync(join(dir, "text.txt"), "blocks");
  const { compile, own } = counting();
  const started = Date.now() - 1000;
  expect(keptCompile([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "-Awarnings"], root, own, compile, room).code).toBe(0);
  // The store is shared with what else runs, which prunes it as it keeps
  // its own compiles: a directory listed may be gone when it's read, as
  // an entry may be when it's stated (#2202).
  const names = (dir: string) => {
    try {
      return readdirSync(dir);
    } catch {
      return [];
    }
  };
  const entries = (store: string) =>
    names(store)
      .filter((build) => !build.startsWith("."))
      .flatMap((build) => names(join(store, build)).flatMap((key) => names(join(store, build, key)).map((name) => join(store, build, key, name))))
      .filter((at) => !at.split("/").at(-1)!.startsWith("."));
  // Its own compile's alone, the entry that read its `lib.rs`: the store is
  // shared with what else runs, whose entries come and go as it's read
  // (#2107).
  const ownEntry = (at: string) => {
    try {
      const read: [string, string][] = JSON.parse(readFileSync(join(at, "entry.json"), "utf8")).read;
      return read.some(([file]) => resolve(root, file) === join(dir, "lib.rs"));
    } catch {
      return false;
    }
  };
  const kept = entries(join(compileCache, "v2")).filter((at) => ownEntry(at) && (statSync(at, { throwIfNoEntry: false })?.mtimeMs ?? 0) >= started);
  expect(kept.length).toBeGreaterThan(0);
  for (const at of kept) expect(Number(/-(\d+)$/.exec(at)?.[1])).toBe(bytesOf(at));
  // Five of it in a store of its own, pruned to the room of two and a half:
  // what's left, as the disk measures it, is at or under the cap.
  const cache = join(fixture("kept-blocks-store"), "compile-cache");
  const each = bytesOf(kept[0]!);
  const name = kept[0]!.split("/").at(-1)!;
  for (let i = 0; i < 5; i++) {
    const at = join(cache, "v2", "current", `k${i}`, name);
    cpSync(kept[0]!, at, { recursive: true });
    utimesSync(at, 1_000 + i, 1_000 + i);
  }
  const cap = Math.floor(2.5 * each);
  expect(prune({ root: cache, current: "current", cap, free: () => Infinity }).removed).toBe(3);
  const left = entries(join(cache, "v2"));
  expect(left.length).toBe(2);
  expect(left.reduce((sum, at) => sum + bytesOf(at), 0)).toBeLessThanOrEqual(cap);
});

test("what's kept is bounded: another build's dropped, then the least recently used past the cap", () => {
  const cache = join(fixture("bounded"), "compile-cache");
  const entry = (build: string, key: string, name: string, bytes: number, used: number) => {
    const at = join(cache, "v2", build, key, `${name}-${bytes}`);
    mkdirSync(at, { recursive: true });
    writeFileSync(join(at, "0"), Buffer.alloc(bytes));
    utimesSync(at, used, used);
    return at;
  };
  const old = entry("another", "k", "a", 100, 1_000);
  const oldest = entry("current", "k1", "a", 400, 2_000);
  const older = entry("current", "k2", "b", 400, 3_000);
  const newer = entry("current", "k2", "c", 400, 4_000);
  const newest = entry("current", "k3", "d", 400, 5_000);
  const layout = join(cache, "v1");
  mkdirSync(layout);
  const pruned = prune({ root: cache, current: "current", cap: 1_000, free: () => Infinity });
  expect(pruned).toEqual({ removed: 2, size: 800, builds: 1 });
  expect([old, oldest, older, newer, newest, layout].map(existsSync)).toEqual([false, false, false, true, true, false]);
  expect(readFileSync(join(cache, "size"), "utf8")).toBe("800");
  // Under the disk's floor, it's pruned until it's above it, or empty:
  // here, once the least recently used is gone.
  const freed = prune({ root: cache, current: "current", cap: 1_000, free: () => (existsSync(newer) ? floorBytes - 1 : floorBytes) });
  expect(freed.removed).toBe(1);
  expect([newer, newest].map(existsSync)).toEqual([false, true]);
  expect(prune({ root: cache, current: "current", cap: 1_000, free: () => 0 })).toEqual({ removed: 1, size: 0, builds: 0 });
  // The default cap.
  expect(capBytes()).toBe(Number(process.env.RUST_JS_COMPILE_CACHE_MB ?? 2048) * 2 ** 20);
  expect(compileCache).toStartWith(join(root, "target"));
});

// Every name in the store that isn't a kept compile is a dot-name, which no
// reader takes for one: a key's mark of a compile it can't record,
// `.unrecordable`, and what a writer's building, `.building-..`. A mark
// named otherwise was read as a kept compile with no size, `NaN` (#2107).
test("the store's other names are dot-names, which pruning neither counts nor removes as kept compiles", () => {
  const cache = join(fixture("dot-names"), "compile-cache");
  const key = join(cache, "v2", "current", "k");
  const kept = join(key, "a-100");
  mkdirSync(kept, { recursive: true });
  writeFileSync(join(kept, "0"), Buffer.alloc(100));
  writeFileSync(join(key, ".unrecordable"), "");
  expect(prune({ root: cache, current: "current", cap: 1_000, free: () => Infinity })).toEqual({ removed: 0, size: 100, builds: 0 });
  expect([kept, join(key, ".unrecordable")].map(existsSync)).toEqual([true, true]);
  expect(prune({ root: cache, current: "current", cap: 0, free: () => Infinity }).removed).toBe(1);
  expect(existsSync(join(key, ".unrecordable"))).toBe(true);
});

// What a writer is building, or a writer that ended before it finished
// left, is on the disk: counted in what's kept, never evicted as a kept
// compile is, till a prune an hour on removes what's left (#2107).
test("a compile being built counts in the store's size till a prune an hour on removes it", () => {
  const cache = join(fixture("building-counted"), "compile-cache");
  const key = join(cache, "v2", "current", "k");
  const building = join(key, ".building-1-young");
  mkdirSync(building, { recursive: true });
  writeFileSync(join(building, "0"), Buffer.alloc(8192));
  const bytes = bytesOf(building);
  expect(prune({ root: cache, current: "current", cap: 1_000_000, free: () => Infinity })).toEqual({ removed: 0, size: bytes, builds: 0 });
  expect(existsSync(building)).toBe(true);
  const hourAgo = (Date.now() - 2 * 60 * 60 * 1000) / 1000;
  utimesSync(building, hourAgo, hourAgo);
  expect(prune({ root: cache, current: "current", cap: 1_000_000, free: () => Infinity })).toEqual({ removed: 0, size: 0, builds: 0 });
  expect(existsSync(building)).toBe(false);
});

test("what the key leaves out of the environment, the compiler never reads", () => {
  const text = compilerRustFiles()
    .map((file) => readFileSync(join(root, file), "utf8"))
    .join("\n");
  // `_`, the shell's last command, is no name a source would spell.
  expect(unread.filter((name) => name !== "_" && text.includes(`"${name}"`))).toEqual([]);
});

// A compile is kept under the build that ran it (#2363): where the
// compiler is linked again while it runs, as Cargo does, its output is
// another build's, kept under none. The one test that moves the
// compiler's time, put back as it was.
test("a compile whose compiler is linked again as it runs isn't kept", () => {
  const dir = fixture("kept-relinked");
  writeFileSync(join(dir, "lib.rs"), "pub fn one() -> u32 {\n    1\n}\n");
  const { ran, compile, own } = counting();
  const cmd = [compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "-Awarnings"];
  const before = statSync(compiler);
  // Another link of it in its place as it compiles: its time moved.
  const relinking = (given: string[]) => {
    const p = compile(given);
    utimesSync(compiler, before.atime, new Date(before.mtimeMs + 2000));
    return p;
  };
  try {
    expect(keptCompile(cmd, root, own, relinking, room).code).toBe(0);
    utimesSync(compiler, before.atime, before.mtime);
    expect(keptCompile(cmd, root, own, compile, room).code).toBe(0);
    expect(ran.length).toBe(2);
  } finally {
    utimesSync(compiler, before.atime, before.mtime);
  }
}, 300_000);

// And an entry another build kept is never given again where this one's
// is asked, however its key came to be shared (#2363).
test("an entry another build kept is never given again", () => {
  const dir = fixture("kept-of-another-build");
  writeFileSync(join(dir, "lib.rs"), "pub fn two() -> u32 {\n    2\n}\n");
  const { ran, compile, own } = counting();
  const cmd = [compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "-Awarnings"];
  expect(keptCompile(cmd, root, own, compile, room).code).toBe(0);
  const store = join(compileCache, "v2");
  const mine = readdirSync(store)
    .filter((build) => !build.startsWith("."))
    .flatMap((build) => readdirSync(join(store, build)).map((key) => join(store, build, key)))
    .flatMap((at) => (statSync(at).isDirectory() ? readdirSync(at).filter((name) => !name.startsWith(".")).map((name) => join(at, name, "entry.json")) : []))
    .filter((file) => existsSync(file) && readFileSync(file, "utf8").includes(join(dir, "lib.rs").slice(root.length + 1)));
  expect(mine.length).toBeGreaterThan(0);
  for (const file of mine) writeFileSync(file, JSON.stringify({ ...JSON.parse(readFileSync(file, "utf8")), identity: "another build" }));
  const compiled = ran.length;
  expect(keptCompile(cmd, root, own, compile, room).code).toBe(0);
  expect(ran.length).toBe(compiled + 1);
}, 300_000);
