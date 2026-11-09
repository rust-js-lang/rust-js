// A small Crater (#1087): each crate of the crate corpus, at its exact
// release (docs/crate-corpus.md's rows, the one list, ADR 1067), its own
// `#[test]`s run natively, `cargo test --lib`, and as JS, `rust-js test`
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
// `target/crater/native` and as JS in `target/crater/js`, the targets each
// crate shares. A crate's verdict is kept by its release and the compiler's
// build, `target/crater/kept/`, and asked again only where either changed.
// Writes `target/crater.json` and prints a line per crate.

import { spawn } from "node:child_process";
import { availableParallelism, homedir } from "node:os";
import {
  cpSync,
  existsSync,
  mkdirSync,
  readFileSync,
  readdirSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { join } from "node:path";
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
        let from = lineAt[pending.at];
        while (from > 1 && /^\s*#\[/.test(lines[from - 2])) from--;
        const attributes = lines.slice(from - 1, lineAt[pending.at]).join("\n");
        found.push({
          kind: "mod",
          name: pending.name,
          from,
          to: lineAt[i],
          test: false,
          cfgTest: /#\[cfg\(test\)\]/.test(attributes),
          file: true,
        });
      }
      pending = undefined;
    }
    if (c === "{") {
      let item: Item | undefined;
      if (pending) {
        // Its attributes: the lines just above it that start with `#[`.
        let from = lineAt[pending.at];
        while (from > 1 && /^\s*#\[/.test(lines[from - 2])) from--;
        const attributes = lines.slice(from - 1, lineAt[pending.at]).join("\n");
        item = {
          kind: pending.kind,
          name: pending.name,
          from,
          to: 0,
          test: /#\[(?:\w+::)*test\b/.test(attributes),
          cfgTest: /#\[cfg\(test\)\]/.test(attributes),
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
 * nothing, where it's the library's own code. */
export function cutFor(file: string, text: string, line: number, testOnly = false): { from: number; test?: string } | undefined {
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
        ...modulePath(file),
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
    });
    let said = "";
    p.stdout.on("data", (d) => (said += d));
    p.stderr.on("data", (d) => (said += d));
    const timer = setTimeout(() => p.kill("SIGKILL"), seconds * 1000);
    p.on("close", (code) => {
      clearTimeout(timer);
      done({ code, said });
    });
  });
}

