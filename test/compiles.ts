// Each compile the tests make, kept by its exact inputs, and given again
// where they're the same, not compiled again (ADR 1099). Its output is a
// pure function of them (#866's net, `test/determinism.test.ts`):
//
//   - the compiler, by its build's identity (ADR 1003), which any change to
//     its sources changes: the one the tests are given, a build of
//     rust-js, never a script standing in for it;
//   - its arguments, where it's run, and its environment, all of it but
//     what only the tests' harness reads (`unread`);
//   - each file it read, as its record of the build says
//     (`--emit=dep-info`, which names every file it wrote too), by its
//     content: its sources, the crates it loaded, its settings and its
//     libraries; and the names of the crates in each `-L` directory, where
//     another of the same name would be found.
//
// The first three are the key, a directory; the files read, which only a
// compile finds, each an entry of it, with what it wrote and printed. One
// that read a file it wrote, as a build reads the manifest of the one
// before, is not kept. A compile that fails isn't, nor one whose record
// rust-js can't make whole: of a crate with hooks, or `--declarations`.
//
// Kept in `target/compile-cache/v<version>/<build>/<key>/<entry>-<bytes>`,
// each use marked. Bounded (`prune`): another build's entries go as a run
// starts, as none is found again; past the cap, the least recently used;
// and nothing's written below the disk's floor, where what's kept is pruned
// down to give it back.

import { createHash } from "node:crypto";
import { closeSync, existsSync, mkdirSync, openSync, readdirSync, readFileSync, readSync, renameSync, rmSync, statSync, utimesSync, writeFileSync } from "node:fs";
import { basename, join, resolve } from "node:path";

import { evict, freeBytes } from "../tooling/bounded.js";
import { floorBytes } from "./bounded";
import type { Exit } from "./child";
import { target } from "./packages-of";
import { compiler as testsCompiler } from "./support";

/** Where compiles are kept, and the layout's version, which a change to
 * the layout or the key bumps. */
export const compileCache = join(target, "compile-cache");
const VERSION = 1;
const store = join(compileCache, `v${VERSION}`);

/** The most it keeps: 2 GB, or `RUST_JS_COMPILE_CACHE_MB`; 0 keeps none,
 * and a compile given it in its environment is neither kept nor found, as
 * the determinism net's are. */
export const capBytes = (env: Record<string, string | undefined> = {}) => Number(env.RUST_JS_COMPILE_CACHE_MB ?? process.env.RUST_JS_COMPILE_CACHE_MB ?? 2048) * 2 ** 20;

/** What only the tests' harness reads of the environment, never the
 * compiler (`test/compiles.test.ts` holds it to the compiler's sources):
 * a check's job's share of the cores, its part of the cases, the compiler
 * by its path, which a check links anew each run, and the shell's. */
export const unread = ["RUST_JS_LOAD_SCALE", "RUST_JS_CASES", "RUST_JS_COMPILER", "RUST_JS_COMPILE_CACHE_MB", "RUST_JS_KEEP_FIXTURES", "RUST_JS_COMPILE_TIMEOUT", "BUN_TEST_WORKER_ID", "_", "PWD", "OLDPWD", "SHLVL"];

const sha = (...parts: (string | Uint8Array)[]) => {
  const hash = createHash("sha256");
  for (const part of parts) hash.update(`${part.length}\0`).update(part);
  return hash.digest("hex");
};

type Stat = { dev: number; ino: number; size: number; mtimeMs: number; ctimeMs: number };
const statOf = (path: string): Stat | undefined => {
  try {
    return statSync(path);
  } catch {
    return undefined;
  }
};

/** The compiler `cmd` runs, if it's the one the tests are given, and a
 * build of rust-js, not a script standing in for it: its identity, its
 * build's (ADR 1003, `--version-json`), the same however often Cargo links
 * it again, as `cargo test` does. Asked once a process of each file. */
