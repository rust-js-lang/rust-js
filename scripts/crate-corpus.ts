// Which crates from crates.io rust-js compiles to JS (ROADMAP M8.2): each in a
// Cargo project of its own, every library of its graph compiled by rust-js
// as Cargo's `RUSTC_WRAPPER`, and what each refused first. A crate with a
// probe, `crate-corpus/<name>.rs`, a library of the project that uses it and
// has a `report()`, is run too where it compiles: its report, the JS's and
// native Rust's, the same or not. Compiled isn't run: bitflags' JS compiled
// and didn't parse.
//
//   bun scripts/crate-corpus.ts [name ...]     the corpus, or the crates named
//   bun scripts/crate-corpus.ts --check [name ...]
//                                              and fail where one does worse
//                                              than docs/crate-corpus.md says
//   bun scripts/crate-corpus.ts --lock name ...
//                                              resolve each one's graph anew,
//                                              its newest releases, write its
//                                              lock, and measure it
//
// The crates, each at its exact release, are docs/crate-corpus.md's rows,
// and every crate of each one's graph is at the release its lock,
// `crate-corpus/<name>.lock`, names, which Cargo is held to (`--locked`).
// Writes `target/crate-corpus.json` and prints a line per crate: its own
// verdict, then each crate of its graph rust-js refused, and why, or that
// crashed, then whether its probe runs as natively.

