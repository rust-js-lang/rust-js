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
//                                        list as decided
//
// Each crate's source, as crates.io packs it, is copied to
// `target/crater/<name>-<version>` and its tests built there, natively in
// `target/crater/native`, which every crate shares, and as JS in
// `target/crater/js/<name>`, its own, gone once it's asked; its library's
// tests and each integration test's file by itself. A crate's verdict is
// kept by its release, the compiler's build, its row's features, how the
// Crater asks and the tooling's files, `target/crater/kept/`,
// and asked again only where one changed.
// Writes `target/crater.json` and prints a line per crate.

import { spawn, spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { availableParallelism } from "node:os";
import { cargoHome } from "./disk";
import {
  cpSync,
  existsSync,
  mkdirSync,
  readFileSync,
  readdirSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { join, resolve } from "node:path";
import { currentBuild } from "../test/compiles";
import { corpusRows, type Crate } from "./crate-corpus";
import { memoryBudget } from "./shard";

const root = join(import.meta.dir, "..");
const work = join(root, "target", "crater");
const compiler =
  process.env.RUST_JS_COMPILER ?? join(root, "target", "debug", "rust-js");

/** Each test's outcome, by its path, `train::tests::test19`. */
export type Outcomes = Record<string, "ok" | "failed" | "ignored">;

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
    outcomes[name] =
      mark === "✖"
        ? "failed"
        : mark === "﹣" || rest.includes("# SKIP")
          ? "ignored"
          : "ok";
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

/** Each error a build said, rustc's or rust-js's, and the place it names,
 * `src/lib.rs:1812`. */
export function errorsAt(
  said: string,
): { why: string; file: string; line: number }[] {
  return [
    ...said.matchAll(/^error(?:\[E\d+\])?: (.*)\n\s*--> ([^:\n]+):(\d+):\d+/gm),
  ].map((m) => ({ why: m[1], file: m[2], line: Number(m[3]) }));
}

/** rustc's cfgs of rust-js's target, `rustc --print cfg --target
 * wasm32-unknown-unknown`'s: a name alone, `target_has_atomic`, or a name
 * and each of its values, `target_os="unknown"`. The one source of them,
 * read once (#1153). */
let wasmCfgs: Map<string, Set<string>> | undefined;
function targetCfgs(): Map<string, Set<string>> {
  if (wasmCfgs) return wasmCfgs;
  const printed = spawnSync("rustc", ["--print", "cfg", "--target", "wasm32-unknown-unknown"], { cwd: root, encoding: "utf8" });
  if (printed.status !== 0) throw new Error(`rustc --print cfg failed: ${printed.stderr}`);
  wasmCfgs = new Map();
  for (const line of printed.stdout.split("\n").filter(Boolean)) {
    const [name, value] = line.split("=");
    const values = wasmCfgs.get(name) ?? new Set<string>();
    if (value !== undefined) values.add(JSON.parse(value));
    wasmCfgs.set(name, values);
  }
  return wasmCfgs;
}

/** Each `#[cfg(..)]` of attributes' text, as `cfg(..)`. */
export function cfgsOf(attributes: string): string[] {
  const found: string[] = [];
  for (let at = attributes.indexOf("#[cfg("); at >= 0; at = attributes.indexOf("#[cfg(", at + 1)) {
    let depth = 0;
    for (let i = at + 5; i < attributes.length; i++) {
      if (attributes[i] === "(") depth++;
      else if (attributes[i] === ")" && --depth === 0) {
        found.push(attributes.slice(at + 2, i + 1));
        break;
      }
    }
  }
  return found;
}

/** Whether `cfg`, `cfg(..)`'s text, holds on wasm32 only because of
 * `test`: false of a build without it, whatever features are on, which the
 * Crater can't know, so a `feature = ".."` is neither (#1153). */
export function onlyForTests(cfg: string): boolean {
  return cfgHolds(cfg, false) === false && cfgHolds(cfg, true) !== false;
}

/** Whether `cfg` holds on wasm32, of a build with `test` or without: true,
 * false, or `undefined` where it's of a feature, which the Crater can't
 * know. Of rustc's own cfgs of the target (`targetCfgs`). */
export function cfgHolds(cfg: string, test: boolean): boolean | undefined {
  const facts = targetCfgs();
  type Truth = boolean | undefined;
  const tokens = cfg.match(/"(?:[^"\\]|\\.)*"|[A-Za-z_][A-Za-z0-9_]*|[(),=]/g) ?? [];
  const evaluate = (): Truth => {
    let i = 0;
    const list = (): Truth[] => {
      const items: Truth[] = [];
      if (tokens[i] !== "(") throw new Error(`unreadable cfg: ${cfg}`);
      i++;
      while (tokens[i] !== ")") {
        items.push(predicate());
        if (tokens[i] === ",") i++;
      }
      i++;
      return items;
    };
    const predicate = (): Truth => {
      const name = tokens[i++];
      if (name === "all") return list().reduce<Truth>((a, b) => (a === false || b === false ? false : a === undefined || b === undefined ? undefined : true), true);
      if (name === "any") return list().reduce<Truth>((a, b) => (a === true || b === true ? true : a === undefined || b === undefined ? undefined : false), false);
      if (name === "not") {
        const [inner] = list();
        return inner === undefined ? undefined : !inner;
      }
      if (tokens[i] === "=") {
        i++;
        const value = JSON.parse(tokens[i++]);
        if (name === "feature") return undefined;
        return facts.get(name)?.has(value) ?? false;
      }
      if (name === "test") return test;
      return facts.has(name) && facts.get(name)!.size === 0;
    };
    if (tokens[i++] !== "cfg") throw new Error(`unreadable cfg: ${cfg}`);
    const [whole] = list();
    return whole;
  };
  return evaluate();
}

/** Where the attributes just above line `at`, from 1, start: each line that
 * starts with `#[`, and each attribute of several lines, uuid's
 * `#[cfg_attr(\n    all(..),\n    wasm_bindgen_test\n)]` between its
 * `#[test]` and its `fn`, its lines from `#[` to `]` one balanced
 * attribute (#1121). */
export function attributesFrom(lines: string[], at: number): number {
  let from = at;
  while (from > 1) {
    const above = lines[from - 2];
    if (/^\s*#\[/.test(above)) {
      from--;
      continue;
    }
    if (!/\]\s*$/.test(above)) break;
    let start = from - 2;
    while (start > 0 && !/^\s*#\[/.test(lines[start])) start--;
    const attribute = lines.slice(start, from - 1).join("\n");
    const opened = (attribute.match(/\[/g) ?? []).length;
    if (!/^\s*#\[/.test(lines[start]) || opened !== (attribute.match(/\]/g) ?? []).length) break;
    from = start + 1;
  }
  return from;
}

/** An item of a Rust source, by a scan of its text: a `fn`'s or a `mod`'s
 * name, the lines its attributes start and its body ends, from 1, and
 * whether it's a `#[test]` or `#[cfg(test)]`. Strings, characters and
 * comments are skipped for their braces; a raw string's aren't. */
export type Item = {
  kind: "fn" | "mod";
  name: string;
  from: number;
  to: number;
  test: boolean;
  cfgTest: boolean;
  /** Its `#[cfg(..)]`s, `cfg(unix)`. */
  cfgs: string[];
  /** A module of a file of its own, `mod tests;`. */
  file?: boolean;
};

export function items(text: string): Item[] {
  const lines = text.split("\n");
  // Each offset's line, from 1.
  const lineAt: number[] = [];
  lines.forEach((line, i) => {
    for (let k = 0; k <= line.length; k++) lineAt.push(i + 1);
  });
  const found: Item[] = [];
  const open: { item: Item | undefined }[] = [];
  let pending: { kind: "fn" | "mod"; name: string; at: number } | undefined;
  for (let i = 0; i < text.length; i++) {
    const c = text[i];
    if (c === "/" && text[i + 1] === "/") {
      i = text.indexOf("\n", i);
      if (i < 0) break;
      continue;
    }
    if (c === "/" && text[i + 1] === "*") {
      i = text.indexOf("*/", i + 2) + 1;
      if (i <= 0) break;
      continue;
    }
    if (c === '"') {
      for (i++; i < text.length && text[i] !== '"'; i++)
        if (text[i] === "\\") i++;
      continue;
    }
    // A character, `'{'`, not a lifetime, `'a`.
    if (
      c === "'" &&
      (text[i + 2] === "'" ||
        (text[i + 1] === "\\" && text.indexOf("'", i + 2) - i <= 10))
    ) {
      i = text.indexOf("'", i + 2);
      continue;
    }
    const word = /^(fn|mod)\s+([A-Za-z_][A-Za-z0-9_]*)/.exec(
      text.slice(i, i + 80),
    );
    if (word && !/[A-Za-z0-9_]/.test(text[i - 1] ?? " ")) {
      pending = { kind: word[1] as "fn" | "mod", name: word[2], at: i };
      i += word[0].length - 1;
      continue;
    }
    if (c === ";" && pending) {
      // A module of a file of its own, `mod tests;`.
      if (pending.kind === "mod") {
        const from = attributesFrom(lines, lineAt[pending.at]);
        const attributes = lines.slice(from - 1, lineAt[pending.at]).join("\n");
        found.push({
          kind: "mod",
          name: pending.name,
          from,
          to: lineAt[i],
          test: false,
          cfgTest: cfgsOf(attributes).some(onlyForTests),
          cfgs: cfgsOf(attributes),
          file: true,
        });
      }
      pending = undefined;
    }
    if (c === "{") {
      let item: Item | undefined;
      if (pending) {
        // Its attributes: those just above it (`attributesFrom`).
        const from = attributesFrom(lines, lineAt[pending.at]);
        const attributes = lines.slice(from - 1, lineAt[pending.at]).join("\n");
        item = {
          kind: pending.kind,
          name: pending.name,
          from,
          to: 0,
          test: /#\[(?:\w+::)*test\b/.test(attributes),
          cfgTest: cfgsOf(attributes).some(onlyForTests),
          cfgs: cfgsOf(attributes),
        };
        pending = undefined;
      }
      open.push({ item });
    }
    if (c === "}") {
      const closed = open.pop();
      if (closed?.item) {
        closed.item.to = lineAt[i];
        found.push(closed.item);
      }
    }
  }
  return found;
}

/** A module's path in its crate, of its file, `src/a/b.rs` or `src/a/b/mod.rs`
 * `a::b`, `src/lib.rs` none. */
export function modulePath(file: string): string[] {
  const parts = file
    .replace(/^src\//, "")
    .replace(/\.rs$/, "")
    .split("/");
  if (
    parts.at(-1) === "mod" ||
    (parts.length === 1 && (parts[0] === "lib" || parts[0] === "main"))
  )
    parts.pop();
  return parts;
}

/** The file that declares the module of `file`, `src/a/b.rs`'s `mod b;` in
 * `src/a.rs` or `src/a/mod.rs`, and the module's name; none of `src/lib.rs`. */
export function declaredIn(file: string, exists: (path: string) => boolean): { parent: string; name: string } | undefined {
  const path = modulePath(file);
  const name = path.at(-1);
  if (!name) return undefined;
  const above = path.slice(0, -1);
  const parent =
    above.length === 0
      ? exists("src/lib.rs")
        ? "src/lib.rs"
        : "src/main.rs"
      : exists(`src/${above.join("/")}.rs`)
        ? `src/${above.join("/")}.rs`
        : `src/${above.join("/")}/mod.rs`;
  return { parent, name };
}

/** Whether `file`'s module, or one it's in, is declared `#[cfg(test)]`,
 * `#[cfg(test)] mod tests;`: built only for tests. */
export function testOnlyFile(file: string, read: (path: string) => string | undefined): boolean {
  const declared = declaredIn(file, (path) => read(path) !== undefined);
  if (!declared) return false;
  const text = read(declared.parent);
  if (text === undefined) return false;
  const all = items(text);
  const at = all.find((item) => item.kind === "mod" && item.file && item.name === declared.name);
  if (!at) return false;
  return (
    at.cfgTest ||
    all.some((item) => item.kind === "mod" && item.cfgTest && item.from < at.from && at.to <= item.to) ||
    testOnlyFile(declared.parent, read)
  );
}

/** What to cut of the test build for an error at `line` of `file`, whose
 * text is `text`: the `#[test]` it's in, by its path, `tests::t`, or a
 * function only tests reach, one inside a `#[cfg(test)]` module or a file
 * only tests build, `testOnly`, each its attributes' first line; or
 * nothing, where it's the library's own code. `path` is its module's, of an
 * integration test's crate `testModulePath`'s. */
export function cutFor(file: string, text: string, line: number, testOnly = false, path = modulePath(file)): { from: number; test?: string } | undefined {
  const all = items(text);
  const around = all
    .filter((item) => item.from <= line && line <= item.to)
    .sort((a, b) => b.from - a.from);
  const test = around.find((item) => item.kind === "fn" && item.test);
  const mods = (at: Item) =>
    all
      .filter(
        (item) =>
          item.kind === "mod" && item.from < at.from && at.to <= item.to,
      )
      .sort((a, b) => a.from - b.from);
  if (test)
    return {
      from: test.from,
      test: [
        ...path,
        ...mods(test).map((m) => m.name),
        test.name,
      ].join("::"),
    };
  // The outermost function around it, inside a module only tests build.
  const fns = around.filter((item) => item.kind === "fn");
  const outer = fns.at(-1);
  if (outer && (testOnly || mods(outer).some((m) => m.cfgTest))) return { from: outer.from };
  return undefined;
}

/** `#[cfg(any())]` above each line of `froms` in `text`, so what starts
 * there isn't built. */
export function cut(text: string, froms: number[]): string {
  const lines = text.split("\n");
  for (const from of [...new Set(froms)].sort((a, b) => b - a)) {
    const indent = /^\s*/.exec(lines[from - 1])?.[0] ?? "";
    lines.splice(
      from - 1,
      0,
      `${indent}#[cfg(any())] // rust-js's Crater: refused`,
    );
  }
  return lines.join("\n");
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
 * getrandom's WebAssembly feature (#1087). */
const asking = 2;

/** What a crate's kept verdict is of, beside its release and the
 * compiler's build: the features its row names, how the Crater asks, and
 * the tooling that builds it, rust-js's own wasm-bindgen family among it,
 * which a change to asks it again (#1121). */
function keptKey(crate: Crate, build: string): string {
  const hash = createHash("sha256").update(`${build}\n${(crate.features ?? []).join(",")}\n${asking}\n`);
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
          : `its tests don't run natively: ${native.said.trim().split("\n").slice(-3).join(" ")}`,
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
  try {
    return await askedAsJs(nativeRan, jsDir, target, manifest, lock, featured);
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
): Promise<Verdict> {
  const js: Outcomes = {};
  const cutTests: Record<string, string> = {};
  // Its library's tests, as natively: no integration test built, nor a
  // dev-dependency only they name.
  if (Object.keys(nativeRan).some((name) => !name.startsWith("tests/"))) {
    const ran = await built(jsDir, target, libraryTestsOnly(manifest, (lib) => sourceNames(join(jsDir, "src"), lib)), lock, featured);
    if ("refused" in ran) return { native: nativeRan, refused: ran.refused };
    if ("problem" in ran) return ran;
    Object.assign(js, ran.js);
    Object.assign(cutTests, ran.cut);
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
    for (const [name, why] of Object.entries(ran.cut)) cutTests[`${file}::${name}`] = why;
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
  writeFileSync(join(jsDir, "Cargo.toml"), manifest);
  const ran = await cutUntilBuilt(jsDir, target, featured, rootFile);
  const withFeature = forTheJs(manifest, lock);
  if (!("problem" in ran && ran.problem.includes("could not compile `getrandom`")) || withFeature === manifest) return ran;
  writeFileSync(join(jsDir, "Cargo.toml"), withFeature);
  return cutUntilBuilt(jsDir, target, featured, rootFile);
}

/** `rust-js test` in `jsDir`, each refused test cut from the source and
 * the rest built again, up to twelve rounds: what ran, and what was cut and
 * why; or where an error is in no test, what the library's code refused;
 * or why it couldn't be asked. `rootFile` is the integration test's file
 * being built, `tests/version.rs`, whose crate's modules are its own, every
 * one of `tests/` only tests build; none of the library's tests. */
async function cutUntilBuilt(
  jsDir: string,
  target: string,
  featured: string[],
  rootFile?: string,
): Promise<{ js: Outcomes; cut: Record<string, string> } | { refused: string[] } | { problem: string }> {
  const cutTests: Record<string, string> = {};
  for (let round = 0; round < 12; round++) {
    const js = await spawned(
      ["bun", join(root, "tooling", "cli.js"), "test", ...featured],
      jsDir,
      { RUST_JS_COMPILER: compiler, CARGO_TARGET_DIR: target, TZ: zone },
      1200,
    );
    const jsRan = jsOutcomes(js.said);
    if (Object.keys(jsRan).length > 0) return { js: jsRan, cut: cutTests };
    const errors = errorsAt(js.said).filter((e) => e.file.startsWith("src/") || (rootFile !== undefined && e.file.startsWith("tests/")));
    // Each of its tests cut: none left to run.
    if (errors.length === 0 && js.code === 0 && Object.keys(cutTests).length > 0) return { js: jsRan, cut: cutTests };
    if (errors.length === 0) {
      return {
        problem:
          js.code === 0 ? "`rust-js test` passed having run no test" : `its JS tests didn't run: ${js.said.trim().split("\n").slice(-3).join(" ")}`,
      };
    }
    const byFile = new Map<string, number[]>();
    const library: string[] = [];
    for (const error of errors) {
      const text = readFileSync(join(jsDir, error.file), "utf8");
      const read = (path: string) => (existsSync(join(jsDir, path)) ? readFileSync(join(jsDir, path), "utf8") : undefined);
      // An integration test's file is its crate's; the library's own code,
      // as that test uses it, is never cut.
      const at = error.file.startsWith("tests/")
        ? cutFor(error.file, text, error.line, true, testModulePath(error.file, rootFile ?? ""))
        : rootFile === undefined
          ? cutFor(error.file, text, error.line, testOnlyFile(error.file, read))
          : undefined;
      if (!at) {
        library.push(`${error.why} (${error.file}:${error.line})`);
        continue;
      }
      if (at.test && !(at.test in cutTests)) cutTests[at.test] = error.why;
      byFile.set(error.file, [...(byFile.get(error.file) ?? []), at.from]);
    }
    if (library.length > 0) return { refused: [...new Set(library)] };
    for (const [file, froms] of byFile) writeFileSync(join(jsDir, file), cut(readFileSync(join(jsDir, file), "utf8"), froms));
  }
  return { problem: "its JS tests still didn't build after 12 rounds of cuts" };
}

/** A module's path in an integration test's crate, whose root is `root`,
 * `tests/version.rs`: none of the root, `util` of `tests/util/mod.rs` or
 * `tests/util.rs`. */
export function testModulePath(file: string, root: string): string[] {
  if (file === root) return [];
  return file
    .replace(/^tests\//, "")
    .replace(/(\/mod)?\.rs$/, "")
    .split("/");
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
  const used = (dependency: string) => named(dependency.replaceAll("-", "_"));
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

/** Of `crate`'s differences, those no decision says, each a wrong answer. */
export function undecided(crate: string, differ: { name: string; js: string }[], decided: Map<string, string>) {
  return differ.filter((d) => !decided.has(`${crate} ${d.name}`));
}

/** A file's inner attributes, `#![..]`, each its own, at its start before
 * any item, comments and blank lines skipped. */
export function innerAttributes(text: string): string {
  const found: string[] = [];
  let at = 0;
  for (;;) {
    const rest = text.slice(at);
    const skipped = /^(\s+|\/\/[^\n]*\n?|\/\*[\s\S]*?\*\/)/.exec(rest);
    if (skipped) {
      at += skipped[0].length;
      continue;
    }
    if (!rest.startsWith("#![")) return found.join("\n");
    let depth = 0;
    let end = 2;
    for (; end < rest.length; end++) {
      if (rest[end] === "[") depth++;
      else if (rest[end] === "]" && --depth === 0) break;
    }
    found.push(rest.slice(0, end + 1));
    at += end + 1;
  }
}

/** Whether the test `name`, `a::b::tests::t`, is of a module a cfg leaves
 * off wasm32 in a test build, `#[cfg(unix)] mod tz_info;`, read of the
 * crate's source through `read` (#1153); or of an integration test's file,
 * `tests/dateutils.rs::t`, whose own `#![cfg(unix)]` leaves it off. */
export function offTarget(name: string, read: (path: string) => string | undefined): boolean {
  const integration = /^(tests\/[^/]+\.rs)::/.exec(name)?.[1];
  if (integration !== undefined) {
    const text = read(integration) ?? "";
    return cfgsOf(innerAttributes(text).replaceAll("#![", "#[")).some((cfg) => cfgHolds(cfg, true) === false);
  }
  const modules = name.split("::").slice(0, -1);
  let file = read("src/lib.rs") !== undefined ? "src/lib.rs" : "src/main.rs";
  let scope: { from: number; to: number } | undefined;
  const path: string[] = [];
  for (const module of modules) {
    const text = read(file);
    if (text === undefined) return false;
    const declared = items(text).find((item) => item.kind === "mod" && item.name === module && (!scope || (scope.from < item.from && item.to <= scope.to)));
    if (!declared) return false;
    if (declared.cfgs.some((cfg) => cfgHolds(cfg, true) === false)) return true;
    path.push(module);
    if (declared.file) {
      const flat = `src/${path.join("/")}.rs`;
      file = read(flat) !== undefined ? flat : `src/${path.join("/")}/mod.rs`;
      scope = undefined;
    } else scope = declared;
  }
  return false;
}

/** A crate's verdict of its tests' outcomes: each native one not run as
 * JS of a module a cfg leaves off wasm32 apart from those that differ. */
function judged(native: Outcomes, js: Outcomes, cut: Record<string, string>, dir: string): Verdict {
  const read = (path: string) => (existsSync(join(dir, path)) ? readFileSync(join(dir, path), "utf8") : undefined);
  const differ = differences(native, js, cut);
  const off = differ.filter((d) => d.js === "not run" && offTarget(d.name, read)).map((d) => d.name);
  return { native, js, cut, differ: differ.filter((d) => !off.includes(d.name)), offTarget: off };
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
  const cut = Object.keys(verdict.cut).length;
  const off = verdict.offTarget?.length ?? 0;
  const same = passed - cut - off - verdict.differ.length;
  const wrong = undecided(crate.name, verdict.differ, decided);
  const known = verdict.differ.length - wrong.length;
  const shown = wrong.slice(0, 5).map((d) => `${d.name} (${d.js})`);
  return `${name}: ${same} of ${passed} as native, ${cut} refused${off > 0 ? `, ${off} not of wasm32` : ""}${known > 0 ? `, ${known} decided` : ""}${wrong.length > 0 ? `, ${wrong.length} WRONG: ${shown.join(", ")}${wrong.length > 5 ? ", .." : ""}` : ""}`;
}

if (import.meta.main) {
  const named = process.argv.slice(2).filter((arg) => !arg.startsWith("--"));
  const rows = corpusRows(
    readFileSync(join(root, "docs", "crate-corpus.md"), "utf8"),
  );
  if ("problem" in rows) throw new Error(rows.problem);
  const chosen =
    named.length > 0
      ? rows.filter((crate) => named.includes(crate.name))
      : rows;
  if (chosen.length === 0)
    throw new Error(`no crate of the corpus is named ${named.join(", ")}`);
  const build = currentBuild(compiler);
  if (!build) throw new Error(`no compiler at ${compiler}: run cargo build`);
  // A crate's native test build and its JS one, each a Cargo build of its
  // graph: half the cores' crates at once, and no more than the memory
  // holds at about 2 GB a crate (scripts/shard.ts).
  const width = Math.max(
    1,
    Math.min(
      Math.floor(availableParallelism() / 2),
      Math.floor(memoryBudget() / 2),
    ),
  );
  const decided = decidedOf(readFileSync(join(import.meta.dir, "crater-decided.txt"), "utf8"));
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
  writeFileSync(
    join(root, "target", "crater.json"),
    JSON.stringify(Object.fromEntries(verdicts), null, 2),
  );
  // With `--check`, a wrong answer no decision says fails it.
  const wrong = chosen.flatMap((crate) => {
    const verdict = verdicts.get(crate.name);
    return verdict && "differ" in verdict ? undecided(crate.name, verdict.differ, decided).map((d) => `${crate.name} ${d.name}`) : [];
  });
  if (process.argv.includes("--check") && wrong.length > 0) {
    console.log(`\n${wrong.length} wrong answer(s), each a P0 to reduce: ${wrong.join(", ")}`);
    process.exitCode = 1;
  }
}
