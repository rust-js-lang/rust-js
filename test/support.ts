import { expect, test } from "bun:test";
import { createHash } from "node:crypto";
import { cpSync, existsSync, linkSync, mkdirSync, readdirSync, readFileSync, readlinkSync, realpathSync, renameSync, rmSync, statSync, symlinkSync, writeFileSync } from "node:fs";
import { basename, dirname, join, resolve } from "node:path";

import { tempRoot } from "./bounded";
import { runSync, stopped } from "./child";
import { keptApps, markUsed, mayKeep, nativeCache, pruneKeptApps } from "./kept";
import { reactLibs } from "../tooling/crates.js";
import { shard } from "../scripts/shard";
import { checkoutTarget, helpersModule, stdDir, target } from "./packages-of";

export const root = join(import.meta.dir, "..");
// The tests' target, a mutant's tests' own with `RUST_JS_TARGET`, and the
// `@rust-js/std` the JS written in it imports: the checkout's, or a
// runtime mutant's own in its target (#720).
export { helpersModule, stdDir, target };
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

/** A directory of the checkout's `target/` named by `contents`: the same
 * contents, the same directory, in any run, of any target's tests
 * (`checkoutTarget`), and the same name in any worktree, the checkout's
 * root said as `<root>` (`portable`). Where a native program's wrapper
 * goes, so the same program is the same kept binary (`nativeBinary`). */
export function contentDirectory(...contents: string[]): string {
  const hash = createHash("sha256");
  for (const content of contents.map(portable)) hash.update(`${content.length}:${content}`);
  const dir = join(checkoutTarget, "native-sources", hash.digest("hex").slice(0, 32));
  mkdirSync(dir, { recursive: true });
  return dir;
}

/**
 * A native program, `source` written to `dir/native.rs` and built by rustc
 * with `flags`: its binary, kept in the machine's `native-cache`
 * (`nativeCache`), every worktree's and whichever target the tests run in,
 * and used again while nothing it was built from has changed. That's
 * rustc's version, the flags, the source and where it is, since
 * `include!("cases.rs")` is of the file beside it, and the libraries a
 * flag names, as `--extern serde=libserde.rlib` does, each by what it
 * holds; and what rustc says the build read, each file an `include!`, a
 * `mod` or an `include_str!` reached, however deep, and each variable an
 * `env!` read, which are checked each time it's used. A path in the
 * checkout is said from the checkout's root (`portable`), which a build
 * leaves out of what it makes (`--remap-path-prefix`), so another
 * worktree's build of the same sources is this one's (#1047). A new build
 * of it is kept beside
 * the ones before, which stay: another test file may be running one. Each
 * use marks it used, and a run drops what no test used for a week when it
 * starts (`pruneKept`).
 * A binary is built once, then, and so run once for the first time: macOS
 * checks each new one as it first runs, which takes longer than building
 * it, and one check at a time, however many test files run side by side.
 * `built` says it's new, so its first run may wait its turn for that.
 * Below the disk's floor it isn't kept (`kept` false): the caller removes
 * it once it's run, or a run of thousands, the metamorphic net's, fills the
 * disk with what its fixtures hold till it ends (#1108).
 */
export function nativeBinary(source: string, dir: string, flags: string[], timeout = 120_000): NativeBuild {
  rustcVersion ??= run(["rustc", "-vV"]);
  const wrapper = resolve(dir, "native.rs");
  const hash = createHash("sha256").update(rustcVersion).update(portable(JSON.stringify(flags))).update(portable(wrapper)).update(portable(source));
  for (const flag of flags) {
    const path = flag.slice(flag.indexOf("=") + 1);
    if (path.startsWith("/") && existsSync(path) && statSync(path).isFile()) hash.update(`${portable(path)} ${sha256(path)}`);
  }
  writeWhole(wrapper, source);
  return keptNative(hash.digest("hex"), wrapper, timeout, (into, depInfo) => [
    ["rustc", ...flags, wrapper, "-o", join(into, "native"), ...(depInfo ? [`--emit=link,dep-info=${depInfo("native")}`] : [])],
  ]);
}

