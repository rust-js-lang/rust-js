// A compile kept by its exact inputs is given again only where every one
// of them is the same (test/compiles.ts, ADR 1099): a changed source, a
// file it read, an option, a variable of its environment, a crate where it
// searches, or another compiler compiles again. And what's kept is
// bounded: another build's dropped, the least recently used past the cap.

import { beforeAll, expect, test } from "bun:test";
import { chmodSync, existsSync, mkdirSync, readdirSync, readFileSync, rmSync, utimesSync, writeFileSync } from "node:fs";
import { join } from "node:path";

import type { Exit } from "./child";
import { freeBytes } from "../tooling/bounded.js";
import { floorBytes } from "./bounded";
import { capBytes, compileCache, keptCompile, parseRecord, prune, unread } from "./compiles";
import { target } from "./packages-of";
import { buildCompiler, compiler, fixture, root } from "./support";

beforeAll(buildCompiler, 600_000);

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
  const kept = (flags: string[] = [], env: Record<string, string> = {}) => keptCompile([compiler, join(dir, "lib.rs"), "-o", js, "--", "-Awarnings", ...flags], root, { ...own, ...env }, compile, room);

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
  // A variable of its environment, an `env!`'s or not.
  kept([], { COMPILES_TEST_VALUE: "a" });
  expect(ran.length).toBe(4);
  kept([], { COMPILES_TEST_VALUE: "a" });
  expect(ran.length).toBe(4);
  // A crate where it searches, which another of the same name would be.
  const search = join(dir, "search");
  mkdirSync(search);
  kept(["-L", search]);
  expect(ran.length).toBe(5);
  writeFileSync(join(search, "libother.rmeta"), "");
  kept(["-L", search]);
  expect(ran.length).toBe(6);
  // The source itself, back as it was first: that compile again.
  writeFileSync(join(dir, "text.txt"), "first");
  kept();
  expect(ran.length).toBe(6);
  expect(readFileSync(js, "utf8")).toBe(first);
  // A compile asked to keep none.
  kept([], { RUST_JS_COMPILE_CACHE_MB: "0" });
  expect(ran.length).toBe(7);
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
  // What it can't say all of is refused: TypeScript's declarations, and a
  // crate's hooks, programs of its own.
  const declared = Bun.spawnSync([compiler, ...args, "--declarations", "--", `--emit=dep-info=${record}`], { cwd: root, stderr: "pipe" });
  expect(declared.stderr.toString()).toContain("--emit=dep-info can't say all a build reads that prints declarations");
  writeFileSync(join(dir, "Cargo.toml"), `[package]\nname = "hooked"\nversion = "0.0.0"\nedition = "2024"\n\n[package.metadata.rust-js.hooks]\ntransform = [["cat"]]\n`);
  const hooked = Bun.spawnSync([compiler, ...args, "--", `--emit=dep-info=${record}`], { cwd: root, stderr: "pipe" });
  expect(hooked.stderr.toString()).toContain("--emit=dep-info can't say all a build reads whose crate has hooks");
});

test("what's kept is bounded: another build's dropped, then the least recently used past the cap", () => {
  const cache = join(fixture("bounded"), "compile-cache");
  const entry = (build: string, key: string, name: string, bytes: number, used: number) => {
    const at = join(cache, "v1", build, key, `${name}-${bytes}`);
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
  const layout = join(cache, "v0");
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

test("what the key leaves out of the environment, the compiler never reads", () => {
  const sources = (dir: string): string[] =>
    readdirSync(dir, { recursive: true, withFileTypes: true })
      .filter((entry) => entry.isFile() && entry.name.endsWith(".rs"))
      .map((entry) => readFileSync(join(entry.parentPath, entry.name), "utf8"));
  const text = sources(join(root, "src")).join("\n");
  // `_`, the shell's last command, is no name a source would spell.
  expect(unread.filter((name) => name !== "_" && text.includes(`"${name}"`))).toEqual([]);
});
