// Running a Rust program natively and as JS, the same way, for the corpus
// (ADR 0088) and generated programs (ADR 0092): what it prints to stdout and
// stderr, and how its `main` ends, as the oracle says it (ADR 0088).

import { createHash } from "node:crypto";
import { existsSync, mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { basename, dirname, join } from "node:path";
import { runInNewContext } from "node:vm";

import { testShareBytes } from "../scripts/shard";
import { compileFailure, run as runAsync, runBuilds, runSync, stopped, type Exit } from "./child";
import { decode, encode, same, type Outcome } from "./oracle";
import { helpersModule, stdDir } from "./packages-of";
import { directives, locatedOtherwise } from "./corpus-cases";
import { keepRun, keptRun, recheck, rechecked } from "./native-outcomes";
import { compiler, nativeProgram, nativeProgramKey, programFlags, root, run, shippedStd } from "./support";

export const node = Bun.which("node");
// The generated JS runs under Node, the runtime rust-js targets (ADR 0095);
// Bun runs the tests, not the JS they check.
export const runtimes: [string, string[]][] = [["node", [node ?? "node"]]];

export type Call = { module: string; fn: string; args: unknown[] };

/** Each call, of the generated JS in `modules` (a name for each file), made
 * under Node by `node-calls.ts`, and what it did. */
export function callInNode(modules: Record<string, string>, calls: Call[], dir: string): Outcome[] {
  const callsFile = join(dir, "calls.json");
  const outcomesFile = join(dir, "outcomes.json");
  rmSync(outcomesFile, { force: true });
  writeFileSync(callsFile, encode({ modules, calls }));
  run([node ?? "node", join(root, "test/node-calls.ts"), callsFile, outcomesFile]);
  return decode(readFileSync(outcomesFile, "utf8"));
}

/** What a run printed, as text for its reports and as bytes to compare
 * (`Exit`), how it ended, and how long it took, of one this process ran;
 * `stopped` where it didn't end by itself, but was stopped: past its time
 * or its output's or memory's limit, or killed (`stopped`), so what it
 * printed is a cut run's, no program's whole answer (#1252); `unplaced`
 * where its hook text says no place (`unplaced`). */
export type Run = { stdout: string; stderr: string; bytes: Exit["bytes"]; outcome: Outcome | string; ms?: number; stopped?: boolean; unplaced?: boolean };

// A case runs for at most this long, so one that never ends fails instead
// of stopping the suite, and is compiled for at most the other, natively or
// by rust-js: `RUST_JS_COMPILE_TIMEOUT` shortens it for a test of a
// compiler that never ends.
const timeout = 10_000;
// A native binary's first run waits for macOS to check it, behind the
// other new ones when the test files run side by side (`nativeBinary`).
const firstRunTimeout = 60_000;
const compileTimeout = Number(process.env.RUST_JS_COMPILE_TIMEOUT ?? 120_000);

/** A program run as a process, `cmd`, a native binary or a binary's JS, as
 * a user runs it (#2447), with `RUST_BACKTRACE` unset, as each side's hook
 * reads it (ADR 0568): what it printed, std's hook's text of each panic on
 * stderr among it, and its status, `{ exit }`, by which `main` ended, as
 * std's runtime gives it (#2292): `0` of one that returned, `101` of a
 * panic, what `process::exit` was given, `134` of an abort, natively a
 * SIGABRT (#847, #1193). A panic's message and where it was raised are its
 * hook's text, compared as the rest of stderr is. Or how it was stopped:
 * past its time, its output's limit, or its memory share, which natively
 * its watchdog says beside `at` (`nativeCommand`), or killed. It runs in
 * `at`, a directory of its own, empty as it starts, as a user runs a
 * program in a folder: what it writes there, a file of `fs`'s, is its
 * own run's. */
export function runProcess(cmd: string[], at: string, limit = timeout): Run {
  const overgrown = pastShareMark(at);
  rmSync(overgrown, { force: true });
  rmSync(at, { recursive: true, force: true });
  mkdirSync(at, { recursive: true });
  const started = performance.now();
  const p = runSync(cmd, at, limit, { RUST_BACKTRACE: undefined });
  const { stdout } = p;
  // A thread's OS id, which native Rust names it by in a panic's hook and
  // a stack overflow's abort, `thread 'main' (174910222) panicked at`, and
  // the JS by its process's, left out of what's compared (#1193, #2292).
  const unnamed = (text: string) => text.replace(/(thread '[^']*') \(\d+\)/g, "$1");
  const stderr = unnamed(p.stderr);
  const bytes = { ...p.bytes, stderr: Buffer.from(unnamed(p.bytes.stderr.toString("latin1")), "latin1") };
  const ms = performance.now() - started;
  const ran = (outcome: Outcome | string): Run => ({ stdout, stderr, bytes, outcome, ms });
  // Killed by its watchdog, `nativeCommand`'s, past its memory share (#1515).
  if (existsSync(overgrown)) {
    const held = Number(readFileSync(overgrown, "utf8")) / 2 ** 20;
    return { ...ran(`stopped past its memory share, ${Math.round(held)} MB of ${Math.round(nativeShare() / 2 ** 20)} MB`), stopped: true };
  }
  if (p.signal === "SIGABRT" && !p.timedOut) return ran(aborted);
  const why = stopped(p, limit);
  if (why) return { ...ran(why), stopped: true };
  return ran({ exit: p.code ?? -1 });
}

/** Whether a run ended as a `main` that returned does, its status 0. */
export const returned = (outcome: Outcome | string): boolean => typeof outcome !== "string" && same(outcome, { exit: 0 });

/** How a run that aborted ended, natively or as JS (`runProcess`). */
const abortCode = 134;
export const aborted: Outcome = { exit: abortCode };

/** A string as a Rust string literal, for the wrappers' `include!`. */
const rustString = (s: string) => JSON.stringify(s);

/** `run`'s hook text without where each panic was raised, `panicked at
 * <somewhere>:`, as a case `located-otherwise` says it panics elsewhere
 * than native Rust (#2292): what's compared of its runs, `unplaced` said,
 * so its JS's are compared so too (`agree`). */
export function unplaced(run: Run): Run {
  const strip = (text: string) => text.replace(/^(thread '[^']*' panicked at ).*:$/gm, "$1<somewhere>:");
  return { ...run, stderr: strip(run.stderr), bytes: { ...run.bytes, stderr: Buffer.from(strip(run.bytes.stderr.toString("latin1")), "latin1") }, unplaced: true };
}

/** Whether `stderr`, a run's, has std's hook's text of a panic of
 * `message`, a directive's line, as its hook writes it, `panicked at
 * <where>:` then the message (#2292). */
export function hookSays(stderr: string, message: string): boolean {
  const escaped = message.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  return new RegExp(`panicked at [^\\n]*:\\n${escaped}\\n`).test(stderr);
}

/** `run`, a native one's, std's files and a registry's named as rust-js
 * names them where a panic's hook names them (`shippedStd`). */
function shippedRun(run: Run): Run {
  const stderr = shippedStd(run.stderr);
  return { ...run, stderr, bytes: { ...run.bytes, stderr: Buffer.from(shippedStd(run.bytes.stderr.toString("latin1")), "latin1") } };
}

// Natively, the case is the program, the file itself, built by rustc as a
// binary, as rust-js builds it (#2447): the same crate, named for the file,
// `file!()` and `module_path!()` its own, and what it reads beside it, an
// `include_str!`, found as the JS's compile finds it. Run as a process, as
// std's runtime runs `main` (`runProcess`). Its run is kept by what it was
// built from (test/native-outcomes.ts), and the same program isn't built or
// run again: not of a case its directives say is `nondeterministic`,
// `of`'s where the program is a rewrite of it (the metamorphic net's), nor
// where it's run `fresh`, again for another of the answers a hash table's
// seed gives.
export function runNative(
  file: string,
  dir: string,
  edition = "2024",
  target: "host" | "wasm32" = "host",
  // rustc's flags besides: a library the case uses, `--extern
  // metamorph_lib=<rlib>` (the metamorphic net's `library`).
  rustc: string[] = [],
  // `located`: where a panic was raised kept of a case `located-otherwise`
  // says differs there, which the corpus checks it still does. `free`: the
  // disk's free bytes its build and run are kept by, a test's own (#2505).
  { of = file, fresh = false, located = false, free }: { of?: string; fresh?: boolean; located?: boolean; free?: () => number } = {},
): Run | string {
  if (target === "wasm32") return runWasm32(file, dir, edition, rustc);
  const ran = runNativeLocated(file, dir, edition, rustc, { of, fresh, free });
  if (located || typeof ran === "string") return ran;
  // The case's directive, of a rewritten variant its case's (`of`).
  if (locatedOtherwise(readFileSync(of, "utf8")) === undefined) return ran;
  return unplaced(ran);
}

function runNativeLocated(file: string, dir: string, edition: string, rustc: string[], { of, fresh, free }: { of: string; fresh: boolean; free?: () => number }): Run | string {
  const flags = [...nativeFlags(edition), ...rustc];
  const key = nativeRunKey(file, edition, rustc);
  const keeps = !fresh && keepsRun(of);
  const kept = keeps ? keptRun(key) : undefined;
  const ran = () => {
    const built = nativeProgram(file, flags, compileTimeout, free);
    if ("error" in built) return built.error;
    const at = join(dir, "runs", "native");
    const run = shippedRun(runProcess(nativeCommand(built.binary, at), at, built.built ? firstRunTimeout : timeout));
    if (keeps && built.kept) keepRun(key, dirname(built.binary), run, undefined, free);
    return ranOnce(built, run);
  };
  if (kept === undefined) return ran();
  if (rechecked()) {
    const problem = recheck(kept, ran);
    if (problem !== undefined) throw new Error(`${file}: ${problem}`);
  }
  return kept;
}

/** The key a native run of `file` is kept by: its build's (`nativeKey`),
 * and the memory it's given (`nativeLimitKey`). */
export function nativeRunKey(file: string, edition = "2024", rustc: string[] = []): string {
  const build = nativeProgramKey(file, [...nativeFlags(edition), ...rustc]);
  return createHash("sha256").update(`${build}\0${nativeLimitKey()}`).digest("hex");
}

/** What a native run's key says of the memory it's given (`nativeCommand`):
 * Linux's `ulimit`, a fixed size; macOS's watchdog's share, which is the
 * share of what's free as a test starts (`testShareBytes`). That share
 * moved with the load, so a run kept beside one check was never found
 * beside the next (#2340): the key says it's the share, not its bytes. A
 * run stopped past it is never kept (`keepRun`), and a reducer's variant,
 * which is given less, is run afresh (#1306). */
export function nativeLimitKey(platform: NodeJS.Platform = process.platform): string {
  return platform === "linux" ? `data ${nativeDataKiB} KiB` : platform === "darwin" ? "share" : "none";
}

/** Whether a native run of the case `file`, or of a program made of it, is
 * kept: not where its directives say it's `nondeterministic`, or can't be
 * read. */
function keepsRun(file: string): boolean {
  const want = existsSync(file) ? directives(readFileSync(file, "utf8")) : undefined;
  return want === undefined || (typeof want === "object" && want.nondeterministic === undefined);
}

/** The memory a native run may write on Linux, CI's, `RLIMIT_DATA`: 4
 * GiB. A program that allocates without end, a reducer's variant whose
 * loop no longer ends among them, aborts at it, where it took the
 * machine's memory, 8 GiB of it, and the runner killed the job, a
 * metamorphic run's, with no line said (dev-loop). Not its address space,
 * which each thread's allocator reserves more of than it writes. macOS
 * keeps no such limit, so there none. */
export const nativeDataKiB = 4 * 1024 * 1024;

/** How a native program is run: of `binary`, given no arguments, as a
 * user runs it, within `nativeDataKiB` on Linux; on macOS, which keeps no
 * limit an allocator heeds, watched by `memory-watch`, which kills it past
 * its share of the machine's memory (`nativeShare`), its footprint
 * counted, what the system compressed too, and says so in the mark beside
 * its run's directory, `at`, that `runProcess` reads (#1515): three runs
 * held 9 GB each on a 16 GB Mac and froze it. */
export function nativeCommand(binary: string, at: string): string[] {
  if (process.platform === "linux") {
    return ["sh", "-c", `ulimit -d ${nativeDataKiB} 2>/dev/null; exec "$0"`, binary];
  }
  if (process.platform !== "darwin") return [binary];
  const watch = `"$WATCH" "$$" ${nativeShare()} "$MARK" </dev/null >/dev/null 2>&1 & exec "$0"`;
  return ["env", `WATCH=${memoryWatch()}`, `MARK=${pastShareMark(at)}`, "sh", "-c", watch, binary];
}

/** The bytes a native run may hold on macOS: a test's share of the
 * machine's memory (`testShareBytes`, scripts/shard.ts, the one
 * authority); asked once, as a run starts. */
export function nativeShare(): number {
  nativeShareBytes ??= testShareBytes();
  return nativeShareBytes;
}
let nativeShareBytes: number | undefined;

/** Where `memory-watch` says a run in `at` held past its share: beside it. */
const pastShareMark = (at: string) => `${at}.past-share`;

/** The watchdog `nativeCommand` runs beside a native program on macOS,
 * test/memory-watch.rs, built once and kept as a native program is. */
function memoryWatch(): string {
  if (memoryWatchBinary !== undefined) return memoryWatchBinary;
  const built = nativeProgram(join(root, "test", "memory-watch.rs"), ["--edition=2024", "-O"]);
  if ("error" in built) throw new Error(`memory-watch doesn't build:\n${built.error}`);
  memoryWatchBinary = built.binary;
  return memoryWatchBinary;
}
let memoryWatchBinary: string | undefined;

/** `run`, a native program's, its binary gone where it isn't kept, below
 * the disk's floor (`nativeBinary`, #1108). */
export function ranOnce(built: { binary: string; kept: boolean }, run: Run): Run {
  if (!built.kept) rmSync(dirname(built.binary), { recursive: true, force: true });
  return run;
}

/** rustc's flags of a native build of a case, or of a library it uses: no
 * debug info, which no run reads, a panic's message being its location's,
 * and which made each of Linux's builds several MB, std's, where it's a
 * few hundred KB without (#1108). */
export const nativeFlags = (edition: string) => [...programFlags(edition), "-Cstrip=debuginfo", "-Awarnings", remapRoot];

// For `wasm32-wasip1`, whose `usize` is rust-js's (ADR 0090): the case
// itself, its `main` run by Bun's WASI, its status the process's. It
// aborts on a panic, so only a case that runs to its end is one. `rustc`:
// its flags besides, a library built for wasm32 too.
function runWasm32(file: string, dir: string, edition: string, rustc: string[] = []): Run | string {
  const flags = [...programFlags(edition), "--target=wasm32-wasip1", "-Cstrip=debuginfo", "-Awarnings", remapRoot, ...rustc];
  const built = nativeProgram(file, flags, compileTimeout);
  if ("error" in built) return built.error;
  const runner = join(root, "test", "wasi-run.ts");
  return ranOnce(built, runProcess([process.execPath, runner, built.binary], join(dir, "runs", "native"), built.built ? firstRunTimeout : timeout));
}

/** A compile that failed: rust-js's clear rejection, or a crash, however
 * it began (`compileFailure`), with everything it said. */
export type CompileError = { kind: "rejected" | "crashed"; reason: string; error: string };

// Where the repository is, left out of the files either names, so a case's
// `file!()` and `Location::file()` are `test/corpus/<case>.rs`, on any
// machine, natively and as JS (ADR 0427).
const remapRoot = `--remap-path-prefix=${root}/=`;

/** How a case is built to JS (#2447). `binary`: the case itself, as a
 * binary crate, `--crate-type bin`, what a user builds and ships, its
 * `main` run as its module loads (`$main`): every net's build, and the
 * corpus's snapshot. `program`: the case included in a crate that exports
 * `entry`, calling `main`, an app's crate as JS calls its exports (ADR
 * 0360): the corpus's second build, run where its JS isn't the binary's.
 * `library`: that crate as another crate uses it, `--library`, with its
 * manifest. The binary's JS is `<dir>/<name>.js`, the others' in a folder
 * of their own, `<dir>/program/`, `<dir>/library/`. */
export type Build = "binary" | "program" | "library";

/** `file`, compiled to JS as `build` says, into `dir`. */
export function compileJs(
  file: string,
  dir: string,
  edition = "2024",
  build: Build = "binary",
  env: Record<string, string> = {},
  // rust-js's own flags besides, `--tidy-until <pass>` say; then the JS is
  // named for them, so each set's JS is kept apart in one `dir`.
  flags: string[] = [],
  // And rustc's, after `--`: a crate of rust-js's bindings it uses, `--extern
  // js=..` (#942).
  rustc: string[] = [],
): { js: string } | CompileError {
  const { cmd, js } = compileCommand(file, dir, edition, build, flags, rustc);
  return compiled(runSync(cmd, root, compileTimeout, env), js);
}

/** `compileJs` of each of `builds` of `file`: those not kept compiled
 * by the worker this process keeps (`runBuilds`, #2301), the program's
 * and the library's, one crate, together in one rustc session, the
 * binary, a crate of its own, in another, beside it. */
export async function compileBuilds(file: string, dir: string, edition: string, builds: Build[]): Promise<({ js: string } | CompileError)[]> {
  const commands = builds.map((build) => compileCommand(file, dir, edition, build, []));
  const sessions = [builds.flatMap((build, i) => (build === "binary" ? [i] : [])), builds.flatMap((build, i) => (build === "binary" ? [] : [i]))].filter((session) => session.length > 0);
  const exits: Exit[] = new Array(builds.length);
  await Promise.all(
    sessions.map(async (session) => {
      const ran = await runBuilds(session.map((i) => commands[i].cmd), root, compileTimeout);
      session.forEach((i, at) => (exits[i] = ran[at]));
    }),
  );
  return exits.map((p, i) => compiled(p, commands[i].js));
}

/** `file`, a library crate `crate` another compile uses, compiled into
 * `dir/<crate>/lib.js`: the manifest and the metadata that compile is
 * given, `--dependency <manifest> -- --extern <crate>=<rmeta>`. */
export function compileJsLibrary(
  file: string,
  dir: string,
  edition: string,
  crate: string,
  flags: string[] = [],
): { manifest: string; rmeta: string } | CompileError {
  const manifest = join(dir, crate, "lib.manifest.json");
  const rmeta = join(dir, `lib${crate}.rmeta`);
  const cmd = [compiler, file, "-o", join(dir, crate, "lib.js"), "--library", "--manifest", manifest, ...flags,
    "--", "--crate-name", crate, `--emit=metadata=${rmeta}`, `--edition=${edition}`, "-Awarnings", remapRoot];
  const done = compiled(runSync(cmd, root, compileTimeout), manifest);
  return "js" in done ? { manifest, rmeta } : done;
}

/** `compileJs`, without blocking the others running: compiles of one
 * `dir`, each named for its flags, run at once, its wrapper written once. */
export async function compileJsAsync(
  file: string,
  dir: string,
  edition = "2024",
  build: Build = "binary",
  env: Record<string, string> = {},
  flags: string[] = [],
): Promise<{ js: string } | CompileError> {
  const { cmd, js } = compileCommand(file, dir, edition, build, flags);
  return compiled(await runAsync(cmd, root, compileTimeout, env), js);
}

/** How `file` is compiled into `dir` as `build` says, and where its JS
 * goes: a program's and a library's wrapper written, where it isn't
 * already, so a compile that's reading it isn't given it emptied by
 * another's write. The profile of the corpus's compiles runs it too, past
 * the kept compiles (scripts/profile-lowering.ts). */
export function compileCommand(file: string, dir: string, edition: string, build: Build, flags: string[], rustc: string[] = []): { cmd: string[]; js: string } {
  // Of many, `--prepare-without` each of preparation's rewrites, a
  // name of a file's length, its end their hash.
  const named = flags.join("-").replaceAll(/[^A-Za-z0-9-]+/g, "_");
  const short = named.length <= 120 ? named : `${named.slice(0, 80)}-${Bun.hash(named).toString(36)}`;
  const name = flags.length === 0 ? "case" : `case-${short}`;
  const after = [`--edition=${edition}`, "-Awarnings", remapRoot, ...rustc];
  if (build === "binary") {
    const js = join(dir, `${name}.js`);
    return { cmd: [compiler, file, "-o", js, ...flags, "--", "--crate-type", "bin", ...after], js };
  }
  const wrapper = join(dir, "lib.rs");
  const text = `include!(${rustString(file)});\npub fn entry() {\n    main()\n}\n`;
  if (!existsSync(wrapper) || readFileSync(wrapper, "utf8") !== text) writeFileSync(wrapper, text);
  // As a library, as a crate another uses is (ADR 0100), and as a
  // program: each in a folder of its own, the crate named as its binary
  // is, for its file, which `type_name` and `module_path!()` say.
  const js = join(dir, build, `${name}.js`);
  const mode = build === "library" ? ["--library", "--manifest", join(dir, "library", `${name}.manifest.json`)] : [];
  const crate = rustc.includes("--crate-name") ? [] : ["--crate-name", basename(file, ".rs").replaceAll("-", "_")];
  return { cmd: [compiler, wrapper, "-o", js, ...mode, ...flags, "--", ...crate, ...after], js };
}

/** A compile's JS, or how it failed. */
function compiled(p: Exit, js: string): { js: string } | CompileError {
  if (p.code === 0 && !stopped(p, compileTimeout)) return { js };
  const { kind, reason } = compileFailure(p, compileTimeout);
  return { kind, reason, error: kind === "crashed" ? `rust-js crashed: ${reason}\n${p.stderr}` : p.stderr };
}

/** `js`, a `build` of a program (`compileJs`), run under `runtime` as a
 * process, as its native binary is (`runProcess`): a binary's by itself,
 * `node case.js`, its `main` run as its module loads; a program's or a
 * library's by `corpus-run.ts`, which runs its `entry` as a binary's
 * `main` is run. `name`: of its run, among the others in `dir`, its
 * directory `<dir>/runs/<name>`. */
export function runJs(runtime: string[], js: string, dir: string, name: string, limit = timeout, build: Build = "binary"): Run {
  const cmd = build === "binary" ? [...runtime, js] : [...runtime, join(root, "test/corpus-run.ts"), js];
  return runProcess(cmd, join(dir, "runs", name), limit);
}

/** How long the JS of a program `native` ran may run before it's taken to
 * never end: the usual 10 s, or, of one native Rust itself takes long to
 * run, twenty times its time. A generated program's string that a loop
 * makes `replace("", ..)` of, its length multiplied each time, took 2.3 s
 * natively and 3.4 s as JS, past the 10 s beside a stage's other variants
 * on a loaded machine, which said it never ended (#1106). */
export const jsLimit = (native: Run) => Math.max(timeout, Math.ceil((native.ms ?? 0) * 20));

export const show = (outcome: Outcome | string) => (typeof outcome === "string" ? outcome : JSON.stringify(outcome));
/** Std's hook's text of a panic, as it writes it on stderr: a newline,
 * `thread '<name>' panicked at <where>:`, its message's first line, and of
 * the process's first panic, its note of a backtrace. */
const hookText = () => /\nthread '[^']*' panicked at [^\n]*:\n[^\n]*\n(?:note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace\n)?/g;

/** `run` as it's compared (#2447): of std's hook's text, the panic that
 * ended `main` whole, the last its hook wrote of a run that failed, and
 * each before it, a panic caught or one a drop's panic aborted as it
 * unwound, left out, its line, its message's first line and its note, as
 * the corpus compared them before (#2453 compares them); and of an abort,
 * the frames std's hook forces of a panic that can't unwind, its `stack
 * backtrace:`, left out, which the JS has none of (ADR 0262's exception,
 * the guide's difference 64). */
export function compared(run: Run): Run {
  const status = typeof run.outcome !== "string" && "exit" in run.outcome ? run.outcome.exit : undefined;
  const said = (text: string) => {
    const unframed = status === abortCode ? text.replace(/^stack backtrace:\n(?:\s+\d+: .*\n(?: +at .*\n)*)*(?:note: Some details are omitted.*\n)?/gm, "") : text;
    const hooks = [...unframed.matchAll(hookText())];
    const caught = status === 0 ? hooks : hooks.slice(0, -1);
    let out = unframed;
    for (const hook of caught.reverse()) out = `${out.slice(0, hook.index)}${out.slice(hook.index! + hook[0].length)}`;
    return out;
  };
  const stderr = said(run.stderr);
  if (stderr === run.stderr) return run;
  return { ...run, stderr, bytes: { ...run.bytes, stderr: Buffer.from(said(run.bytes.stderr.toString("latin1")), "latin1") } };
}

/** Whether two runs printed the same bytes and ended the same, each as
 * it's `compared`: of one `unplaced` (a case `located-otherwise`'s),
 * each's hook text compared without where its panics were raised. */
export function agree(a: Run, b: Run): boolean {
  if (a.unplaced !== b.unplaced) return agree(a.unplaced ? a : unplaced(a), b.unplaced ? b : unplaced(b));
  [a, b] = [compared(a), compared(b)];
  return (
    a.bytes.stdout.equals(b.bytes.stdout) &&
    a.bytes.stderr.equals(b.bytes.stderr) &&
    typeof a.outcome !== "string" &&
    typeof b.outcome !== "string" &&
    same(a.outcome, b.outcome)
  );
}

/** The case at `file`'s output with standard error written into standard
 * output, as a terminal shows them, natively and as JS under Node: the
 * order of the two streams' writes, which runs that keep them apart can't
 * tell (#1490). And its lines as a browser's console shows them, where
 * there's no stream: each stream's lines, `log` or `error`, in the order
 * their newlines were written, natively (`linesInOrder`) and as JS run in
 * a context with no `process`, what's left unended written at the end. */
export function mergedStreams(
  file: string,
  dir: string,
  edition = "2024",
): { native: string; js: string; nativeLines: string[]; browserLines: string[] } {
  const built = nativeProgram(file, nativeFlags(edition), compileTimeout);
  if ("error" in built) throw new Error(built.error);
  // As `runProcess` runs it: with no `RUST_BACKTRACE`, a thread's OS id
  // left out, and natively, std's files named as rust-js names them.
  const { RUST_BACKTRACE: _, ...env } = process.env;
  const spawn = (cmd: string[]) => Bun.spawnSync(cmd, { cwd: root, timeout: timeout * 3, env });
  const said = (text: string) => text.replace(/(thread '[^']*') \(\d+\)/g, "$1");
  const merged = (cmd: string[]) => said(spawn(["sh", "-c", '"$0" "$@" 2>&1', ...cmd]).stdout.toString());
  const native = shippedStd(merged([built.binary]));
  const apart = spawn([built.binary]);
  // A browser's console has no hook's text: a panic out of `main` is
  // thrown on to the page (`$main`).
  const unhooked = (text: string) => text.replace(hookText(), "");
  const nativeLines = linesInOrder(unhooked(native), shippedStd(said(apart.stdout.toString())), unhooked(shippedStd(said(apart.stderr.toString()))));
  const compiled = compileJs(file, dir, edition);
  if (!("js" in compiled)) throw new Error(`rust-js can't compile it: ${JSON.stringify(compiled).slice(0, 400)}`);
  const js = merged([node ?? "node", compiled.js]);
  return { native, js, nativeLines, browserLines: consoleLines(compiled.js) };
}

/** Of `merged`, the bytes of `out` and `err` interleaved, each stream's
 * lines, `log ..` or `error ..`, in the order their newlines come, what's
 * left unended of each after them: what a browser's console shows of a
 * program whose streams are line buffered. Which bytes are whose is found
 * by aligning the two with `merged`, any of the ways they interleave. */
export function linesInOrder(merged: string, out: string, err: string): string[] {
  // `can[i][j]`: whether `merged` from `i + j` on is `out` from `i` and
  // `err` from `j` interleaved.
  const can = Array.from({ length: out.length + 1 }, () => new Array<boolean>(err.length + 1).fill(false));
  for (let i = out.length; i >= 0; i--) {
    for (let j = err.length; j >= 0; j--) {
      const at = i + j;
      can[i][j] =
        at === merged.length
          ? i === out.length && j === err.length
          : (i < out.length && out[i] === merged[at] && can[i + 1][j]) ||
            (j < err.length && err[j] === merged[at] && can[i][j + 1]);
    }
  }
  if (!can[0][0]) throw new Error("the merged output isn't the two streams' interleaved");
  const lines: string[] = [];
  const held = { log: "", error: "" };
  const put = (stream: "log" | "error", c: string) => {
    if (c !== "\n") held[stream] += c;
    else {
      lines.push(`${stream} ${held[stream]}`);
      held[stream] = "";
    }
  };
  let [i, j] = [0, 0];
  while (i + j < merged.length) {
    if (i < out.length && out[i] === merged[i + j] && can[i + 1][j]) put("log", out[i++]);
    else put("error", err[j++]);
  }
  for (const stream of ["log", "error"] as const) if (held[stream] !== "") lines.push(`${stream} ${held[stream]}`);
  return lines;
}

/** `js`, a case's program, run as a browser runs it, with the runtime
 * before it, in a context with no `process`, its `main` run as it loads
 * (`$main`): each line its console is given, `log ..` or `error ..`,
 * what's left unended written as the task ends; a panic out of `main`
 * ends it, as the native run's status says. */
function consoleLines(js: string): string[] {
  const lines: string[] = [];
  const tasks: (() => void)[] = [];
  const context = {
    console: { log: (s: string) => lines.push(`log ${s}`), error: (s: string) => lines.push(`error ${s}`) },
    queueMicrotask: (task: () => void) => tasks.push(task),
    TextDecoder,
  };
  // The helpers, and std's bodies compiled beside them, `iterator_map`'s
  // among them, each module `@rust-js/std`'s entry re-exports, in its
  // order, as the browser's bundle holds them.
  const modules = [...readFileSync(join(stdDir, "index.js"), "utf8").matchAll(/^export \* from "\.\/(.+\.js)";$/gm)].map((m) => m[1]);
  const runtime = (modules.length > 0 ? modules : ["runtime.js"])
    .map((module) => (module === "runtime.js" ? helpersModule : join(stdDir, module)))
    .map((file) => readFileSync(file, "utf8").replace(/^import \{[^}]*\} from "\.\/runtime\.js";$/gm, "").replace(/^export /gm, ""))
    .join("\n");
  const code = readFileSync(js, "utf8")
    .replace(/^import \{[^}]*\} from "@rust-js\/std";$/m, "")
    .replace(/^export /gm, "")
    .replace(/^\/\/# sourceMappingURL=.*$/m, "");
  try {
    runInNewContext(`${runtime}\n${code}`, context);
  } catch {
    // A panic out of `main`, which the native run's outcome says.
  }
  for (const task of tasks) task();
  return lines;
}
