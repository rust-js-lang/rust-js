// Mutations of the compiler, each a bug it had, or one a rule of it keeps
// out, put back: the tests named for each must fail against a compiler
// built with it, and pass against the compiler as it is (ADR 0093). A
// mutation that no longer applies, doesn't build, or that its tests don't
// catch fails the run, so the tests are shown to see what they're for.
//
//   bun scripts/mutations.ts                  # every mutation
//   bun scripts/mutations.ts copy-on-read ..  # the ones named
//   bun scripts/mutations.ts --changed=main   # a change's, since where it left main
//   bun scripts/mutations.ts --shard=2/6      # a sixth of them, as CI splits them
//   bun scripts/mutations.ts --batch=1        # each built alone, not 32 to a build
//   bun scripts/mutations.ts --typecheck=origin/main  # a change's files' mutants type-checked, none run
//
// The mutations themselves are in `mutations/`, one list for each source
// file. Each builds natively a few test programs: on macOS, with the app
// that runs them under Developer Tools, or the first run of each is slow
// (AGENTS.md).

import { createHash } from "node:crypto";
import { chmodSync, cpSync, existsSync, linkSync, mkdirSync, mkdtempSync, readdirSync, readFileSync, rmSync, statSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { basename, dirname, join, resolve } from "node:path";

import { run, runSync, stopped, type Exit } from "../test/child";
import { corpusHarness, corpusMutantCases, corpusMutants, mutantOf } from "../test/corpus-mutants";
import { installPackages } from "../test/packages-of";
import { caseParts, compilerBuilds, drivesBrowser, machine, memoryBudget, shardArg, timedShard } from "./shard";
import { casesReaching, ensureMap, noMap } from "./testMap";
import { elapsed, shown, timed } from "./timing";
import { passed, summary } from "./test-summary";
import { acquire, holdLease, ownShare } from "./leases";
import { makeSourceWorkspace, sourceMutation } from "./source-mutations";
import { latentSurvivors, type Results } from "./latent-survivors";
import { binaryHash, builtBinary, freshShare, unapplied, verdictKind } from "./mutant-builds";
import { resultsFile, verdictOf, type Result as CrateResult, type Verdict } from "./crate-corpus";
import { devTimes, ownsTimes } from "./devTimes";
import { floorBytes } from "../test/bounded";
import { freeBytes } from "../tooling/bounded.js";

const root = join(import.meta.dir, "..");

export type Mutation = {
  name: string;
  /** What it breaks, as Rust would see it. */
  breaks: string;
  file: string;
  find: string;
  replace: string;
  /** `bun test` arguments whose tests must catch it. */
  tests?: string[];
  /** Or, where no program the compiler is given reaches its code, the
   * `cargo test` filter whose unit tests must: a test's path, or its
   * module's, from the crate's root, `lower::mir::tidy::jumps::tests::x`.
   * A mutation names this or `tests`, not both (#915). */
  unitTests?: string;
  /** A non-crate source generator prerequisite, charged as a build. */
  prepare?: string[];
  /** Its tests check the corpus's JS snapshots too: it changes only how
   * the JS reads, as one that takes away the readable form of something
   * a more general one also does right, and how it reads is what's checked. */
  snapshots?: boolean;
  /** What its tests run with besides, the iterator matrix's row that
   * catches it, `ITERATOR_MATRIX`'s (#1971). */
  env?: Record<string, string>;
};

// Mutations are grouped by their repository source file. Each list's
// explicit Mutation.file owns the path; a catalog filename is not inferred.
export const lists: { path: string; mutations: Mutation[] }[] = [];
for (const path of [...new Bun.Glob("**/*.ts").scanSync(join(import.meta.dir, "mutations"))].sort()) {
  lists.push({ path, mutations: (await import(`./mutations/${path}`)).mutations });
}
export const mutations: Mutation[] = lists.flatMap((list) => list.mutations);

/** The code whose job is how the JS reads, docs/architecture.md's
 * "Readability and printing" and "Body readability passes", as globs of
 * the repository's paths: a mutation of it that its tests miss is judged
 * by the corpus's snapshots too (#864). test/mutations.test.ts holds it to
 * the architecture. */
export const readabilityOwners = [
  "src/prepare.rs",
  "src/prepare/**",
  "src/to_oxc.rs",
  "src/to_oxc/**",
  "src/format.rs",
  "src/format/**",
  "src/typescript.rs",
  "src/prettier.rs",
  "src/node_session.rs",
  "src/lower/mir/tidy.rs",
  "src/lower/mir/tidy/**",
];

/** Whether `file` is of `readabilityOwners`. */
export const ofReadabilityOwner = (file: string) => readabilityOwners.some((glob) => new Bun.Glob(glob).match(file));

/** The lines of `source`, from 1, that `mutation`'s `find` is, where it's
 * there once. */
/** Whether a run builds the compiler as it is itself: of a mutation of the
 * crate, or of the corpus's harness, as each mutant is built beside it; or
 * where none is given, a run of the scripts' mutations alone on a machine
 * that built none, as CI's mutation shards are (check 43872319768 failed
 * for the attribution's, "build the read-only compiler"). */
export function buildsUnmutated({ crate, harness, given }: { crate: boolean; harness: boolean; given: boolean }): boolean {
  return crate || harness || !given;
}

export function siteLines(mutation: Mutation, source: string): [number, number] | undefined {
  const at = source.indexOf(mutation.find);
  if (at < 0 || source.indexOf(mutation.find, at + 1) >= 0) return undefined;
  const first = source.slice(0, at).split("\n").length;
  return [first, first + mutation.find.replace(/\n$/, "").split("\n").length - 1];
}

/** A file a change touches, and the lines of it, as it is now, that it
 * changed: each `[first, last]`, from 1. */
/** Lines a change makes in a file as it is now, or, `base`, takes from it
 * as it was: a test renamed, moved or deleted is where it was. */
export type Change = { file: string; lines: [number, number][]; base?: boolean };

/** A change's mutations (DEVELOPMENT.md, ADR 0093): those it adds or edits,
 * of `base`'s; those of a function it changes, the function the mutated
 * code is in; and those whose tests it changes, a test they name as it is
 * or as it was, renamed or moved away, or a corpus case. What a change
 * elsewhere does to them, as making the code one guards redundant, the
 * nightly run of all of them finds. `read` gives a file as it is now, and
 * `readBase` as it was. */
export function changedMutations(
  now: Mutation[],
  base: Mutation[],
  changes: Change[],
  read: (file: string) => string | undefined,
  readBase: (file: string) => string | undefined,
): Mutation[] {
  const own = new Set(ownMutations(now, base));
  const changed = new Map(changes.filter((c) => !c.base).map((c) => [c.file, c.lines]));
  const taken = new Map(changes.filter((c) => c.base).map((c) => [c.file, c.lines]));
  const overlaps = (lines: [number, number][], [first, last]: [number, number]) => lines.some(([a, b]) => a <= last && b >= first);
  // A test file's tests the change touches, by name: all of them where it
  // changes what they share, above the first. A corpus case is its own test.
  const touchedIn = (lines: [number, number][] | undefined, source: string | undefined): string[] | "all" => {
    if (!lines || source === undefined) return [];
    const blocks = testBlocks(source);
    if (blocks.length === 0 || overlaps(lines, [1, blocks[0].first - 1])) return "all";
    return blocks.filter((block) => overlaps(lines, [block.first, block.last])).map((block) => block.name);
  };
  // Each file's once, and read only where the change touches it: a `git
  // show` for each of 5,000 mutations was 25 s of a run's choosing (#923).
  const touched = new Map<string, string[] | "all">();
  const touchedTests = (file: string): string[] | "all" => {
    // The corpus's tests are its cases, each its own file; what they share
    // is its harness, whose own mutants show it still catches (#918).
    if (file === "test/corpus.test.ts") return [];
    const known = touched.get(file);
    if (known !== undefined) return known;
    const [lines, linesWas] = [changed.get(file), taken.get(file)];
    const [is, was] = [lines ? touchedIn(lines, read(file)) : [], linesWas ? touchedIn(linesWas, readBase(file)) : []];
    const tests = is === "all" || was === "all" ? "all" : [...is, ...was];
    touched.set(file, tests);
    return tests;
  };
  const cases = changes.filter((c) => c.file.startsWith("test/corpus/")).map((c) => c.file.slice("test/corpus/".length));
  // Whether the change touches the unit tests a `cargo test` filter runs:
  // the tests of a source file of its path, as it is or as it was.
  const within = (outer: string, path: string) => outer === "" || path === outer || path.startsWith(`${outer}::`);
  const unitTestsTouched = (filter: string) =>
    changes.some((c) => {
      const module = unitTestsModule(c.file);
      if (module === undefined || !(within(module, filter) || within(filter, module))) return false;
      const source = c.base ? readBase(c.file) : read(c.file);
      return source !== undefined && overlaps(c.lines, unitTestLines(c.file, source));
    });
  return now.filter((m) => {
    if (own.has(m)) return true;
    const source = changed.has(m.file) ? read(m.file) : undefined;
    if (source !== undefined) {
      const at = source.indexOf(m.find);
      // One that no longer applies runs, to say so.
      if (at < 0) return true;
      const first = source.slice(0, at).split("\n").length;
      const block = enclosingFunction(source, first, first + m.find.split("\n").length - 1);
      if (overlaps(changed.get(m.file)!, block)) return true;
    }
    if (m.unitTests !== undefined) return unitTestsTouched(m.unitTests);
    const [file, ...args] = m.tests!;
    const at = args.indexOf("-t");
    const filter = at >= 0 ? new RegExp(args[at + 1]) : undefined;
    const tests = touchedTests(file);
    if (tests === "all" || (tests.length > 0 && (!filter || tests.some((name) => filter.test(name))))) return true;
    return file === "test/corpus.test.ts" && filter !== undefined && cases.some((name) => filter.test(name));
  });
}

/** Those of `now` a change adds or edits: each one `base`, the lists as
 * they were, hasn't as it is. */
export function ownMutations(now: Mutation[], base: Mutation[]): Mutation[] {
  const was = new Set(base.map((m) => JSON.stringify(m)));
  return now.filter((m) => !was.has(JSON.stringify(m)));
}

/** The mutations a change since where this branch left `base` adds or
 * edits (`ownMutations`): check:fast's, of what isn't committed, `HEAD`. */
export async function ownSince(base: string): Promise<Mutation[]> {
  return ownMutations(mutations, (await since(base)).mutations);
}

/** Whether a change since where this branch left `base` touches the corpus
 * harness, `test/corpus.test.ts` and what it runs (`corpusHarness`): its
 * mutants run then, in place of every mutation that names a case (#918). */
export async function harnessChangedSince(base: string): Promise<boolean> {
  const { files } = await since(base);
  const harness = new Set(corpusHarness());
  return files.some((file) => harness.has(file));
}

/** The module of the crate's source file `file`, as a `cargo test` filter
 * names it from the crate's root, `src/lower/mir/tidy/jumps.rs`'s
 * `lower::mir::tidy::jumps`; the root's is `""`. Nothing for a file that
 * isn't the crate's Rust. */
export function unitTestsModule(file: string): string | undefined {
  if (!file.startsWith("src/") || !file.endsWith(".rs")) return undefined;
  const path = file.slice("src/".length, -".rs".length).replace(/(?:^|\/)mod$/, "");
  return path === "main" ? "" : path.split("/").join("::");
}

/** The lines of `source`, the crate's `file`, that its unit tests are: a
 * tests module's file whole, or from its first `#[cfg(test)]` to its end;
 * none where it has none. */
export function unitTestLines(file: string, source: string): [number, number] {
  const lines = source.split("\n");
  if (/(?:^|\/)tests(?:\.rs$|\/)/.test(file)) return [1, lines.length];
  const at = lines.findIndex((line) => line.trim() === "#[cfg(test)]");
  return at < 0 ? [0, 0] : [at + 1, lines.length];
}

/** The mutations, of `among`, a change since `base` runs (`--changed`). */
export async function changedSince(base: string, among: Mutation[] = mutations): Promise<Mutation[]> {
  const { from, changes, mutations: old } = await since(base);
  const read = (file: string) => {
    try {
      return readFileSync(join(root, file), "utf8");
    } catch {
      return undefined;
    }
  };
  const readBase = (file: string) => {
    const shown = runSync(["git", "show", `${from}:${file}`], root, 60_000);
    return shown.code === 0 ? shown.stdout : undefined;
  };
  return changedMutations(among, old, changes, read, readBase);
}

/** The lines of the function lines `first` to `last` are in, the innermost,
 * as rustfmt and Prettier lay one out: `fn` or `function` where it starts,
 * and `}` at its indent where it ends. Code in none is its own lines and
 * ten each side. */
export function enclosingFunction(source: string, first: number, last: number): [number, number] {
  const lines = source.split("\n");
  const start = /^(\s*)(?:(?:pub(?:\([^)]*\))?|export|async|const|unsafe|extern\s+"[^"]*"|default)\s+)*(?:fn|function)\s/;
  for (let i = first - 1; i >= 0; i--) {
    const m = lines[i].match(start);
    if (!m) continue;
    const trimmed = lines[i].trimEnd();
    // One of one line, `fn n() -> u32 { 1 }`, or a declaration, `fn n();`.
    let end = trimmed.endsWith("}") || trimmed.endsWith(";") ? i : -1;
    for (let j = i + 1; end < 0 && j < lines.length; j++) if (lines[j] === `${m[1]}}`) end = j;
    if (end + 1 >= last) return [i + 1, end + 1];
  }
  return [Math.max(1, first - 10), last + 10];
}

/** A test file's tests, each its name and its lines: from the comment
 * above it to the one above the next. */
export function testBlocks(source: string): { name: string; first: number; last: number }[] {
  const lines = source.split("\n");
  const starts: { name: string; line: number }[] = [];
  lines.forEach((line, i) => {
    const m = line.match(/^test(?:\.(?:skipIf|if|todo)\([^)]*\))?\(\s*(["'`])(.*?)\1/);
    if (m) starts.push({ name: m[2], line: i });
  });
  const header = (line: number) => {
    let i = line;
    while (i > 0 && lines[i - 1].startsWith("//")) i--;
    return i;
  };
  return starts.map((s, k) => ({
    name: s.name,
    first: header(s.line) + 1,
    last: k + 1 < starts.length ? header(starts[k + 1].line) : lines.length,
  }));
}

/** A file not yet added to git, `file`: a change of all its lines. */
function untrackedChange(file: string): Change {
  let lines = 1;
  try {
    lines = Math.max(1, readFileSync(join(root, file), "utf8").split("\n").length);
  } catch {}
  return { file, lines: [[1, lines]] };
}

/** The lines `git diff -U0` says each file's change is in, as it is now,
 * and, `base`, as it was. */
export function changedLines(diff: string): Change[] {
  const changes: Change[] = [];
  let was: Change | undefined;
  let is: Change | undefined;
  // Lines none of are on one side are between two there: both are where
  // the change is.
  const span = (from: number, count: number): [number, number] => (count === 0 ? [from, from + 1] : [from, from + count - 1]);
  for (const line of diff.split("\n")) {
    if (line.startsWith("--- ")) {
      was = line === "--- /dev/null" ? undefined : { file: line.slice("--- a/".length), lines: [], base: true };
      if (was) changes.push(was);
      continue;
    }
    if (line.startsWith("+++ ")) {
      is = line === "+++ /dev/null" ? undefined : { file: line.slice("+++ b/".length), lines: [] };
      if (is) changes.push(is);
      continue;
    }
    const hunk = line.match(/^@@ -(\d+)(?:,(\d+))? \+(\d+)(?:,(\d+))? @@/);
    if (!hunk) continue;
    const count = (n: string | undefined) => (n === undefined ? 1 : Number(n));
    was?.lines.push(span(Number(hunk[1]), count(hunk[2])));
    is?.lines.push(span(Number(hunk[3]), count(hunk[4])));
  }
  return changes;
}

/** The files changed since where this branch left `base`, and the
 * mutations as they were there. */
async function since(base: string): Promise<{ from: string; files: string[]; changes: Change[]; mutations: Mutation[] }> {
  const git = (args: string[]) => {
    const p = runSync(["git", ...args], root, 60_000);
    if (p.code !== 0) throw new Error(`git ${args.join(" ")} failed:\n${p.stderr}`);
    return p.stdout;
  };
  const from = git(["merge-base", base, "HEAD"]).trim();
  // And each file not yet added, whole: a new source file, its list or a
  // new corpus case, which `git diff` doesn't see (#923).
  const added = git(["ls-files", "--others", "--exclude-standard"]).split("\n").filter(Boolean);
  const files = [...git(["diff", "--name-only", from]).split("\n").filter(Boolean), ...added];
  const changes = [...changedLines(git(["diff", "-U0", "--no-color", from])), ...added.map(untrackedChange)];
  // Each list as it was there, where the change touched it; a new one had none.
  const there = mkdtempSync(join(tmpdir(), "mutations-base-"));
  const old: Mutation[] = [];
  const copies: string[] = [];
  for (const { path, mutations: listed } of lists) {
    const file = `scripts/mutations/${path}`;
    if (!files.includes(file)) {
      old.push(...listed);
      continue;
    }
    const shown = runSync(["git", "show", `${from}:${file}`], root, 60_000);
    if (shown.code !== 0) continue;
    const copy = join(there, path);
    mkdirSync(dirname(copy), { recursive: true });
    writeFileSync(copy, shown.stdout);
    copies.push(copy);
  }
  // Each read once all are written: Bun keeps what a directory held when it
  // first imports from it, so one written after isn't found.
  for (const copy of copies) old.push(...(await import(copy)).mutations);
  rmSync(there, { recursive: true, force: true });
  return { from, files, changes, mutations: old };
}

// Where the mutated crate is built, and the compilers kept: one copy of
// the crate, remade for each mutation, and one target, so only rust-js is
// built again.
const work = join(root, "target", "mutants");
// And `std/`, whose manifest the compiler builds in (ADR 0561), and
// `vendor/`, the crates Cargo.toml patches in by path (ADR 1121); and each
// file the crate includes from beyond them, its unit tests' table of std's
// ways to collect a `Vec` (#1239) and the contract's page its table of
// facts is held to (#1671 change 4 M4), which test/mutations.test.ts holds
// to what `src/` includes.
export const crateFiles = [
  "Cargo.toml",
  "Cargo.lock",
  "build.rs",
  "rust-toolchain.toml",
  "src",
  "std",
  "vendor",
  "test/vec-collect-ways.txt",
  "docs/library-contract.md",
];
const buildTimeout = 20 * 60_000;
const testTimeout = 20 * 60_000;

/** Where a run's builds read the crate from: the checkout until the run
 * takes its snapshot, `snapshotCrate`, then that, so an edit to `src/`
 * while a run goes on is the next run's, never half of this one's. */
let crateSource = root;

/** The crate's files as they are now, `crateFiles` of `from`, the
 * checkout, into `to`, writing only what differs, and every build and
 * mutation's site of the run read from there. */
export function snapshotCrate(to: string, from = root): string[] {
  const touched = syncTree(from, to, crateFiles);
  crateSource = to;
  return touched;
}

/** A file of the crate as the run reads it: its snapshot's, once taken. */
export function crateText(file: string): string {
  return readFileSync(join(crateSource, file), "utf8");
}

/** How a test run ended, to say of one that reported nothing: why it was
 * stopped, or its exit code (#1368). */
function endedAs(exit: Exit): string {
  return stopped(exit, testTimeout) ?? `exited ${exit.code}`;
}

/** The source as it is, with `mutation` in it, or why it can't be. */
export function mutate(source: string, mutation: Mutation): string | { problem: string } {
  const at = source.indexOf(mutation.find);
  if (at < 0) return { problem: `doesn't apply: ${mutation.file} has no \`${mutation.find.split("\n")[0].trim()}\`` };
  if (source.indexOf(mutation.find, at + 1) >= 0) return { problem: `applies more than once in ${mutation.file}` };
  return source.slice(0, at) + mutation.replace + source.slice(at + mutation.find.length);
}

/** Rust's delimiters in `source`, each `(`, `[`, `{`, `)`, `]` or `}` and
 * where it is, past its comments, strings and chars: what rustc's lexer
 * matches before it parses anything. */
export function delimiters(source: string): { char: string; at: number }[] {
  const found: { char: string; at: number }[] = [];
  // From each token that matters to the next, past what's between.
  const next = /\/\/|\/\*|[bc]?r#*"|"|'|[()[\]{}]/g;
  const char = /'(?:\\(?:x[0-9a-fA-F]{2}|u\{[0-9a-fA-F]{1,6}\}|.)|[^\\'\n])'/uy;
  for (let m = next.exec(source); m; m = next.exec(source)) {
    const [token] = m;
    const i = m.index;
    if (token === "//") {
      const end = source.indexOf("\n", i);
      next.lastIndex = end < 0 ? source.length : end;
    } else if (token === "/*") {
      // Block comments nest.
      let depth = 1;
      let at = i + 2;
      while (at < source.length && depth > 0) {
        if (source.startsWith("/*", at)) (depth++, (at += 2));
        else if (source.startsWith("*/", at)) (depth--, (at += 2));
        else at++;
      }
      next.lastIndex = at;
    } else if (token.endsWith('"') && token !== '"') {
      // A raw string, where its `r` starts no identifier's tail.
      if (/\w/.test(source[i - 1] ?? "")) {
        next.lastIndex = i + token.length - 1;
        continue;
      }
      const hashes = token.slice(token.indexOf("r") + 1, -1);
      const end = source.indexOf(`"${hashes}`, i + token.length);
      next.lastIndex = end < 0 ? source.length : end + 1 + hashes.length;
    } else if (token === '"') {
      let at = i + 1;
      while (at < source.length && source[at] !== '"') at += source[at] === "\\" ? 2 : 1;
      next.lastIndex = at + 1;
    } else if (token === "'") {
      // A char, or a lifetime's or a label's quote.
      char.lastIndex = i;
      if (char.test(source)) next.lastIndex = char.lastIndex;
    } else found.push({ char: token, at: i });
  }
  return found;
}

/** Why `source`'s delimiters don't match, as rustc's lexer would say before
 * it parses anything, or nothing where they do: a mutation's build that
 * doesn't is no build of what it says it is (#479). */
export function unmatched(source: string, firstLine = 1): string | undefined {
  const open: { char: string; at: number }[] = [];
  const line = (at: number) => firstLine - 1 + source.slice(0, at).split("\n").length;
  for (const d of delimiters(source)) {
    if ("([{".includes(d.char)) open.push(d);
    else if (open.pop()?.char !== { ")": "(", "]": "[", "}": "{" }[d.char]) return `unexpected closing delimiter \`${d.char}\` on line ${line(d.at)}`;
  }
  return open.length > 0 ? `unclosed delimiter \`${open[open.length - 1].char}\` on line ${line(open[open.length - 1].at)}` : undefined;
}

/** Where a mutation goes in a build of many (ADR 1031): the text of the
 * crate's `file` from `start` to `end`, a function's body or the
 * `include_str!` of a runtime helper, which that build has once for each of
 * its mutations and once as it is, each chosen by name as it runs. */
export type Site = { file: string; start: number; end: number };

/** A mutation, and where it goes in a build of many, if it can be in one. */
export type Sited = { mutation: Mutation; site?: Site };

/** Where `mutation` goes in a build of many, or nowhere: one outside a
 * function's body is built alone, as is one in a body that can't choose as
 * it runs, a `const fn`'s, which can't ask, or one returning `impl Trait`,
 * whose two bodies are two types. `read` gives a file of the crate. */
export function site(mutation: Mutation, read: (file: string) => string): Site | undefined {
  const source = read(mutation.file);
  const at = source.indexOf(mutation.find);
  if (at < 0 || source.indexOf(mutation.find, at + 1) >= 0) return undefined;
  // A helper's JS is the compiler's by its one `include_str!`.
  if (mutation.file.startsWith("src/runtime/") && mutation.file.endsWith(".js")) {
    const call = `include_str!("runtime/${mutation.file.slice("src/runtime/".length)}")`;
    const including = read("src/runtime.rs");
    const start = including.indexOf(call);
    if (start < 0 || including.indexOf(call, start + 1) >= 0) return undefined;
    return { file: "src/runtime.rs", start, end: start + call.length };
  }
  if (!mutation.file.endsWith(".rs")) return undefined;
  const lines = source.split("\n");
  const first = source.slice(0, at).split("\n").length;
  const [from, to] = enclosingFunction(source, first, first + mutation.find.split("\n").length - 1);
  if (!/\bfn\s/.test(lines[from - 1] ?? "") || to <= from) return undefined;
  const offset = (line: number) => lines.slice(0, line - 1).reduce((n, text) => n + text.length + 1, 0);
  // Its body: from the `{` that ends its signature, the first outside its
  // parameters' parentheses, as a struct's pattern there has one of its own
  // (#479), to the `}` at its indent.
  const close = offset(to) + lines[to - 1].indexOf("}");
  let open = -1;
  let depth = 0;
  for (const { char, at } of delimiters(source.slice(offset(from), close))) {
    if (char === "(" || char === "[") depth++;
    else if (char === ")" || char === "]") depth--;
    else if (char === "{" && depth === 0) {
      open = offset(from) + at;
      break;
    }
  }
  if (open < 0) return undefined;
  if (/\bconst\s+(?:unsafe\s+)?fn\b|->\s*impl\b/.test(source.slice(offset(from), open))) return undefined;
  if (at <= open || at + mutation.find.length > close) return undefined;
  return { file: mutation.file, start: open + 1, end: close };
}

/** Why `mutation`'s build wouldn't lex, as its file with it in place, and,
 * `at` its site in a build of many, as that site and its arm there; or
 * nothing where both would. A build that fails lexing is no build of what
 * the mutation says it is: of 2026-11-02's nightly, eight of one function,
 * whose site was cut at a pattern's `{` in its signature, made four
 * shards' every build of them split down to one, twenty minutes each (#479). */
export function unparsed(mutation: Mutation, at: Site | undefined, read: (file: string) => string): string | undefined {
  if (!mutation.file.endsWith(".rs")) return undefined;
  const mutated = mutate(read(mutation.file), mutation);
  if (typeof mutated !== "string") return undefined;
  const alone = unmatched(mutated);
  if (alone) return `doesn't lex: ${mutation.file} with it has an ${alone}`;
  if (!at) return undefined;
  const text = read(at.file);
  const original = text.slice(at.start, at.end);
  const arm = mutate(original, mutation);
  const from = text.slice(0, at.start).split("\n").length;
  const site = unmatched(original, from) ?? (typeof arm === "string" ? unmatched(arm, from) : undefined);
  return site && `doesn't lex in a build of many: its site, of ${at.file} from line ${from}, has an ${site}`;
}

/** `sited` in builds of at most `size`, in the order they're given: each
 * site once in a build, with all its mutations there, and none of a build's
 * sites in another, as a function's in one it holds. */
export function batches(sited: Sited[], size: number): Sited[][] {
  const overlap = (a: Site, b: Site) => a.file === b.file && a.start !== b.start && a.start < b.end && b.start < a.end;
  const made: Sited[][] = [];
  let left = sited.filter((s) => s.site !== undefined);
  while (left.length > 0) {
    const batch: Sited[] = [];
    const rest: Sited[] = [];
    for (const s of left) {
      if (batch.length < size && !batch.some((other) => overlap(other.site!, s.site!))) batch.push(s);
      else rest.push(s);
    }
    made.push(batch);
    left = rest;
  }
  return made;
}

/** Which mutation a build of many is, as it runs: its file's name, which
 * the runner gives each mutation's copy, or `RUST_JS_MUTANT`, for a test
 * that runs a copy of its own. Written at the end of the crate's root. */
const chooser = `
/// Which of the mutations built in this compiler is (scripts/mutations.ts).
#[doc(hidden)]
pub(crate) fn __mutant_is(name: &str) -> bool {
    static CHOSEN: std::sync::OnceLock<(String, String)> = std::sync::OnceLock::new();
    let (file, said) = CHOSEN.get_or_init(|| {
        let file = std::env::current_exe().ok().and_then(|path| path.file_name().map(|name| name.to_string_lossy().into_owned()));
        (file.unwrap_or_default(), std::env::var("RUST_JS_MUTANT").unwrap_or_default())
    });
    file == name || said == name
}
`;

/** The crate's files a build of `batch` writes, by their paths: of one
 * mutation, it in place; of more, each site once as it is and once for each
 * of its mutations, chosen by name as the compiler runs. Or why it can't be. */
export function schema(batch: Sited[], read: (file: string) => string): Map<string, string> | { problem: string } {
  const files = new Map<string, string>();
  if (batch.length === 1) {
    const [{ mutation }] = batch;
    const mutated = mutate(read(mutation.file), mutation);
    if (typeof mutated !== "string") return mutated;
    return files.set(mutation.file, mutated);
  }
  const bySite = new Map<string, Sited[]>();
  for (const s of batch) {
    const key = `${s.site!.file}:${s.site!.start}`;
    bySite.set(key, [...(bySite.get(key) ?? []), s]);
  }
  const sites = [...bySite.values()].sort((a, b) => b[0].site!.start - a[0].site!.start);
  for (const at of sites) {
    const { file, start, end } = at[0].site!;
    const text = files.get(file) ?? read(file);
    const original = text.slice(start, end);
    let chain = "";
    for (const { mutation } of at) {
      let arm: string | { problem: string };
      if (mutation.file.endsWith(".js")) {
        const mutated = mutate(read(mutation.file), mutation);
        if (typeof mutated !== "string") return mutated;
        files.set(`${mutation.file}.mutant-${mutation.name}`, mutated);
        arm = original.replace(/"\)$/, `.mutant-${mutation.name}")`);
      } else {
        arm = mutate(original, mutation);
        if (typeof arm !== "string") return arm;
      }
      chain += `if crate::__mutant_is(${JSON.stringify(mutation.name)}) {${arm}} else `;
    }
    files.set(file, text.slice(0, start) + `${chain}{${original}}` + text.slice(end));
  }
  return files.set("src/main.rs", (files.get("src/main.rs") ?? read("src/main.rs")) + chooser);
}

/** Each file under `paths` of `dir`, a file or a directory's, by its path from `dir`. */
function listed(dir: string, paths: string[]): string[] {
  const found: string[] = [];
  const walk = (path: string) => {
    const full = join(dir, path);
    if (!existsSync(full)) return;
    if (statSync(full).isDirectory()) for (const entry of readdirSync(full)) walk(join(path, entry));
    else found.push(path);
  };
  for (const path of paths) walk(path);
  return found;
}

/** `to`'s `paths` made what `from`'s are, writing only the files that differ
 * and removing those `from` hasn't: what's the same keeps its time, so cargo
 * rebuilds only what changed. The files written or removed. */
export function syncTree(from: string, to: string, paths: string[]): string[] {
  const touched: string[] = [];
  const wanted = new Set(listed(from, paths));
  for (const path of wanted) {
    const source = readFileSync(join(from, path));
    const target = join(to, path);
    if (existsSync(target) && readFileSync(target).equals(source)) continue;
    mkdirSync(dirname(target), { recursive: true });
    writeFileSync(target, source);
    touched.push(path);
  }
  for (const path of listed(to, paths)) {
    if (wanted.has(path)) continue;
    rmSync(join(to, path));
    touched.push(path);
  }
  return touched;
}

/** A compiler built from this checkout's crate with `files` written over
 * it, or as it is, or, `unit`, its unit tests' binary, `cargo test`'s; or
 * why it can't be built, `held` where the build isn't what it must be
 * (scripts/mutant-builds.ts). The crate is kept from the last build, what it
 * wrote put back: one built as it is is kept too, while the crate's sources
 * are what it was built from. `fresh`, with no incremental cache. */
async function build(files?: Map<string, string>, slot = 0, unit = false, fresh = false): Promise<string | { problem: string; held?: true }> {
  // Each worker builds in a crate and a target of its own (`--jobs`).
  const crate = slot === 0 ? join(work, "crate") : join(work, `crate-${slot}`);
  const target = slot === 0 ? join(work, "target") : join(work, `target-${slot}`);
  mkdirSync(crate, { recursive: true });
  syncTree(crateSource, crate, crateFiles);
  const kept = join(work, "bin", unit ? "unmutated-tests" : "unmutated");
  const sources = createHash("sha256");
  for (const path of files ? [] : listed(crate, crateFiles).sort()) sources.update(path).update(readFileSync(join(crate, path)));
  const built = sources.digest("hex");
  const stamp = join(work, unit ? "unmutated-tests.stamp" : "unmutated.stamp");
  if (!files && existsSync(kept) && existsSync(stamp) && readFileSync(stamp, "utf8") === built) return kept;
  for (const [path, text] of files ?? []) {
    mkdirSync(dirname(join(crate, path)), { recursive: true });
    writeFileSync(join(crate, path), text);
  }
  const missing = files && unapplied(files, crate, crateSource);
  if (missing) return { problem: `INCONCLUSIVE: its build isn't of it: ${missing}`, held: true };
  // Unit tests are the binary's own, built in the same target as the
  // compiler, its dependencies shared: built once for a build of many,
  // each mutation's tests run as it by `RUST_JS_MUTANT` (#915).
  const command = unit
    ? ["cargo", "test", "--no-run", "--quiet", "--locked", "--bin", "rust-js", "--message-format=json", "--target-dir", target]
    : ["cargo", "build", "--quiet", "--locked", "--message-format=json", "--target-dir", target];
  const p = await run(command, crate, buildTimeout, fresh ? { CARGO_INCREMENTAL: "0" } : {});
  if (p.code !== 0 || stopped(p, buildTimeout)) {
    const why = stopped(p, buildTimeout) ?? p.stderr.split("\n").find((line) => line.startsWith("error")) ?? `exited ${p.code}`;
    return { problem: `doesn't build: ${why}` };
  }
  const made = builtBinary(p.stdout, unit);
  if (made === undefined) return { problem: `doesn't build: cargo named no ${unit ? "unit tests' " : ""}binary` };
  if (files && !made.rebuilt) return { problem: "INCONCLUSIVE: cargo didn't compile the crate again with it", held: true };
  const binary = made.binary;
  if (files) return binary;
  mkdirSync(join(work, "bin"), { recursive: true });
  cpSync(binary, kept);
  writeFileSync(stamp, built);
  return kept;
}

/** What a run of unit tests says of a mutant: `caught` where one failed,
 * `survived` where some ran and each passed, or `inconclusive`, where it
 * was stopped, none ran, or it failed otherwise, which says nothing. */
export function judgeUnit(p: Exit): "caught" | "survived" | "inconclusive" {
  if (stopped(p, testTimeout)) return "inconclusive";
  const said = p.stdout.match(/^test result: (ok|FAILED)\. (\d+) passed; (\d+) failed;/m);
  if (!said) return "inconclusive";
  const [, result, passed, failed] = said;
  if (p.code === 0 && result === "ok" && Number(passed) > 0) return "survived";
  return p.code === 101 && result === "FAILED" && Number(failed) > 0 ? "caught" : "inconclusive";
}

/** How the unit tests `filter` names do with `binary`, as `mutant` of a
 * build of many, or as it is. */
function unitTest(binary: string, filter: string, mutant = ""): Promise<Exit> {
  return run([binary, filter], root, testTimeout, { RUST_JS_MUTANT: mutant });
}

/** `own`, a mutant's tests' target, given the helpers' module `compiler`
 * writes, `@rust-js/std`'s `runtime.js`, where it isn't the checkout's: the
 * JS imports its helpers from the package (ADR 0103), so a helper's
 * mutation is the package's too. It is the target's own, as the compiler
 * is the mutant's own, and every program its tests write there imports it.
 * So the mutant's tests run beside any others, and the checkout's
 * `std/runtime.js` is never
 * rewritten: a `check:fast` beside them read the mutant's (#720). One that
 * writes none, as the control that compiles nothing, runs with none, an
 * empty module: with the checkout's, a test that reads the package alone,
 * a bundle of it, passed with a compiler that compiles nothing, and its
 * runtime mutations were judged unrun (#1097's run, shard 5). One whose runtime couldn't be asked is that ask's exit, which
 * says nothing of the mutant: run with the checkout's runtime, a runtime
 * mutant's tests pass, and it would be said to survive. */
async function ownRuntime(compiler: string, own: string): Promise<Exit | undefined> {
  const written = await runtimeOf(compiler);
  if (typeof written !== "string") return written;
  if (written === "") installPackages(own, "// rust-js wrote no runtime.\nexport {};\n");
  else if (written !== readFileSync(join(root, "std", "runtime.js"), "utf8")) installPackages(own, written);
  return undefined;
}

/** The helpers' module `compiler` writes, or none (`""`), or, where it
 * couldn't be asked, the exit of the last ask: asked once of each, and never
 * in a way that stops the run, as a `runSync` of each of 8,000 mutants' did,
 * its tests waiting while it was asked. An ask killed by a signal is asked
 * again, up to three times: a whole run of the 1,114 runtime mutations here
 * saw ten of them killed by SIGKILL as it began, under a load of 24. */
const runtimes = new Map<string, Promise<string | Exit>>();
function runtimeOf(compiler: string): Promise<string | Exit> {
  let written = runtimes.get(compiler);
  if (written === undefined) {
    written = (async () => {
      for (let attempt = 1; ; attempt++) {
        const p = await run([compiler, "--runtime-module"], root, buildTimeout);
        if (p.signal && attempt < 3) continue;
        if (stopped(p, buildTimeout)) return p;
        return p.code === 0 ? p.stdout : "";
      }
    })();
    runtimes.set(compiler, written);
  }
  return written;
}


/** What a run of a mutant's tests says of it: `caught` by a test that
 * failed, or a setup of theirs, the runner ending as it does when one does;
 * `survived`, as its tests ran and passed; or `inconclusive`, as the
 * runner, its tests or their hooks ran out of time, it was stopped, or it
 * failed before any test or setup did, which says nothing of it.
 *
 * A setup that failed counts: a `beforeAll` that threw, which bun test
 * prints as `(fail) (unnamed)`, or a `describe`'s body, an error between
 * tests. The same tests passed as the compiler is (the run's controls, a
 * source mutation's own), so what failed it was the mutant's doing: what
 * it compiled there was wrong, as await-value-unread's examples/fetch.rs
 * was (ADR 0093). */
export function judge(p: Exit, output: string): "caught" | "survived" | "inconclusive" {
  // A mutant compiler that runs away, growing past what a run of tests is
  // given, is one broken as surely as one that fails a test: what took
  // check.yml's shards' runners down, the library mutant's recursive clone
  // at 14 GB (#1097).
  if (p.overgrown) return "caught";
  if (stopped(p, testTimeout)) return "inconclusive";
  if (p.code === 0) return passed(p.code, true, output) ? "survived" : "inconclusive";
  return p.code === 1 && failures(output).length > 0 ? "caught" : "inconclusive";
}

/** What a failed error between tests is called among `failures`. */
export const betweenTests = "(between tests)";

/** The failures of a `bun test` run that say something of its compiler:
 * each test that failed, by its name, a setup's `(unnamed)` too, and
 * `betweenTests` for a `describe`'s body that threw; none that ran out of
 * time, which says only that the machine was slow. */
export function failures(output: string): string[] {
  // Not only at a line's start: a long diff bun test cuts short doesn't end
  // its last line, and the `(fail)` after it is printed on that line.
  const failed = [...output.matchAll(/\(fail\) (.*)$\n?(  \^ .* timed out\b)?/gm)].filter((m) => m[2] === undefined);
  const between = [...output.matchAll(/^# Unhandled error between tests\n-+\n([\s\S]*?)\n-+$/gm)].filter((m) => !/\btimed out\b/.test(m[1]));
  return [...new Set([...failed.map((m) => m[1]!.replace(/ \[[\d.]+m?s\]$/, "")), ...between.map(() => betweenTests)])];
}

/** `tests`, a mutation's `bun test` arguments, narrowed to the tests named
 * `failed` by `-t`, each by its `describe`s and name as bun prints it; or
 * `tests` whole where one is a setup's or a `describe`'s, which no name
 * chooses. */
export function narrowedTo(tests: string[], failed: string[]): string[] {
  if (failed.includes("(unnamed)") || failed.includes(betweenTests)) return tests;
  const pattern = failed.map((test) => test.split(" > ").map((part) => part.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")).join(".*")).join("|");
  return [...testSet(tests).files, "-t", pattern];
}

/** Whether a run's verdicts are recorded, so each catch is confirmed
 * (`judgeTwice`): a run of every mutation, the nightly's, whose survivors
 * are listed as latent, and not a change's (`--changed`) or named ones',
 * per push, locally and on CI, which judge once. A flake there only delays
 * a survivor's finding to the next recorded run, where confirming every
 * catch cost half again a change's run (ADR 0093, amended). `--retire`'s
 * verdicts confirm their own (`failingWith`). */
export function recorded(args: string[]): boolean {
  return !args.some((arg) => arg.startsWith("--changed=")) && args.every((arg) => arg.startsWith("--"));
}

/** A mutant's verdict by a run of its tests (`judge`), a catch confirmed:
 * a test that fails once with it and passes when run again with it is a
 * flake, which says nothing of it, where it hid a survivor as a catch (a
 * `--retire` of `momentary-alias-read-taken`). Each test that failed is run
 * again with it, `again`, those alone, and where none fails, its `tests`
 * whole once more, as one may fail only after the others: it's caught
 * where a test fails both times. Each passed as the compiler is, the run's
 * controls (`--retire`'s, `failingWith`); one that failed once and passed
 * again is a flake, the verdict inconclusive, which fails the run as a
 * survivor does, `flake` naming it to be fixed (ADR 0093). */
export async function judgeTwice(
  exit: Exit,
  output: string,
  tests: string[],
  again: (tests: string[]) => Promise<{ exit: Exit; output: string }>,
): Promise<{ verdict: "caught" | "survived" | "inconclusive"; again: string; flake?: string }> {
  const verdict = judge(exit, output);
  if (verdict !== "caught" || exit.overgrown) return { verdict, again: "" };
  const failed = failures(output);
  const narrowed = narrowedTo(tests, failed);
  let said = "";
  for (const rerun of narrowed === tests ? [tests] : [narrowed, tests]) {
    const r = await again(rerun);
    said += `\n--- run again with it: bun test ${rerun.join(" ")}\n${r.output}`;
    if (r.exit.overgrown || failures(r.output).some((name) => failed.includes(name))) return { verdict: "caught", again: said };
  }
  return { verdict: "inconclusive", again: said, flake: `${failed.join(", ")} failed with it once and passed run again with it, a flake, which says nothing of it: fix the flake` };
}



/** The tests a `bun test` run's output says failed, by their names: a
 * setup's that failed, `(unnamed)`, too. */
export function failedTests(log: string): string[] {
  return [...new Set([...log.matchAll(/\(fail\) (.*?)(?: \[[\d.]+m?s\])?$/gm)].map((m) => m[1]!))];
}

// Each test run's own target is numbered: the controls run at once with
// one compiler (`main`).
let testRuns = 0;

/** The bytes one run of a mutant's tests may hold, it and every process it
 * starts: half the machine's budget, 4 GB at least. A run's peak is a
 * test file's compiles, under 2 GB; one past this is a compiler that runs
 * away, which took CI's runners of 16 GB down before (#1097). */
const testsMemory = () => Math.max(4, memoryBudget() / 2) * 2 ** 30;

/** How `tests` do with `compiler`, what they printed, and how many ran:
 * to its first failure, or, `bail` false, each of them, with `env` besides. */
async function test(
  tests: string[],
  compiler: string,
  snapshots: Snapshots = false,
  { bail = true, env = {}, directory = root }: { bail?: boolean; env?: Record<string, string>; directory?: string } = {},
): Promise<{ passed: boolean; ran: number; output: string; exit: Exit }> {
  // What the JS does is what's checked: a corpus snapshot differs with
  // nearly any change to the compiler, a mutation's or not. One that only
  // changes how the JS reads is checked by its snapshots.
  // Its own target, where the crates it builds with `compiler` are no other
  // test's, nor another mutant's: run beside the suite, `check:fast`'s, the
  // react crate a mutant built was read half written, and the suite's tests
  // failed; beside another mutant, each wrote the other's `js` crate.
  const own = join(work, "tests", `${basename(compiler)}-${++testRuns}`);
  rmSync(own, { recursive: true, force: true });
  mkdirSync(own, { recursive: true });
  const unasked = await ownRuntime(compiler, own);
  if (unasked) {
    rmSync(own, { recursive: true, force: true });
    const output = `its runtime couldn't be asked: ${compiler} --runtime-module ${stopped(unasked, buildTimeout)}\n${unasked.stderr}`;
    return { passed: false, ran: 0, output, exit: unasked };
  }
  // The first failure is the answer: a mutant is caught by any test, and
  // a control passes only where none fails (ADR 1031). Each test has the
  // minute the suite gives it (`test/setup.ts`, `test:serial`): files run
  // one after another have Bun's five seconds each, which a JSX test of
  // three seconds alone ran past on a nightly's machine of two workers,
  // and its mutation was inconclusive.
  const p = await run(
    [process.execPath, "test", "--timeout=60000", ...(bail ? ["--bail"] : []), ...tests],
    directory,
    testTimeout,
    testsEnv(compiler, own, snapshots, env),
    testsMemory(),
  );
  rmSync(own, { recursive: true, force: true });
  const output = p.stdout + p.stderr;
  const { pass, fail } = summary(output);
  return { passed: passed(p.code, !stopped(p, testTimeout), output), ran: pass + fail, output, exit: p };
}

/** How a run of the corpus judges its cases: by what the JS does alone
 * (`false`), by that and its snapshots (`true`, a mutation's
 * `snapshots`), or by its snapshots alone, run neither natively nor as JS
 * (`"alone"`, of a mutation of how the JS reads its tests miss, #864). */
export type Snapshots = boolean | "alone";

/** What a mutant's tests, or a control's, run with besides the runner's
 * own environment, `env` with them: `compiler`, and `own`, its target. */
export function testsEnv(compiler: string, own: string, snapshots: Snapshots, env: Record<string, string> = {}): Record<string, string | undefined> {
  return {
    ...env,
    RUST_JS_COMPILER: compiler,
    // Which mutation of a build of many it is, for a test that runs a
    // copy of the compiler under another name.
    RUST_JS_MUTANT: basename(compiler),
    RUST_JS_TARGET: own,
    // A compile that takes long, as an exponential one, is stopped, and
    // its test fails, before the runner runs out of time on it, which
    // would say nothing: a corpus case compiles in a second or two.
    RUST_JS_COMPILE_TIMEOUT: "60000",
    ...(snapshots === "alone" ? { RUST_JS_SNAPSHOTS: "only" } : snapshots ? {} : { RUST_JS_SNAPSHOTS: "ignore" }),
    // On GitHub's machines bun test writes a failure as an annotation, one
    // line of its whole diff, which a run cut short as it exited, leaving no
    // `(fail)` to judge by: "inconclusive" of what was caught. Its plain
    // output is the same everywhere.
    GITHUB_ACTIONS: undefined,
    // Never a bless: a mutant's tests would write its JS over the corpus's
    // snapshots, and its tests pass. `BLESS=1 bun run check:fast` gave it
    // to the change's mutations (#729).
    BLESS: undefined,
  };
}

/** A set of tests a mutation names, `bun test` arguments: its files, and
 * the pattern `-t` gives, if any. */
export function testSet(tests: string[]): { files: string[]; pattern?: string } {
  const at = tests.indexOf("-t");
  return { files: tests.filter((arg, i) => arg.startsWith("test/") && (at < 0 || i !== at + 1)), pattern: at >= 0 ? tests[at + 1] : undefined };
}

/** What a `bun test` run's JUnit report says of each test it ran, by its
 * name, a `describe`'s before it: passed or not. One `-t` filtered out
 * isn't there. */
export function testResults(junit: string): Map<string, boolean> {
  const text = (s: string) => s.replace(/&(lt|gt|quot|apos|amp);/g, (_, e) => ({ lt: "<", gt: ">", quot: '"', apos: "'", amp: "&" })[e as string]!);
  const said = new Map<string, boolean>();
  for (const [, attrs, body] of junit.matchAll(/<testcase\b([^>]*?)(?:\/>|>([\s\S]*?)<\/testcase>)/g)) {
    if (body?.includes("<skipped")) continue;
    const attr = (key: string) => text(attrs.match(new RegExp(`\\b${key}="([^"]*)"`))?.[1] ?? "");
    const name = [attr("classname"), attr("name")].filter(Boolean).join(" ");
    const passed = !body || !/<(failure|error)\b/.test(body);
    if (said.get(name) !== false) said.set(name, passed);
  }
  return said;
}

/** Whether a set's tests pass, of `ran`, what each of its files' tests did
 * by name; `undefined` where none of them ran. */
export function setPasses(set: { files: string[]; pattern?: string }, ran: Map<string, Map<string, boolean>>): boolean | undefined {
  // As `bun test -t` reads a name: with each `describe`'s before it.
  const named = (name: string) => set.pattern === undefined || new RegExp(set.pattern).test(name);
  const results = set.files.flatMap((file) => [...(ran.get(file) ?? [])].filter(([name]) => named(name)).map(([, passed]) => passed));
  return results.length === 0 ? undefined : results.every((passed) => passed);
}

/** The corpus harness's mutants run (#918), each a wrapper of `compiler`
 * on each case it's a wrong answer of, in its mode, `beside` the rest: each
 * its name and what it said, `caught` on each case, or `passed, as it must`
 * on each of a mutant that isn't a wrong answer there. */
async function corpusMutantsRun(compiler: string, beside: <T>(work: () => Promise<T>) => Promise<T>): Promise<[string, string][]> {
  const cases = corpusMutantCases();
  mkdirSync(join(work, "bin"), { recursive: true });
  return Promise.all(
    corpusMutants.map(async (mutant): Promise<[string, string]> => {
      const names = cases.filter(({ want }) => mutantOf(mutant, want)).map(({ name }) => name);
      const wrapper = join(work, "bin", `corpus-${mutant.name}`);
      writeFileSync(wrapper, `#!/bin/sh\nexec ${JSON.stringify(process.execPath)} ${JSON.stringify(join(root, "test", "corpus-mutants.ts"))} ${mutant.name} ${JSON.stringify(compiler)} "$@"\n`);
      chmodSync(wrapper, 0o755);
      const report = join(work, "reports", `corpus-${mutant.name}.xml`);
      mkdirSync(dirname(report), { recursive: true });
      const pattern = `^(${names.map((name) => name.replaceAll(".", "\\.")).join("|")})$`;
      const { output } = await beside(() =>
        test([`--reporter=junit`, `--reporter-outfile=${report}`, "test/corpus.test.ts", "-t", pattern], wrapper, mutant.mode, { bail: false }),
      );
      const ran = testResults(existsSync(report) ? readFileSync(report, "utf8") : "");
      rmSync(report, { force: true });
      rmSync(wrapper, { force: true });
      const log = join(work, "logs", `corpus-${mutant.name}.log`);
      mkdirSync(dirname(log), { recursive: true });
      writeFileSync(log, output);
      const unran = names.filter((name) => !ran.has(name));
      if (unran.length > 0) return [mutant.name, `INCONCLUSIVE: ${unran.join(", ")} didn't run; see ${log}`];
      const wrong = names.filter((name) => ran.get(name) === mutant.caught);
      if (wrong.length > 0) return [mutant.name, `${mutant.caught ? "SURVIVED" : "CAUGHT, as it mustn't be"} on ${wrong.join(", ")}: ${mutant.breaks}; see ${log}`];
      return [mutant.name, `${mutant.caught ? "caught" : "passed, as it must"} on each of ${names.length} cases`];
    }),
  );
}

/** What a run found of each mutation it ran, `target/mutants/results.json` (`Results`): CI keeps each
 * shard's, and the next check of `main` runs what changed since and what wasn't caught (`ci.ts`'s `resultsBase`). */
function writeResults(results: Map<string, string>) {
  const commit = Bun.spawnSync(["git", "rev-parse", "HEAD"], { cwd: root }).stdout.toString().trim();
  mkdirSync(work, { recursive: true });
  const found: Results = { commit, results: Object.fromEntries(results) };
  writeFileSync(join(work, "results.json"), JSON.stringify(found, null, 2) + "\n");
}

/** The flags `main` takes alone. */
const alone = ["--list", "--browser"];

/** The flags `main` takes with `=` and a value, each with the form it's
 * given in. */
const valued: Record<string, string> = {
  "--changed": "--changed=<base>",
  "--also": "--also=<name>,<name>",
  "--shard": "--shard=<i>/<n>",
  "--timings": "--timings=<file>",
  "--jobs": "--jobs=<n>",
  "--batch": "--batch=<n>",
  "--tests": "--tests=<n>",
  "--fresh": "--fresh=<share>",
  "--base": "--base=<commit>",
  "--retire": "--retire=<name>[,<name>..]",
  "--typecheck": "--typecheck=<base>",
};

/** The flags of `valued` whose value is a list, which may be empty: CI's
 * `--also={retry}` is `--also=` where no mutation is retried, the usual
 * case, an empty list (#2001's refusal of it made every shard red). */
const listFlags = ["--also"];

/** Of `args`, each flag `main` doesn't take as it's given: one it doesn't
 * know, or one that takes a value given none, a bare `--changed`, which ran
 * every mutation, about 14,000 (#2001), said with its form. A list's flag
 * with `=` and nothing after is an empty list, not none. */
export function refusedFlags(args: string[]): string[] {
  return args
    .filter((arg) => arg.startsWith("--"))
    .flatMap((arg) => {
      if (alone.includes(arg)) return [];
      const [flag, value] = arg.includes("=") ? [arg.slice(0, arg.indexOf("=")), arg.slice(arg.indexOf("=") + 1)] : [arg, undefined];
      const form = valued[flag];
      if (form === undefined) return [`no flag ${arg}; there are ${[...alone, ...Object.values(valued)].join(", ")}`];
      if (value === undefined || (value === "" && !listFlags.includes(flag))) return [`${flag} takes a value: ${form}`];
      return [];
    });
}

/** Whether `file` is the crate's, what a mutant compiler is built of, not
 * a script's. */
export function ofCrate(file: string): boolean {
  return crateFiles.some((path) => file === path || file.startsWith(`${path}/`));
}

/** The mutations `--typecheck=<base>` checks, of `changed`, the files the
 * change since `base` touches: each of the crate's Rust whose file it
 * touches, or whose list. A change to a function's signature leaves a
 * mutation's `replace` written against the old one, which a push's check
 * of `find` alone passes and CI's shards then can't build: main went red
 * so on 2026-11-18 (`own-method-call-other-this`, js/make.rs). */
export function ofTouched(changed: string[], among = lists): Mutation[] {
  const touched = new Set(changed);
  return among.flatMap((list) =>
    list.mutations.filter(
      (m) => ofCrate(m.file) && m.file.endsWith(".rs") && (touched.has(m.file) || touched.has(`scripts/mutations/${list.path}`)),
    ),
  );
}

/** The mutations of a build of many, `files` as `schema` wrote them, whose
 * arms rustc's diagnostics `said` place an error in: of each error's
 * `file:line`, the arm the nearest `__mutant_is("name")` before it opens.
 * Each is checked alone after, as an error placed past an arm, in the
 * `else` its type must agree with, is blamed on the arm before it. */
export function blamed(said: string, files: Map<string, string>): Set<string> {
  const names = new Set<string>();
  const lines = said.split("\n");
  const at: [string, number][] = [];
  for (let i = 0; i < lines.length; i++) {
    const short = /^([^\s:]+\.rs):(\d+):\d+: error/.exec(lines[i]!);
    if (short) at.push([short[1]!, Number(short[2])]);
    else if (/^error/.test(lines[i]!)) {
      const arrow = /^\s*--> ([^\s:]+\.rs):(\d+):\d+/.exec(lines[i + 1] ?? "");
      if (arrow) at.push([arrow[1]!, Number(arrow[2])]);
    }
  }
  const opening = "if crate::__mutant_is(";
  for (const [path, line] of at) {
    const key = [...files.keys()].find((file) => path === file || path.endsWith(`/${file}`));
    if (key === undefined) continue;
    const text = files.get(key)!;
    const end = text.split("\n").slice(0, line).join("\n").length;
    const opened = text.lastIndexOf(opening, end);
    const name = opened < 0 ? undefined : /^"((?:[^"\\]|\\.)*)"/.exec(text.slice(opened + opening.length))?.[1];
    if (name !== undefined) names.add(name);
  }
  return names;
}

/** Each of `sited` type-checked as its build would be, by `check`, which
 * says rustc's diagnostics where the crate with `files` written over it
 * doesn't type-check, or nothing where it does: as the runner builds
 * them, of many at once, each site once for each of its mutations, chosen
 * by name (`schema`), every site of the crate in two checks. Of one that
 * fails, the mutations its errors are in (`blamed`) each checked alone and
 * the rest again together, or, where none is placed, it split in two;
 * until one alone fails, which it names. Why each that doesn't, by name. */
export async function typechecked(
  sited: Sited[],
  check: (files: Map<string, string>, unit: boolean) => Promise<string | undefined>,
  read: (file: string) => string = crateText,
): Promise<Map<Mutation, string>> {
  const problems = new Map<Mutation, string>();
  const ofUnit = (unit: boolean) => sited.filter((s) => (s.mutation.unitTests !== undefined) === unit);
  const queue: Sited[][] = [
    ...batches(ofUnit(true), Infinity),
    ...batches(ofUnit(false), Infinity),
    ...sited.filter((s) => s.site === undefined).map((s) => [s]),
  ];
  for (let batch = queue.shift(); batch !== undefined; batch = queue.shift()) {
    const files = schema(batch, read);
    if (!(files instanceof Map)) {
      for (const { mutation } of batch) problems.set(mutation, files.problem);
      continue;
    }
    const said = await check(files, batch[0]!.mutation.unitTests !== undefined);
    if (said === undefined) continue;
    const names = batch.length > 1 ? blamed(said, files) : new Set<string>();
    const placed = batch.filter((s) => names.has(s.mutation.name));
    if (placed.length > 0) {
      queue.push(...placed.map((s) => [s]));
      if (placed.length < batch.length) queue.push(batch.filter((s) => !names.has(s.mutation.name)));
    } else if (batch.length > 1) queue.push(batch.slice(0, batch.length / 2), batch.slice(batch.length / 2));
    else problems.set(batch[0]!.mutation, `doesn't type-check: ${said.split("\n").find((line) => /\berror\b/.test(line)) ?? said.split("\n")[0]}`);
  }
  return problems;
}

/** Where `--typecheck` writes its mutants: a crate of its own, so a run in
 * target/mutants goes on beside it, checked into the checkout's target,
 * whose dependencies clippy's check already made. */
const checking = join(root, "target", "typecheck");

/** `--typecheck=<base>`: each mutation of a file the change since `base`
 * touches (`ofTouched`), type-checked, `cargo check`, none built or
 * tested: a `replace` that no longer type-checks against the change, which
 * CI's shard would call "doesn't build", is said by name before the push. */
async function typecheck(base: string): Promise<void> {
  const started = performance.now();
  const listed = runSync(["git", "diff", "--name-only", base], root, 60_000);
  if (listed.code !== 0) throw new Error(`git diff --name-only ${base}: ${listed.stderr}`);
  const chosen = ofTouched(listed.stdout.split("\n").filter(Boolean));
  const problems = new Map<Mutation, string>();
  const sited: Sited[] = [];
  for (const mutation of chosen) {
    const mutated = mutate(crateText(mutation.file), mutation);
    const at = site(mutation, crateText);
    const problem = typeof mutated === "string" ? unparsed(mutation, at, crateText) : mutated.problem;
    if (problem) problems.set(mutation, problem);
    else sited.push({ mutation, site: at });
  }
  const crate = join(checking, "crate");
  let checks = 0;
  const check = async (files: Map<string, string>, unit: boolean) => {
    checks++;
    mkdirSync(crate, { recursive: true });
    syncTree(crateSource, crate, crateFiles);
    for (const [path, text] of files) {
      mkdirSync(dirname(join(crate, path)), { recursive: true });
      writeFileSync(join(crate, path), text);
    }
    const command = ["cargo", "check", "--quiet", "--locked", "--message-format=short", ...(unit ? ["--tests"] : []), "--target-dir", join(root, "target")];
    const p = await run(command, crate, buildTimeout);
    if (p.code === 0 && !stopped(p, buildTimeout)) return undefined;
    return stopped(p, buildTimeout) ?? (p.stderr || `exited ${p.code}`);
  };
  for (const [mutation, problem] of await typechecked(sited, check)) problems.set(mutation, problem);
  const files = new Set(chosen.map((m) => m.file)).size;
  console.log(`${chosen.length} mutations of ${files} file${files === 1 ? "" : "s"} type-checked in ${checks} checks, ${((performance.now() - started) / 1000).toFixed(0)} s`);
  if (problems.size > 0) {
    throw new Error([...problems].map(([m, problem]) => `${m.name} (${m.file}): ${problem}`).join("\n"));
  }
}

/** The mutations `--retire=a,b` names, each flag's, comma-separated, in
 * order, once each; none where no flag asks a retirement. */
export function retiringNames(args: string[]): string[] | undefined {
  const flags = args.filter((arg) => arg.startsWith("--retire="));
  if (flags.length === 0) return undefined;
  const names = flags.flatMap((arg) => arg.slice("--retire=".length).split(",")).filter((name) => name.length > 0);
  if (names.length === 0) throw new Error("--retire= names no mutation");
  return [...new Set(names)];
}

/** The trailer a commit that removes a mutation of the crate carries, its
 * evidence: `bun scripts/mutations.ts --retire=<name>` prints it where the
 * mutant passed everything (ADR 0093, amended). */
export const retiredTrailer = "Retired-mutation";

/** `--retire=<name>`: whether `mutation` can go, as no test reaches what it
 * changes. Its mutant, built alone, runs check:fast's suite, every test
 * file and the corpus whole with its snapshots, the browsers' too, and the
 * compiler's unit tests; and React's port at its pin regenerated by it,
 * `react-in-rust.ts regen`, its JS compared with what the pin committed
 * (`portDiffers`), which check:fast leaves to CI: a retirement that passed
 * the suite refused 5 of the port's Flight entries in `js::validate`, and
 * only a regen by hand caught it. The port's control, the compiler as it
 * is, regenerates first: where it differs, the port says nothing either
 * way, and the retirement is refused until the pin is fixed. And crates.io's
 * corpus, every row, each crate's verdict held to the compiler's as it is,
 * a fall or a rise (`verdictsDiffer`), its control first, held to the rows'
 * bars as the port's is to its pin: check:fast's suite runs none of it with
 * another compiler, and a fallback itertools reaches was called unreached.
 * Where each
 * passes, the evidence, a trailer for the commit that removes it;
 * otherwise each test that failed with it, which holds its code and
 * becomes its row. Twice in a day a mutation was called unreachable after
 * its own rows alone ran: a lent adapter's mark, and a shadow check whose
 * removal failed 8 corpus cases.
 *
 * Several at once, `--retire=a,b,c`, share their controls, the unit tests,
 * React's port and the crate corpus as the compiler is, run once before
 * the first mutant (#2043): a failing control refuses them all, and each
 * mutant then runs alone against them, its own verdict and trailer. */
async function retire(chosen: Mutation[]): Promise<void> {
  const names = chosen.map((m) => m.name).join(", ");
  const low = belowFloor(freeBytes(root));
  if (low) throw new Error(`--retire of ${names} judges nothing: ${low}`);
  const unmutatedUnits = await build(undefined, 0, true);
  if (typeof unmutatedUnits !== "string") throw new Error(`the unit tests as the crate is ${unmutatedUnits.problem}`);
  const unitControl = await unitFailures(unmutatedUnits);
  // React's port, as the compiler is: what each mutant's regen is held to.
  const asItIs = await build();
  if (typeof asItIs !== "string") throw new Error(`the compiler as it is ${asItIs.problem}`);
  const portControl = join(work, "bin", "retire-port-control");
  mkdirSync(dirname(portControl), { recursive: true });
  rmSync(portControl, { force: true });
  cpSync(asItIs, portControl);
  const controlDiffers = await portDiffers(portControl);
  if (controlDiffers.length > 0) throw new Error(`port control fails: ${controlDiffers.join(", ")}; fix the pin first: React's port at its pin regenerates otherwise than it committed with the compiler as it is, so it says nothing of ${names}`);
  // The crate corpus as the compiler is, held to its rows' bars: what each
  // mutant's verdicts are held to.
  const crateControl = await crateVerdicts(portControl, true);
  rmSync(portControl, { force: true });
  if (typeof crateControl === "string") throw new Error(`crate corpus control fails: ${crateControl}; fix it first: crates.io's corpus isn't what docs/crate-corpus.md says with the compiler as it is, so it says nothing of ${names}`);
  const trailers: string[] = [];
  // One mutant that says nothing, its build or a run of its tests, leaves
  // the others' verdicts as they are.
  const saysNothing = (mutation: Mutation, error: unknown) => {
    console.log(`\n${mutation.name} says nothing: ${error instanceof Error ? error.message : error}`);
    process.exitCode = 1;
  };
  // Each mutant's suite in turn, its failures' reruns and their controls
  // back to back after it (`pairedReruns`); and its port and crate corpus
  // beside the next one's suite, which builds and runs nothing they share:
  // the crate corpus's phase leaves half the cores free.
  let judged: Promise<void> = Promise.resolve();
  for (const mutation of chosen) {
    let suite: RetireSuite;
    try {
      suite = await retireSuite(mutation, unitControl, asItIs);
    } catch (error) {
      saysNothing(mutation, error);
      continue;
    }
    const before = judged;
    judged = (async () => {
      await before;
      try {
        const trailer = await retireVerdict(mutation, suite, crateControl);
        if (trailer === undefined) process.exitCode = 1;
        else trailers.push(trailer);
      } catch (error) {
        saysNothing(mutation, error);
      }
    })();
  }
  await judged;
  if (chosen.length > 1 && trailers.length > 0) console.log(`\nThe evidence of each that can be retired, trailers of the commit that removes them:\n\n${trailers.join("\n")}`);
}

/** Of a compiler's unit tests, built as `binary`, those that failed and how
 * many passed: a mutant's, or its control's as the crate is. */
async function unitFailures(binary: string): Promise<{ failed: Set<string>; passed: number }> {
  const p = await unitTest(binary, "");
  if (judgeUnit(p) === "inconclusive") throw new Error(`the compiler's unit tests (${binary}) said nothing: ${endedAs(p)}\n${(p.stdout + p.stderr).slice(-1500)}`);
  return { failed: new Set([...p.stdout.matchAll(/^test (\S+) \.\.\. FAILED$/gm)].map((m) => m[1]!)), passed: Number(p.stdout.match(/^test result: \w+\. (\d+) passed;/m)?.[1] ?? 0) };
}

/** What `retireSuite` found of a mutant, which `retireVerdict` judges:
 * each suite test that failed with it, and of those, which failed again with
 * it and which fail as the compiler is, run back to back (`pairedReruns`). */
type RetireSuite = {
  mutant: string;
  unit: { passed: number };
  unitCaught: string[];
  unitSilent: string[];
  ran: string;
  failed: { file: string; test: string }[];
  again: Set<string>;
  control: Set<string>;
};

/** Why a run with `free` bytes on the disk judges nothing of a mutant, or
 * nothing where it's at or above `floor`, the disk's floor of the loop's
 * stores (test/bounded.ts). Below it the stores keep nothing, and tests
 * fail with any compiler: source-mutations' "would cross the development
 * disk floor", native-outcomes' kept runs. A retirement's control, run at
 * another time with more free, passed them, and they were blamed on the
 * mutant (2026-11-18). */
export function belowFloor(free: number, floor = floorBytes): string | undefined {
  if (free >= floor) return undefined;
  const gib = (bytes: number) => `${(bytes / 2 ** 30).toFixed(1)} GiB`;
  return `the disk has ${gib(free)} free, below its floor of ${gib(floor)}, where the loop's stores keep nothing and tests fail with any compiler: free some, and run it again`;
}

/** `retire`'s of one mutant, its suite: built, its unit tests held to
 * theirs as the crate is, and check:fast's suite run with it. */
async function retireSuite(mutation: Mutation, unitControl: { failed: Set<string> }, unmutated: string): Promise<RetireSuite> {
  const files = schema([{ mutation }], crateText);
  if (!(files instanceof Map)) throw new Error(`${mutation.name}: ${files.problem}`);
  const built = await build(files, 0);
  if (typeof built !== "string") throw new Error(`${mutation.name}'s mutant ${built.problem}`);
  // Named as the mutation, beside the sysroot its builds share, as a
  // mutant's tests run it (`judgeMutant`).
  const mutant = join(work, "bin", mutation.name);
  mkdirSync(dirname(mutant), { recursive: true });
  rmSync(mutant, { force: true });
  cpSync(built, mutant);
  // A test fails with it only where it passes as the compiler is, in the
  // same environment: its control. The compiler's unit tests as the crate
  // is, and each suite test that failed with the mutant run again with the
  // compiler as it is, as a mutant's tests run, in a target of its own.
  // Some fail there with any compiler, the compile cache's in a mutant's
  // own target say, and a unit test that reads the checkout's snapshots
  // from the crate's copy: they say nothing, and are said so.
  const units = await build(files, 0, true);
  if (typeof units !== "string") throw new Error(`${mutation.name}'s unit tests ${units.problem}`);
  const unit = await unitFailures(units);
  const unitCaught = [...unit.failed].filter((name) => !unitControl.failed.has(name)).map((name) => `cargo test: ${name}`);
  const unitSilent = [...unit.failed].filter((name) => unitControl.failed.has(name)).map((name) => `cargo test: ${name}`);
  // Its own target and runtime, as a mutant's tests have (`test`).
  const own = join(work, "tests", `${mutation.name}-retire`);
  rmSync(own, { recursive: true, force: true });
  mkdirSync(own, { recursive: true });
  const unasked = await ownRuntime(mutant, own);
  if (unasked) throw new Error(`${mutation.name}'s runtime couldn't be asked: ${endedAs(unasked)}\n${unasked.stderr}`);
  console.log(`${mutation.name}: ${unitCaught.length === 0 ? "the compiler's unit tests passed" : `${unitCaught.length} of the compiler's unit tests failed`}; check:fast's suite with it, which takes some minutes`);
  // Each judged where the disk is above its floor, before and after: below
  // it, what fails says nothing of the mutant.
  const lowBefore = belowFloor(freeBytes(root));
  if (lowBefore) throw new Error(lowBefore);
  const suite = await run(["bun", "scripts/checkFast.ts", `--compiler=${mutant}`], root, retireTimeout, testsEnv(mutant, own, true));
  rmSync(own, { recursive: true, force: true });
  const lowAfter = belowFloor(freeBytes(root));
  if (lowAfter) throw new Error(`its suite ran into the floor: ${lowAfter}`);
  const output = suite.stdout + suite.stderr;
  if (stopped(suite, retireTimeout)) throw new Error(`check:fast with ${mutation.name} ${stopped(suite, retireTimeout)}: it says nothing\n${output.slice(-3000)}`);
  const failed = [...output.matchAll(/^failed with it: (\S+)\t(.*)$/gm)].map((m) => ({ file: m[1]!, test: m[2]! }));
  if (suite.code !== 0 && failed.length === 0) throw new Error(`check:fast with ${mutation.name} failed, naming no test: it says nothing of it\n${output.slice(-3000)}`);
  const ran = output.match(/^(?:all passed: |\d+ failed in )(.*)$/m)?.[1] ?? "check:fast";
  // Each that failed, run again with it, and as the compiler is, back to
  // back, here, so the load and the disk are one for both: a catch fails
  // both times with it and passes as it is (`judgeTwice`, `byControl`).
  const { again, control } = await pairedReruns(failed, mutant, unmutated);
  return { mutant, unit, unitCaught, unitSilent, ran, failed, again, control };
}

/** Of `failed`, a suite's tests that failed with `mutant`, each file's run
 * again with it and then at once with `unmutated`, the compiler as it is,
 * as a copy the next mutant's build may not replace: those that failed
 * again with it, and those that fail as it is, by `file\ttest`
 * (`failingWith`). Back to back, so a test that fails with any compiler
 * under the load or the disk it met fails both, and says nothing; each
 * pair where the disk is above its floor (`belowFloor`). */
async function pairedReruns(failed: { file: string; test: string }[], mutant: string, unmutated: string): Promise<{ again: Set<string>; control: Set<string> }> {
  const again = new Set<string>();
  const control = new Set<string>();
  if (failed.length === 0) return { again, control };
  const unmutatedCopy = join(work, "bin", "retire-control");
  mkdirSync(dirname(unmutatedCopy), { recursive: true });
  rmSync(unmutatedCopy, { force: true });
  cpSync(unmutated, unmutatedCopy);
  for (const file of new Set(failed.map((f) => f.file))) {
    const low = belowFloor(freeBytes(root));
    if (low) throw new Error(`its failures' reruns met the floor: ${low}`);
    const ofFile = failed.filter((f) => f.file === file);
    for (const test of await failingWith(ofFile, mutant, failures)) again.add(test);
    for (const test of await failingWith(ofFile, unmutatedCopy, failedTests)) control.add(test);
  }
  rmSync(unmutatedCopy, { force: true });
  return { again, control };
}

/** `retire`'s verdict of one mutant whose suite ran (`retireSuite`): React's
 * port and the crate corpus with it, held to the controls, and its suite's
 * failures by their paired reruns: its trailer where nothing caught it, or
 * none, each test that did printed. */
async function retireVerdict(
  mutation: Mutation,
  { mutant, unit, unitCaught, unitSilent, ran, failed, again, control }: RetireSuite,
  crateControl: Map<string, Verdict>,
): Promise<string | undefined> {
  console.log(`${mutation.name}: React's port at its pin regenerated with it`);
  const portCaught = (await portDiffers(mutant)).map((file) => `react-in-rust regen: ${file}`);
  console.log(`${mutation.name}: the crate corpus, every row, measured with it`);
  const crates = await crateVerdicts(mutant, false);
  const crateCaught = typeof crates === "string" ? [`crate corpus: failed with it: ${crates}`] : verdictsDiffer(crateControl, crates);
  rmSync(mutant, { force: true });
  const { caught: suiteCaught, silent: suiteSilent, flaked } = byControl(failed, again, control);
  const silent = [...unitSilent, ...suiteSilent];
  if (silent.length > 0) console.log(`\nthese fail as the compiler is too, in a mutant's environment, and say nothing of it:\n${silent.map((name) => `  ${name}`).join("\n")}`);
  if (flaked.length > 0) console.log(`\nthese failed with it once and passed run again with it, flakes, which say nothing of it:\n${flaked.map((name) => `  ${name}`).join("\n")}`);
  const caught = [...unitCaught, ...suiteCaught, ...portCaught, ...crateCaught];
  // Nothing caught it but a flake: no evidence either way.
  if (caught.length === 0 && flaked.length > 0) throw new Error(`only flakes failed with it, which say nothing of it: fix them, and run \`--retire=${mutation.name}\` again`);
  if (caught.length === 0) {
    const head = runSync(["git", "rev-parse", "--short=9", "HEAD"], root, 10_000).stdout.trim();
    console.log(`\n${mutation.name} can be retired: nothing tests what it changes. The evidence, a trailer of the commit that removes it:\n`);
    const trailer = `${retiredTrailer}: ${mutation.name}: with it, at ${head}, the compiler's ${unit.passed} unit tests passed, and check:fast's suite, every test file and the corpus whole with its snapshots (${ran})${silent.length > 0 ? `, but ${silent.length} that fail as the compiler is too` : ""}, React's port at its pin regenerated as it committed, and the crate corpus's ${crateControl.size} rows each as the compiler is`;
    console.log(trailer);
    return trailer;
  }
  console.log(`\n${mutation.name} can't be retired: these tests failed with it, and pass as the compiler is, each a row that holds what it changes:`);
  for (const name of caught) console.log(`  ${name}`);
  return undefined;
}

/** Of the tests that failed with a mutant, those that caught it, which
 * failed again with it (`again`) and pass as the compiler is; those that
 * say nothing, which fail as the compiler is too, in a mutant's environment
 * (`control`); and those `flaked`, which passed run again with it. Each
 * set by `file\ttest` (`failingWith`). */
export function byControl(failed: { file: string; test: string }[], again: Set<string>, control: Set<string>): { caught: string[]; silent: string[]; flaked: string[] } {
  const said = (f: { file: string; test: string }) => `${f.file}: ${f.test}`;
  const silent = failed.filter((f) => control.has(`${f.file}\t${f.test}`));
  const telling = failed.filter((f) => !control.has(`${f.file}\t${f.test}`));
  return { caught: telling.filter((f) => again.has(`${f.file}\t${f.test}`)).map(said), silent: silent.map(said), flaked: telling.filter((f) => !again.has(`${f.file}\t${f.test}`)).map(said) };
}

/** Of `failed`, the tests of a suite's run with a mutant, which fail with
 * `compiler`, in a mutant's environment, by `file\ttest`, as `read` says
 * of each run: each file's run again, as a mutant's tests run it, by their
 * names, or whole where its setup failed (`narrowedTo`). The compiler as it
 * is runs as a copy, as the build the next mutant writes may replace it. */
async function failingWith(failed: { file: string; test: string }[], compiler: string, read: (output: string) => string[]): Promise<Set<string>> {
  const own = join(work, "tests", `retire-${basename(compiler)}`);
  const failing = new Set<string>();
  for (const file of new Set(failed.map((f) => f.file))) {
    const tests = narrowedTo([file], failed.filter((f) => f.file === file).map((f) => f.test));
    rmSync(own, { recursive: true, force: true });
    mkdirSync(own, { recursive: true });
    const unasked = await ownRuntime(compiler, own);
    if (unasked) throw new Error(`${compiler} couldn't be asked its runtime: ${endedAs(unasked)}`);
    const p = await run([process.execPath, "test", "--timeout=60000", ...tests], root, retireTimeout, testsEnv(compiler, own, true));
    for (const test of read(p.stdout + p.stderr)) failing.add(`${file}\t${test}`);
  }
  rmSync(own, { recursive: true, force: true });
  return failing;
}

/** Of React's port at its pin, regenerated by `compiler` (`bun
 * scripts/react-in-rust.ts regen`), each file whose JS differs from what
 * the pin committed, or that it didn't compile; none where it's the pin's.
 * What `--retire` holds a mutant to, beside the suite. */
async function portDiffers(compiler: string): Promise<string[]> {
  const p = await run(["bun", "scripts/react-in-rust.ts", "regen"], root, portTimeout, { RUST_JS_COMPILER: compiler });
  if (stopped(p, portTimeout)) throw new Error(`React's port's regen with ${compiler} ${stopped(p, portTimeout)}: it says nothing`);
  if (p.code === 0) return [];
  const differs = regenDiffers(p.stdout + p.stderr);
  if (differs.length === 0) throw new Error(`React's port's regen with ${compiler} failed, naming nothing: it says nothing\n${(p.stdout + p.stderr).slice(-3000)}`);
  return differs;
}

/** Of what a failed regen of React's port said, what differs: that its
 * crates didn't compile, and each file whose JS isn't the pin's, as `git
 * status` lists it. */
export function regenDiffers(said: string): string[] {
  const files = [...said.matchAll(/^- `(.+)`$/gm)].map((m) => m[1]!.trim());
  const compiled = !/^### react-in-rust at \w+ doesn't compile/m.test(said);
  return [...(compiled ? [] : ["its crates didn't compile"]), ...files];
}

/** Of crates.io's corpus measured with `compiler`, every row (`bun
 * scripts/crate-corpus.ts`), each crate's verdict (`verdictOf`); with
 * `check`, held to docs/crate-corpus.md's bars. Or, where the run failed,
 * what it said last. What `--retire` holds a mutant to, beside the suite
 * and React's port: a fallback whose retirement passed both, which
 * itertools reaches, was called unreached; check:fast's suite runs no
 * crate of the corpus with another compiler. */
async function crateVerdicts(compiler: string, check: boolean): Promise<Map<string, Verdict> | string> {
  const p = await run(["bun", "scripts/crate-corpus.ts", ...(check ? ["--check"] : [])], root, crateTimeout, { RUST_JS_COMPILER: compiler });
  if (stopped(p, crateTimeout)) throw new Error(`the crate corpus with ${compiler} ${stopped(p, crateTimeout)}: it says nothing`);
  if (p.code !== 0) return (p.stdout + p.stderr).trim().split("\n").slice(-3).join(" ");
  const results = JSON.parse(readFileSync(join(root, resultsFile), "utf8")) as CrateResult[];
  return new Map(results.map((result) => [result.name, verdictOf(result)]));
}

/** Of the crate corpus's verdicts with a mutant, each crate's that isn't
 * what it is with the compiler as it is, `control`'s: one that falls, and
 * one that rises, a refused crate the mutant compiles, which reaches the
 * code too. */
export function verdictsDiffer(control: Map<string, Verdict>, mutant: Map<string, Verdict>): string[] {
  return [...control]
    .filter(([name, verdict]) => mutant.get(name) !== verdict)
    .map(([name, verdict]) => `crate corpus: ${name} ${mutant.get(name) ?? "unmeasured"} with it, ${verdict} as the compiler is`);
}

/** What the crate corpus's every row may take: twice a run's here with a
 * compiler of a new build, whose crates Cargo compiles again. */
const crateTimeout = 15 * 60_000;

/** What a regen of React's port may take: twice one after a change to the
 * compiler, some 100 s, and its fetch. */
const portTimeout = 10 * 60_000;

/** What `--retire`'s run of check:fast's suite may take: twice its usual
 * with `--all`, some eight minutes here, and the corpus's snapshots. */
const retireTimeout = 40 * 60_000;

/** Each mutation a catalog's text lists: its name, its file, and what it
 * is apart from its name, its file and `find`, by which one renamed is
 * known. */
export function catalogEntries(text: string): { name: string; file?: string; key: string; find?: string }[] {
  const starts = [...text.matchAll(/^\s*name: "((?:[^"\\]|\\.)*)",$/gm)];
  return starts.map((m, i) => {
    const body = text.slice(m.index! + m[0].length, starts[i + 1]?.index ?? text.length);
    const file = body.match(/^\s*file: "([^"]+)",$/m)?.[1];
    const written = body.match(/^\s*find:([\s\S]*?)^\s*replace:/m)?.[1]?.trim().replace(/,$/, "");
    // Its code, where `find` is one string literal, in double quotes or
    // single; otherwise unread.
    let find: string | undefined;
    try {
      if (written !== undefined && /^"(?:[^"\\]|\\.)*"$/.test(written)) find = JSON.parse(written) as string;
      else if (written !== undefined && /^'(?:[^'\\]|\\.)*'$/.test(written)) {
        find = JSON.parse(`"${written.slice(1, -1).replace(/\\'/g, "'").replace(/(?<!\\)"/g, '\\"')}"`) as string;
      }
    } catch {}
    return { name: m[1]!, file, key: `${file}\0${written?.replace(/\s+/g, " ")}`, find };
  });
}

/** The mutations of the crate a commit retired, by its catalogs' text
 * before and after it, the files it touched, and `source`, a file's text
 * after it: each that went while the code it changes stayed. Not one
 * renamed, nor one moved to another of them, nor one whose code went with
 * it, which can't apply (a refactor's, folded into another), nor a
 * script's. */
export function removedOfCrate(before: string, after: string, source: (file: string) => string): string[] {
  const now = catalogEntries(after);
  const names = new Set(now.map((e) => e.name));
  const keys = new Set(now.map((e) => e.key));
  return catalogEntries(before)
    .filter((e) => !names.has(e.name) && !keys.has(e.key) && e.file !== undefined && ofCrate(e.file))
    .filter((e) => e.find === undefined || source(e.file!).includes(e.find))
    .map((e) => e.name);
}

/** Of a commit's message, the mutations it says are retired, each with
 * `--retire`'s evidence. */
export function retiredBy(message: string): Set<string> {
  return new Set([...message.matchAll(new RegExp(`^${retiredTrailer}: ([^:\\s]+): with it, at [0-9a-f]+, `, "gm"))].map((m) => m[1]!));
}


async function main() {
  const args = process.argv.slice(2);
  // A flag it doesn't know is refused, not run past: `--list`, before it
  // was one, ran all a change's mutations (#923); and one that takes a
  // value given none, a bare `--changed`, which ran every one (#2001).
  const refused = refusedFlags(args);
  if (refused.length > 0) throw new Error(refused.join("\n"));
  // A change's mutants type-checked, `typecheck`, no other run.
  const checked = args.find((arg) => arg.startsWith("--typecheck="))?.slice("--typecheck=".length);
  if (checked !== undefined) {
    if (args.length > 1) throw new Error("--typecheck=<base> runs alone: no mutation named, no other flag");
    return typecheck(checked);
  }
  const named = args.filter((arg) => !arg.startsWith("--"));
  const unknown = named.filter((name) => !mutations.some((m) => m.name === name));
  if (unknown.length > 0) throw new Error(`no mutation ${unknown.join(", ")}; there are ${mutations.map((m) => m.name).join(", ")}`);
  let chosen = named.length > 0 ? mutations.filter((m) => named.includes(m.name)) : mutations;
  // Mutations to retire, their evidence asked (`retire`), no other run.
  const retiring = retiringNames(args);
  if (retiring !== undefined) {
    if (named.length > 0 || args.some((arg) => arg.startsWith("--") && !arg.startsWith("--retire="))) throw new Error("--retire=<name>[,<name>..] runs alone: no other mutation or flag");
    chosen = retiring.map((name) => {
      const mutation = mutations.find((m) => m.name === name);
      if (mutation === undefined) throw new Error(`no mutation ${name}`);
      if (!ofCrate(mutation.file)) {
        throw new Error(`${name} is a mutation of ${mutation.file}, a script, not the crate: its tests are its evidence, and the retirement's rule is the crate's (ADR 0093, amended)`);
      }
      return mutation;
    });
  }
  const changed = args.find((arg) => arg.startsWith("--changed="))?.slice("--changed=".length);
  if (changed) {
    chosen = await changedSince(changed, chosen);
  }
  // And those named again, `--also=a,b`, which a run since was given and
  // didn't catch: CI's check of what the last run left (`ci.ts`'s
  // `resultsBase`). One no longer listed is gone, not run.
  const also = (args.find((arg) => arg.startsWith("--also="))?.slice("--also=".length) ?? "").split(",").filter(Boolean);
  chosen = [...chosen, ...mutations.filter((m) => also.includes(m.name) && !chosen.includes(m))];
  const sharded = shardArg(args);
  if (sharded) {
    const names = timedShard(chosen.map((m) => m.name), sharded.index, sharded.count, args.find((arg) => arg.startsWith("--timings="))?.slice(10), (name) => mutations.find((m) => m.name === name)?.file);
    chosen = chosen.filter((m) => names.includes(m.name));
  }
  // The corpus harness's mutants (#918): of a change to the harness, and
  // of a run of every mutation; on the first of its machines.
  const harness = (sharded === undefined || sharded.index === 1) && (changed ? await harnessChangedSince(changed) : named.length === 0);
  // Whether what would run drives a browser, `browser` or `none`, and none
  // run: CI's mutations install one where it does (`drivesBrowser`).
  if (args.includes("--browser")) {
    const files = new Set(chosen.flatMap((mutation) => (mutation.tests ? [mutation.tests[0]] : [])));
    console.log([...files].some((file) => drivesBrowser(root, file)) ? "browser" : "none");
    return;
  }
  // Which would run, each its name and file, and none run.
  if (args.includes("--list")) {
    for (const m of chosen) console.log(`${m.name}\t${m.file}`);
    console.log(`${chosen.length} mutations chosen${harness ? ", and the corpus harness's mutants" : ""}`);
    return;
  }
  if (chosen.length === 0 && !harness) {
    writeResults(new Map());
    console.log("no mutations to run");
    return;
  }
  // The machine shared with concurrent runs, this one its share
  // (scripts/leases.ts); and the checkout's mutants' build, `target/mutants`,
  // one run's at a time: two in one checkout built each other's mutants
  // there, and judged them.
  holdLease("mutations");
  const released = acquire(join(root, "target", "mutants.lock"), {
    wait: 6 * 3_600_000,
    waiting: "waiting for this checkout's other mutation run, which builds in target/mutants",
  });
  if (!released) throw new Error("this checkout's other mutation run held target/mutants for 6 hours");
  const sourceWorkspaces = join(work, "sources");
  rmSync(sourceWorkspaces, {recursive: true, force: true});
  process.on("exit", () => {
    rmSync(sourceWorkspaces, {recursive: true, force: true});
    released();
  });
  // A killed run's compilers, which it didn't get to remove; not the one
  // built without a mutation, kept while the crate is what it was built from.
  for (const entry of existsSync(join(work, "bin")) ? readdirSync(join(work, "bin")) : []) {
    if (entry !== "unmutated" && entry !== "unmutated-tests") rmSync(join(work, "bin", entry), { recursive: true, force: true });
  }
  // Each mutation's tests pass as the compiler is, and run at all, so
  // their failing is the mutation's doing.
  const sources = new Set(chosen.filter((m) => !ofCrate(m.file)));
  // The crate as the run began, which each build and site reads: a run of
  // an hour read the checkout's `src/` at each build, so an edit meanwhile
  // went into the mutants built after it, and the next edit waited for
  // the run to end.
  const given = resolve(process.env.RUST_JS_COMPILER ?? join(root, "target/debug/rust-js"));
  const buildsItself = buildsUnmutated({ crate: chosen.some(m => !sources.has(m)), harness, given: existsSync(given) });
  if (buildsItself) snapshotCrate(join(work, "snapshot"));
  if (retiring !== undefined) return retire(chosen);
  const unmutated = buildsItself ? await build() : given;
  if (typeof unmutated !== "string") throw new Error(`the compiler as it is ${unmutated.problem}`);
  // And they use the compiler they're given: with one that compiles
  // nothing, each fails, or a mutation passing them would say nothing.
  // Its directory too: a run of source mutations alone builds no
  // compiler there first, as a checkout's first run doesn't.
  const broken = join(work, "bin", "broken");
  mkdirSync(dirname(broken), { recursive: true });
  writeFileSync(broken, "#!/bin/sh\necho 'error: rust-js compiles nothing here' >&2\nexit 101\n");
  chmodSync(broken, 0o755);
  // Each set of tests as its mutations run them, with snapshots or without,
  // answered from one run of each file it names, each test's result by its
  // name, as the compiler is and as one that compiles nothing: 3,274 sets'
  // runs of their own were most of a run's time (ADR 1031). The corpus in
  // parts, each a process, as the suite runs it; as many at once as there
  // are cores.
  const sets = [...new Map(chosen.filter((m) => m.tests && !sources.has(m)).map((m) => [[m.snapshots ? "snapshots" : "", ...m.tests!].join("\0"), m])).values()].map((m) => ({
    snapshots: m.snapshots === true,
    tests: m.tests!,
    ...testSet(m.tests!),
  }));
  type Run = { compiler: string; snapshots: boolean; file: string; pattern?: string; part?: string };
  const runs: Run[] = [];
  for (const snapshots of [false, true]) {
    const ofMode = sets.filter((set) => set.snapshots === snapshots);
    for (const file of new Set(ofMode.flatMap((set) => set.files))) {
      const naming = ofMode.filter((set) => set.files.includes(file));
      const pattern = naming.some((set) => set.pattern === undefined) ? undefined : naming.map((set) => `(?:${set.pattern})`).join("|");
      const parts = file === "test/corpus.test.ts" ? caseParts(machine().cores) : 1;
      for (const compiler of [unmutated, broken]) {
        for (let part = 1; part <= parts; part++) runs.push({ compiler, snapshots, file, pattern, part: parts > 1 ? `${part}/${parts}` : undefined });
      }
    }
  }
  const ran = new Map<string, Map<string, Map<string, boolean>>>();
  const ranOf = (run: Run) => `${run.compiler}\0${run.snapshots}`;
  let reports = 0;
  const printedBy = new Map<string, string>();
  await Promise.all(
    Array.from({ length: Math.min(machine().cores, runs.length) }, async () => {
      for (let run = runs.shift(); run !== undefined; run = runs.shift()) {
        const report = join(work, "reports", `${++reports}.xml`);
        mkdirSync(dirname(report), { recursive: true });
        const args = [`--reporter=junit`, `--reporter-outfile=${report}`, run.file, ...(run.pattern === undefined ? [] : ["-t", run.pattern])];
        const { output, exit } = await test(args, run.compiler, run.snapshots, { bail: false, env: run.part ? { RUST_JS_CASES: run.part } : {} });
        const junit = existsSync(report) ? readFileSync(report, "utf8") : "";
        rmSync(report, { force: true });
        // What the run as the compiler is printed, of one that fails or runs
        // nothing: its setup's error, Node missing say, is why; and how it
        // ended, a signal, its memory or its exit code, where it printed
        // nothing more than bun's header (#1368).
        if (run.compiler === unmutated) {
          const key = `${run.snapshots}\0${run.file}`;
          printedBy.set(key, `${printedBy.get(key) ?? ""}${output.slice(-1500)}\n(${endedAs(exit)}${run.part ? `, part ${run.part}` : ""})\n`);
        }
        const byFile = ran.get(ranOf(run)) ?? new Map<string, Map<string, boolean>>();
        ran.set(ranOf(run), byFile);
        const results = byFile.get(run.file) ?? new Map<string, boolean>();
        for (const [name, passed] of testResults(junit)) if (results.get(name) !== false) results.set(name, passed);
        byFile.set(run.file, results);
      }
    }),
  );
  const problems: string[] = [];
  for (const set of sets) {
    const as = setPasses(set, ran.get(`${unmutated}\0${set.snapshots}`) ?? new Map());
    if (as !== true) {
      const printed = set.files.map((file) => printedBy.get(`${set.snapshots}\0${file}`) ?? "").join("\n");
      problems.push(`\`bun test ${set.tests.join(" ")}\` ${as === undefined ? "runs nothing" : "doesn't pass"} as the compiler is:\n${printed.slice(-1500)}`);
    } else if (setPasses(set, ran.get(`${broken}\0${set.snapshots}`) ?? new Map()) === true) {
      problems.push(`\`bun test ${set.tests.join(" ")}\` passes with a compiler that compiles nothing: it isn't using the one it's given`);
    }
  }
  // And each unit tests' filter, as the crate is: some run, and pass.
  const filters = [...new Set(chosen.flatMap((m) => (m.unitTests === undefined ? [] : [m.unitTests])))];
  if (filters.length > 0) {
    const unitTests = await build(undefined, 0, true);
    if (typeof unitTests !== "string") throw new Error(`the unit tests as the crate is ${unitTests.problem}`);
    for (const [filter, p] of await Promise.all(filters.map(async (filter) => [filter, await unitTest(unitTests, filter)] as const))) {
      if (judgeUnit(p) !== "survived") problems.push(`\`cargo test ${filter}\` doesn't run and pass as the crate is:\n${(p.stdout + p.stderr).slice(-1500)}`);
    }
  }
  if (problems.length > 0) throw new Error(problems.join("\n\n"));
  // Each build by one of `--jobs` workers, each building in a crate of its
  // own, as many as the cores and the memory hold (`compilerBuilds`): two
  // on a CI machine's four cores, the second's first build cold, a minute or
  // two of the hours its shard takes.
  const said = Number(args.find((arg) => arg.startsWith("--jobs="))?.slice("--jobs=".length));
  const jobs = said >= 1 ? said : compilerBuilds(ownShare(machine().cores));
  // And of up to `--batch` mutations each, every one chosen by name as the
  // compiler runs (ADR 1031): a build of each alone was 85% of a run's time.
  // `--batch=1` builds each alone.
  const size = Number(args.find((arg) => arg.startsWith("--batch="))?.slice("--batch=".length)) || 32;
  const read = crateText;
  const results = new Map<Mutation, string>();
  // One whose build wouldn't lex is said so by name, not built: a build of
  // many with it in splits down to it, twenty minutes of a shard (#479).
  const sited: Sited[] = [];
  for (const mutation of chosen) {
    if (sources.has(mutation)) { sited.push({mutation}); continue; }
    const at = size > 1 ? site(mutation, read) : undefined;
    const problem = unparsed(mutation, at, read);
    if (problem) results.set(mutation, problem);
    else sited.push({ mutation, site: at });
  }
  // Those unit tests catch in builds of their own, of the unit tests'
  // binary alone.
  const ofUnit = (unit: boolean) => sited.filter((s) => (s.mutation.unitTests !== undefined) === unit);
  const queue: Sited[][] = [...batches(ofUnit(true), size), ...batches(ofUnit(false), size), ...sited.filter((s) => s.site === undefined).map((s) => [s])];
  // What each took, its build's share and its tests, in seconds.
  const took = new Map<Mutation, { build: number; tests: number }>();
  // As many mutants' tests at once as there are cores, `--tests=N`: a test
  // run is a core's, where a build is two or three of them. A mutant whose
  // compiler writes another runtime runs beside the others too, as its
  // runtime is its target's own (`ownRuntime`, #720).
  let testing = 0;
  const waiters: (() => void)[] = [];
  const wake = () => waiters.splice(0).forEach((resume) => resume());
  const turn = () => new Promise<void>((resume) => waiters.push(resume));
  const testsAtOnce = Number(args.find((arg) => arg.startsWith("--tests="))?.slice("--tests=".length)) || ownShare(machine().cores);
  const beside = async <T>(work: () => Promise<T>) => {
    while (testing >= testsAtOnce) await turn();
    testing++;
    try {
      return await work();
    } finally {
      testing--;
      wake();
    }
  };
  // A mutation of how the JS reads (`readabilityOwners`) that its tests
  // miss, judged by the corpus's snapshots (#864): each case that reaches
  // its site, by the test map, or every case where there's none or it
  // can't say, compiled by the mutant and its JS compared with its snapshot
  // alone, in parts at once. A case whose JS isn't its snapshot catches it,
  // where the compiler as it is makes that snapshot; the first in each part
  // ends it.
  const caseNames = readdirSync(join(root, "test", "corpus")).filter((name) => name.endsWith(".rs"));
  // HEAD's map, recorded here where the machine keeps none of HEAD or an
  // ancestor, once a run, as the first survivor asks it: CI's comes of its
  // `test-map` job, and its run fails without one.
  let recording: ReturnType<typeof ensureMap> | undefined;
  const reaching = async (mutation: Mutation) => {
    const lines = siteLines(mutation, read(mutation.file));
    if (!lines) return { kind: "all" as const, why: "its site isn't in its file once" };
    const reached = casesReaching(mutation.file, lines[0], lines[1], caseNames);
    if (reached.kind !== "all" || !reached.why.startsWith(noMap) || process.env.GITHUB_ACTIONS) return reached;
    recording ??= (async () => {
      console.log(`${mutation.name}: its tests passed, and ${noMap}: recording HEAD's`);
      const made = await ensureMap();
      console.log(`the test map of ${made.commit.slice(0, 9)}, recorded in ${shown(made.seconds)}`);
      return made;
    })();
    await recording;
    return casesReaching(mutation.file, lines[0], lines[1], caseNames);
  };
  const snapshotsJudge = async (mutation: Mutation, mutant: string): Promise<{ caught: boolean; said: string; output: string }> => {
    const reached = await reaching(mutation);
    // Never every case: without a map CI's run fails (`noMap`, its
    // `test-map` job keeps one), and a site the map can't place is the
    // mutation's to name the case that catches it, `snapshots: true`.
    if (reached.kind === "all") {
      if (reached.why.startsWith(noMap)) throw new Error(`${mutation.name}: its tests passed, and ${reached.why}; CI's \`test-map\` job keeps one`);
      return { caught: false, said: `${reached.why}: name the corpus case that catches it in its tests, \`snapshots: true\``, output: "" };
    }
    const chosen = reached.cases;
    if (chosen.length === 0) return { caught: false, said: `${reached.why}: no case reaches it`, output: "" };
    const named = (names: string[]) => `^(${names.map((name) => name.replaceAll(".", "\\.")).join("|")})$`;
    const parts = Math.max(1, Math.min(caseParts(machine().cores), Math.ceil(chosen.length / 50)));
    const ran = await Promise.all(
      Array.from({ length: parts }, (_, i) =>
        beside(() => test(["test/corpus.test.ts", "-t", named(chosen)], mutant, "alone", { env: { RUST_JS_CASES: `${i + 1}/${parts}` } })),
      ),
    );
    const output = ran.map((r) => r.output).join("\n");
    const failed = [...output.matchAll(/^\(fail\) (\S+\.rs)\b/gm)].map((m) => m[1]);
    const of = `${chosen.length} cases`;
    if (failed.length === 0) return { caught: false, said: `of ${of}, each is its snapshot`, output };
    // Each that caught it is its snapshot as the compiler is.
    const control = await beside(() => test(["test/corpus.test.ts", "-t", named(failed)], unmutated, "alone", { bail: false }));
    if (!control.passed) return { caught: false, said: `of ${of}, ${failed.join(", ")} isn't its snapshot as the compiler is either`, output: `${output}\n${control.output}` };
    return { caught: true, said: `caught by the snapshots of ${failed.join(", ")}, of ${of}`, output };
  };
  // The corpus harness's mutants (#918), with the compiler as it is and no
  // build, beside the builds: each on its cases, in its mode, at once.
  const harnessResults = harness ? corpusMutantsRun(unmutated, beside) : Promise.resolve([]);
  // The builds' tests, which run on after their workers go on to build.
  const buildsTesting: Promise<unknown>[] = [];
  let builds = 0;
  const sourceFiles = sources.size ? runSync(["git", "ls-files", "-z"], root, 10_000) : undefined;
  if (sourceFiles && sourceFiles.code !== 0) throw new Error(sourceFiles.stderr);
  const tracked = sourceFiles?.stdout.split("\0").filter(file => file && existsSync(join(root, file))) ?? [];
  const template = join(sourceWorkspaces, ".template");
  if (sources.size) {
    const started = performance.now();
    const bytes = await makeSourceWorkspace(root, template, tracked);
    console.log(`source template: ${bytes} bytes in ${shown((performance.now() - started) / 1000)}`);
  }
  // Each build's binary by its hash, what built it, and of `--fresh`'s
  // builds, each mutation whose verdicts differ.
  const binaries = new Map<string, string>();
  if (chosen.some((m) => !sources.has(m)) || harness) binaries.set(binaryHash(unmutated), "the compiler as it is");
  const freshly = freshShare(args);
  const freshDiffers: string[] = [];
  const keep = (compiler: string, name: string) => {
    mkdirSync(join(work, "bin"), { recursive: true });
    const kept = join(work, "bin", name);
    cpSync(compiler, kept);
    return kept;
  };
  // What `compiler`, a build with `mutation`, says of it, its log written
  // as `log`: by its unit tests, or by its tests and, where they miss one
  // of how the JS reads, its cases' snapshots (#864). Run as a link in
  // `bin`, named as the mutation, which chooses it, beside the std sysroot
  // its builds share.
  const confirming = recorded(args);
  const judgeMutant = async (mutation: Mutation, compiler: string, log: string, bin = join(work, "bin")): Promise<{ result: string; tests: number }> => {
    const logged = join(work, "logs", `${log}.log`);
    mkdirSync(join(work, "logs"), { recursive: true });
    if (mutation.unitTests !== undefined) {
      const started = performance.now();
      const p = await beside(() => unitTest(compiler, mutation.unitTests!, mutation.name));
      writeFileSync(logged, p.stdout + p.stderr);
      const verdict = judgeUnit(p);
      if (verdict === "inconclusive") console.log(`--- ${log}, inconclusive: its log's end\n${(p.stdout + p.stderr).split("\n").slice(-60).join("\n")}\n---`);
      const result = verdict === "caught" ? "caught" : verdict === "survived" ? `SURVIVED: \`cargo test ${mutation.unitTests}\` passed with it` : `INCONCLUSIVE: see ${logged}`;
      return { result, tests: (performance.now() - started) / 1000 };
    }
    const mutant = join(bin, mutation.name);
    mkdirSync(bin, { recursive: true });
    rmSync(mutant, { force: true });
    try {
      linkSync(compiler, mutant);
    } catch {
      cpSync(compiler, mutant);
    }
    let started = 0;
    const byTests = await beside(() => {
      started = performance.now();
      return test(mutation.tests!, mutant, mutation.snapshots, { env: mutation.env });
    });
    const { ran, exit } = byTests;
    const twice = confirming
      ? await judgeTwice(exit, byTests.output, mutation.tests!, (tests) => beside(() => test(tests, mutant, mutation.snapshots, { env: mutation.env })))
      : { verdict: judge(exit, byTests.output), again: "", flake: undefined };
    let output = byTests.output + twice.again;
    let verdict = twice.verdict;
    if (verdict === "survived" && ofReadabilityOwner(mutation.file)) {
      const bySnapshots = await snapshotsJudge(mutation, mutant);
      output += `\n--- the snapshots of the cases that reach it (#864): ${bySnapshots.said}\n${bySnapshots.output}`;
      if (bySnapshots.caught) {
        verdict = "caught";
        console.log(`${log}: its tests passed; ${bySnapshots.said}`);
      }
    }
    const tests = (performance.now() - started) / 1000;
    rmSync(mutant, { force: true });
    runtimes.delete(mutant);
    // Its log, whatever it says, for what it caught or didn't; and its
    // end, here, as on CI the log is left on the machine: two nightlies'
    // inconclusive mutations, each caught here, said nothing of what
    // stopped them.
    writeFileSync(logged, output);
    if (verdict === "inconclusive") console.log(`--- ${log}, inconclusive: its log's end\n${output.split("\n").slice(-60).join("\n")}\n---`);
    const result = verdict === "caught" ? "caught" : verdict === "survived"
      ? `SURVIVED: its ${ran} tests passed with it${mutation.snapshots ? "" : ", snapshots not compared"}`
      : twice.flake ? `INCONCLUSIVE: ${twice.flake}; see ${logged}` : `INCONCLUSIVE: no test failed, or the runner didn't end; see ${logged}`;
    return { result, tests };
  };
  const one = async (batch: Sited[], slot: number) => {
    if (sources.has(batch[0].mutation)) {
      const mutation = batch[0].mutation;
      const execute = async (ownsTestSlot = false) => {
        const answer = await sourceMutation(mutation, template, work, tracked,
          (directory, env, tests = mutation.tests!) => {
            const run = () => test(tests, unmutated, mutation.snapshots, {directory, env: {...env, ...mutation.env}});
            return ownsTestSlot ? run() : beside(run);
          }, async (exit, output, again) => (confirming ? judgeTwice(exit, output, mutation.tests!, again) : { verdict: judge(exit, output), again: "" }), mutate);
        results.set(mutation, answer.result);
        took.set(mutation, {build: answer.build, tests: answer.tests});
        mkdirSync(join(work, "logs"), {recursive: true});
        writeFileSync(join(work, "logs", mutation.name + ".log"), answer.output);
        console.log(`source ${mutation.name}: ${answer.result}; peak workspace ${answer.bytes} bytes`);
      };
      // Generator chains occupy an existing build worker; pure modules do
      // not hold one while their tests use the existing test-slot budget.
      if (mutation.prepare) await execute();
      else buildsTesting.push(beside(() => execute(true)));
      return;
    }
    const began = performance.now();
    const files = schema(batch, read);
    const unit = batch[0].mutation.unitTests !== undefined;
    const compiler = files instanceof Map ? await build(files, slot, unit) : files;
    const building = (performance.now() - began) / 1000;
    const names = batch.length === 1 ? batch[0].mutation.name : `${batch[0].mutation.name} and ${batch.length - 1} more`;
    if (typeof compiler !== "string") {
      // A build of many that fails is two of half as many: the mutation that
      // doesn't build alone says so as its own. One that isn't of its
      // mutations says so of each (scripts/mutant-builds.ts).
      if (batch.length > 1 && !("held" in compiler)) {
        console.log(`a build of ${batch.length} mutations ${compiler.problem}: as two of ${Math.ceil(batch.length / 2)}`);
        queue.push(batch.slice(0, batch.length / 2), batch.slice(batch.length / 2));
      } else for (const { mutation } of batch) results.set(mutation, compiler.problem);
      return;
    }
    // Its binary is no other build's of the run, the compiler's as it is
    // among them: one that is, a build that reused what it shouldn't, would
    // judge that one's mutations, or none, by these names.
    const hash = binaryHash(compiler);
    const same = binaries.get(hash);
    if (same !== undefined) {
      for (const { mutation } of batch) results.set(mutation, `INCONCLUSIVE: its build is byte for byte ${same}'s`);
      return;
    }
    binaries.set(hash, `the build of ${names}`);
    // The build's tests run while the worker builds the next: its compiler
    // kept once, and each mutation's name a link to it, which chooses it.
    const kept = keep(compiler, `.build-${slot}-${++builds}`);
    // And, of `--fresh`'s share, the nightly's, built again with no
    // incremental cache, each of its mutations judged by both.
    let freshKept: string | undefined;
    if (files instanceof Map && Math.random() < freshly) {
      const again = await build(files, slot, unit, true);
      if (typeof again === "string") freshKept = keep(again, `.fresh-${slot}-${builds}`);
      else freshDiffers.push(`${names}: built fresh, ${again.problem}`);
    }
    const tested = batch.map(async ({ mutation }) => {
      const judged = await judgeMutant(mutation, kept, mutation.name);
      took.set(mutation, { build: building / batch.length, tests: judged.tests });
      results.set(mutation, judged.result);
      if (freshKept === undefined) return;
      const fresh = await judgeMutant(mutation, freshKept, `${mutation.name}.fresh`, join(work, "bin", "fresh"));
      const said = `${mutation.name}: built incrementally, ${judged.result}; fresh, ${fresh.result}`;
      console.log(`fresh\t${said}`);
      if (verdictKind(fresh.result) !== verdictKind(judged.result)) freshDiffers.push(said);
    });
    // A mutant's compiler is its tests' alone: a debug build, hundreds of
    // megabytes, which kept for each build would fill a disk.
    buildsTesting.push(
      Promise.all(tested).finally(() => {
        rmSync(kept, { force: true });
        if (freshKept) rmSync(freshKept, { force: true });
      }),
    );
  };
  // Each other worker's target, where it has none, a copy of the first's,
  // its dependencies built: CI's cache keeps the first's alone, so the
  // second built every dependency again, four at once beside the first's
  // builds, and the runner was stopped for its memory five minutes in
  // (#1097). Copied before any worker builds, while the first's is still.
  for (let slot = 1; slot < Math.min(jobs, queue.length); slot++) {
    const target = join(work, `target-${slot}`);
    if (!existsSync(target) && existsSync(join(work, "target"))) cpSync(join(work, "target"), target, { recursive: true });
  }
  // A worker with nothing to take waits while another's build may yet split.
  let busy = 0;
  await Promise.all(
    Array.from({ length: Math.min(jobs, queue.length) }, async (_, slot) => {
      for (;;) {
        const next = queue.shift();
        if (!next) {
          if (busy === 0) return;
          await new Promise((resume) => setTimeout(resume, 1000));
          continue;
        }
        busy++;
        try {
          await one(next, slot);
        } finally {
          busy--;
        }
      }
    }),
  );
  await Promise.all(buildsTesting);
  const rows: [Mutation, string][] = chosen.map((mutation) => [mutation, results.get(mutation) ?? "not run"]);
  for (const [mutation, result] of rows) console.log(`${mutation.name}\t${result}`);
  const summary = process.env.GITHUB_STEP_SUMMARY;
  if (summary) {
    const cell = (s: string) => s.replaceAll("|", "\\|");
    const lines = ["## Mutations", "", "| Mutation | Breaks | Result | Build | Tests |", "|---|---|---|---|---|"];
    for (const [mutation, result] of rows) {
      const time = took.get(mutation);
      lines.push(`| ${mutation.name} | ${cell(mutation.breaks)} | ${cell(result)} | ${time ? shown(time.build) : ""} | ${time ? shown(time.tests) : ""} |`);
    }
    writeFileSync(summary, lines.join("\n") + "\n", { flag: "a" });
  }
  // The slowest, where a run's time goes; and each one's, for the shards
  // to be balanced by.
  const slowest = [...took].sort(([, a], [, b]) => b.build + b.tests - (a.build + a.tests)).slice(0, 10);
  for (const [mutation, time] of slowest) console.log(`slow\t${mutation.name}\tbuild ${shown(time.build)}, tests ${shown(time.tests)}`);
  writeFileSync(join(work, "timings.json"), JSON.stringify(Object.fromEntries([...took].map(([mutation, time]) => [mutation.name, time])), null, 2) + "\n");
  writeResults(new Map(rows.map(([mutation, result]) => [mutation.name, result])));
  const harnessRows = await harnessResults;
  for (const [name, result] of harnessRows) console.log(`corpus-mutant ${name}\t${result}`);
  const harnessMissed = harnessRows.filter(([, result]) => !result.startsWith("caught") && !result.startsWith("passed, as it must"));
  if (harnessRows.length > 0) console.log(`${harnessRows.length - harnessMissed.length} of ${harnessRows.length} of the corpus harness's mutants as they must be`);
  if (harnessMissed.length > 0) process.exitCode = 1;
  // Of `--base=<sha>`, a survivor there too is latent, not this run's (ADR 0093, amended).
  const base = args.find((arg) => arg.startsWith("--base="))?.slice("--base=".length);
  const latent = base ? await latentSurvivors(root, work, base, Object.fromEntries(rows.map(([m, r]) => [m.name, r])), (name) => mutations.find((m) => m.name === name)?.breaks ?? "") : [];
  const missed = rows.filter(([mutation, result]) => result !== "caught" && !latent.includes(mutation.name));
  // Said last, where a `tail` of the run sees it: the rows above may be
  // more than it shows (#923).
  console.log(`${rows.filter(([, r]) => r === "caught").length} of ${rows.length} mutations caught${latent.length > 0 ? `, ${latent.length} latent` : ""}${missed.length > 0 ? `; not: ${missed.map(([m]) => m.name).join(", ")}` : ""}`);
  // Here, each one's time, its build and its tests, as many as run; CI's
  // are its own.
  if (!process.env.GITHUB_ACTIONS && rows.length > 0) {
    // A run of one or two builds each alone, no build shared, its each
    // five times a shared one's: it's said against runs of as few.
    console.log(timed("mutation, each", elapsed() / rows.length, { detail: `${rows.length} run`, ran: rows.length < 3 ? "fewer than 3" : undefined }));
  }
  if (missed.length > 0) process.exitCode = 1;
  // And `--fresh`'s builds, each mutation's verdict the same as the build
  // with an incremental cache gave, or the run fails.
  if (freshly > 0) console.log(freshDiffers.length === 0 ? "each fresh build's verdicts are its incremental build's" : `fresh builds that judged otherwise:\n${freshDiffers.join("\n")}`);
  if (freshDiffers.length > 0) process.exitCode = 1;
}

if (import.meta.main) {
  // Its line in the machine's log, however it ends: a retirement's, a
  // type-check's or a run's of its mutations.
  const args = process.argv.slice(2);
  const kind = args.some((arg) => arg.startsWith("--retire=")) ? "retire" : args.some((arg) => arg.startsWith("--typecheck=")) ? "mutations typecheck" : "mutations";
  const note = args.filter((arg) => !arg.startsWith("--")).length > 0 || kind === "retire" ? args.join(" ").slice(0, 200) : "";
  if (ownsTimes(kind)) process.on("exit", (code) => devTimes(kind, { started: 0, ok: code === 0, note }));
  await main();
}