const identities = new Map<string, string | undefined>();
function compilerIdentity(program: string, env: Record<string, string | undefined>): { identity: string; file: Stat } | undefined {
  if (process.env.RUST_JS_MUTANT || capBytes(env) <= 0) return undefined;
  const ours = statOf(testsCompiler);
  const given = statOf(program);
  if (!ours || !given || ours.dev !== given.dev || ours.ino !== given.ino) return undefined;
  const file = `${given.dev}:${given.ino}:${given.size}:${given.mtimeMs}`;
  if (!identities.has(file)) {
    let identity: string | undefined;
    try {
      const head = Buffer.alloc(2);
      const fd = openSync(program, "r");
      readSync(fd, head, 0, 2, 0);
      closeSync(fd);
      const p = head.toString() === "#!" ? undefined : Bun.spawnSync([program, "--version-json"], { stdout: "pipe", stderr: "pipe" });
      const json = p?.exitCode === 0 ? p.stdout.toString() : "";
      if (/"build": *"[0-9a-f]+"/.test(json)) identity = sha(json);
    } catch {}
    identities.set(file, identity);
  }
  const identity = identities.get(file);
  return identity === undefined ? undefined : { identity, file: given };
}

/** The name of the tests' compiler's build's kept compiles, `prune`'s
 * `current`. */
export function currentBuild(program = testsCompiler): string | undefined {
  return compilerIdentity(program, {})?.identity.slice(0, 24);
}

// What rust-js does that isn't a compile of a crate, or that writes or
// reads what a record can't say: Cargo's build, std's sysroot beside the
// compiler. A compile that prints declarations is kept where
// `RUST_JS_TYPESCRIPT` names the printer, whose files its record names;
// else rust-js says it can't be recorded (`unrecordable`).
const notKept = new Set(["--version-json", "--version", "--runtime-module", "--tidy-passes", "--prepare-passes", "--rustc", "--checks", "--format-jsx", "--cargo", "--std-bodies"]);

/** A file's content, by its hash, read again only where the file has
 * changed since, by its inode, size and times: the toolchain's crates a
 * record names, 65 MB, aren't read again at each compile. */
const hashes = new Map<string, string>();
function fingerprint(path: string): string | undefined {
  const s = statOf(path);
  if (!s) return undefined;
  const key = `${path}\0${s.dev}:${s.ino}:${s.size}:${s.mtimeMs}:${s.ctimeMs}`;
  let hash = hashes.get(key);
  if (hash === undefined) {
    try {
      hash = sha(readFileSync(path));
    } catch {
      return undefined;
    }
    hashes.set(key, hash);
  }
  return hash;
}

/** What the key of `args` is made of: where it's run, its arguments, its
 * environment but what the harness alone reads, and each `-L`
 * directory's crates' names. */
function keyOf(identity: string, args: string[], cwd: string, env: Record<string, string | undefined>): string {
  const environment = Object.entries({ ...process.env, ...env })
    .filter(([name, value]) => value !== undefined && !unread.includes(name))
    .sort(([a], [b]) => (a < b ? -1 : 1));
  const searched: string[] = [];
  const rustc = args.slice(args.indexOf("--") + 1);
  rustc.forEach((arg, i) => {
    const given = arg === "-L" ? rustc[i + 1] : arg.startsWith("-L") ? arg.slice(2) : undefined;
    if (given === undefined) return;
    const dir = resolve(cwd, given.slice(given.indexOf("=") + 1));
    let names: string[] = [];
    try {
      names = readdirSync(dir).filter((name) => /\.(rlib|rmeta|so|dylib)$/.test(name));
    } catch {}
    searched.push(`${dir}\0${names.sort().join("\0")}`);
  });
  return sha(JSON.stringify([VERSION, identity, cwd, args, environment, searched]));
}

/** A record of a build, as `make` reads one: its first rule's targets,
 * what it wrote, and what they're made of, what it read, each relative
 * to where it ran; and each variable an `env!` read. */
export function parseRecord(text: string): { wrote: string[]; read: string[]; env: string[] } {
  const split = (files: string) =>
    files
      .split(/(?<!\\) /)
      .filter((file) => file !== "")
      .map((file) => file.replaceAll("\\ ", " "));
  const rule = text.split("\n").find((line) => !line.startsWith("#") && line.includes(": ")) ?? "";
  const at = rule.indexOf(": ");
  const env = text.split("\n").filter((line) => line.startsWith("# env-dep:"));
  return { wrote: split(rule.slice(0, at)), read: split(rule.slice(at + 2)), env };
}

