// Running a Rust program natively and as JS, the same way, for the corpus
// (ADR 0088) and generated programs (ADR 0092): what it prints to stdout and
// stderr, and how its `main` ends, as the oracle says it (ADR 0088).

import { existsSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";

import { compileFailure, run as runAsync, runSync, stopped, type Exit } from "./child";
import { decode, encode, same, type Outcome } from "./oracle";
import { compiler, contentDirectory, nativeBinary, root, run } from "./support";

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
 * (`Exit`), how it ended, and how long it took, of one this process ran. */
export type Run = { stdout: string; stderr: string; bytes: Exit["bytes"]; outcome: Outcome | string; ms?: number };

// A case runs for at most this long, so one that never ends fails instead
// of stopping the suite, and is compiled for at most the other, natively or
// by rust-js: `RUST_JS_COMPILE_TIMEOUT` shortens it for a test of a
// compiler that never ends.
const timeout = 10_000;
// A native binary's first run waits for macOS to check it, behind the
// other new ones when the test files run side by side (`nativeBinary`).
const firstRunTimeout = 60_000;
const compileTimeout = Number(process.env.RUST_JS_COMPILE_TIMEOUT ?? 120_000);

/** Runs `cmd`, which writes how `main` ended to `outcomeFile`, and says
 * what it printed and how it ended. A run that fails after writing its
 * outcome, as an unhandled rejection after `main` returns makes it, failed:
 * a run counts only if it exits 0, and only by the outcome it wrote itself;
 * or if it aborted, as Rust does where a drop panics while a panic unwinds
 * (#847): natively killed by SIGABRT, and as JS, `$abort`'s exit, 134, the
 * code a shell gives SIGABRT, which `corpus-run.ts` writes as it exits.
 * Either is `{ exit: 134 }`. */
export function execute(cmd: string[], outcomeFile: string, limit = timeout): Run {
  rmSync(outcomeFile, { force: true });
  const started = performance.now();
  const p = runSync(cmd, root, limit);
  const { stdout, stderr, bytes } = p;
  const ms = performance.now() - started;
  const ran = (outcome: Outcome | string): Run => ({ stdout, stderr, bytes, outcome, ms });
  if (p.signal === "SIGABRT" && !p.timedOut) return ran(aborted);
  const why = stopped(p, limit);
  if (why) return ran(why);
  if (!existsSync(outcomeFile)) return ran(`exited ${p.code} without an outcome`);
  const outcome = readFileSync(outcomeFile, "utf8");
  if (p.code === abortCode && same(JSON.parse(outcome), aborted)) return ran(aborted);
  if (p.code !== 0) return ran(`exited ${p.code} after it ended ${outcome}`);
  return ran(JSON.parse(outcome));
}

/** How a run that aborted ended, natively or as JS (`execute`). */
const abortCode = 134;
export const aborted: Outcome = { exit: abortCode };

/** A string as a Rust string literal, for the wrappers' `include!`. */
const rustString = (s: string) => JSON.stringify(s);

// Natively, the case is a module whose `main` the wrapper calls, catching a
// panic as the JS runner does. The panic hook is silenced, so stderr is only
// what the program writes. The case is included where it is, so what it
// reads beside it, an `include_str!`, is found as the JS's compile finds it;
// the wrapper is where its text says, so a case is one kept binary.
export function runNative(
  file: string,
  dir: string,
  edition = "2024",
  target: "host" | "wasm32" = "host",
  // rustc's flags besides: a library the case uses, `--extern
  // metamorph_lib=<rlib>` (the metamorphic net's `library`).
  rustc: string[] = [],
): Run | string {
  if (target === "wasm32") return runWasm32(file, dir, edition, rustc);
  const source = `mod case {
    include!(${rustString(file)});
    pub fn entry() { main() }
}
fn json(s: &str) -> String {
    let mut out = String::from("\\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\\\\""),
            '\\\\' => out.push_str("\\\\\\\\"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}
fn main() {
    std::panic::set_hook(Box::new(|_| {}));
    let outcome = match std::panic::catch_unwind(case::entry) {
        Ok(()) => String::from("{\\"value\\":null}"),
        Err(e) => match e.downcast_ref::<&str>().map(|s| s.to_string()).or_else(|| e.downcast_ref::<String>().cloned()) {
            Some(message) => format!("{{\\"panic\\":{}}}", json(&message)),
            None => String::from("\\"a panic whose payload isn't a string\\""),
        },
    };
    std::fs::write(std::env::args().nth(1).expect("an outcome file"), outcome).expect("the outcome is written");
}
`;
  const built = nativeBinary(source, contentDirectory(source), [...nativeFlags(edition), ...rustc], compileTimeout);
  if ("error" in built) return built.error;
  const outcomeFile = join(dir, "native.json");
  return ranOnce(built, execute([built.binary, outcomeFile], outcomeFile, built.built ? firstRunTimeout : timeout));
}

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
export const nativeFlags = (edition: string) => [`--edition=${edition}`, "-Coverflow-checks=off", "-Cstrip=debuginfo", "-Awarnings", remapRoot];

// For `wasm32-wasip1`, whose `usize` is rust-js's (ADR 0090): the case's
// `main` as it is, run by Bun's WASI, which writes its outcome where it
// returns. It aborts on a panic, so only a case that runs to its end is one.
// `rustc`: its flags besides, a library built for wasm32 too.
function runWasm32(file: string, dir: string, edition: string, rustc: string[] = []): Run | string {
  const source = `mod case {
    include!(${rustString(file)});
    pub fn entry() { main() }
}
fn main() {
    case::entry();
}
`;
  const flags = [`--edition=${edition}`, "--target=wasm32-wasip1", "-Coverflow-checks=off", "-Cstrip=debuginfo", "-Awarnings", remapRoot, ...rustc];
  const built = nativeBinary(source, contentDirectory(source), flags, compileTimeout);
  if ("error" in built) return built.error;
  const outcomeFile = join(dir, "native.json");
  const runner = join(root, "test", "wasi-run.ts");
  return ranOnce(built, execute([process.execPath, runner, built.binary, outcomeFile], outcomeFile, built.built ? firstRunTimeout : timeout));
}

/** A compile that failed: rust-js's clear rejection, or a crash, however
 * it began (`compileFailure`), with everything it said. */
export type CompileError = { kind: "rejected" | "crashed"; reason: string; error: string };

// Where the repository is, left out of the files either names, so a case's
// `file!()` and `Location::file()` are `test/corpus/<case>.rs`, on any
// machine, natively and as JS (ADR 0427).
const remapRoot = `--remap-path-prefix=${root}/=`;

// As JS, the case is the crate's root, with `entry` exported to call `main`.
export function compileJs(
  file: string,
  dir: string,
  edition = "2024",
  library = false,
  env: Record<string, string> = {},
  // rust-js's own flags besides, `--tidy-until <pass>` say; then the JS is
  // named for them, so each set's JS is kept apart in one `dir`.
  flags: string[] = [],
  // And rustc's, after `--`: a crate of rust-js's bindings it uses, `--extern
  // js=..` (#942).
  rustc: string[] = [],
): { js: string } | CompileError {
  const { cmd, js } = compileCommand(file, dir, edition, library, flags, rustc);
  return compiled(runSync(cmd, root, compileTimeout, env), js);
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
  library = false,
  env: Record<string, string> = {},
  flags: string[] = [],
): Promise<{ js: string } | CompileError> {
  const { cmd, js } = compileCommand(file, dir, edition, library, flags);
  return compiled(await runAsync(cmd, root, compileTimeout, env), js);
}

/** How `file` is compiled into `dir`, and where its JS goes: its wrapper
 * written, where it isn't already, so a compile that's reading it isn't
 * given it emptied by another's write. */
function compileCommand(file: string, dir: string, edition: string, library: boolean, flags: string[], rustc: string[] = []): { cmd: string[]; js: string } {
  const wrapper = join(dir, "lib.rs");
  const text = `include!(${rustString(file)});\npub fn entry() {\n    main()\n}\n`;
  if (!existsSync(wrapper) || readFileSync(wrapper, "utf8") !== text) writeFileSync(wrapper, text);
  // As a library, as a crate another uses is (ADR 0100): in a folder of its own.
  // Of many, `--prepare-without` each of preparation's rewrites, a
  // name of a file's length, its end their hash.
  const named = flags.join("-").replaceAll(/[^A-Za-z0-9-]+/g, "_");
  const short = named.length <= 120 ? named : `${named.slice(0, 80)}-${Bun.hash(named).toString(36)}`;
  const name = flags.length === 0 ? "case" : `case-${short}`;
  const js = library ? join(dir, "library", `${name}.js`) : join(dir, `${name}.js`);
  const mode = library ? ["--library", "--manifest", join(dir, "library", `${name}.manifest.json`)] : [];
  return { cmd: [compiler, wrapper, "-o", js, ...mode, ...flags, "--", `--edition=${edition}`, "-Awarnings", remapRoot, ...rustc], js };
}

/** A compile's JS, or how it failed. */
function compiled(p: Exit, js: string): { js: string } | CompileError {
  if (p.code === 0 && !stopped(p, compileTimeout)) return { js };
  const { kind, reason } = compileFailure(p, compileTimeout);
  return { kind, reason, error: kind === "crashed" ? `rust-js crashed: ${reason}\n${p.stderr}` : p.stderr };
}

export function runJs(runtime: string[], js: string, dir: string, name: string, limit = timeout): Run {
  const outcomeFile = join(dir, `${name}.json`);
  // Its arguments are as native's: its path, then where its outcome goes.
  return execute([...runtime, join(root, "test/corpus-run.ts"), js, outcomeFile, js, outcomeFile], outcomeFile, limit);
}

/** How long the JS of a program `native` ran may run before it's taken to
 * never end: the usual 10 s, or, of one native Rust itself takes long to
 * run, twenty times its time. A generated program's string that a loop
 * makes `replace("", ..)` of, its length multiplied each time, took 2.3 s
 * natively and 3.4 s as JS, past the 10 s beside a stage's other variants
 * on a loaded machine, which said it never ended (#1106). */
export const jsLimit = (native: Run) => Math.max(timeout, Math.ceil((native.ms ?? 0) * 20));

export const show = (outcome: Outcome | string) => (typeof outcome === "string" ? outcome : JSON.stringify(outcome));
export const agree = (a: Run, b: Run) =>
  a.bytes.stdout.equals(b.bytes.stdout) &&
  a.bytes.stderr.equals(b.bytes.stderr) &&
  typeof a.outcome !== "string" &&
  typeof b.outcome !== "string" &&
  same(a.outcome, b.outcome);