/** Where Cargo unpacked `crate`'s source, fetching it first where it hasn't. */
async function source(crate: Crate): Promise<string | { problem: string }> {
  const registry = join(
    process.env.CARGO_HOME ?? join(homedir(), ".cargo"),
    "registry",
    "src",
  );
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

async function measure(crate: Crate, build: string): Promise<Verdict> {
  const kept = join(
    work,
    "kept",
    `${crate.name}-${crate.version}-${build}.json`,
  );
  if (existsSync(kept)) return JSON.parse(readFileSync(kept, "utf8"));
  const from = await source(crate);
  if (typeof from !== "string") return from;
  const dir = join(work, `${crate.name}-${crate.version}`);
  rmSync(dir, { recursive: true, force: true });
  cpSync(from, dir, { recursive: true });
  // Its own lock where it packs one, as its tests were run against.
  const locked = existsSync(join(dir, "Cargo.lock")) ? ["--locked"] : [];
  const native = await spawned(
    ["cargo", "test", "--lib", "--no-fail-fast", ...locked],
    dir,
    { CARGO_TARGET_DIR: join(work, "native"), TZ: zone },
    900,
  );
  const nativeRan = nativeOutcomes(native.said);
  if (native.code === 0 && /^running 0 tests$/m.test(native.said) && Object.keys(nativeRan).length === 0) {
    return { problem: "it has no library tests: its tests are integration tests alone" };
  }
  if (Object.keys(nativeRan).length === 0) {
    return {
      problem: `its tests don't run natively: ${native.said.trim().split("\n").slice(-3).join(" ")}`,
    };
  }
  // The JS's own copy, whose refused tests are cut from it, round by round:
  // what one cut leaves may be refused next, a helper's callers.
  const jsDir = `${dir}-js`;
  rmSync(jsDir, { recursive: true, force: true });
  cpSync(from, jsDir, { recursive: true });
  // Its library's tests alone, as natively: no integration test is built.
  writeFileSync(
    join(jsDir, "Cargo.toml"),
    libraryTestsOnly(readFileSync(join(jsDir, "Cargo.toml"), "utf8"), (lib) => sourceNames(join(jsDir, "src"), lib)),
  );
  const cutTests: Record<string, string> = {};
  // Its unit of an earlier run, which a build that stopped may have left
  // as done with no tests written: built again.
  await spawned(
    [
      "cargo",
      "clean",
      "--package",
      crate.name,
      "--target",
      "wasm32-unknown-unknown",
    ],
    jsDir,
    { CARGO_TARGET_DIR: join(work, "js") },
    120,
  );
  let verdict: Verdict | undefined;
  for (let round = 0; round < 12 && !verdict; round++) {
    const js = await spawned(
      ["bun", join(root, "tooling", "cli.js"), "test"],
      jsDir,
      { RUST_JS_COMPILER: compiler, CARGO_TARGET_DIR: join(work, "js"), TZ: zone },
      1200,
    );
    const jsRan = jsOutcomes(js.said);
    if (Object.keys(jsRan).length > 0) {
      verdict = {
        native: nativeRan,
        js: jsRan,
        cut: cutTests,
        differ: differences(nativeRan, jsRan, cutTests),
      };
      break;
    }
    const errors = errorsAt(js.said).filter((e) => e.file.startsWith("src/"));
    // Each of its tests cut: none left to run.
    if (errors.length === 0 && js.code === 0 && Object.keys(cutTests).length > 0) {
      verdict = { native: nativeRan, js: jsRan, cut: cutTests, differ: differences(nativeRan, jsRan, cutTests) };
      break;
    }
    if (errors.length === 0) {
      verdict = {
        problem:
          js.code === 0 ? "`rust-js test` passed having run no test" : `its JS tests didn't run: ${js.said.trim().split("\n").slice(-3).join(" ")}`,
      };
      break;
    }
    const byFile = new Map<string, number[]>();
    const library: string[] = [];
    for (const error of errors) {
      const text = readFileSync(join(jsDir, error.file), "utf8");
      const read = (path: string) => (existsSync(join(jsDir, path)) ? readFileSync(join(jsDir, path), "utf8") : undefined);
      const at = cutFor(error.file, text, error.line, testOnlyFile(error.file, read));
      if (!at) {
        library.push(`${error.why} (${error.file}:${error.line})`);
        continue;
      }
      if (at.test && !(at.test in cutTests)) cutTests[at.test] = error.why;
      byFile.set(error.file, [...(byFile.get(error.file) ?? []), at.from]);
    }
    if (library.length > 0) {
      verdict = { native: nativeRan, refused: [...new Set(library)] };
      break;
    }
    for (const [file, froms] of byFile)
      writeFileSync(
        join(jsDir, file),
        cut(readFileSync(join(jsDir, file), "utf8"), froms),
      );
  }
  verdict ??= {
    problem: "its JS tests still didn't build after 12 rounds of cuts",
  };
  // One that couldn't be asked is asked again next time.
  if (!("problem" in verdict)) {
    mkdirSync(join(work, "kept"), { recursive: true });
    writeFileSync(kept, JSON.stringify(verdict));
  }
  return verdict;
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
  const same = passed - cut - verdict.differ.length;
  const wrong = undecided(crate.name, verdict.differ, decided);
  const known = verdict.differ.length - wrong.length;
  const shown = wrong.slice(0, 5).map((d) => `${d.name} (${d.js})`);
  return `${name}: ${same} of ${passed} as native, ${cut} refused${known > 0 ? `, ${known} decided` : ""}${wrong.length > 0 ? `, ${wrong.length} WRONG: ${shown.join(", ")}${wrong.length > 5 ? ", .." : ""}` : ""}`;
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
