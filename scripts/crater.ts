// A small Crater (#1087): each crate of the crate corpus, at its exact
// release (docs/crate-corpus.md's rows, the one list, ADR 1067), its own
// `#[test]`s run natively, `cargo test --lib --tests`, and as JS, `rust-js test`
// (ADR 0916), and each test's outcome compared. A test that passes natively
// and fails or is missing as JS is a wrong answer to reduce, a P0. A test
// whose build rust-js refuses is cut from the JS's copy of the source,
// `#[cfg(any())]` above it, by the place each error names, and the rest
// built again, so each test is said by itself: as native, a wrong answer,
// or refused and why, support to grow. A refusal in the library's own code
// refuses the crate.
//
//   bun scripts/crater.ts [name ...]     the corpus's crates, or those named
//   bun scripts/crater.ts --check [name ...]
//                                        and fail on a wrong answer that
//                                        scripts/crater-decided.txt doesn't
//                                        list, or a listed one that runs
//                                        as native
//   bun scripts/crater.ts --shard=2/4 --out=c2.json [--check]
//                                        every fourth crate, from the
//                                        second, its verdicts written
//   bun scripts/crater.ts --merge [--check] c1.json c2.json ..
//                                        the shards, said as one run
//
// Each crate's source, as crates.io packs it, is copied to
// `target/crater/<name>-<version>` and its tests built there, natively in
// `target/crater/native`, which every crate shares, and as JS in
// `target/crater/js/<name>`, its own, gone once it's asked; its library's
// tests and each integration test's file by itself. A crate's verdict is
// kept by its release, the compiler's build, its row's features, how the
// Crater asks and the tooling's files, `target/crater/kept/`,
// and asked again only where one changed.
// What it reads and cuts of a crate's source is scripts/crater-cuts.ts's.
// Writes `target/crater.json` and prints a line per crate; then what stops
// them, each reason ranked by the crates and tests it stops,
// `target/crater-refusals.json` (`ranked`, #1174).