type Entry = {
  read: [string, string][];
  wrote: [string, number][];
  stdout: string;
  stderr: string;
};

/** A kept compile of `key` whose every file read is as it was, its files
 * written again where they aren't so, and what it printed. Where a
 * manifest of a build before is there, which a compile reads for what's
 * stale, it's one the kept compile wrote, or its compile isn't this one. */
function found(dir: string, cwd: string, manifest: string | undefined): Exit | undefined {
  let entries: string[];
  try {
    entries = readdirSync(dir).filter((name) => !name.startsWith("."));
  } catch {
    return undefined;
  }
  for (const name of entries) {
    const at = join(dir, name);
    let entry: Entry;
    try {
      entry = JSON.parse(readFileSync(join(at, "entry.json"), "utf8"));
    } catch {
      continue;
    }
    if (!entry.read.every(([file, hash]) => fingerprint(resolve(cwd, file)) === hash)) continue;
    if (manifest !== undefined && existsSync(manifest)) {
      const kept = entry.wrote.find(([file]) => resolve(cwd, file) === manifest);
      if (!kept || !readFileSync(manifest).equals(readFileSync(join(at, String(kept[1]))))) continue;
    }
    for (const [file, index] of entry.wrote) {
      const path = resolve(cwd, file);
      const bytes = readFileSync(join(at, String(index)));
      // As rust-js publishes, a file that's the same is left as it is.
      if (existsSync(path) && readFileSync(path).equals(bytes)) continue;
      mkdirSync(resolve(path, ".."), { recursive: true });
      writeFileSync(path, bytes);
    }
    const now = new Date();
    try {
      utimesSync(at, now, now);
    } catch {}
    const stdout = Buffer.from(entry.stdout, "base64");
    const stderr = Buffer.from(entry.stderr, "base64");
    return { code: 0, signal: null, timedOut: false, overflowed: false, stdout: stdout.toString(), stderr: stderr.toString(), bytes: { stdout, stderr } };
  }
  return undefined;
}

/** The space free on the disk the cache is on, as a compile's keep asks
 * it: the real disk's, or what a test gives it (#963). */
type Free = () => number;
const diskFree: Free = () => freeBytes(target);

/** Whether there's room to keep `bytes` more: the disk's space `free()`
 * above its floor, said once a process where it isn't; and the cache under
 * its cap, as the last prune found it with what this process has kept
 * since, or pruned to make room. */
let below = false;
let keptSince = 0;
function room(bytes: number, freeNow: Free): boolean {
  const free = freeNow();
  if (free < floorBytes + bytes) {
    if (!below) console.error(`compile cache: not kept, ${(free / 2 ** 30).toFixed(1)} GB free, under the ${floorBytes / 2 ** 30} GB floor`);
    below = true;
    return false;
  }
  keptSince += bytes;
  let size = Number.NaN;
  try {
    size = Number(readFileSync(join(compileCache, "size"), "utf8"));
  } catch {}
  if (Number.isNaN(size) || size + keptSince > capBytes()) {
    prune({ cap: capBytes() - bytes, free: freeNow });
    keptSince = bytes;
  }
  return true;
}

/** `entry` of the compile kept in `dir`: its files written whole beside
 * it, then moved into place, so a process reading the cache never reads
 * half of one; one kept at the same time by another is the same. */