/** A native build, kept or not, or why rustc refused it. */
export type NativeBuild = { binary: string; built: boolean; kept: boolean } | { error: string };

/**
 * A native program of several crates (`crates`, each a library of those it
 * `uses`, in order) and a `main` of the last's, built by rustc with `flags`
 * and kept in the machine's `native-cache` as `nativeBinary`'s are: by
 * rustc's version, the flags, each crate's name, file and text, and `main`,
 * and used again while what each build read is unchanged. crates.test.ts's
 * chains of crates built each crate and its program again every run, some
 * 290 compiles, 15–20 CPU-seconds of it (#882).
 */
export function nativeCrates(crates: { name: string; file: string; uses: string[] }[], main: string, flags: string[], timeout = 120_000): NativeBuild {
  rustcVersion ??= run(["rustc", "-vV"]);
  const wrapper = join(contentDirectory("crates", main, ...crates.map((c) => c.file)), "main.rs");
  writeWhole(wrapper, main);
  const hash = createHash("sha256").update(rustcVersion).update(portable(JSON.stringify(flags))).update(portable(main));
  for (const { name, file, uses } of crates) hash.update(`${name}\0${uses.join(",")}\0${portable(file)}\0${readFileSync(file, "utf8")}\0`);
  const app = crates[crates.length - 1].name;
  return keptNative(hash.digest("hex"), wrapper, timeout, (into, depInfo) => [
    ...crates.map(({ name, file, uses }) => [
      "rustc", ...flags, "--crate-type=rlib", "--crate-name", name, file, "-o", join(into, `lib${name}.rlib`),
      ...uses.flatMap((used) => ["--extern", `${used}=${join(into, `lib${used}.rlib`)}`]), "-L", `dependency=${into}`,
      ...(depInfo ? [`--emit=link,dep-info=${depInfo(name)}`] : []),
    ]),
    ["rustc", ...flags, wrapper, "-o", join(into, "native"), "--extern", `${app}=${join(into, `lib${app}.rlib`)}`, "-L", `dependency=${into}`,
      ...(depInfo ? [`--emit=link,dep-info=${depInfo("native")}`] : [])],
  ]);
}

/** The build `steps` make, rustc's commands into a directory, kept in the
 * machine's `native-cache` under `key` beside the builds of other inputs
 * (each recorded in its `inputs.json`, what rustc said each step read, but
 * `wrapper`, which the key holds), or, where one's unchanged, found again.
 * Built beside the others, then moved in whole, as the build of what it
 * read: a test file running beside this one never runs half of one. Below
 * the disk's floor, built in a fixture of the test's, used and not kept
 * (test/bounded.ts). */
function keptNative(key: string, wrapper: string, timeout: number, steps: (into: string, depInfo?: (step: string) => string) => string[][]): NativeBuild {
  const program = join(nativeCache, key.slice(0, 32));
  mkdirSync(program, { recursive: true });
  for (const kept of readdirSync(program).filter((name) => !name.startsWith("."))) {
    if (unchanged(join(program, kept, "inputs.json"))) {
      markUsed(join(program, kept));
      return { binary: join(program, kept, "native"), built: false, kept: true };
    }
  }
  const compile = (into: string, depInfo?: (step: string) => string) => {
    for (const cmd of steps(into, depInfo)) {
      const p = runSync(cmd, root, timeout);
      const why = stopped(p, timeout);
      if (why || p.code !== 0) return `rustc can't compile it${why ? `: it ${why}` : ""}:\n${p.stderr}`;
    }
    return undefined;
  };
  if (!mayKeep(nativeCache)) {
    const unkept = fixture("native-unkept");
    const error = compile(unkept);
    return error === undefined ? { binary: join(unkept, "native"), built: true, kept: false } : { error };
  }
  const building = join(program, `.building-${process.pid}-${Date.now()}`);
  mkdirSync(building);
  const depInfos: string[] = [];
  const error = compile(building, (step) => {
    const file = join(building, `${step}.d`);
    depInfos.push(file);
    return file;
  });
  if (error !== undefined) {
    rmSync(building, { recursive: true, force: true });
    return { error };
  }
  const each = depInfos.map((file) => inputs(readFileSync(file, "utf8"), wrapper));
  const read = JSON.stringify({ files: each.flatMap((i) => i.files), env: each.flatMap((i) => i.env) });
  writeFileSync(join(building, "inputs.json"), read);
  const kept = join(program, createHash("sha256").update(read).digest("hex").slice(0, 32));
  try {
    renameSync(building, kept);
  } catch {
    // Another file built it at the same time, from the same sources.
    rmSync(building, { recursive: true, force: true });
  }
  return { binary: join(kept, "native"), built: true, kept: true };
}