import { spawn, spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { cargoHome } from "./disk";
import {
  cpSync,
  existsSync,
  mkdirSync,
  readFileSync,
  readdirSync,
  rmSync,
  statSync,
  writeFileSync,
} from "node:fs";
import { join, resolve } from "node:path";
import { currentBuild } from "../test/compiles";
import { floorBytes } from "../test/bounded";
import { bytesOf, evict, freeBytes, type Kept } from "../tooling/bounded.js";
import { corpusRows, type Crate } from "./crate-corpus";
import { cut, cutFor, dropped, errorsAt, items, modulePath, offTarget, placeOf, testModulePath, testOnlyFile, underItems } from "./crater-cuts";
import { atOnce } from "./shard";

const root = join(import.meta.dir, "..");
const work = join(root, "target", "crater");
const compiler =
  process.env.RUST_JS_COMPILER ?? join(root, "target", "debug", "rust-js");

/** Each test's outcome, by its path, `train::tests::test19`: as JS, one
 * that failed with rust-js's refusal at run time, an error whose message
 * is "rust-js does not support ..", is that, with its message (#1192). */
export type Outcomes = Record<string, "ok" | "failed" | "ignored" | `${typeof refusedAtRunTime}${string}`>;

/** How an outcome of a test refused at run time starts, its message after. */
export const refusedAtRunTime = "refused at run time: ";

/** What a crate's tests did: natively, and as JS, each test cut from its
 * build with why; and each test that passed natively and, not cut, didn't
 * as JS. Or why its library's own code was refused, or why it couldn't be
 * asked. */
export type Verdict =
  | {
      native: Outcomes;
      js: Outcomes;
      cut: Record<string, string>;
      differ: { name: string; js: string }[];
      /** Native tests that, not cut, failed as JS with rust-js's refusal
       * at run time, each with its message: refusals, no wrong answer,
       * each a refusal to make a compile error where it can be (#1192). */
      refusedAtRun?: { name: string; why: string }[];
      /** Native tests of a module a cfg leaves off wasm32, chrono's
       * `#[cfg(unix)] mod tz_info`'s: no test of rust-js's target. */
      offTarget?: string[];
    }
  | { native: Outcomes; refused: string[] }
  | { problem: string };

/** `cargo test`'s line of each test, `test a::b ... ok`. */
export function nativeOutcomes(said: string): Outcomes {
  const outcomes: Outcomes = {};
  for (const [, name, how] of said.matchAll(
    /^test (\S+) \.\.\. (ok|FAILED|ignored)\b/gm,
  )) {
    outcomes[name] =
      how === "FAILED" ? "failed" : how === "ok" ? "ok" : "ignored";
  }
  return outcomes;
}

/** Node's runner's line of each test, `✔ a::b (0.1ms)`, `✖ a::b`, or one
 * skipped, `﹣ a::b # SKIP`. */
export function jsOutcomes(said: string): Outcomes {
  const outcomes: Outcomes = {};
  for (const [, mark, name, rest] of said.matchAll(
    /^\s*(✔|✖|﹣) (\S+)(.*)$/gm,
  )) {
    // The heading of the failures said again after the run, no test.
    if (mark === "✖" && name === "failing" && rest === " tests:") continue;
    outcomes[name] =
      mark === "✖"
        ? "failed"
        : mark === "﹣" || rest.includes("# SKIP")
          ? "ignored"
          : "ok";
  }
  // Node's runner says each failure's error under its name again, after
  // the run: one that's rust-js's refusal at run time is refused (#1192).
  for (const [, name, why] of said.matchAll(/^\s*✖ (\S+).*\n\s*(?:\w*Error: )?(rust-js does not support .*)$/gm)) {
    if (outcomes[name] === "failed") outcomes[name] = `${refusedAtRunTime}${why}`;
  }
  return outcomes;
}

/** rust-js's refusals in what a build said, each once, in order. */
export function refusals(said: string): string[] {
  return [
    ...new Set(
      [...said.matchAll(/^error: (rust-js does not support .*)$/gm)].map(
        (m) => m[1],
      ),
    ),
  ];
}

/** Each test that passed natively and, not cut from the JS's build,
 * failed or wasn't run as JS. */
export function differences(
  native: Outcomes,
  js: Outcomes,
  cut: Record<string, string> = {},
): { name: string; js: string }[] {
  return Object.entries(native)
    .filter(
      ([name, how]) => how === "ok" && js[name] !== "ok" && !(name in cut),
    )
    .map(([name]) => ({ name, js: js[name] ?? "not run" }));
}

/** What a build that failed said first went wrong: its first error, of
 * the crate it was building, or its last lines where it said none, never
 * Cargo's last words, `build failed, waiting for other jobs` or a note,
 * futures-util's (#1174). */
export function firstError(said: string): string {
  const lines = said.trim().split("\n");
  const at = lines.findIndex((line) => /^error(?:\[E\d+\])?: /.test(line) && !line.startsWith("error: aborting"));
  if (at < 0) return lines.slice(-3).join(" ");
  const place = lines.slice(at + 1, at + 3).find((line) => /^\s*--> /.test(line));
  return [lines[at], place?.trim()].filter(Boolean).join(" ");
}

/** Each process group `spawned` started that hasn't closed: a Crater
 * stopped, its own process ended by a signal, ends them too, as each is a
 * group of its own that outlived it, its builds and test runs going on
 * (#1174). */
const groups = new Set<number>();

/** A run's own process, ended by `signal`, ending each group it started. */
function endedWith(signal: "SIGINT" | "SIGTERM" | "SIGHUP") {
  for (const pid of groups) {
    try {
      process.kill(-pid, "SIGKILL");
    } catch {
      // Gone already.
    }
  }
  process.exit(128 + ({ SIGINT: 2, SIGTERM: 15, SIGHUP: 1 } as const)[signal]);
}

/** The most the Crater keeps of `target/crater/` but its verdicts, `kept/`:
 * its 100 crates' native test builds are 13 GiB in one target (2026-11-11,
 * #1174), which the disk's budget can't hold (#1213, ADR 1118). Past it they
 * go, and its next run builds them again: 1,360 s for the 100, not 383 s. */
export const craterCap = 6 * 2 ** 30;

/** `target/crater/` held to `cap`, its least recently changed entries
 * first, its verdicts never, nothing kept below the disk's floor. */
export function boundCrater({ dir = work, cap = craterCap, free = () => freeBytes(dir) }: { dir?: string; cap?: number; free?: () => number } = {}): { removed: number; size: number } {
  if (!existsSync(dir)) return { removed: 0, size: 0 };
  const entries: Kept[] = readdirSync(dir)
    .filter((name) => name !== "kept")
    .map((name) => join(dir, name))
    .map((path) => ({ path, bytes: bytesOf(path), used: statSync(path).mtimeMs }));
  return evict(entries, { cap, free, floor: floorBytes });
}

function spawned(
  cmd: string[],
  cwd: string,
  env: Record<string, string>,
  seconds: number,
): Promise<{ code: number | null; said: string }> {
  return new Promise((done) => {
    const p = spawn(cmd[0], cmd.slice(1), {
      cwd,
      env: { ...process.env, ...env },
      stdio: ["ignore", "pipe", "pipe"],
      // A group of its own, which its timeout ends whole: the test runner
      // `rust-js test` starts, and the JS it runs, never left behind.
      detached: true,
    });
    if (p.pid !== undefined) groups.add(p.pid);
    let said = "";
    p.stdout.on("data", (d) => (said += d));
    p.stderr.on("data", (d) => (said += d));
    const timer = setTimeout(() => {
      said += `\nrust-js's Crater: timed out after ${seconds} s\n`;
      try {
        process.kill(-(p.pid ?? 0), "SIGKILL");
      } catch {
        p.kill("SIGKILL");
      }
    }, seconds * 1000);
    p.on("close", (code) => {
      clearTimeout(timer);
      if (p.pid !== undefined) groups.delete(p.pid);
      done({ code, said });
    });
  });
}

/** Where Cargo unpacked `crate`'s source, fetching it first where it hasn't. */
async function source(crate: Crate): Promise<string | { problem: string }> {
  const registry = join(cargoHome, "registry", "src");
  const find = () =>
    existsSync(registry)
      ? readdirSync(registry)
          .map((index) =>
            join(registry, index, `${crate.name}-${crate.version}`),
          )
          .find((dir) => existsSync(join(dir, "Cargo.toml")))
      : undefined;
  const found = find();
  if (found) return found;
  const fetching = join(work, "fetch", crate.name);
  mkdirSync(join(fetching, "src"), { recursive: true });
  writeFileSync(join(fetching, "src", "lib.rs"), "");
  writeFileSync(
    join(fetching, "Cargo.toml"),
    `[package]\nname = "fetch"\nversion = "0.0.0"\nedition = "2021"\n\n[dependencies]\n${crate.name} = "=${crate.version}"\n\n[workspace]\n`,
  );
  const fetched = await spawned(["cargo", "fetch"], fetching, {}, 300);
  return (
    find() ?? {
      problem: `cargo couldn't fetch it: ${fetched.said.trim().split("\n").at(-1)}`,
    }
  );
}

/** How the Crater asks, kept with each verdict: another way of asking asks
 * each crate again. 2: its integration tests too, each file by itself, and
 * getrandom's WebAssembly feature (#1087). 4: an impl and a `use` only
 * tests build cut as a function is, base64's (#1174). 6: of a `use` of
 * several names, those unresolved taken off, and its lints at most
 * warnings, a function's macro's tests cut with it and 40 rounds, and a
 * listed test cut first, a dev-dependency a feature names kept, and a
 * failed build's first error said, and getrandom's feature asked of what Cargo
 * couldn't compile, not of a message (#1174). 13: a test a cfg of its own
 * leaves off wasm32 is no test of rust-js's target, unicode-ident's
 * 64-bit `test_trieset_size`, and each package built by its ID, which
 * serde_json's own tests bringing another serde_json left ambiguous. */
const asking = 13;

/** What a crate's kept verdict is of, beside its release and the
 * compiler's build: the features its row names, how the Crater asks, and
 * the tooling that builds it, rust-js's own wasm-bindgen family among it,
 * which a change to asks it again (#1121). */
function keptKey(crate: Crate, build: string): string {
  const hash = createHash("sha256").update(`${build}\n${(crate.features ?? []).join(",")}\n${asking}\n`);
  // And the tests the list cuts of it first.
  hash.update(JSON.stringify(cutFirst(crate.name, decidedOf(readFileSync(join(import.meta.dir, "crater-decided.txt"), "utf8")))));
  const walk = (dir: string) => {
    for (const entry of readdirSync(dir, { withFileTypes: true }).sort((a, b) => a.name.localeCompare(b.name))) {
      if (entry.name === "node_modules" || entry.name === "target") continue;
      const at = join(dir, entry.name);
      if (entry.isDirectory()) walk(at);
      else if (/\.(js|rs|toml|txt)$/.test(entry.name)) hash.update(`${at.slice(root.length)}\n`).update(readFileSync(at));
    }
  };
  walk(join(root, "tooling"));
  return hash.digest("hex").slice(0, 24);
}

/** Each test binary `cargo test --no-run` built: its target, `src/lib.rs`
 * or an integration test's file, `tests/version.rs`, and its path. Run each
 * by itself, its outcomes are its own: `cargo test`'s `Running` lines are
 * its stderr's and the tests' its stdout's, which a pipe doesn't keep in
 * order, so itertools' quick.rs tests were read as the next file's. */
export function executables(said: string): { target: string; path: string }[] {
  return [...said.matchAll(/^\s+Executable (?:unittests )?(\S+) \((.+)\)$/gm)].map((m) => ({ target: m[1], path: m[2] }));
}

/** The integration tests' files of `outcomes`, `tests/version.rs`, each
 * once, in order. */
export function integrationFiles(outcomes: Outcomes): string[] {
  return [...new Set(Object.keys(outcomes).flatMap((name) => /^(tests\/[^/]+\.rs)::/.exec(name)?.[1] ?? []))];
}

async function measure(crate: Crate, build: string): Promise<Verdict> {
  const kept = join(
    work,
    "kept",
    `${crate.name}-${crate.version}-${keptKey(crate, build)}.json`,
  );
  if (existsSync(kept)) return JSON.parse(readFileSync(kept, "utf8"));
  const verdict = await asked(crate);
  // One that couldn't be asked is asked again next time.
  if (!("problem" in verdict)) {
    mkdirSync(join(work, "kept"), { recursive: true });
    writeFileSync(kept, JSON.stringify(verdict));
  }
  return verdict;
}

/** `crate`'s tests natively and as JS, each compared. */
async function asked(crate: Crate): Promise<Verdict> {
  const from = await source(crate);
  if (typeof from !== "string") return from;
  const dir = join(work, `${crate.name}-${crate.version}`);
  rmSync(dir, { recursive: true, force: true });
  cpSync(from, dir, { recursive: true });
  // Its own lock where it packs one, as its tests were run against.
  const locked = existsSync(join(dir, "Cargo.lock")) ? ["--locked"] : [];
  // The features its row names, its default ones off, Cargo's own flags,
  // the same to both runs (#1121).
  const featured = crate.features ? ["--no-default-features", "--features", crate.features.join(",")] : [];
  const nativeEnv = { CARGO_TARGET_DIR: join(work, "native"), TZ: zone };
  // Its library's tests and its integration tests'; where those don't
  // build, its library's alone: each test binary built, then run by itself,
  // an integration test's outcomes keyed by its file, `tests/version.rs::t1`
  // (`executables`). A binary's tests aren't counted.
  let native = await spawned(["cargo", "test", "--lib", "--tests", "--no-run", ...locked, ...featured], dir, nativeEnv, 900);
  if (executables(native.said).length === 0) {
    native = await spawned(["cargo", "test", "--lib", "--no-run", ...locked, ...featured], dir, nativeEnv, 900);
  }
  const nativeRan: Outcomes = {};
  for (const { target, path } of executables(native.said)) {
    const prefix = target === "src/lib.rs" ? "" : /^tests\/[^/]+\.rs$/.test(target) ? `${target}::` : undefined;
    if (prefix === undefined) continue;
    const ran = await spawned([resolve(dir, path)], dir, { TZ: zone, CARGO_MANIFEST_DIR: dir }, 900);
    for (const [name, how] of Object.entries(nativeOutcomes(ran.said))) nativeRan[`${prefix}${name}`] = how;
  }
  if (Object.keys(nativeRan).length === 0) {
    return {
      problem:
        native.code === 0
          ? "it has no tests: crates.io's package has none of its library's or integration tests"
          : `its tests don't run natively: ${firstError(native.said)}`,
    };
  }
  // The JS's own copy, whose refused tests are cut from it, round by round:
  // what one cut leaves may be refused next, a helper's callers.
  const jsDir = `${dir}-js`;
  rmSync(jsDir, { recursive: true, force: true });
  cpSync(from, jsDir, { recursive: true });
  const manifest = readFileSync(join(jsDir, "Cargo.toml"), "utf8");
  const lock = existsSync(join(jsDir, "Cargo.lock")) ? readFileSync(join(jsDir, "Cargo.lock"), "utf8") : "";
  // Its JS builds' target, its own: crates built at once in one target
  // raced on a dependency both build, num-traits' metadata of one build
  // read as the other's, and chrono's whole run failed (#1087). Made anew
  // each time, as a build that stopped may have left its unit as done
  // with no tests written, and gone after, as its verdict is kept.
  const target = join(work, "js", crate.name);
  rmSync(target, { recursive: true, force: true });
  // The tests listed to cut before its JS is built, each under its issue,
  // typenum's that hangs rust-js's layout (#1269): cut, never asked.
  const first = cutFirst(crate.name, decidedOf(readFileSync(join(import.meta.dir, "crater-decided.txt"), "utf8")));
  for (const [test, why] of Object.entries(first)) {
    const read = (path: string) => (existsSync(join(jsDir, path)) ? readFileSync(join(jsDir, path), "utf8") : undefined);
    const at = placeOf(test, read);
    if (!at) return { problem: `scripts/crater-decided.txt cuts ${test}, which isn't a test of its source: take its line off` };
    writeFileSync(join(jsDir, at.file), cut(read(at.file)!, [at.from]));
    first[test] = why;
  }
  try {
    return await askedAsJs(nativeRan, jsDir, target, manifest, lock, featured, first);
  } finally {
    rmSync(target, { recursive: true, force: true });
  }
}

/** A crate's tests as JS, in `jsDir`, built in `target`, compared with
 * `nativeRan`: its library's, then each integration test's file. */
async function askedAsJs(
  nativeRan: Outcomes,
  jsDir: string,
  target: string,
  manifest: string,
  lock: string,
  featured: string[],
  first: Record<string, string>,
): Promise<Verdict> {
  const js: Outcomes = {};
  const cutTests: Record<string, string> = { ...first };
  // Its library's tests, as natively: no integration test built, nor a
  // dev-dependency only they name.
  if (Object.keys(nativeRan).some((name) => !name.startsWith("tests/"))) {
    const ran = await built(jsDir, target, libraryTestsOnly(manifest, (lib) => sourceNames(join(jsDir, "src"), lib)), lock, featured);
    if ("refused" in ran) return { native: nativeRan, refused: ran.refused };
    if ("problem" in ran) return { problem: ran.problem };
    Object.assign(js, ran.js);
    Object.assign(cutTests, underItems(Object.keys(nativeRan).filter((name) => !name.startsWith("tests/")), ran.items), ran.cut);
  }
  // Each integration test's file by itself, its library as `rust-js build`
  // compiles it, so each error is its own.
  for (const file of integrationFiles(nativeRan)) {
    const named = (lib: string) => sourceNames(join(jsDir, "src"), lib) || sourceNames(join(jsDir, "tests"), lib);
    const only = oneIntegrationTest(manifest, file, named);
    if (only === undefined) continue;
    const ran = await built(jsDir, target, only, lock, featured, file);
    const its = Object.keys(nativeRan).filter((name) => name.startsWith(`${file}::`));
    // Its library refused as its test uses it: each of its tests refused.
    if ("refused" in ran || "problem" in ran) {
      const why = "refused" in ran ? ran.refused[0] : ran.problem;
      for (const name of its) cutTests[name] = why;
      continue;
    }
    for (const [name, how] of Object.entries(ran.js)) js[`${file}::${name}`] = how;
    const inFile = its.map((name) => name.slice(file.length + 2));
    for (const [name, why] of Object.entries({ ...underItems(inFile, ran.items), ...ran.cut })) cutTests[`${file}::${name}`] = why;
  }
  return judged(nativeRan, js, cutTests, jsDir);
}

/** `cutUntilBuilt` of `manifest`, its Cargo.toml; of one getrandom doesn't
 * build for, the WebAssembly feature its release has turned on
 * (`forTheJs`). Only then: a graph with getrandom's backend in it isn't one
 * rust-js's own wasm-bindgen family serves, and one that never builds
 * getrandom, chrono's library tests, would lose it (ADR 1112). */
async function built(
  jsDir: string,
  target: string,
  manifest: string,
  lock: string,
  featured: string[],
  rootFile?: string,
): ReturnType<typeof cutUntilBuilt> {
  // Each build starts from the crate's own lock: one build's manifest
  // drops the dev-dependencies it doesn't name, so Cargo prunes them from
  // the lock, and the next build, which names them again, would resolve
  // them anew at their newest, not the releases native Rust ran,
  // ppv-lite86 0.2.21 and its zerocopy where itertools' lock has 0.2.17.
  const relocked = () => lock !== "" && writeFileSync(join(jsDir, "Cargo.lock"), lock);
  writeFileSync(join(jsDir, "Cargo.toml"), manifest);
  relocked();
  const ran = await cutUntilBuilt(jsDir, target, featured, rootFile);
  const withFeature = forTheJs(manifest, lock);
  if (!("problem" in ran && ran.uncompiled.includes("getrandom")) || withFeature === manifest) return ran;
  writeFileSync(join(jsDir, "Cargo.toml"), withFeature);
  relocked();
  return cutUntilBuilt(jsDir, target, featured, rootFile);
}

/** How many rounds of cuts a build is given: base64's rstest tests,
 * each round's cuts of its helpers showing the next, took more than the
 * twelve it had (#1174). */
const rounds = 40;

/** `rust-js test` in `jsDir`, each refused test cut from the source and
 * the rest built again, up to `rounds` rounds: what ran, and what was cut
 * and why, the tests by their paths and the functions only tests reach,
 * `items`, by theirs; or where an error is in no test, what the library's code refused;
 * or why it couldn't be asked. `rootFile` is the integration test's file
 * being built, `tests/version.rs`, whose crate's modules are its own, every
 * one of `tests/` only tests build; none of the library's tests. */
async function cutUntilBuilt(
  jsDir: string,
  target: string,
  featured: string[],
  rootFile?: string,
): Promise<{ js: Outcomes; cut: Record<string, string>; items: Record<string, string> } | { refused: string[] } | { problem: string; uncompiled: string[] }> {
  const cutTests: Record<string, string> = {};
  const items: Record<string, string> = {};
  // Its first round's first error: what the rest of its cuts follow from.
  let first: string | undefined;
  for (let round = 0; round < rounds; round++) {
    const js = await spawned(
      ["bun", join(root, "tooling", "cli.js"), "test", ...featured],
      jsDir,
      // Its lints at most warnings: the copy's cuts are the Crater's edits,
      // which a lint its author denies refused, base64's
      // `unused_import_braces` of a `use` a name was taken off (#1174). A
      // lint changes no program, and native Rust built the author's source.
      { RUST_JS_COMPILER: compiler, CARGO_TARGET_DIR: target, TZ: zone, RUSTFLAGS: "--cap-lints=warn" },
      1200,
    );
    const jsRan = jsOutcomes(js.said);
    if (Object.keys(jsRan).length > 0) return { js: jsRan, cut: cutTests, items };
    const errors = errorsAt(js.said).filter((e) => e.file.startsWith("src/") || (rootFile !== undefined && e.file.startsWith("tests/")));
    // Each of its tests cut: none left to run.
    if (errors.length === 0 && js.code === 0 && Object.keys(cutTests).length + Object.keys(items).length > 0) return { js: jsRan, cut: cutTests, items };
    if (errors.length === 0) {
      return {
        // Each crate Cargo said it couldn't compile, the one fact `built`
        // asks of a failure, not its words (#1174).
        uncompiled: [...js.said.matchAll(/could not compile `([^`]+)`/g)].map((m) => m[1]),
        problem:
          js.code === 0 ? "`rust-js test` passed having run no test" : `its JS tests didn't run: ${firstError(js.said)}`,
      };
    }
    first ??= errors[0].why;
    const byFile = new Map<string, number[]>();
    // Each name of a `use` of several to take off its line, by file.
    const drops = new Map<string, { line: number; name: string }[]>();
    const library: string[] = [];
    for (const error of errors) {
      const text = readFileSync(join(jsDir, error.file), "utf8");
      const read = (path: string) => (existsSync(join(jsDir, path)) ? readFileSync(join(jsDir, path), "utf8") : undefined);
      // An integration test's file is its crate's; the library's own code,
      // as that test uses it, is never cut.
      const at = error.file.startsWith("tests/")
        ? cutFor(error.file, text, error.line, true, testModulePath(error.file, rootFile ?? ""), error.why)
        : rootFile === undefined
          ? cutFor(error.file, text, error.line, testOnlyFile(error.file, read), modulePath(error.file), error.why)
          : undefined;
      if (!at) {
        library.push(`${error.why} (${error.file}:${error.line})`);
        continue;
      }
      if (at.test && !(at.test in cutTests)) cutTests[at.test] = error.why;
      if (at.item && !(at.item in items)) items[at.item] = error.why;
      if (at.drop) drops.set(error.file, [...(drops.get(error.file) ?? []), ...at.drop.map((name) => ({ line: at.from, name }))]);
      else byFile.set(error.file, [...(byFile.get(error.file) ?? []), at.from]);
    }
    if (library.length > 0) return { refused: [...new Set(library)] };
    // Names taken off their lines first, which moves none, then the cuts.
    for (const [file, names] of drops) {
      writeFileSync(join(jsDir, file), names.reduce((text, { line, name }) => dropped(text, line, name), readFileSync(join(jsDir, file), "utf8")));
    }
    for (const [file, froms] of byFile) writeFileSync(join(jsDir, file), cut(readFileSync(join(jsDir, file), "utf8"), froms));
  }
  // What its first round cut for, so the ranking says what stops it, not
  // how many rounds it took: base64's `rand` `Distribution` of its test
  // helpers, which each rstest test reaches, one a round (#1174).
  return { uncompiled: [], problem: `its JS tests still didn't build after ${rounds} rounds of cuts${first ? `, the first for: ${reasonOf(first)}` : ""}` };
}

/** The time zone both runs have, so a test of local time, chrono's
 * `Local`, is asked of one zone natively and as JS, whatever the machine's
 * (#1121): one with daylight saving time, its offset of the date. */
const zone = "America/New_York";

/** Whether a Rust file under `dir` names `lib`, a crate, as a word. */
function sourceNames(dir: string, lib: string): boolean {
  const word = new RegExp(`\\b${lib}\\b`);
  const walk = (at: string): boolean =>
    readdirSync(at, { withFileTypes: true }).some((entry) =>
      entry.isDirectory() ? walk(join(at, entry.name)) : entry.name.endsWith(".rs") && word.test(readFileSync(join(at, entry.name), "utf8")),
    );
  return existsSync(dir) && walk(dir);
}

/** A crate's manifest, `text`, building no integration test: each
 * `[[test]]` table gone, and `autotests = false`, where crates.io's
 * normalized manifest lists them or Cargo would find them. */
export function libraryTestsOnly(text: string, named: (lib: string) => boolean = () => true): string {
  // A dev-dependency of a name the library's source never says, one only
  // its integration tests use, chrono's wasm32 `wasm-bindgen-test`: not
  // built, as no test that uses it is, so its graph is the library's
  // tests' alone (ADR 1112).
  // Or one a feature names, `real_blackbox = ["criterion/real_blackbox"]`,
  // tinyvec's: Cargo reads a manifest whose feature names a dependency it
  // hasn't as no manifest at all (#1174).
  const features = /^\[features\]\n([\s\S]*?)(?=^\[|(?![\s\S]))/m.exec(text)?.[1] ?? "";
  const inFeatures = new Set([...features.matchAll(/"(?:dep:)?([\w-]+)\??\//g), ...features.matchAll(/"dep:([\w-]+)"/g)].map((m) => m[1]));
  const used = (dependency: string) => inFeatures.has(dependency) || named(dependency.replaceAll("-", "_"));
  const tables = text
    .split(/^(?=\[)/m)
    .filter((table) => !table.startsWith("[[test]]"))
    .filter((table) => {
      const own = /^\[(?:target\.[^\]]*?\.)?dev-dependencies\.([\w-]+)\]/.exec(table);
      return !own || used(own[1]);
    });
  return tables
    .map((table) => {
      if (table.startsWith("[package]")) {
        return table.replace(/^autotests\s*=.*\n/m, "").replace("[package]\n", "[package]\nautotests = false\n");
      }
      if (/^\[(?:target\.[^\]]*?\.)?dev-dependencies\]/.test(table)) {
        return table
          .split("\n")
          .filter((line, i) => i === 0 || !/^[\w-]+\s*=/.test(line) || used(line.split(/\s*=/)[0]))
          .join("\n");
      }
      return table;
    })
    .join("");
}

/** A crate's manifest, `text`, building the integration test of `file`
 * alone, `tests/version.rs`, and none of its library's tests: its
 * `[[test]]` table kept, the others gone, `autotests = false`, its
 * library's `test = false`, and no dev-dependency neither names
 * (`libraryTestsOnly`'s `named`). None where the test isn't one Cargo runs
 * with its harness, `harness = false`, whose outcomes say no tests. */
export function oneIntegrationTest(text: string, file: string, named: (lib: string) => boolean = () => true): string | undefined {
  const tables = libraryTestsOnly(text, named).split(/^(?=\[)/m);
  const its = text.split(/^(?=\[)/m).find((table) => table.startsWith("[[test]]") && table.includes(`path = "${file}"`));
  const table = its ?? `[[test]]\nname = "${file.replace(/^tests\//, "").replace(/\.rs$/, "")}"\npath = "${file}"\n\n`;
  if (/^harness\s*=\s*false/m.test(table)) return undefined;
  const lib = tables.findIndex((t) => t.startsWith("[lib]"));
  const withLib = lib >= 0 ? tables.map((t, i) => (i === lib ? t.replace(/^test\s*=.*\n/m, "").replace("[lib]\n", "[lib]\ntest = false\n") : t)) : [...tables, "[lib]\ntest = false\n\n"];
  return [...withLib, table.endsWith("\n\n") ? table : `${table}\n`].join("");
}

/** A crate's manifest, `text`, as the JS's copy builds it: getrandom's
 * WebAssembly feature on, of each release its lock has, `js` of 0.2's and
 * `wasm_js` of 0.3's and 0.4's, as a wasm-bindgen app's manifest turns it
 * on, which wasm32 needs; its backend's `wasm-bindgen` and `js-sys` are
 * rust-js's own (ADR 1112). */
export function forTheJs(text: string, lock: string): string {
  const majors = [...new Set([...lock.matchAll(/^name = "getrandom"\nversion = "(0\.[234])\.\d+"/gm)].map((m) => m[1]))];
  const features = majors.map((major) => {
    const feature = major === "0.2" ? "js" : "wasm_js";
    return `[target.'cfg(target_arch = "wasm32")'.dev-dependencies.crater-getrandom-${major.replace(".", "-")}]\npackage = "getrandom"\nversion = "${major}"\nfeatures = ["${feature}"]\n\n`;
  });
  return [text.endsWith("\n") ? text : `${text}\n`, ...features].join("");
}

/** The differences decided (ADR 0262), `scripts/crater-decided.txt`'s
 * lines, `num-traits float::tests::copysign  why`: each test's why, by its
 * crate and path, `num-traits float::tests::copysign`. */
export function decidedOf(text: string): Map<string, string> {
  const found = new Map<string, string>();
  for (const line of text.split("\n")) {
    const said = /^(\S+) (\S+)\s{2,}(.+)$/.exec(line);
    if (said && !line.startsWith("#")) found.set(`${said[1]} ${said[2]}`, said[3]);
  }
  return found;
}

/** The tests of `crate` the list says to cut before its JS is built, by
 * a reason `cut: #1269 ..`, each with its reason after `cut: `: one that
 * can't be asked, typenum's `uint::tests::binary`, whose layout takes
 * rust-js half an hour (#1174). Taken off with its issue's fix. */
export function cutFirst(crate: string, decided: Map<string, string>): Record<string, string> {
  return Object.fromEntries(
    [...decided]
      .filter(([key, why]) => key.startsWith(`${crate} `) && why.startsWith("cut: "))
      .map(([key, why]) => [key.slice(crate.length + 1), why.slice("cut: ".length)]),
  );
}

/** Of `crate`'s listed differences, those listed under an open issue,
 * `#1264 ..`, a wrong answer known and being fixed, not a decision (#1174). */
export function knownOf(crate: string, differ: { name: string; js: string }[], decided: Map<string, string>) {
  return differ.filter((d) => /^#\d+\b/.test(decided.get(`${crate} ${d.name}`) ?? ""));
}

/** Each listed test of `verdict`'s crate that ran as JS as it did natively:
 * fixed, or decided no longer, so its line is taken off and the list only
 * shrinks, as the metamorphic net's known list does (#1174). */
export function listedButSame(crate: string, verdict: Verdict, decided: Map<string, string>): string[] {
  if (!("js" in verdict)) return [];
  return [...decided.keys()]
    .filter((key) => key.startsWith(`${crate} `))
    .map((key) => key.slice(crate.length + 1))
    .filter((test) => verdict.native[test] === "ok" && verdict.js[test] === "ok");
}

/** Of `crate`'s differences, those no decision says, each a wrong answer. */
export function undecided(crate: string, differ: { name: string; js: string }[], decided: Map<string, string>) {
  return differ.filter((d) => !decided.has(`${crate} ${d.name}`));
}

/** A crate's verdict of its tests' outcomes: each native one not run as
 * JS of a module a cfg leaves off wasm32 apart from those that differ. */
export function judged(native: Outcomes, js: Outcomes, cut: Record<string, string>, dir: string): Verdict {
  const read = (path: string) => (existsSync(join(dir, path)) ? readFileSync(join(dir, path), "utf8") : undefined);
  const all = differences(native, js, cut);
  const off = all.filter((d) => d.js === "not run" && offTarget(d.name, read)).map((d) => d.name);
  // A refusal at run time is no wrong answer: a column of its own (#1192).
  const refusedAtRun = all
    .filter((d) => d.js.startsWith(refusedAtRunTime))
    .map((d) => ({ name: d.name, why: d.js.slice(refusedAtRunTime.length) }));
  const differ = all.filter((d) => !off.includes(d.name) && !d.js.startsWith(refusedAtRunTime));
  return { native, js, cut, differ, refusedAtRun, offTarget: off };
}

/** A refusal's reason, its place taken off, `(src/lib.rs:12)`: one reason
 * of every place it's said (#1174). */
export function reasonOf(why: string): string {
  return why.replace(/ \([^()\s]+\.rs:\d+\)$/, "");
}

/** What stops the crates' tests, each reason ranked by the crates it
 * stops and then the native tests (#1174): a test cut for it, or refused
 * at run time with it, or every test of a crate whose library it refuses;
 * each of a library's reasons stops all its tests, as each must go before
 * any runs. A crate that couldn't be asked is ranked by why too, so what
 * no row says still shows. So ecosystem work is chosen by need. */
export function ranked(verdicts: Map<string, Verdict>): { why: string; crates: string[]; tests: number }[] {
  const reasons = new Map<string, { crates: Set<string>; tests: number }>();
  const stops = (why: string, crate: string, tests: number) => {
    const reason = reasons.get(why) ?? { crates: new Set<string>(), tests: 0 };
    reason.crates.add(crate);
    reason.tests += tests;
    reasons.set(why, reason);
  };
  for (const [crate, verdict] of verdicts) {
    if ("problem" in verdict) {
      stops(`not asked: ${verdict.problem}`, crate, 0);
      continue;
    }
    const passed = (name: string) => verdict.native[name] === "ok";
    if ("refused" in verdict) {
      const tests = Object.keys(verdict.native).filter(passed).length;
      for (const why of new Set(verdict.refused.map(reasonOf))) stops(why, crate, tests);
      continue;
    }
    for (const [name, why] of Object.entries(verdict.cut)) if (passed(name)) stops(reasonOf(why), crate, 1);
    for (const { why } of verdict.refusedAtRun ?? []) stops(`at run time: ${why}`, crate, 1);
  }
  return [...reasons]
    .map(([why, { crates, tests }]) => ({ why, crates: [...crates].sort(), tests }))
    .sort((a, b) => b.crates.length - a.crates.length || b.tests - a.tests || a.why.localeCompare(b.why));
}

/** A crate's verdict in a line. */
export function said(crate: Crate, verdict: Verdict, decided: Map<string, string> = new Map()): string {
  const name = `${crate.name} ${crate.version}`;
  if ("problem" in verdict) return `${name}: not asked, ${verdict.problem}`;
  const passed = Object.values(verdict.native).filter(
    (how) => how === "ok",
  ).length;
  if ("refused" in verdict)
    return `${name}: its library refused, ${passed} native tests unrun: ${verdict.refused.slice(0, 3).join("; ")}`;
  // Of the tests that passed natively, as `passed` counts: a test cut
  // that failed or was ignored natively is neither (#1174, proc-macro2's).
  const cut = Object.keys(verdict.cut).filter((test) => verdict.native[test] === "ok").length;
  const off = verdict.offTarget?.length ?? 0;
  const atRun = verdict.refusedAtRun ?? [];
  const same = passed - cut - off - atRun.length - verdict.differ.length;
  const wrong = undecided(crate.name, verdict.differ, decided);
  const open = knownOf(crate.name, verdict.differ, decided);
  const issues = [...new Set(open.map((d) => decided.get(`${crate.name} ${d.name}`)!.match(/^#\d+/)![0]))];
  const known = verdict.differ.length - wrong.length - open.length;
  const shown = wrong.slice(0, 5).map((d) => `${d.name} (${d.js})`);
  const refusedAtRun = atRun.slice(0, 3).map((d) => `${d.name} (${d.why})`);
  return `${name}: ${same} of ${passed} as native, ${cut} refused${atRun.length > 0 ? `, ${atRun.length} refused at run time: ${refusedAtRun.join(", ")}${atRun.length > 3 ? ", .." : ""}` : ""}${off > 0 ? `, ${off} not of wasm32` : ""}${known > 0 ? `, ${known} decided` : ""}${open.length > 0 ? `, ${open.length} WRONG known (${issues.join(", ")})` : ""}${wrong.length > 0 ? `, ${wrong.length} WRONG: ${shown.join(", ")}${wrong.length > 5 ? ", .." : ""}` : ""}`;
}

/** A shard's verdicts, as `--shard --out` writes them: which of how many
 * it is, and each crate's verdict by its name (#1174). */
export type Shard = { shard: number; of: number; verdicts: Record<string, Verdict> };

/** The crates of `rows` shard `shard` of `of` asks, from 1: every
 * `of`th, from the `shard`th, as the rustc suite's shards are. */
export function inShard<T>(rows: T[], shard: number, of: number): T[] {
  return rows.filter((_, i) => i % of === shard - 1);
}

/** Shards taken as one run: each crate's verdict, or what's wrong with
 * them, a shard missing or two of one, so a night whose machine failed
 * isn't read as a crate's verdict gone. */
export function merged(shards: Shard[]): Map<string, Verdict> | { problem: string } {
  const of = shards[0]?.of;
  if (of === undefined) return { problem: "there are no shards" };
  const seen = shards.map((s) => s.shard).sort((a, b) => a - b);
  const all = Array.from({ length: of }, (_, i) => i + 1);
  if (shards.some((s) => s.of !== of) || seen.join() !== all.join()) {
    return { problem: `shards ${seen.join(", ")} of ${of}: each of ${all.join(", ")} once` };
  }
  return new Map(shards.flatMap((s) => Object.entries(s.verdicts)));
}

/** A run's verdicts said: `target/crater.json`, what stops them, ranked,
 * and with `check`, a wrong answer no line lists or a listed one that runs
 * as native fails it. */
function reported(chosen: Crate[], verdicts: Map<string, Verdict>, decided: Map<string, string>, check: boolean) {
  // `target/` is a merge's to make: the nightly's report job builds nothing.
  mkdirSync(join(root, "target"), { recursive: true });
  writeFileSync(
    join(root, "target", "crater.json"),
    JSON.stringify(Object.fromEntries(verdicts), null, 2),
  );
  // What stops them, ranked: each reason's crates and tests (#1174).
  const reasons = ranked(verdicts);
  writeFileSync(join(root, "target", "crater-refusals.json"), JSON.stringify(reasons, null, 2));
  if (reasons.length > 0) console.log("\nWhat stops them, by crates and then tests:");
  for (const { why, crates, tests } of reasons) {
    console.log(`  ${crates.length} crate${crates.length === 1 ? "" : "s"}, ${tests} test${tests === 1 ? "" : "s"}: ${why} (${crates.join(", ")})`);
  }
  // With `--check`, a wrong answer no decision says fails it.
  const wrong = chosen.flatMap((crate) => {
    const verdict = verdicts.get(crate.name);
    return verdict && "differ" in verdict ? undecided(crate.name, verdict.differ, decided).map((d) => `${crate.name} ${d.name}`) : [];
  });
  if (check && wrong.length > 0) {
    console.log(`\n${wrong.length} wrong answer(s), each a P0 to reduce: ${wrong.join(", ")}`);
    process.exitCode = 1;
  }
  // And a listed one that runs as native now, its line to take off.
  const same = chosen.flatMap((crate) => {
    const verdict = verdicts.get(crate.name);
    return verdict ? listedButSame(crate.name, verdict, decided).map((test) => `${crate.name} ${test}`) : [];
  });
  if (check && same.length > 0) {
    console.log(`\n${same.length} listed in scripts/crater-decided.txt run(s) as native now: take each off: ${same.join(", ")}`);
    process.exitCode = 1;
  }
}

if (import.meta.main) {
  for (const signal of ["SIGINT", "SIGTERM", "SIGHUP"] as const) process.on(signal, () => endedWith(signal));
  const args = process.argv.slice(2);
  const option = (name: string) => args.find((arg) => arg.startsWith(`--${name}=`))?.slice(name.length + 3);
  const check = args.includes("--check");
  const rows = corpusRows(
    readFileSync(join(root, "docs", "crate-corpus.md"), "utf8"),
  );
  if ("problem" in rows) throw new Error(rows.problem);
  const decided = decidedOf(readFileSync(join(import.meta.dir, "crater-decided.txt"), "utf8"));
  if (args.includes("--merge")) {
    // The shards' verdicts, `--shard --out`'s files, as one run.
    const shards = args.filter((arg) => !arg.startsWith("--")).map((file) => JSON.parse(readFileSync(file, "utf8")) as Shard);
    const verdicts = merged(shards);
    if ("problem" in verdicts) throw new Error(verdicts.problem);
    const chosen = rows.filter((crate) => verdicts.has(crate.name));
    for (const crate of chosen) console.log(said(crate, verdicts.get(crate.name)!, decided));
    reported(chosen, verdicts, decided, check);
  } else {
    const named = args.filter((arg) => !arg.startsWith("--"));
    const [shard, of] = (option("shard") ?? "1/1").split("/").map(Number);
    if (!(of >= 1 && shard >= 1 && shard <= of)) throw new Error(`--shard=${option("shard")}: write it \`--shard=2/4\``);
    const chosen = inShard(
      named.length > 0
        ? rows.filter((crate) => named.includes(crate.name))
        : rows,
      shard,
      of,
    );
    if (chosen.length === 0)
      throw new Error(`no crate of the corpus is named ${named.join(", ")}`);
    const build = currentBuild(compiler);
    if (!build) throw new Error(`no compiler at ${compiler}: run cargo build`);
    // A crate's native test build and its JS one, each a Cargo build of its
    // graph: as many crates at once as the cores, about two a crate, and the
    // memory, about 2 GB a crate, hold (scripts/shard.ts' `atOnce`).
    const width = atOnce("crateBuild");
    const verdicts = new Map<string, Verdict>();
    let next = 0;
    await Promise.all(
      Array.from({ length: Math.min(width, chosen.length) }, async () => {
        while (next < chosen.length) {
          const crate = chosen[next++];
          const verdict = await measure(crate, build);
          verdicts.set(crate.name, verdict);
          console.log(said(crate, verdict, decided));
        }
      }),
    );
    const out = option("out");
    if (out) writeFileSync(out, JSON.stringify({ shard, of, verdicts: Object.fromEntries(verdicts) } satisfies Shard));
    reported(chosen, verdicts, decided, check);
  }
}