function keep(dir: string, read: [string, string][], wrote: string[], cwd: string, p: Exit, free: Free) {
  const files = wrote.map((file) => readFileSync(resolve(cwd, file)));
  const bytes = files.reduce((sum, file) => sum + file.length, 0) + p.bytes.stdout.length + p.bytes.stderr.length;
  if (!room(bytes, free)) return;
  const name = `${sha(JSON.stringify(read)).slice(0, 24)}-${bytes}`;
  if (existsSync(join(dir, name))) return;
  const building = join(dir, `.building-${process.pid}-${Math.random().toString(36).slice(2)}`);
  mkdirSync(building, { recursive: true });
  files.forEach((file, i) => writeFileSync(join(building, String(i)), file));
  const entry: Entry = { read, wrote: wrote.map((file, i) => [file, i]), stdout: p.bytes.stdout.toString("base64"), stderr: p.bytes.stderr.toString("base64") };
  writeFileSync(join(building, "entry.json"), JSON.stringify(entry));
  try {
    renameSync(building, join(dir, name));
  } catch {
    rmSync(building, { recursive: true, force: true });
  }
}

/** A compile, `cmd`, kept or given again, as `compile` runs it; or `cmd`
 * run as it is, where it isn't one this cache keeps. */
type Compile = (cmd: string[]) => Exit;
type CompileAsync = (cmd: string[]) => Promise<Exit>;

function plan(cmd: string[], cwd: string, env: Record<string, string | undefined>) {
  const [program, ...args] = cmd;
  // A crate's compile, `rust-js [flags] <input.rs> ..`: not Cargo's call of
  // rust-js as rustc, `rust-js rustc ..`, nor what isn't a compile.
  const ours = args.includes("--") ? args.slice(0, args.indexOf("--")) : args;
  if (!ours.some((arg) => arg.endsWith(".rs")) || /^rustc(\.exe)?$/.test(basename(args[0] ?? ""))) return undefined;
  if (ours.some((arg) => notKept.has(arg)) || args.some((arg) => arg === "--emit" || /^--emit=.*dep-info/.test(arg))) return undefined;
  const compiler = compilerIdentity(program, env);
  if (!compiler) return undefined;
  const dir = join(store, compiler.identity.slice(0, 24), keyOf(compiler.identity, args, cwd, env).slice(0, 32));
  const record = join(store, ".records", `${process.pid}-${Math.random().toString(36).slice(2)}.d`);
  const separated = args.includes("--") ? cmd : [...cmd, "--"];
  const manifest = ours.includes("--manifest") ? resolve(cwd, ours[ours.indexOf("--manifest") + 1] ?? "") : undefined;
  return { dir, record, recorded: [...separated, `--emit=dep-info=${record}`], compiler, manifest };
}

/** Keep what `p`, the compile that wrote `record`, did, if it can be. */
function settle(p: Exit, planned: NonNullable<ReturnType<typeof plan>>, cwd: string, free: Free) {
  const { dir, record, compiler } = planned;
  try {
    if (p.code !== 0 || p.signal || p.timedOut || p.overflowed || !existsSync(record)) return;
    const { wrote, read, env } = parseRecord(readFileSync(record, "utf8"));
    const written = new Set(wrote.map((file) => resolve(cwd, file)));
    const fingerprints: [string, string][] = [];
    for (const file of read) {
      const path = resolve(cwd, file);
      // The compiler is its identity, not its file's content again.
      const s = statOf(path);
      if (s && s.dev === compiler.file.dev && s.ino === compiler.file.ino) continue;
      // A build that read what it wrote, a manifest before it, isn't one
      // of its inputs alone.
      if (written.has(path)) return;
      const hash = fingerprint(path);
      if (hash === undefined) return;
      fingerprints.push([file, hash]);
    }
    // An `env!`'s variable is in the key, as all the environment is.
    if (env.some((line) => unread.includes(line.slice("# env-dep:".length).split("=")[0]))) return;
    mkdirSync(dir, { recursive: true });
    keep(dir, fingerprints, wrote, cwd, p, free);
  } catch {
    // Not kept: compiled again next time.
  } finally {
    rmSync(record, { force: true });
  }
}

// rust-js's refusal to record a build it can't say all of, which is then
// run as it was asked for.
const unrecordable = "--emit=dep-info can't say all a build reads";