import { spawn, spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { copyFileSync, existsSync, mkdirSync, readdirSync, readFileSync, realpathSync, rmSync, statSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { bytesOf, evict, freeBytes, type Kept } from "../tooling/bounded.js";
import { cargoBuild, cargoEnv } from "../tooling/cargo.js";
import { channel } from "./toolchain";
import { cargoHome, floorBytes } from "../test/bounded";
import { run as runChild, stopped, type Exit } from "../test/child";
import { atOnce } from "./shard";
import { mirLabels } from "./labels";
import { refuseUnmatched } from "../test/case-lists";

/** The target a run keeps what it builds and writes in: the checkout's, or
 * with `RUST_JS_TARGET`, a mutant's tests' own (ADR 0093), so mutants
 * measured at once share no project and no results file. */
const ownTarget = process.env.RUST_JS_TARGET ? resolve(process.env.RUST_JS_TARGET) : join(resolve(import.meta.dir, ".."), "target");

/** Where each crate's Cargo project and targets are kept between runs,
 * `<crate>/`, so a run builds only what changed. */
export const crateProjects = join(ownTarget, "crate-corpus");

/** The most the crates' projects keep together. From nothing they're about
 * 600 MB, built in 43 s on the M1 (2026-11-10, 23 crates); each build of
 * the compiler adds about a copy of every crate's dependencies, which no run
 * removes, so they'd grown to 25 GB, `url`'s 190 copies of each, 5.8 GB
 * (#1213). At 2 GiB a crate's rebuilt about every third build of the
 * compiler, two seconds of it. With the top crates by downloads, 100 rows,
 * they're 1.2 GiB from nothing (2026-11-10, #1174), and 4 GiB keeps that
 * rate. */
export const crateProjectsCap = 4 * 2 ** 30;

/** The crates' projects in `dir` held to `cap`, the least recently measured
 * first, and nothing kept below the disk's floor, `free` its free bytes
 * (tooling/bounded.js's `evict`). How many it removed, and the size of
 * what's left. */
export function boundCrateProjects({ dir = crateProjects, cap = crateProjectsCap, free = () => freeBytes(dir) }: { dir?: string; cap?: number; free?: () => number } = {}): { removed: number; size: number } {
  return evict(crateProjectsKept(dir), { cap, free, floor: floorBytes });
}

/** Each crate's project in `dir`, with when it was last used, its `used`
 * mark's time. */
export function crateProjectsKept(dir: string): Kept[] {
  let names: string[];
  try {
    names = readdirSync(dir);
  } catch {
    return [];
  }
  return names
    .map((name) => join(dir, name))
    .filter((path) => statSync(path).isDirectory())
    .map((path) => {
      const marked = join(path, "used");
      return { path, bytes: bytesOf(path), used: statSync(existsSync(marked) ? marked : path).mtimeMs };
    });
}

export type Verdict = "runs" | "compiles" | "refused" | "blocked";

/** A crate of the corpus, as its row in docs/crate-corpus.md says: its exact
 * release; the features it's built with, its default ones off, where the
 * row names any; and the verdict that's its bar, with what an open issue
 * says it does now, where one does; and whether a push's check samples it
 * (`sampled`). */
export type Crate = { name: string; version: string; features?: string[]; bar: Verdict; open?: { now: Verdict; issue: string }; sampled?: true };

/** The first cell of the header of a table whose rows a push's check
 * samples, one in `sampledOneIn` (#1174): the top crates by downloads,
 * each graph refused somewhere rebuilt on every run, 94 s of the 100 rows
 * where the rest take 6. The nightly runs every row. */
export const sampledHeader = "Crate, one in 8 a push";

/** How many of a sampled table's rows a push's check asks one of. */
export const sampledOneIn = 8;

/** Whether a push's check of `seed`, its commit, asks `crate`: every row
 * but a sampled table's, and of those the ones a hash of the seed and the
 * crate's name picks, a new set each commit (#1174). */
export function sampledIn(crate: Crate, seed: string): boolean {
  if (!crate.sampled) return true;
  return createHash("sha256").update(`${seed}\n${crate.name}`).digest().readUInt32BE(0) % sampledOneIn === 0;
}

/** docs/crate-corpus.md's table, the corpus's one list (ADR 1067): each row
 * whose second cell is a verdict is a crate, its first cell the crate's
 * name, its exact release and, in parentheses, its features, `chrono 0.4.45
 * (`alloc`)`. A row of no exact release is refused: crates.io's newest
 * release had turned crates red with no change here (either 1.19.0,
 * smallvec 1.16.4), and a new release is measured by editing its row, its
 * build metadata too where it has some, `toml 1.2.1+spec-1.1.0`. The
 * verdict is `runs`, or `runs (refused: #467)` where an open issue says
 * the crate does worse now. A row of a table whose header's first cell is
 * `sampledHeader` is `sampled`. Or why the table can't be read. */
export function corpusRows(doc: string): Crate[] | { problem: string } {
  const crates: Crate[] = [];
  let sampled = false;
  for (const line of doc.split("\n")) {
    const cells = line.split("|").map((cell) => cell.trim());
    if (cells.length > 3 && cells[2] === "Verdict") sampled = cells[1] === sampledHeader;
    const said = cells[2]?.match(/^(runs|compiles|refused|blocked)(?: \((runs|compiles|refused|blocked): (#\d+)\))?$/);
    if (cells.length <= 3 || !said) continue;
    // A release's build metadata is part of it, toml's `1.2.1+spec-1.1.0`.
    const named = cells[1].match(/^([\w-]+) (\d+\.\d+\.\d+(?:\+[\w.-]+)?)(?: \((`[\w-]+`(?:, `[\w-]+`)*)\))?$/);
    if (!named) {
      return {
        problem: `docs/crate-corpus.md's row \`${cells[1]}\` names no exact release: write the crate's name and version, \`either 1.19.0\`, and its features after them, \`chrono 0.4.45 (\`alloc\`)\``,
      };
    }
    const [, bar, now, issue] = said as [string, Verdict, Verdict | undefined, string | undefined];
    crates.push({
      name: named[1],
      version: named[2],
      ...(named[3] ? { features: named[3].split(", ").map((feature) => feature.slice(1, -1)) } : {}),
      bar,
      ...(now && issue ? { open: { now, issue } } : {}),
      ...(sampled ? { sampled: true as const } : {}),
    });
  }
  return crates;
}

/** The dependency on `crate` its Cargo project writes: its release exactly,
 * and only its row's features, where it names any. */
export function dependency(crate: Crate): string {
  const spec = [
    `version = "=${crate.version}"`,
    ...(crate.features ? ["default-features = false", `features = [${crate.features.map((f) => JSON.stringify(f)).join(", ")}]`] : []),
  ].join(", ");
  return `${crate.name} = { ${spec} }`;
}

/** Where the lock of `name`'s graph is: every crate of it at one release,
 * the row's crate at its row's, so a crate's verdict is this checkout's
 * alone, never a dependency's new release (ADR 1067, amended). */
export const lockOf = (name: string) => join(import.meta.dir, "crate-corpus", `${name}.lock`);

/** The release of the crate `name` a lock, a Cargo.lock's text, names, or
 * none. */
export function lockedVersion(lock: string, name: string): string | undefined {
  return lock.match(new RegExp(`\\[\\[package\\]\\]\\nname = "${name}"\\nversion = "([^"]+)"`))?.[1];
}

/** The crates of its graph Cargo's `stderr` says rust-js crashed on: each a
 * crate that didn't compile, `process didn't exit successfully`, where a
 * refusal is a compile error Cargo reports as one. Each named by its
 * package's name, with what it ended with: its signal or exit status. */
export function crashes(stderr: string): { name: string; how: string }[] {
  const said = /could not compile `([^`]+)` \(lib\)[^\n]*\n\nCaused by:\n {2}process didn't exit successfully: .*\(([^()]*)\)$/gm;
  return [...stderr.matchAll(said)].map(([, name, how]) => ({ name: name!, how: how! }));
}

type Refusal = { crate: string; error: string };
export type Result = {
  name: string;
  version: string;
  compiled: string[];
  refused: Refusal[];
  // Each crate rust-js crashed on: a stack overflow, a panic. regex-syntax's
  // went unseen, neither compiled nor refused, for weeks (#511).
  crashed: Refusal[];
  own: "compiles" | "refused" | "blocked";
  // What Cargo said last where nothing was refused, though the crate wasn't
  // compiled: a crash, or a build script's failure.
  unexplained?: string;
  // Its probe's report, the JS's beside native Rust's: `same`, or where
  // they differ, or why the probe didn't compile or run.
  runs?: string;
  // Each label MIR made in the JS of what it compiled, `<crate>/<file>:
  // <function>` (scripts/labels.ts), which `--check` holds to
  // `listedLabels` (#2215).
  labels: string[];
  // Each file of its JS that names this machine's `$CARGO_HOME`, which
  // `--check` fails of (#2435).
  homes: string[];
  // Where each crate of its graph it compiled has its JS, by its id,
  // `itertools@0.14.0`, which `--check` cuts `snapshotted`'s from.
  js: Record<string, string>;
};

/** The labels MIR made that the crate corpus's JS has today, each
 * `<crate>@<version>/<file>: <function>` (`Result`'s `labels`), listed
 * under #2215 till their structuring loses them, as std's are
 * (test/readability.test.ts): `--check` fails of one not listed, and of
 * one listed that's gone, which comes off the list. */
export const listedLabels = new Set<string>([
  "httparse@1.10.1/lib.js: parseHeadersIterUninit",
  "itertools's probe/lib.js: stepBuffering",
  "itertools@0.14.0/tuple_impl.js: tuple12TupleCollect_collectFromIter",
  "itertools@0.14.0/tuple_impl.js: tuple11TupleCollect_collectFromIter",
  "itertools@0.14.0/tuple_impl.js: tuple10TupleCollect_collectFromIter",
  "itertools@0.14.0/tuple_impl.js: tuple9TupleCollect_collectFromIter",
  "itertools@0.14.0/tuple_impl.js: tuple8TupleCollect_collectFromIter",
  "itertools@0.14.0/tuple_impl.js: tuple7TupleCollect_collectFromIter",
  "itertools@0.14.0/tuple_impl.js: tuple6TupleCollect_collectFromIter",
  "itertools@0.14.0/tuple_impl.js: tuple5TupleCollect_collectFromIter",
  "itertools@0.14.0/tuple_impl.js: tuple4TupleCollect_collectFromIter",
  "itertools@0.14.0/tuple_impl.js: tuple3TupleCollect_collectFromIter",
  "itertools@0.14.0/tuple_impl.js: tuple2TupleCollect_collectFromIter",
  "miniz_oxide@0.9.1/inflate/core.js: initTree",
  "rustls-pki-types@1.15.1/server_name.js: validate",
  "uuid@1.29.0/error.js: intoErr",
]);

/** What's wrong of the crate corpus's labels MIR made, `labeled`, against
 * `listed`: each not listed, and each listed that's gone, of a crate
 * `measured` this run, as one not chosen says nothing (#2215). */
export function labelProblems(labeled: ReadonlySet<string>, listed: ReadonlySet<string>, measured: (crate: string) => boolean): string[] {
  const problems: string[] = [];
  for (const label of labeled) {
    if (!listed.has(label)) problems.push(`${label}: a label MIR made, \`bb..:\`, which no Rust wrote: structure it away, or list it in scripts/crate-corpus.ts's \`listedLabels\` under #2215`);
  }
  for (const label of listed) {
    if (measured(label.split(/@|'s probe/)[0]!) && !labeled.has(label)) problems.push(`${label}: listed in \`listedLabels\`, and gone: take it off the list`);
  }
  return problems;
}

/** Of the JS under `dir`, a crate's, `id`'s, each label MIR made, by file,
 * and each file that names this machine's `$CARGO_HOME` (`namesCargoHome`). */
function readUnder(id: string, dir: string): { labels: string[]; homes: string[] } {
  if (!existsSync(dir)) return { labels: [], homes: [] };
  const files = [...new Bun.Glob("**/*.js").scanSync(dir)].sort().map((file) => ({ file, text: readFileSync(join(dir, file), "utf8") }));
  return {
    labels: files.flatMap(({ file, text }) => mirLabels(text, `${id}/${file}`)),
    homes: files.filter(({ text }) => namesCargoHome(text)).map(({ file }) => `${id}/${file}`),
  };
}

/** Whether `text`, a crate's JS, names `home`, this machine's `$CARGO_HOME`,
 * where Cargo keeps what it fetched: rust-js names each file of a fetched
 * crate under `/cargo/`, whatever the machine (ADR 0262's difference 63),
 * where a crate's own panic locations named the builder's home in an app's
 * JS (#2435). `--check` fails of any. */
export function namesCargoHome(text: string, home = cargoHome): boolean {
  return text.includes(`${home}/`);
}

/** The functions of crates' JS a snapshot holds, each `<crate>@<version>/
 * <file>: <function>`, a top-level declaration of that file: what a change
 * of the compiler makes of a real crate's code where no corpus case makes
 * it, as itertools' `putBack` keeps an unread drop flag under a mutant no
 * copy of it in the corpus reproduces (#1979). `--check` cuts each from
 * the crate's JS and holds it to `snapshots/<crate>@<version>/<file>`,
 * which `BLESS=1` writes: a crate's whole file is too large to hold,
 * itertools' adaptors.js 44,000 lines, and it moves with most changes. */
export const snapshotted: readonly string[] = [
  "itertools@0.14.0/adaptors.js: putBack",
  "itertools@0.14.0/adaptors.js: interleave",
  "itertools@0.14.0/adaptors.js: whileSomeIterator_next",
  "itertools@0.14.0/diff.js: diffClone_clone",
  "itertools@0.14.0/ziptuple.js: zipTuple2DoubleEndedIterator_nextBack",
];

/** Where the snapshots of `snapshotted` are, a file each of a crate's. */
export const snapshotsDir = join(import.meta.dir, "crate-corpus", "snapshots");

/** The top-level declaration `name` of `text`, a crate's JS, with the
 * comments just above it, to its closing line at the first column; or
 * none. */
export function declarationOf(text: string, name: string): string | undefined {
  const lines = text.split("\n");
  const opens = new RegExp(`^(?:export )?(?:async )?(?:function\\*? |class |const |let )${name.replace(/\$/g, "\\$")}\\b`);
  const at = lines.findIndex((line) => opens.test(line));
  if (at === -1) return undefined;
  let from = at;
  while (from > 0 && /^(?:\/\*\*|\/\/| \*)/.test(lines[from - 1]!)) from--;
  let to = at;
  if (!/[;}]$/.test(lines[at]!)) while (to + 1 < lines.length && !/^[}\]]/.test(lines[to]!)) to++;
  return lines.slice(from, to + 1).join("\n");
}

/** What a snapshot of `crate`'s `file`, the functions `names`, says: a
 * header naming them and how it's written, then each, cut from `text`. */
function snapshotOf(crate: string, file: string, names: string[], text: string): { snapshot: string; missing: string[] } {
  const missing: string[] = [];
  const cut = names.map((name) => declarationOf(text, name) ?? (missing.push(name), ""));
  const header = `// ${crate}'s ${file}: ${names.join(", ")}, as scripts/crate-corpus.ts's \`snapshotted\` lists them.\n// \`BLESS=1 bun scripts/crate-corpus.ts --check ${crate.split("@")[0]}\` writes it.\n`;
  return { snapshot: `${header}\n${cut.join("\n\n")}\n`, missing };
}

/** What's wrong of `snapshotted`'s snapshots, of each crate `measured` this
 * run, its JS where `compiledAt` says: a function listed that its file
 * doesn't declare, a snapshot that differs, and a snapshot no entry lists.
 * With `bless`, each that differs or isn't listed is written or removed
 * instead. */
export function snapshotProblems(compiledAt: (crate: string) => string | undefined, measured: (crate: string) => boolean, { bless = false, listed = snapshotted, dir = snapshotsDir }: { bless?: boolean; listed?: readonly string[]; dir?: string } = {}): string[] {
  const problems: string[] = [];
  const files = new Map<string, string[]>();
  for (const entry of listed) {
    const [path, name] = entry.split(": ") as [string, string];
    files.set(path, [...(files.get(path) ?? []), name]);
  }
  for (const [path, names] of files) {
    const crate = path.split("/")[0]!;
    const js = compiledAt(crate);
    if (!js) {
      if (measured(crate.split("@")[0]!)) problems.push(`${path}: snapshotted, and ${crate} wasn't compiled: a row's crate at another release names its entries anew in \`snapshotted\``);
      continue;
    }
    const file = path.slice(crate.length + 1);
    const source = join(js, file);
    if (!existsSync(source)) {
      problems.push(`${path}: snapshotted, and ${crate}'s JS has no ${file}, in ${js}: take its entries off \`snapshotted\`, or name the file it's in now`);
      continue;
    }
    const { snapshot, missing } = snapshotOf(crate, file, names, readFileSync(source, "utf8"));
    for (const name of missing) problems.push(`${path}: ${name}: snapshotted, and ${file} declares no \`${name}\`: take it off \`snapshotted\``);
    const kept = join(dir, path);
    const was = existsSync(kept) ? readFileSync(kept, "utf8") : undefined;
    if (was === snapshot) continue;
    if (bless) {
      mkdirSync(dirname(kept), { recursive: true });
      writeFileSync(kept, snapshot);
    } else {
      const [a, b] = [(was ?? "").split("\n"), snapshot.split("\n")];
      const at = (a.length >= b.length ? a : b).findIndex((_, i) => a[i] !== b[i]);
      const where = was === undefined ? ", which isn't written" : `, first at its line ${at + 1}: ${JSON.stringify(a[at] ?? "")}, now ${JSON.stringify(b[at] ?? "")}`;
      problems.push(`${path}: its JS isn't scripts/crate-corpus/snapshots/${path}${where}: \`BLESS=1 bun scripts/crate-corpus.ts --check ${crate.split("@")[0]}\` writes it; read what it moved`);
    }
  }
  const unlisted = existsSync(dir) ? [...new Bun.Glob("*/**/*.js").scanSync(dir)].filter((path) => !files.has(path) && measured(path.split("@")[0]!)) : [];
  for (const path of unlisted) {
    if (bless) rmSync(join(dir, path));
    else problems.push(`scripts/crate-corpus/snapshots/${path}: no entry of \`snapshotted\` lists it: remove it`);
  }
  return problems;
}

/** What a crate measured does, its row's verdict: `runs` where it compiles
 * and its probe's JS reports what native Rust does, else whether it
 * compiles. `--check` holds it to the row's bar, and `--retire` a mutant's
 * to the compiler's as it is (scripts/mutations.ts). */
export function verdictOf(result: Result): Verdict {
  return result.own === "compiles" && result.runs === "same" ? "runs" : result.own;
}

/** Where a run writes what it measured of each crate, in its target. */
export const resultsFile = join(ownTarget, "crate-corpus.json");

/** `text` written to `path`, unless it's there already: a file written
 * again has a new date, which Cargo takes for a change (#998). */
export function written(path: string, text: string) {
  if (existsSync(path) && readFileSync(path, "utf8") === text) return;
  writeFileSync(path, text);
}

/** `spawnSync`'s result, without blocking the other crates measured at once. */
function spawned(cmd: string, args: string[], options: { cwd: string; env?: NodeJS.ProcessEnv }): Promise<{ status: number | null; stdout: string; stderr: string }> {
  return new Promise((done) => {
    const child = spawn(cmd, args, { ...options, stdio: ["ignore", "pipe", "pipe"] });
    const [out, err]: [Buffer[], Buffer[]] = [[], []];
    child.stdout.on("data", (chunk: Buffer) => out.push(chunk));
    child.stderr.on("data", (chunk: Buffer) => err.push(chunk));
    child.on("close", (status) => done({ status, stdout: Buffer.concat(out).toString("utf8"), stderr: Buffer.concat(err).toString("utf8") }));
  });
}

/** The first line where the JS's report isn't native Rust's. */
function difference(native: string, js: string): string {
  const [a, b] = [native.split("\n"), js.split("\n")];
  const at = a.findIndex((line, i) => line !== b[i]);
  const line = at === -1 ? a.length : at;
  return `line ${line + 1}: native ${JSON.stringify(a[line] ?? "")}, JS ${JSON.stringify(b[line] ?? "")}`;
}

async function main() {
  const root = resolve(import.meta.dir, "..");
  const compiler = resolve(process.env.RUST_JS_COMPILER ?? join(root, "target/debug/rust-js"));
  const toolchain = channel;
  const corpus = corpusRows(readFileSync(join(root, "docs/crate-corpus.md"), "utf8"));
  if (!Array.isArray(corpus)) {
    console.error(corpus.problem);
    process.exit(1);
  }
  const check = process.argv.includes("--check");
  const lock = process.argv.includes("--lock");
  // A push's check: every row but a sampled table's, and of those the
  // ones its commit picks (#1174).
  const sample = process.argv.includes("--sample");
  const wanted = process.argv.slice(2).filter((arg) => arg !== "--check" && arg !== "--lock" && arg !== "--sample");
  refuseUnmatched(wanted, new Set(corpus.map((c) => c.name)), { list: `the crates named, ${wanted.join(" ")},`, what: "rows of docs/crate-corpus.md" });
  // A graph is resolved anew only as asked, crate by crate, never the
  // whole corpus at once by forgetting to name one.
  if (lock && !wanted.length) {
    console.error("`--lock` resolves the graph of each crate named: name each, `bun scripts/crate-corpus.ts --lock url`");
    process.exit(1);
  }
  const seed = sample ? spawnSync("git", ["rev-parse", "HEAD"], { cwd: root, encoding: "utf8" }).stdout.trim() : "";
  if (sample && !seed) {
    console.error("`--sample` picks a push's rows by its commit, and `git rev-parse HEAD` gave none");
    process.exit(1);
  }
  const chosen = wanted.length ? corpus.filter((c) => wanted.includes(c.name)) : sample ? corpus.filter((c) => sampledIn(c, seed)) : corpus;
  if (sample) console.log(`${chosen.filter((c) => c.sampled).map((c) => c.name).join(", ") || "none"} of the sampled rows, one in ${sampledOneIn}, by ${seed.slice(0, 9)}`);
  const unlocked = lock ? [] : chosen.filter((c) => !existsSync(lockOf(c.name)));
  if (unlocked.length) {
    const names = unlocked.map((c) => c.name).join(" ");
    console.error(`${names}: no lock of its graph in scripts/crate-corpus/: \`bun scripts/crate-corpus.ts --lock ${names}\` writes it`);
    process.exit(1);
  }
  // Each crate measured apart, in a Cargo project and target of its own,
  // several at once: one at a time, a graph's one big crate held one core
  // while the rest sat idle. Each is printed in the table's order.
  const measure = async (crate: Crate): Promise<Result> => {
    const dir = join(crateProjects, crate.name);
    mkdirSync(dir, { recursive: true });
    // Measured now: what `boundCrateProjects` keeps longest.
    writeFileSync(join(dir, "used"), "");
    const probe = join(import.meta.dir, "crate-corpus", `${crate.name}.rs`);
    // Each file of the project written only where it differs: a file
    // written again is a new date, which Cargo takes for a change, so the
    // probe was compiled again by rust-js and natively, its binary linked
    // again, on every run, a change or none (#998).
    written(
      join(dir, "Cargo.toml"),
      `[package]\nname = "probe"\nversion = "0.0.0"\nedition = "2024"\n\n[lib]\npath = "lib.rs"\n\n[[bin]]\nname = "native"\npath = "main.rs"\n\n[dependencies]\n${dependency(crate)}\n\n[workspace]\n`,
    );
    written(join(dir, "lib.rs"), existsSync(probe) ? readFileSync(probe, "utf8") : "");
    written(join(dir, "main.rs"), "fn main() {\n    print!(\"{}\", probe::report());\n}\n");
    // Its graph at its lock's releases; or, `--lock`, at the newest each
    // requirement allows today, written as its lock.
    if (lock) {
      rmSync(join(dir, "Cargo.lock"), { force: true });
      const made = spawnSync("cargo", [`+${toolchain}`, "generate-lockfile"], { cwd: dir, encoding: "utf8" });
      if (made.status !== 0) {
        console.error(`${crate.name}: its graph doesn't resolve:\n${made.stderr.trim()}`);
        process.exit(1);
      }
    } else written(join(dir, "Cargo.lock"), readFileSync(lockOf(crate.name), "utf8"));
    // The patch an app's build writes, rust-js's own wasm-bindgen family
    // where the graph has one it serves, and its lock moved onto it, as
    // `rust-js build`'s `cargoBuild` writes them (ADR 1112): the graph the
    // row's crate has in an app.
    const unpatched = readFileSync(join(dir, "Cargo.lock"), "utf8");
    try {
      await cargoBuild({ manifestPath: join(dir, "Cargo.toml"), toolchain });
    } catch (error) {
      console.error(`${crate.name}: ${error instanceof Error ? error.message : String(error)}`);
      process.exit(1);
    }
    if (lock) copyFileSync(join(dir, "Cargo.lock"), lockOf(crate.name));
    else if (readFileSync(join(dir, "Cargo.lock"), "utf8") !== unpatched) {
      console.error(`${crate.name}: its lock, scripts/crate-corpus/${crate.name}.lock, isn't its graph with rust-js's own wasm-bindgen family patched: \`bun scripts/crate-corpus.ts --lock ${crate.name}\` resolves it again`);
      process.exit(1);
    }
    // Every crate the probe builds is rust-js's to compile, a registry's too.
    // Each a library, as a crate of crates.io is to the apps that use it.
    const metadata = spawnSync("cargo", [`+${toolchain}`, "metadata", "--format-version=1", "--locked"], { cwd: dir, maxBuffer: 256 * 1024 * 1024, encoding: "utf8" });
    // A row whose release or features moved since its lock was written:
    // Cargo's `--locked` won't change it.
    if (metadata.status !== 0) {
      const said = metadata.stderr.trim().split("\n").slice(-3).join("\n");
      console.error(`${crate.name}: its lock, scripts/crate-corpus/${crate.name}.lock, isn't its row's graph:\n${said}\n\`bun scripts/crate-corpus.ts --lock ${crate.name}\` resolves it again`);
      process.exit(1);
    }
    const packages = JSON.parse(metadata.stdout).packages as { name: string; version: string; manifest_path: string; metadata: { "rust-js"?: { bindings?: boolean } } | null }[];
    // A binding crate, the js crate rust-js's own js-sys maps onto, is
    // rustc's, its metadata rust-js's to read (ADR 0414).
    const crates = packages.filter((pkg) => pkg.metadata?.["rust-js"]?.bindings !== true).map((pkg) => realpathSync(dirname(pkg.manifest_path)));
    const run = await spawned(
      "cargo",
      [`+${toolchain}`, "check", "--lib", "--keep-going", "--locked", "--message-format=json", "--target", "wasm32-unknown-unknown"],
      { cwd: dir, env: cargoEnv(compiler, undefined, { apps: [], crates }) },
    );
    const compiled: string[] = [];
    const refused = new Map<string, string>();
    const labels = new Set<string>();
    const js: Record<string, string> = {};
    const homes = new Set<string>();
    // The probe's JS: its manifest is the first line of the `.rust-js` beside
    // its metadata, and its crate root's JS beside that.
    let probeJs: string | undefined;
    for (const line of run.stdout.split("\n").filter((l) => l.startsWith("{"))) {
      const message = JSON.parse(line);
      // `registry+https://..#name@1.2.3`, or `path+file://..#probe@0.0.0`.
      const id = String(message.package_id ?? "").split("#").pop() ?? "";
      if (message.reason === "compiler-artifact" && message.target?.kind?.some((k: string) => k === "lib" || k === "rlib")) {
        compiled.push(id);
        const marker = message.filenames?.find((f: string) => f.endsWith(".rmeta"))?.replace(/\.rmeta$/, ".rust-js");
        // Its JS, beside its manifest, the marker's first line.
        // A probe, each crate's own, by the crate it probes.
        const owner = id.startsWith("probe@") ? `${crate.name}'s probe` : id;
        if (marker && existsSync(marker)) {
          const dir = dirname(readFileSync(marker, "utf8").split("\n")[0]!);
          if (!id.startsWith("probe@")) js[id] = dir;
          const read = readUnder(owner, dir);
          for (const label of read.labels) labels.add(label);
          for (const file of read.homes) homes.add(file);
        }
        if (id.startsWith("probe@") && marker && existsSync(marker)) {
          probeJs = join(dirname(readFileSync(marker, "utf8").split("\n")[0]), "lib.js");
        }
      }
      if (message.reason === "compiler-message" && String(message.message?.level).startsWith("error") && !refused.has(id)) {
        const text = String(message.message.message);
        if (!text.startsWith("aborting due to")) refused.set(id, text);
      }
    }
    const own = (id: string) => id.startsWith(`${crate.name}@`);
    // The probe is the corpus's, not the crate's graph.
    const probeRefused = [...refused].find(([id]) => id.startsWith("probe@"))?.[1];
    const result: Result = {
      name: crate.name,
      version: crate.version,
      compiled: compiled.filter((id) => !id.startsWith("probe@")),
      refused: [...refused].filter(([id]) => !id.startsWith("probe@")).map(([c, error]) => ({ crate: c, error })),
      crashed: crashes(run.stderr)
        .filter(({ name }) => name !== "probe")
        .map(({ name, how }) => {
          const pkg = packages.find((p) => p.name === name);
          return { crate: pkg ? `${name}@${pkg.version}` : name, error: `rust-js crashed: ${how}` };
        }),
      own: compiled.some(own) ? "compiles" : refused.size && [...refused.keys()].some(own) ? "refused" : "blocked",
      labels: [...labels].sort(),
      homes: [...homes].sort(),
      js,
    };
    if (result.own !== "compiles" && !result.refused.length && !result.crashed.length) {
      result.unexplained = run.stderr.trim().split("\n").slice(-12).join("\n");
    }
    if (result.own === "compiles" && existsSync(probe)) {
      if (probeRefused !== undefined) result.runs = `the probe is refused: ${probeRefused}`;
      else if (!probeJs) result.runs = "the probe wasn't compiled";
      else {
        // Native Rust's report, built for the host, apart from rust-js's;
        // each stopped, with what it started, past its time (#1108's orphan).
        const [native, js] = await Promise.all([
          runChild(["cargo", `+${toolchain}`, "run", "--quiet", "--locked", "--bin", "native", "--target-dir", join(dir, "native")], dir, 600_000),
          runChild([process.execPath, "-e", `import { report } from ${JSON.stringify(probeJs)};\nprocess.stdout.write(report());`], dir, 120_000),
        ]);
        const failed = (p: Exit, limit: number) => stopped(p, limit) ?? (p.code !== 0 ? `exited ${p.code}` : undefined);
        if (failed(native, 600_000)) result.runs = `native Rust failed (${failed(native, 600_000)}): ${native.stderr.trim().split("\n").slice(-3).join(" ")}`;
        else if (failed(js, 120_000)) result.runs = `the JS failed (${failed(js, 120_000)}): ${js.stderr.trim().split("\n").slice(-6).join(" ")}`;
        else result.runs = native.stdout === js.stdout ? "same" : `differs at ${difference(native.stdout, js.stdout)}`;
      }
    }
    return result;
  };
  // As many crates at once as the cores and memory hold, each `cargo
  // check` of its graph keeping about two busy (scripts/shard.ts' `atOnce`).
  const width = atOnce("crateCheck");
  const measured: Promise<Result>[] = [];
  let next = 0;
  const workers = Array.from({ length: Math.min(width, chosen.length) }, async () => {
    while (next < chosen.length) {
      const at = next++;
      measured[at] = measure(chosen[at]);
      await measured[at];
    }
  });
  await Promise.all(workers);
  const results: Result[] = await Promise.all(measured);
  for (const result of results) {
    const crate = chosen.find((c) => c.name === result.name)!;
    const crashed = result.crashed.length ? `, ${result.crashed.length} crashed` : "";
    console.log(`${crate.name}@${crate.version}: ${result.own}, ${result.compiled.length} compiled, ${result.refused.length} refused${crashed}`);
    for (const { crate: c, error } of [...result.refused, ...result.crashed]) console.log(`  ${c}: ${error}`);
    if (result.unexplained) console.log(result.unexplained.replace(/^/gm, "  | "));
    if (result.runs) console.log(`  runs: ${result.runs}`);
  }

  writeFileSync(resultsFile, JSON.stringify(results, null, 2));

  // `--check`, the nightly's: what docs/crate-corpus.md records of each
  // crate is the bar, a crate doing worse failing the run, as chrono and
  // rust_decimal went from runs to refused unseen (#467, #468). A row
  // marked with an open issue, `runs (refused: #467)`, holds while the
  // crate does just that, and fails once it does better, so the mark goes
  // with the issue.
  if (check) {
    const rank: Record<Verdict, number> = { runs: 3, compiles: 2, refused: 1, blocked: 1 };
    const failed: string[] = [];
    for (const r of results) {
      const row = chosen.find((c) => c.name === r.name)!;
      const got = verdictOf(r);
      if (row.open && got === row.open.now) console.log(`${r.name}: ${got}, as ${row.open.issue} says, where it ${row.bar}`);
      else if (row.open && rank[got] > rank[row.open.now]) failed.push(`${r.name}: ${got}, better than ${row.open.issue} says: write \`${got}\` in its row`);
      else if (rank[got] < rank[row.open?.now ?? row.bar]) failed.push(`${r.name}: ${got}, where docs/crate-corpus.md says ${row.open?.now ?? row.bar}`);
    }
    // The labels MIR made, of every crate's JS, one set: a crate several
    // graphs compile, num-traits, counted once.
    const measured = (crate: string) => results.some((r) => r.name === crate || r.compiled.some((id) => id.startsWith(`${crate}@`)));
    failed.push(...labelProblems(new Set(results.flatMap((r) => r.labels)), listedLabels, measured));
    for (const file of new Set(results.flatMap((r) => r.homes))) failed.push(`${file}: its JS names this machine's \`$CARGO_HOME\`, ${cargoHome}, where a fetched crate's files are named under \`/cargo/\` (#2435)`);
    // Each snapshotted crate's functions, of the first graph that compiled
    // it, against their snapshots; `BLESS=1` writes them.
    const compiledAt = (crate: string) => results.map((r) => r.js[crate]).find((dir) => dir !== undefined);
    failed.push(...snapshotProblems(compiledAt, measured, { bless: !!process.env.BLESS }));
    for (const line of failed) console.error(line);
    if (failed.length) process.exitCode = 1;
  }
  // Bounded last, as the check reads the snapshotted crates' JS where its
  // projects keep it: bounded first, it read none of a crate it removed.
  boundCrateProjects();
}

if (import.meta.main) await main();
