import { expect, test } from "bun:test";
import { createHash } from "node:crypto";
import { existsSync, mkdirSync, mkdtempSync, readdirSync, readFileSync, renameSync, rmSync, statSync, symlinkSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";

import { runSync, stopped } from "./child";
import { markUsed } from "./kept";
import { reactLibs } from "../tooling/crates.js";
import { shard } from "../scripts/shard";

export const root = join(import.meta.dir, "..");
// With `RUST_JS_TARGET`, another: a mutant's tests', which build the
// crates with its compiler where no other test reads them (ADR 0093).
export const target = process.env.RUST_JS_TARGET ? resolve(process.env.RUST_JS_TARGET) : join(root, "target");
// This checkout's debug build, or with `RUST_JS_COMPILER`, another, as the
// rustc tests workflow's build job makes one for its machines to share.
const given = process.env.RUST_JS_COMPILER;
export const compiler = given ? resolve(given) : join(target, "debug", "rust-js");

/** What `cmd` printed, with `env` added to its environment, or why it
 * failed. It's stopped after `timeout` ms:
 * as the tests load, no test's deadline applies yet. The default is what
 * a setup hook is allowed, for the first build. */
export function run(cmd: string[], timeout = 600_000, env: Record<string, string | undefined> = {}): string {
  const p = runSync(cmd, root, timeout, env);
  const why = stopped(p, timeout);
  if (p.code !== 0 || why) throw new Error(`${cmd.join(" ")} failed${why ? `: it ${why}` : ""}:\n${p.stderr}`);
  return p.stdout;
}

/**
 * What generator `script` writes, given `args` and then a scratch file to
 * write to: the one way a generated file is checked against what's
 * committed, without writing over it.
 */
export function generatorOutput(script: string, ...args: string[]): string {
  const out = join(fixture(`generated-${script.replace(/\W/g, "-")}`), "out");
  run([process.execPath, script, ...args, out]);
  return readFileSync(out, "utf8");
}

let rustcVersion: string | undefined;

/** What a kept native program was built from, beyond its key: each file
 * rustc read, and each variable an `env!` did, with what it was. */
type Inputs = { files: [string, string][]; env: [string, string | null][] };

const sha256 = (file: string) => createHash("sha256").update(readFileSync(file)).digest("hex");

/** `text` as `path`, a new file moved into place: another process reading it,
 * as rustc reads a source two test files share, reads the file before or
 * this one, never one half written. */
export function writeWhole(path: string, text: string) {
  const writing = `${path}.${process.pid}`;
  writeFileSync(writing, text);
  renameSync(writing, path);
}

/** A directory of `target/` named by `contents`: the same contents, the
 * same directory, in any run. Where a native program's wrapper goes, so
 * the same program is the same kept binary (`nativeBinary`). */
export function contentDirectory(...contents: string[]): string {
  const hash = createHash("sha256");
  for (const content of contents) hash.update(`${content.length}:${content}`);
  const dir = join(target, "native-sources", hash.digest("hex").slice(0, 32));
  mkdirSync(dir, { recursive: true });
  return dir;
}

/**
 * A native program, `source` written to `dir/native.rs` and built by rustc
 * with `flags`: its binary, kept in `target/native-cache/` and used again
 * while nothing it was built from has changed. That's rustc's version, the
 * flags, the source and where it is, since `include!("cases.rs")` is of the
 * file beside it, and the libraries a flag names, as `--extern
 * serde=libserde.rlib` does, which name where it's kept; and what rustc
 * says the build read, each file an `include!`, a `mod` or an
 * `include_str!` reached, however deep, and each variable an `env!` read,
 * which are checked each time it's used. A new build of it is kept beside
 * the ones before, which stay: another test file may be running one. Each
 * use marks it used, and a run drops what no test used for a week when it
 * starts (`pruneKept`).
 * A binary is built once, then, and so run once for the first time: macOS
 * checks each new one as it first runs, which takes longer than building
 * it, and one check at a time, however many test files run side by side.
 * `built` says it's new, so its first run may wait its turn for that.
 */
export function nativeBinary(source: string, dir: string, flags: string[], timeout = 120_000): { binary: string; built: boolean } | { error: string } {
  rustcVersion ??= run(["rustc", "-vV"]);
  const wrapper = resolve(dir, "native.rs");
  const hash = createHash("sha256").update(rustcVersion).update(JSON.stringify(flags)).update(wrapper).update(source);
  for (const flag of flags) {
    const path = flag.slice(flag.indexOf("=") + 1);
    if (path.startsWith("/") && existsSync(path) && statSync(path).isFile()) hash.update(`${path} ${statSync(path).mtimeMs} ${statSync(path).size}`);
  }
  writeWhole(wrapper, source);
  const program = join(target, "native-cache", hash.digest("hex").slice(0, 32));
  mkdirSync(program, { recursive: true });
  for (const kept of readdirSync(program).filter((name) => !name.startsWith("."))) {
    if (unchanged(join(program, kept, "inputs.json"))) {
      markUsed(join(program, kept));
      return { binary: join(program, kept, "native"), built: false };
    }
  }
  // Built beside the others, then moved in whole, as the build of what it
  // read: a test file running beside this one never runs half of one.
  const building = join(program, `.building-${process.pid}-${Date.now()}`);
  mkdirSync(building);
  const depInfo = join(building, "native.d");
  const p = runSync(["rustc", ...flags, wrapper, "-o", join(building, "native"), `--emit=link,dep-info=${depInfo}`], root, timeout);
  const why = stopped(p, timeout);
  if (why || p.code !== 0) {
    rmSync(building, { recursive: true, force: true });
    return { error: `rustc can't compile it${why ? `: it ${why}` : ""}:\n${p.stderr}` };
  }
  const read = JSON.stringify(inputs(readFileSync(depInfo, "utf8")));
  writeFileSync(join(building, "inputs.json"), read);
  const kept = join(program, createHash("sha256").update(read).digest("hex").slice(0, 32));
  try {
    renameSync(building, kept);
  } catch {
    // Another file built it at the same time, from the same sources.
    rmSync(building, { recursive: true, force: true });
  }
  return { binary: join(kept, "native"), built: true };
}

/** What a build read, from rustc's dep-info: a line `<file>:` for each file,
 * spaces escaped, and `# env-dep:<NAME>=<value>` for each variable. */
function inputs(depInfo: string): Inputs {
  const files: [string, string][] = [];
  const env: [string, string | null][] = [];
  for (const line of depInfo.split("\n")) {
    const variable = line.match(/^# env-dep:([^=]+)/);
    if (variable) env.push([variable[1], process.env[variable[1]] ?? null]);
    else if (/[^\\]:$/.test(line) && !line.includes(": ")) {
      const file = line.slice(0, -1).replaceAll("\\ ", " ");
      files.push([file, sha256(file)]);
    }
  }
  return { files, env };
}

/** Is what a kept build read, recorded in `record`, still what it was? */
function unchanged(record: string): boolean {
  if (!existsSync(record)) return false;
  const { files, env }: Inputs = JSON.parse(readFileSync(record, "utf8"));
  return files.every(([file, hash]) => existsSync(file) && sha256(file) === hash) && env.every(([name, value]) => (process.env[name] ?? null) === value);
}

/**
 * `work`, done once for `claim`, a directory no one has claimed: the first
 * process to ask does it, and one asking at the same time waits for it to
 * finish. The claim is made whole, with its process's pid in it, and moved
 * into place, which only one can do; it's never taken over. Work that
 * failed, or whose process ended before it finished, is an error for the
 * others, and so is waiting ten minutes.
 */
export function once(claim: string, work: () => unknown, limit = 600_000): void {
  mkdirSync(dirname(claim), { recursive: true });
  const mine = `${claim}.${process.pid}`;
  rmSync(mine, { recursive: true, force: true });
  mkdirSync(mine);
  writeFileSync(join(mine, "pid"), String(process.pid));
  let claimed = false;
  try {
    renameSync(mine, claim);
    claimed = true;
  } catch {
    rmSync(mine, { recursive: true, force: true });
  }
  if (claimed) {
    try {
      work();
    } catch (error) {
      writeFileSync(join(claim, "failed"), String(error instanceof Error ? error.message : error));
      throw error;
    }
    writeFileSync(join(claim, "done"), "");
    return;
  }
  const until = Date.now() + limit;
  for (;;) {
    if (existsSync(join(claim, "done"))) return;
    const failed = readIfThere(join(claim, "failed"));
    if (failed !== undefined) throw new Error(`${claim} failed: ${failed}`);
    const holder = Number(readIfThere(join(claim, "pid")));
    // Looked at again: it may have finished, and ended, since.
    if (!alive(holder) && !existsSync(join(claim, "done"))) throw new Error(`process ${holder}, doing ${claim}, ended before it finished`);
    if (Date.now() > until) throw new Error(`waited ${limit / 60_000} minutes for process ${holder}, doing ${claim}`);
    Bun.sleepSync(50);
  }
}

let built = false;
/**
 * This checkout's compiler, built once for the run, whichever file asks
 * first, however many run side by side (`bun test --parallel`): a build
 * in each would take `target/debug/rust-js` away from the others while
 * Cargo links it again, though nothing's changed. One file builds it, the
 * others wait for it and then don't, and what they run is given it, as
 * `RUST_JS_COMPILER`, so a script doesn't build it again either. A run is
 * its workers' parent, or this process if there are none, by pid and when
 * it started, so no other run, before or after, is the same one.
 */
export function buildCompiler() {
  if (built || given) return;
  // As a shell builds it, with `.cargo/config.toml`'s `RUSTC_BOOTSTRAP`, not
  // `.env.test`'s for the tests' programs: built with the other, cargo
  // rebuilds every dependency, and again the next time (DEVELOPMENT.md).
  once(join(target, "tests-build", thisRun()), () => run(["cargo", "build", "--quiet"], 600_000, { RUSTC_BOOTSTRAP: undefined }));
  process.env.RUST_JS_COMPILER = compiler;
  built = true;
}

/** This test run, the same in each of its workers: its runner's pid, and
 * when it started. */
let runId: string | undefined;
function thisRun(): string {
  if (runId === undefined) {
    const runner = String(process.env.BUN_TEST_WORKER_ID ? process.ppid : process.pid);
    const started = run(["ps", "-o", "lstart=", "-p", runner]).trim();
    runId = createHash("sha256").update(`${runner} ${started}`).digest("hex").slice(0, 16);
  }
  return runId;
}

function readIfThere(file: string): string | undefined {
  return existsSync(file) ? readFileSync(file, "utf8") : undefined;
}

function alive(pid: number): boolean {
  try {
    process.kill(pid, 0);
    return true;
  } catch {
    return false;
  }
}
/** A Cargo package, `app`, its own workspace, in `dir`: `rust` at `lib`,
 * and React's crate by path where it uses it, as a host builds an app's
 * Rust (ADR 0101). `toml` is more of its `Cargo.toml`. What `checkCargo`
 * and a Cargo session are given of it. */
export function cargoApp(dir: string, rust: string, { lib = "src/lib.rs", react = false, toml = "" } = {}) {
  mkdirSync(dirname(join(dir, lib)), { recursive: true });
  writeFileSync(join(dir, lib), rust);
  const path = lib === "src/lib.rs" ? "" : `\n[lib]\npath = "${lib}"\n`;
  const dependencies = react ? `react = { package = "rust-js-react", path = ${JSON.stringify(join(root, "react"))} }\n` : "";
  writeFileSync(join(dir, "Cargo.toml"), `[package]\nname = "app"\nversion = "0.0.0"\nedition = "2024"\n${path}\n[dependencies]\n${dependencies}${toml ? `\n${toml}` : ""}\n[workspace]\n`);
  return { manifestPath: join(dir, "Cargo.toml"), packageName: "app", offline: true };
}

/** `@rust-js/runtime` and `@rust-js/std`, installed in an app outside the
 * checkout, as a package manager installs them: what its JS imports (ADRs
 * 0103, 0561). */
export function installRuntime(app: string) {
  mkdirSync(join(app, "node_modules", "@rust-js"), { recursive: true });
  symlinkSync(join(root, "runtime"), join(app, "node_modules", "@rust-js", "runtime"));
  symlinkSync(join(root, "std"), join(app, "node_modules", "@rust-js", "std"));
}

// Each test's own directory, in `target/fixtures/<pid>-<id>/`, its test
// file's: a test runner's worker runs several files, each its own module,
// so its own exit. Each removes its own as it exits, or every run would
// leave them, tens of gigabytes in days; `RUST_JS_KEEP_FIXTURES=1` keeps
// them to look at. A worker ended without exiting leaves its own, which the
// next run removes: whose process is gone. Not by age: what a tarball
// unpacks into is as old as the tarball says. Removing a directory removes
// the links in it, not what they link to.
export const fixtures = join(target, "fixtures");
const mine = join(fixtures, `${process.pid}-${crypto.randomUUID().slice(0, 8)}`);

/** What a binding crate packages, as its `Cargo.toml`'s `include` names
 * it: each file of each path it lists, relative to the crate, sorted. The
 * one list of a crate's files, which its crates.io package and its npm
 * package are held to, so a file the crate gains is packaged without a
 * second list to keep. */
export function crateIncluded(crate: string): string[] {
  const dir = join(root, crate);
  const manifest = Bun.TOML.parse(readFileSync(join(dir, "Cargo.toml"), "utf8")) as { package: { include: string[] } };
  return manifest.package.include
    .flatMap((path) => {
      const at = join(dir, path);
      if (!statSync(at).isDirectory()) return [path];
      return readdirSync(at, { recursive: true, withFileTypes: true })
        .filter((entry) => entry.isFile())
        .map((entry) => join(entry.parentPath, entry.name).slice(dir.length + 1));
    })
    .sort();
}

export function fixture(name: string): string {
  if (!existsSync(mine)) {
    mkdirSync(mine, { recursive: true });
    for (const entry of readdirSync(fixtures)) {
      const owner = /^(\d+)-/.exec(entry)?.[1];
      if (!owner || alive(Number(owner))) continue;
      try {
        rmSync(join(fixtures, entry), { recursive: true, force: true });
      } catch {
        // Another file's sweep is removing it too: it goes either way.
      }
    }
    if (!process.env.RUST_JS_KEEP_FIXTURES) process.on("exit", () => rmSync(mine, { recursive: true, force: true }));
  }
  return mkdtempSync(join(mine, `${name}-`));
}

/** serde, serde_derive and serde_json (ADR 0077): the flags that find them,
 * as metadata for rust-js or as libraries for a native build. */
const serdeFlags: Record<string, string[]> = {};
export function buildSerde(kind: "rmeta" | "rlib" = "rmeta"): string[] {
  serdeFlags[kind] ??= run(["serde/build.sh", ...(kind === "rlib" ? ["--rlib"] : [])]).trim().split(/\s+/);
  return serdeFlags[kind];
}

/** Each file of `paths`, a file or a directory's, read whole: their digest. */
function digest(paths: string[]): string {
  const hash = createHash("sha256");
  const add = (path: string) => {
    if (statSync(path).isDirectory()) {
      for (const entry of readdirSync(path).sort()) add(join(path, entry));
    } else {
      hash.update(`${path}\0`).update(readFileSync(path)).update("\0");
    }
  };
  for (const path of paths) add(path);
  return hash.digest("hex");
}

/** Run `work` unless what it made from `inputs` is still there: each of
 * `outputs`, and `stamp` saying it was made from what `inputs` hold now.
 * Whether it ran. */
export function unlessUnchanged(stamp: string, inputs: string[], outputs: string[], work: () => unknown): boolean {
  const now = digest(inputs);
  if (outputs.every((output) => existsSync(output)) && readIfThere(stamp) === now) return false;
  rmSync(stamp, { force: true });
  work();
  writeFileSync(stamp, now);
  return true;
}

/** The js, webapi and react crates' metadata, in `target`: built once for
 * the run, as each of its workers is a process of its own, which would
 * otherwise write them again as others read them; and not again while their
 * sources, the toolchain and the compiler are what they were made from:
 * `rust-js --rustc` is rustc, but with rust-js's syntax pass, which writes
 * the props companions in it (ADR 0213), a mutation's of it too. */
let react = false;
export function buildReact() {
  if (!react) {
    buildCompiler();
    const inputs = [
      "react/src", "react/build.sh", "react/cfg.js", "react/versions.json",
      "webapi/src", "webapi/build.sh", "builtins/src", "builtins/build.sh", "rust-toolchain.toml",
    ].map((path) => join(root, path)).concat(compiler);
    const outputs = reactLibs.map((lib) => join(target, `lib${lib}.rmeta`));
    once(join(target, "tests-react", thisRun()), () =>
      unlessUnchanged(join(target, "libreact.stamp"), inputs, outputs, () => run(["react/build.sh", "-o", join(target, "libreact.rmeta")])),
    );
    react = true;
  }
}

/** The node crate's metadata, and the webapi and js crates' it uses beside
 * it, in `target/node`, built once for the run and not again while what
 * they're made from is what it was, as the react crate's are (`buildReact`):
 * in a directory of their own, as node's build writes the webapi crate's
 * metadata too, which another test file may be reading from `target`. Ten
 * tests built it each, 1.5 s each. Where it is: `-L` it, and `--extern
 * node=<it>/libnode.rmeta`. */
let node: string | undefined;
export function buildNode(): string {
  const dir = join(target, "node");
  if (node === undefined) {
    buildCompiler();
    const inputs = ["node/src", "node/build.sh", "webapi/src", "webapi/build.sh", "builtins/src", "builtins/build.sh", "rust-toolchain.toml"]
      .map((path) => join(root, path))
      .concat(compiler);
    const outputs = ["node", "webapi", "js"].map((lib) => join(dir, `lib${lib}.rmeta`));
    once(join(target, "tests-node", thisRun()), () =>
      unlessUnchanged(join(dir, "libnode.stamp"), inputs, outputs, () => run(["node/build.sh", "-o", join(dir, "libnode.rmeta")])),
    );
    node = dir;
  }
  return node;
}

/** The generated JS files under `dir`, by their paths from there: no maps. */
function generated(dir: string): string[] {
  if (!existsSync(dir)) return [];
  return readdirSync(dir, { recursive: true }).map(String).filter((path) => /\.jsx?$/.test(path)).sort();
}

/**
 * Compare the JS in `actual` with its snapshot in `snapshot`, file by file:
 * the same files, each with the same text. With `BLESS=1` (`bun run bless`),
 * write `actual` as the snapshot instead, unless `bless` is false: for a check
 * against a snapshot another compile made. `normalize` evens out what may
 * differ without meaning anything, like where the compiler saw the source.
 */
export function expectSnapshot(
  actual: string,
  snapshot: string,
  {
    normalize = (text: string) => text,
    hint = "the generated JS changed: if that's intended, run `bun run bless` and review the diff",
    bless = true,
  }: { normalize?: (text: string) => string; hint?: string; bless?: boolean } = {},
) {
  const files = generated(actual);
  if (bless && process.env.BLESS) {
    rmSync(snapshot, { recursive: true, force: true });
    for (const path of files) {
      mkdirSync(dirname(join(snapshot, path)), { recursive: true });
      writeFileSync(join(snapshot, path), normalize(readFileSync(join(actual, path), "utf8")));
    }
    return;
  }
  expect(files, hint).toEqual(generated(snapshot));
  for (const path of files) {
    const text = normalize(readFileSync(join(actual, path), "utf8"));
    expect(text, `${path}: ${hint}`).toBe(readFileSync(join(snapshot, path), "utf8"));
  }
}

/** `RUST_JS_CASES=i/n`: the cases of the `i`th of `n` parts, each case in
 * one, for `check:fast`'s processes, which run a file's parts at once; all of
 * them where it isn't set. */
export function caseShard<T extends string>(cases: T[]): T[] {
  const [i, n] = (process.env.RUST_JS_CASES ?? "1/1").split("/").map(Number);
  return shard(cases, i, n);
}

/** What isn't a case runs in the first part alone. */
export const firstPart = (process.env.RUST_JS_CASES ?? "1/1").startsWith("1/") ? test : test.skip;

/** What a plain `cargo check` of an app warns of past what it can't see,
 * running no rust-js: one line each. It doesn't expand `jsx!`, so what only
 * JSX uses is unused to it (ADR 0113); nor run rust-js's pass, so a
 * component's capitalized name is flagged (ADR 0439). Its crates' own
 * warnings, which Cargo sums up, aren't the app's. */
export function plainCheckWarnings(stderr: string): string[] {
  const unseen = /^warning: (unused variable: |unused import: |static `\w+` is never used|function `[A-Z]\w*` should have a snake case name)/;
  return stderr
    .split("\n")
    .filter((line) => /^(warning|error)\b/.test(line) && !/\(lib\) generated \d+ warnings?/.test(line))
    .filter((line) => !unseen.test(line));
}