/** `cmd` run by `compile`, through the cache where it's a compile it keeps. */
export function keptCompile(cmd: string[], cwd: string, env: Record<string, string | undefined>, compile: Compile, free: Free = diskFree): Exit {
  const planned = plan(cmd, cwd, env);
  if (!planned) return compile(cmd);
  if (existsSync(join(planned.dir, "unrecordable"))) return compile(cmd);
  const hit = found(planned.dir, cwd, planned.manifest);
  if (hit) return hit;
  mkdirSync(join(store, ".records"), { recursive: true });
  const p = compile(planned.recorded);
  if (p.code !== 0 && p.stderr.includes(unrecordable)) {
    mkdirSync(planned.dir, { recursive: true });
    writeFileSync(join(planned.dir, "unrecordable"), "");
    return compile(cmd);
  }
  settle(p, planned, cwd, free);
  return p;
}

/** `keptCompile`, without blocking the others running. */
export async function keptCompileAsync(
  cmd: string[],
  cwd: string,
  env: Record<string, string | undefined>,
  compile: CompileAsync,
  free: Free = diskFree,
): Promise<Exit> {
  const planned = plan(cmd, cwd, env);
  if (!planned) return compile(cmd);
  if (existsSync(join(planned.dir, "unrecordable"))) return compile(cmd);
  const hit = found(planned.dir, cwd, planned.manifest);
  if (hit) return hit;
  mkdirSync(join(store, ".records"), { recursive: true });
  const p = await compile(planned.recorded);
  if (p.code !== 0 && p.stderr.includes(unrecordable)) {
    mkdirSync(planned.dir, { recursive: true });
    writeFileSync(join(planned.dir, "unrecordable"), "");
    return compile(cmd);
  }
  settle(p, planned, cwd, free);
  return p;
}

/** Each kept compile under `store`, its directory, size and when last
 * used; and what a process ended before it finished left, an hour on. */
function entries(store: string): { path: string; bytes: number; used: number }[] {
  const all: { path: string; bytes: number; used: number }[] = [];
  const hour = Date.now() - 60 * 60 * 1000;
  const list = (dir: string) => {
    try {
      return readdirSync(dir).filter((name) => {
        if (!name.startsWith(".")) return true;
        if (name.startsWith(".building-") && (statOf(join(dir, name))?.mtimeMs ?? 0) < hour) rmSync(join(dir, name), { recursive: true, force: true });
        return false;
      });
    } catch {
      return [];
    }
  };
  for (const name of list(join(store, ".records"))) {
    if ((statOf(join(store, ".records", name))?.mtimeMs ?? 0) < hour) rmSync(join(store, ".records", name), { force: true });
  }
  for (const build of list(store)) {
    for (const key of list(join(store, build))) {
      for (const name of list(join(store, build, key))) {
        const bytes = Number(/-(\d+)$/.exec(name)?.[1]);
        if (Number.isNaN(bytes)) continue;
        const path = join(store, build, key, name);
        const s = statOf(path);
        if (s) all.push({ path, bytes, used: s.mtimeMs });
      }
    }
  }
  return all;
}

/** Bound what's kept in `root`: of a build other than `current`, all of
 * it, as it's never found again; of a layout before this one, all of it;
 * then the least recently used until what's left is under `cap`, and,
 * where the disk's `free` space is under its floor, until it's above it or
 * nothing's left. What's left is said in `size`, which `room` reads. What
 * it did, to say. */
export function prune({ cap = capBytes(), current, free, root = compileCache }: { cap?: number; current?: string; free?: () => number; root?: string } = {}): {
  removed: number;
  size: number;
  builds: number;
} {
  const store = join(root, `v${VERSION}`);
  let builds = 0;
  for (const name of existsSync(root) ? readdirSync(root) : []) {
    if (/^v\d+$/.test(name) && name !== `v${VERSION}`) rmSync(join(root, name), { recursive: true, force: true });
  }
  if (current !== undefined && existsSync(store)) {
    for (const build of readdirSync(store)) {
      if (build.startsWith(".") || build === current) continue;
      rmSync(join(store, build), { recursive: true, force: true });
      builds++;
    }
  }
  const { removed, size } = evict(entries(store), { cap, free: free ?? (() => freeBytes(existsSync(root) ? root : target)), floor: floorBytes });
  if (existsSync(store)) writeFileSync(join(root, "size"), String(size));
  return { removed, size, builds };
}