/** `text` with the checkout's root, where it says it, said as `<root>`: a
 * kept build's key and inputs, the same in every worktree (#1047). */
export const portable = (text: string) => text.replaceAll(`${root}/`, "<root>/");
/** A path `portable` said, in this checkout. */
const inCheckout = (path: string) => path.replace(/^<root>\//, `${root}/`);
/** A variable's value, portable, or `null` where it's unset. */
const fromRoot = (value: string | undefined) => (value === undefined ? null : portable(value));

/** What a build read, from rustc's dep-info: a line `<file>:` for each file,
 * spaces escaped, and `# env-dep:<NAME>=<value>` for each variable; but
 * its `wrapper`, whose source the key holds said from the root, as the
 * wrapper's text names the checkout where it `include!`s (#1047). */
function inputs(depInfo: string, wrapper: string): Inputs {
  const files: [string, string][] = [];
  const env: [string, string | null][] = [];
  for (const line of depInfo.split("\n")) {
    const variable = line.match(/^# env-dep:([^=]+)/);
    if (variable) env.push([variable[1], fromRoot(process.env[variable[1]])]);
    else if (/[^\\]:$/.test(line) && !line.includes(": ")) {
      const file = line.slice(0, -1).replaceAll("\\ ", " ");
      if (file !== wrapper) files.push([portable(file), sha256(file)]);
    }
  }
  return { files, env };
}

/** Is what a kept build read, recorded in `record`, still what it was? */
function unchanged(record: string): boolean {
  if (!existsSync(record)) return false;
  const { files, env }: Inputs = JSON.parse(readFileSync(record, "utf8"));
  return (
    files.every(([file, hash]) => existsSync(inCheckout(file)) && sha256(inCheckout(file)) === hash) &&
    env.every(([name, value]) => fromRoot(process.env[name]) === value)
  );
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
 * The `@rust-js/std` the tests' JS imports is this checkout's: a worktree
 * with no `node_modules` of its own finds a checkout's above it,
 * `.claude/worktrees/<name>`'s, whose `std/` is that checkout's, so every
 * program would run another commit's helpers. Checked once a run.
 */
let packagesOwn = false;
function ownPackages() {
  if (packagesOwn) return;
  const inside = realpathSync(root) + "/";
  let found: string;
  try {
    found = realpathSync(Bun.resolveSync("@rust-js/std/package.json", root));
  } catch {
    throw new Error(`@rust-js/std isn't installed here: run \`bun install --frozen-lockfile\` in ${root}`);
  }
  if (!found.startsWith(inside)) {
    throw new Error(
      `@rust-js/std resolves to ${found}, outside this checkout, ${root}: its JS would run another checkout's. ` +
        `Run \`bun install --frozen-lockfile\` in ${root}`,
    );
  }
  packagesOwn = true;
}

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
  ownPackages();
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

/** `file`'s text, where it's there as it's read: another process may
 * remove it between a look and a read. */
function readIfThere(file: string): string | undefined {
  try {
    return readFileSync(file, "utf8");
  } catch {
    return undefined;
  }
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
export function cargoApp(at: string | KeptApp, rust: string, { lib = "src/lib.rs", react = false, toml = "" } = {}) {
  // Into a fixture, or written through a kept app, each where it differs.
  const dir = typeof at === "string" ? at : at.dir;
  const write = (path: string, text: string) => {
    if (typeof at !== "string") return void at.write(path, text);
    mkdirSync(dirname(join(dir, path)), { recursive: true });
    writeFileSync(join(dir, path), text);
  };
  write(lib, rust);
  const path = lib === "src/lib.rs" ? "" : `\n[lib]\npath = "${lib}"\n`;
  const dependencies = react ? `react = { package = "rust-js-react", path = ${JSON.stringify(join(root, "react"))} }\n` : "";
  write("Cargo.toml", `[package]\nname = "app"\nversion = "0.0.0"\nedition = "2024"\n${path}\n[dependencies]\n${dependencies}${toml ? `\n${toml}` : ""}\n[workspace]\n`);
  return { manifestPath: join(dir, "Cargo.toml"), packageNames: ["app"], offline: true };
}

/** `@rust-js/std`, installed in an app outside the checkout, as a package
 * manager installs it: what its JS imports (ADRs 0103, 0561, #1095). */
export function installStd(app: string) {
  mkdirSync(join(app, "node_modules", "@rust-js"), { recursive: true });
  symlinkSync(stdDir, join(app, "node_modules", "@rust-js", "std"));
}

// Each test's own directory, `target/fixtures/<test file>/<name>_<n>`, the
// `n`th its process asked for of that name: the same path in each run of
// the file, so a compile of the same source there is the same compile, and
// kept (`test/compiles.ts`, ADR 1099). Each is its process's, claimed by
// `<name>_<n>.owner` beside it, its pid, made by a link, which only one
// process makes: one another process running holds is passed for the next
// `n`, and one whose process is gone is taken over, emptied first. Each
// process removes its own as its tests end (`releaseFixtures`, which
// test/setup.ts runs after a file's tests: Bun's test runner never emits
// `exit`) or as a script exits; and the first claim of a process removes
// each a process gone left, one killed before it could (#914). Before
// that, each run left all of them: 7.6 GB after one `check:fast --all`.
// `RUST_JS_KEEP_FIXTURES=1` keeps a run's to look at, until the next run.
// Not by age: what a tarball unpacks into is as old as the tarball says.
// Removing a directory removes the links in it, not what they link to.
export const fixtures = join(target, "fixtures");
/** Where fixtures outside the checkout are claimed (`outsideFixture`). */
const outsideFixtures = join(tempRoot, "rust-js-fixtures");
// Named by its file's hash, a letter first: what a test looks for in the
// JS, a block's `bb5` or a word of its name, never in the path its header
// names, as `-bb6e90b4` was (`\bbb\d+`), and `strict-switches` would be.
const scope = `x${createHash("sha256").update(basename(Bun.main)).digest("hex").slice(0, 8)}`;
const counts = new Map<string, number>();
let owned: string[] | undefined;

/** `owner`, a claim, made this process's: made where there's none, or
 * taken over where its process is gone, the one rename of it that succeeds
 * being the one process's that takes it. Whether it's this process's. */
function takeClaim(owner: string): boolean {
  const writing = `${owner}.${process.pid}`;
  writeFileSync(writing, String(process.pid));
  let got = false;
  try {
    linkSync(writing, owner);
    got = true;
  } catch {
    if (!alive(Number(readIfThere(owner)))) {
      const gone = `${owner}.gone-${process.pid}`;
      try {
        renameSync(owner, gone);
        rmSync(gone, { force: true });
        linkSync(writing, owner);
        got = true;
      } catch {}
    }
  }
  rmSync(writing, { force: true });
  return got;
}

/** Remove the fixtures under `bases` whose process is gone, each taken as
 * a claim is first, so one another process takes at once is its; and
 * those of before #874's layout, `<pid>-x<id>/`, whose process is gone.
 * Never fails a test: what another process removes or claims as it looks
 * is that process's. */
function sweepFixtures(bases: string[]) {
  const list = (dir: string) => {
    try {
      return readdirSync(dir);
    } catch {
      return [];
    }
  };
  for (const base of bases) {
    for (const each of list(base)) {
      try {
        const before = /^(\d+)-x/.exec(each)?.[1];
        if (before !== undefined) {
          if (!alive(Number(before))) rmSync(join(base, each), { recursive: true, force: true });
          continue;
        }
        for (const name of list(join(base, each)).filter((n) => n.endsWith(".owner"))) {
          const owner = join(base, each, name);
          if (alive(Number(readIfThere(owner))) || !takeClaim(owner)) continue;
          rmSync(owner.slice(0, -".owner".length), { recursive: true, force: true });
          rmSync(owner, { force: true });
        }
      } catch {
        // Another process's at once: it goes, or it's theirs.
      }
    }
  }
}

/** The kept apps this process claimed given up, kept, as its tests end. */
/** Release this process's apps, then keep the store within its cap: what a
 * run makes is kept past its cap while its processes hold it, never past
 * their end (#1213), which waited for the next run's start before. */
function releaseKeptApps() {
  const released = keptClaims?.splice(0) ?? [];
  for (const owner of released) rmSync(owner, { force: true });
  if (released.length > 0) pruneKeptApps();
}

/** Remove the fixtures this process claimed, as its tests end. */
export function releaseFixtures() {
  releaseKeptApps();
  if (process.env.RUST_JS_KEEP_FIXTURES) return;
  for (const dir of owned?.splice(0) ?? []) {
    rmSync(dir, { recursive: true, force: true });
    rmSync(`${dir}.owner`, { force: true });
  }
}

/** `<name>_<n>` in `base`, claimed by this process for its life, empty:
 * the first `n` from the last this process took that no live process
 * holds. */
function claim(base: string, name: string): string {
  if (owned === undefined) {
    owned = [];
    process.on("exit", releaseFixtures);
    if (!process.env.RUST_JS_KEEP_FIXTURES) sweepFixtures([fixtures, outsideFixtures]);
  }
  mkdirSync(base, { recursive: true });
  const counted = `${base}\0${name}`;
  for (let n = counts.get(counted) ?? 0; ; n++) {
    const dir = join(base, `${name}_${n}`);
    if (!takeClaim(`${dir}.owner`)) continue;
    counts.set(counted, n + 1);
    rmSync(dir, { recursive: true, force: true });
    mkdirSync(dir);
    owned.push(dir);
    return dir;
  }
}

/** A test's own directory outside the checkout, `@rust-js/std` installed
 * in it, as an app's is: where no Prettier is
 * found, which a fixture in the checkout finds, the checkout's own, and
 * whose crates' directories it then puts in Cargo's key (ADR 1003, #684).
 * Claimed and removed as a fixture is, in the system's temporary
 * directory, `rust-js-fixtures/<test file>/`. */
export function outsideFixture(name: string): string {
  const dir = realpathSync(claim(join(outsideFixtures, scope), name));
  installStd(dir);
  return dir;
}

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

let keptClaims: string[] | undefined;

/**
 * An app's directory kept between runs, `target/kept-apps/<test file>/<name>_<n>`,
 * claimed by this process as a fixture is, so two runs at once never share
 * one; its Cargo target kept in it, so a run of a change that's none of the
 * app's finds its crates built, as Cargo judges them, by its fingerprints
 * and the compiler's `-vV` identity (ADR 1003): each app built again every
 * run, the most of `check:fast --all`'s CPU (#882). Its files are written
 * through it, each only where its text differs, so what's the same keeps
 * its time; `done()`, once they're written, removes each file this run
 * didn't write, an earlier run's, one a test wrote mid-run, a `Cargo.lock`
 * that would change what its build resolves, but its Cargo target; what a
 * build writes beside its Rust it writes again, Cargo finding it done or
 * not. A test that changes a file after, to build again, writes it as it
 * would anywhere: its time is new.
 */
let keeping: boolean | undefined;

/** Whether this process keeps its apps, asked where its first is made. */
function keepsApps(): boolean {
  keeping ??= mayKeep(keptApps);
  return keeping;
}

export class KeptApp {
  readonly dir: string;
  private readonly written = new Set<string>();
  /** `keep`, whether it's kept, the process's one answer: below the disk's
   * floor where it first asks, a fixture, emptied, as nothing's kept there.
   * Asked once, not each time, as what other tests write moves the disk's
   * free space across the floor mid-test, and an app kept then not wasn't
   * found again in the directory it was kept in (#1190). */
  constructor(name: string, keep = keepsApps()) {
    if (!keep) {
      this.dir = fixture(name);
      return;
    }
    if (keptClaims === undefined) {
      keptClaims = [];
      process.on("exit", releaseKeptApps);
    }
    const base = join(keptApps, scope);
    mkdirSync(base, { recursive: true });
    for (let n = 0; ; n++) {
      const dir = join(base, `${name}_${n}`);
      if (!takeClaim(`${dir}.owner`)) continue;
      keptClaims.push(`${dir}.owner`);
      mkdirSync(dir, { recursive: true });
      markUsed(dir);
      this.dir = dir;
      return;
    }
  }

  /** `text` at `path`, of the app's, where it isn't that already. */
  write(path: string, text: string | Buffer): string {
    const at = join(this.dir, path);
    this.written.add(path);
    const now = existsSync(at) && statSync(at).isFile() ? readFileSync(at) : undefined;
    if (now !== undefined && now.equals(Buffer.from(text))) return at;
    rmSync(at, { recursive: true, force: true });
    mkdirSync(dirname(at), { recursive: true });
    const writing = `${at}.${process.pid}`;
    writeFileSync(writing, text);
    renameSync(writing, at);
    return at;
  }

  /** Each file under `from` that `keep` keeps, written at its path under `to`. */
  copy(from: string, to = "", keep: (path: string) => boolean = () => true) {
    const walk = (dir: string) => {
      for (const entry of readdirSync(dir, { withFileTypes: true })) {
        const path = join(dir, entry.name);
        const rel = path.slice(from.length + 1);
        if (!keep(rel)) continue;
        if (entry.isDirectory()) walk(path);
        else if (entry.isFile()) this.write(join(to, rel), readFileSync(path));
      }
    };
    walk(from);
  }

  /** A link at `path` to `target`, where it isn't one to it already. */
  link(path: string, linked: string) {
    const at = join(this.dir, path);
    this.written.add(path);
    try {
      if (readlinkSync(at) === linked) return;
    } catch {
      // None there, or not a link.
    }
    rmSync(at, { recursive: true, force: true });
    mkdirSync(dirname(at), { recursive: true });
    symlinkSync(linked, at);
  }

  /** The claim given up, as a process's ends: the next to ask for the
   * name is given the directory, as the next run is. */
  release() {
    const owner = `${this.dir}.owner`;
    rmSync(owner, { force: true });
    keptClaims = keptClaims?.filter((claim) => claim !== owner);
  }

  /** Each file this run didn't write removed, whoever wrote it, an earlier
   * run's or a test's mid-run, a `Cargo.lock` too: its files are this run's.
   * But where its Cargo target is, `keep`, `target/` or, as `rust-js build`
   * holds it, `node_modules/.cache/`; what a build writes beside its Rust,
   * which a build writes again however much Cargo finds done. */
  done(keep = ["target", join("node_modules", ".cache")]): string {
    const walk = (rel: string): boolean => {
      let left = false;
      for (const entry of readdirSync(join(this.dir, rel), { withFileTypes: true })) {
        const path = join(rel, entry.name);
        if (this.written.has(path) || keep.includes(path)) {
          left = true;
          continue;
        }
        if (entry.isDirectory() && [...this.written, ...keep].some((w) => w.startsWith(`${path}/`))) {
          if (walk(path)) left = true;
          else rmSync(join(this.dir, path), { recursive: true, force: true });
          continue;
        }
        rmSync(join(this.dir, path), { recursive: true, force: true });
      }
      return left;
    };
    walk("");
    return this.dir;
  }
}

export function fixture(name: string): string {
  return claim(join(fixtures, scope), name);
}

/** The run's seed, `<seed>/target`: an app's Cargo target with the binding
 * crates, `js`, `webapi` and `react`, checked as each app's first build
 * checks them. Where the run gives one, `RUST_JS_APPS_SEED`, a directory
 * `check:fast` removes as it ends, it's the run's, built by the first file
 * to ask, the others waiting for it; else this file's, a fixture, removed
 * with its others. */
let appsSeed: string | undefined;
function builtSeed(): string {
  // Asked again once its file's fixtures are gone, as a worker's next file asks.
  if (appsSeed === undefined || !existsSync(appsSeed)) {
    const base = process.env.RUST_JS_APPS_SEED ?? fixture("apps-seed");
    const seed = join(base, "seed");
    once(seed, () => {
      // A crate of its own name, `seed`, not an app's in the targets made of it.
      const app = join(seed, "app");
      mkdirSync(join(app, "src"), { recursive: true });
      writeFileSync(join(app, "src", "lib.rs"), "pub fn seed() {}\n");
      const crate = (name: string, dir: string) => `${name} = { package = "rust-js-${dir}", path = ${JSON.stringify(join(root, dir))} }\n`;
      writeFileSync(join(app, "Cargo.toml"), `[package]\nname = "seed"\nversion = "0.0.0"\nedition = "2024"\n\n[dependencies]\n${crate("js", "builtins")}${crate("react", "react")}\n[workspace]\n`);
      const built = runSync([process.execPath, join(root, "tooling", "cli.js"), "build"], app, 600_000, {
        RUST_JS_COMPILER: compiler,
        CARGO_TARGET_DIR: join(seed, "target"),
      });
      if (built.code !== 0) throw new Error(`the apps' seed didn't build:\n${built.stdout}${built.stderr}`);
    });
    appsSeed = join(seed, "target");
  }
  return appsSeed;
}

/**
 * `targetDir`, an app's Cargo target its first build makes, made from the
 * run's seed: the binding crates, `js`, `webapi` and `react`, checked once
 * a run, not by each app. Since #892 their key is the compiler's alone, the
 * same unit in every app, which Cargo finds done by its fingerprint; each
 * built them again, 3.4–4.1 s of `webapi` under load (#1005). Cloned, not
 * shared: Cargo holds a target for one build at a time, and a test may
 * read its own. Without rustc's incremental state, which a crate found
 * done never reads. Not for a test that counts what's in its target, or
 * that the first build compiles the bindings. `targetDir`.
 */
export function seeded(targetDir: string): string {
  if (existsSync(targetDir)) throw new Error(`${targetDir} is there already: a seed is a new target's`);
  const seed = builtSeed();
  cpSync(seed, targetDir, { recursive: true, preserveTimestamps: true, filter: (path) => !/[/\\]incremental([/\\]|$)/.test(path) });
  return targetDir;
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

/** The react-native crate's metadata, and the js crate's it uses beside
 * it, in `target/react-native`, built once for the run and not again while
 * what they're made from is what it was, as the node crate's are
 * (`buildNode`). Where it is: `-L` it, and `--extern
 * react_native=<it>/libreact_native.rmeta`. */
let reactNative: string | undefined;
export function buildReactNative(): string {
  const dir = join(target, "react-native");
  if (reactNative === undefined) {
    buildCompiler();
    const inputs = ["react-native/src", "react-native/build.sh", "builtins/src", "builtins/build.sh", "rust-toolchain.toml"]
      .map((path) => join(root, path))
      .concat(compiler);
    const outputs = ["react_native", "js"].map((lib) => join(dir, `lib${lib}.rmeta`));
    once(join(target, "tests-react-native", thisRun()), () =>
      unlessUnchanged(join(dir, "libreact_native.stamp"), inputs, outputs, () =>
        run(["react-native/build.sh", "-o", join(dir, "libreact_native.rmeta")]),
      ),
    );
    reactNative = dir;
  }
  return reactNative;
}

const portsJs = new Map<string, string>();
/** A port in `test/ports/<port>` (React's, as react-in-rust ports it, copied
 * with the modules it uses), compiled once a run against the js, node and
 * webapi crates: the directory its modules' JS and declarations are in,
 * each by its file. */
export function portedDir(port: string): string {
  if (!portsJs.has(port)) {
    const out = fixture(port);
    const node = buildNode();
    const crates = ["js", "node", "webapi"].flatMap((lib) => ["--extern", `${lib}=${join(node, `lib${lib}.rmeta`)}`]);
    run([compiler, join(root, "test/ports", port, "lib.rs"), "-o", join(out, "lib.js"), "--declarations", "--", ...crates, "-L", node]);
    portsJs.set(port, out);
  }
  return portsJs.get(port)!;
}

/** A port's JS (`portedDir`), of each of its modules, by file. */
export function ported(port: string): (file: string) => string {
  const dir = portedDir(port);
  return (file) => readFileSync(join(dir, file), "utf8");
}

/** React's shared/ReactPerformanceTrackProperties, as react-in-rust ports
 * it (`test/ports/performance-track-properties`): its JS, a function of it
 * at a time. */
export function performanceTrack(): (name: string) => string {
  const js = ported("performance-track-properties")("ReactPerformanceTrackProperties.js");
  return (name) => {
    const at = js.indexOf(`function ${name}(`);
    const ends = ["\nfunction ", "\nexport function ", "\nconst "].map((e) => js.indexOf(e, at + 1)).filter((e) => e > 0);
    return js.slice(at, Math.min(js.length, ...ends));
  };
}

/** The generated JS files under `dir`, by their paths from there, and
 * their declarations where `declarations` holds: no maps. */
function generated(dir: string, declarations = false): string[] {
  if (!existsSync(dir)) return [];
  const kept = declarations ? /\.(jsx?|d\.ts)$/ : /\.jsx?$/;
  return readdirSync(dir, { recursive: true }).map(String).filter((path) => kept.test(path)).sort();
}

/**
 * Compare the JS in `actual` with its snapshot in `snapshot`, file by file:
 * the same files, each with the same text. With `BLESS=1` (`bun run bless`),
 * write `actual` as the snapshot instead, unless `bless` is false: for a check
 * against a snapshot another compile made. `normalize` evens out what may
 * differ without meaning anything, like where the compiler saw the source.
 * `declarations`: its `.d.ts` files too.
 */
export function expectSnapshot(
  actual: string,
  snapshot: string,
  {
    normalize = (text: string) => text,
    hint = "the generated JS changed: if that's intended, run `bun run bless` and review the diff",
    bless = true,
    declarations = false,
  }: { normalize?: (text: string) => string; hint?: string; bless?: boolean; declarations?: boolean } = {},
) {
  const files = generated(actual, declarations);
  if (bless && process.env.BLESS) {
    rmSync(snapshot, { recursive: true, force: true });
    for (const path of files) {
      mkdirSync(dirname(join(snapshot, path)), { recursive: true });
      writeFileSync(join(snapshot, path), normalize(readFileSync(join(actual, path), "utf8")));
    }
    return;
  }
  expect(files, hint).toEqual(generated(snapshot, declarations));
  for (const path of files) {
    const text = normalize(readFileSync(join(actual, path), "utf8"));
    expect(text, `${path}: ${hint}`).toBe(readFileSync(join(snapshot, path), "utf8"));
  }
}

/** `RUST_JS_CASES=i/n`: the cases of the `i`th of `n` parts, each case in
 * one, for `check:fast`'s processes, which run a file's parts at once; all of
 * them where it isn't set. `i/n:<file>`: the `i`th part of only the cases
 * `<file>` names, a line each, those a change can affect, as plain
 * `check:fast` picks them (scripts/testMap.ts, #877). */
export function caseShard<T extends string>(cases: T[]): T[] {
  const given = process.env.RUST_JS_CASES ?? "1/1";
  const split = given.indexOf(":");
  const [i, n] = (split < 0 ? given : given.slice(0, split)).split("/").map(Number);
  if (split < 0) return shard(cases, i, n);
  const named = new Set(readFileSync(given.slice(split + 1), "utf8").split("\n").filter(Boolean));
  return shard(
    cases.filter((name) => named.has(name)),
    i,
    n,
  );
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

/** Each `- item: \`example\`` line of docs/guide/limitations.md, its item,
 * its example and the name of limitations.test.ts's test that compiles it,
 * which a mutation's `-t` may name. */
export function limitations(): { item: string; example: string; title: string }[] {
  const page = readFileSync(join(root, "docs/guide/limitations.md"), "utf8");
  return [...page.matchAll(/^- (.+?): `([^`]+)`$/gm)].map((m) => ({
    item: m[1],
    example: m[2],
    title: `refused, as docs/guide/limitations.md says: ${m[1]}`,
  }));
}
