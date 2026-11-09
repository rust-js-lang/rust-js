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
//
// The mutations themselves are in `mutations/`, one list for each source
// file. Each builds natively a few test programs: on macOS, with the app
// that runs them under Developer Tools, or the first run of each is slow
// (AGENTS.md).

import { createHash } from "node:crypto";
import { chmodSync, cpSync, existsSync, linkSync, mkdirSync, mkdtempSync, readdirSync, readFileSync, rmSync, statSync, writeFileSync } from "node:fs";
import { availableParallelism, tmpdir } from "node:os";
import { basename, dirname, join } from "node:path";

import { run, runSync, stopped, type Exit } from "../test/child";
import { corpusHarness, corpusMutantCases, corpusMutants, mutantOf } from "../test/corpus-mutants";
import { installPackages } from "../test/packages-of";
import { caseParts, compilerBuilds, shard, shardArg } from "./shard";
import { casesReaching } from "./testMap";
import { elapsed, shown, timed } from "./timing";
import { passed, summary } from "./test-summary";

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
  /** Its tests check the corpus's JS snapshots too: it changes only how
   * the JS reads, as one that takes away the readable form of something
   * a more general one also does right, and how it reads is what's checked. */
  snapshots?: boolean;
};

// The mutations, one list for each source file, by its path from `src/`:
// `src/lower/traits.rs`'s are in `mutations/lower/traits.ts`.
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
// And `std/`, whose manifest the compiler builds in (ADR 0561).
const crateFiles = ["Cargo.toml", "Cargo.lock", "build.rs", "rust-toolchain.toml", "src", "std"];
const buildTimeout = 20 * 60_000;
const testTimeout = 20 * 60_000;

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
 * why it can't be built. The crate is kept from the last build, what it
 * wrote put back: one built as it is is kept too, while the crate's sources
 * are what it was built from. */
async function build(files?: Map<string, string>, slot = 0, unit = false): Promise<string | { problem: string }> {
  // Each worker builds in a crate and a target of its own (`--jobs`).
  const crate = slot === 0 ? join(work, "crate") : join(work, `crate-${slot}`);
  const target = slot === 0 ? join(work, "target") : join(work, `target-${slot}`);
  mkdirSync(crate, { recursive: true });
  syncTree(root, crate, crateFiles);
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
  // Unit tests are the binary's own, built in the same target as the
  // compiler, its dependencies shared: built once for a build of many,
  // each mutation's tests run as it by `RUST_JS_MUTANT` (#915).
  const command = unit
    ? ["cargo", "test", "--no-run", "--quiet", "--locked", "--bin", "rust-js", "--message-format=json", "--target-dir", target]
    : ["cargo", "build", "--quiet", "--locked", "--target-dir", target];
  const p = await run(command, crate, buildTimeout);
  if (p.code !== 0 || stopped(p, buildTimeout)) {
    const why = stopped(p, buildTimeout) ?? p.stderr.split("\n").find((line) => line.startsWith("error")) ?? `exited ${p.code}`;
    return { problem: `doesn't build: ${why}` };
  }
  const binary = unit ? testsBinary(p.stdout) : join(target, "debug", "rust-js");
  if (binary === undefined) return { problem: "doesn't build: cargo named no unit tests' binary" };
  if (files) return binary;
  mkdirSync(join(work, "bin"), { recursive: true });
  cpSync(binary, kept);
  writeFileSync(stamp, built);
  return kept;
}

/** The unit tests' binary `cargo test --no-run --message-format=json`
 * says it built, of `stdout`. */
export function testsBinary(stdout: string): string | undefined {
  for (const line of stdout.split("\n")) {
    if (!line.startsWith("{")) continue;
    const message = JSON.parse(line);
    if (message.reason === "compiler-artifact" && message.profile?.test === true && typeof message.executable === "string") return message.executable;
  }
  return undefined;
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

/** `own`, a mutant's tests' target, given the `@rust-js/runtime` `compiler`
 * writes, where it isn't the checkout's: the JS imports its helpers from
 * the package (ADR 0103), so a helper's mutation is the package's too. It
 * is the target's own, as the compiler is the mutant's own, and every
 * program its tests write there imports it. So the mutant's tests run
 * beside any others, and the checkout's `runtime/index.js` is never
 * rewritten: a `check:fast` beside them read the mutant's (#720). One that
 * writes none, as the control that compiles nothing, runs with the
 * checkout's. One whose runtime couldn't be asked is that ask's exit, which
 * says nothing of the mutant: run with the checkout's runtime, a runtime
 * mutant's tests pass, and it would be said to survive. */
async function ownRuntime(compiler: string, own: string): Promise<Exit | undefined> {
  const written = await runtimeOf(compiler);
  if (typeof written !== "string") return written;
  if (written !== "" && written !== readFileSync(join(root, "runtime", "index.js"), "utf8")) installPackages(own, written);
  return undefined;
}

/** The `@rust-js/runtime` `compiler` writes, or none (`""`), or, where it
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
 * failed, the runner ending as it does when one does; `survived`, as its
 * tests ran and passed; or `inconclusive`, as the runner, its tests or
 * their hooks ran out of time, it was stopped, or failed before any test
 * did, which says nothing of it. */
export function judge(p: Exit, output: string): "caught" | "survived" | "inconclusive" {
  if (stopped(p, testTimeout)) return "inconclusive";
  if (p.code === 0) return passed(p.code, true, output) ? "survived" : "inconclusive";
  // Not only at a line's start: a long diff bun test cuts short doesn't end
  // its last line, and the `(fail)` after it is printed on that line.
  const failed = [...output.matchAll(/\(fail\) (.*)$\n?(  \^ .* timed out\b)?/gm)].filter(
    (m) => !m[1].startsWith("(unnamed)") && m[2] === undefined,
  );
  return p.code === 1 && failed.length > 0 ? "caught" : "inconclusive";
}

// Each test run's own target is numbered: the controls run at once with
// one compiler (`main`).
let testRuns = 0;

/** How `tests` do with `compiler`, what they printed, and how many ran:
 * to its first failure, or, `bail` false, each of them, with `env` besides. */
async function test(
  tests: string[],
  compiler: string,
  snapshots: Snapshots = false,
  { bail = true, env = {} }: { bail?: boolean; env?: Record<string, string> } = {},
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
  const p = await run([process.execPath, "test", "--timeout=60000", ...(bail ? ["--bail"] : []), ...tests], root, testTimeout, testsEnv(compiler, own, snapshots, env));
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

/** The flags `main` takes, each alone or with `=` and its value. */
const flags = ["--changed", "--list", "--shard", "--jobs", "--batch", "--tests"];

async function main() {
  const args = process.argv.slice(2);
  // A flag it doesn't know is refused, not run past: `--list`, before it
  // was one, ran all a change's mutations (#923).
  const unknownFlags = args.filter((arg) => arg.startsWith("--") && !flags.some((flag) => arg === flag || arg.startsWith(`${flag}=`)));
  if (unknownFlags.length > 0) throw new Error(`no flag ${unknownFlags.join(", ")}; there are ${flags.join(", ")}`);
  const named = args.filter((arg) => !arg.startsWith("--"));
  const unknown = named.filter((name) => !mutations.some((m) => m.name === name));
  if (unknown.length > 0) throw new Error(`no mutation ${unknown.join(", ")}; there are ${mutations.map((m) => m.name).join(", ")}`);
  let chosen = named.length > 0 ? mutations.filter((m) => named.includes(m.name)) : mutations;
  const changed = args.find((arg) => arg.startsWith("--changed="))?.slice("--changed=".length);
  if (changed) {
    chosen = await changedSince(changed, chosen);
  }
  const sharded = shardArg(args);
  if (sharded) {
    const names = shard(chosen.map((m) => m.name), sharded.index, sharded.count);
    chosen = chosen.filter((m) => names.includes(m.name));
  }
  // The corpus harness's mutants (#918): of a change to the harness, and
  // of a run of every mutation; on the first of its machines.
  const harness = (sharded === undefined || sharded.index === 1) && (changed ? await harnessChangedSince(changed) : named.length === 0);
  // Which would run, each its name and file, and none run.
  if (args.includes("--list")) {
    for (const m of chosen) console.log(`${m.name}\t${m.file}`);
    console.log(`${chosen.length} mutations chosen${harness ? ", and the corpus harness's mutants" : ""}`);
    return;
  }
  if (chosen.length === 0 && !harness) {
    console.log("no mutations to run");
    return;
  }
  // A killed run's compilers, which it didn't get to remove; not the one
  // built without a mutation, kept while the crate is what it was built from.
  for (const entry of existsSync(join(work, "bin")) ? readdirSync(join(work, "bin")) : []) {
    if (entry !== "unmutated" && entry !== "unmutated-tests") rmSync(join(work, "bin", entry), { recursive: true, force: true });
  }
  // Each mutation's tests pass as the compiler is, and run at all, so
  // their failing is the mutation's doing.
  const unmutated = await build();
  if (typeof unmutated !== "string") throw new Error(`the compiler as it is ${unmutated.problem}`);
  // And they use the compiler they're given: with one that compiles
  // nothing, each fails, or a mutation passing them would say nothing.
  const broken = join(work, "bin", "broken");
  writeFileSync(broken, "#!/bin/sh\necho 'error: rust-js compiles nothing here' >&2\nexit 101\n");
  chmodSync(broken, 0o755);
  // Each set of tests as its mutations run them, with snapshots or without,
  // answered from one run of each file it names, each test's result by its
  // name, as the compiler is and as one that compiles nothing: 3,274 sets'
  // runs of their own were most of a run's time (ADR 1031). The corpus in
  // parts, each a process, as the suite runs it; as many at once as there
  // are cores.
  const sets = [...new Map(chosen.filter((m) => m.tests).map((m) => [[m.snapshots ? "snapshots" : "", ...m.tests!].join("\0"), m])).values()].map((m) => ({
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
      const parts = file === "test/corpus.test.ts" ? caseParts(availableParallelism()) : 1;
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
    Array.from({ length: Math.min(availableParallelism(), runs.length) }, async () => {
      for (let run = runs.shift(); run !== undefined; run = runs.shift()) {
        const report = join(work, "reports", `${++reports}.xml`);
        mkdirSync(dirname(report), { recursive: true });
        const args = [`--reporter=junit`, `--reporter-outfile=${report}`, run.file, ...(run.pattern === undefined ? [] : ["-t", run.pattern])];
        const { output } = await test(args, run.compiler, run.snapshots, { bail: false, env: run.part ? { RUST_JS_CASES: run.part } : {} });
        const junit = existsSync(report) ? readFileSync(report, "utf8") : "";
        rmSync(report, { force: true });
        // What the run as the compiler is printed, of one that fails or runs
        // nothing: its setup's error, Node missing say, is why.
        if (run.compiler === unmutated) printedBy.set(`${run.snapshots}\0${run.file}`, `${printedBy.get(`${run.snapshots}\0${run.file}`) ?? ""}${output.slice(-1500)}`);
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
  const jobs = said >= 1 ? said : compilerBuilds(availableParallelism());
  // And of up to `--batch` mutations each, every one chosen by name as the
  // compiler runs (ADR 1031): a build of each alone was 85% of a run's time.
  // `--batch=1` builds each alone.
  const size = Number(args.find((arg) => arg.startsWith("--batch="))?.slice("--batch=".length)) || 32;
  const read = (file: string) => readFileSync(join(root, file), "utf8");
  const results = new Map<Mutation, string>();
  // One whose build wouldn't lex is said so by name, not built: a build of
  // many with it in splits down to it, twenty minutes of a shard (#479).
  const sited: Sited[] = [];
  for (const mutation of chosen) {
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
  const testsAtOnce = Number(args.find((arg) => arg.startsWith("--tests="))?.slice("--tests=".length)) || availableParallelism();
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
  const snapshotsJudge = async (mutation: Mutation, mutant: string): Promise<{ caught: boolean; said: string; output: string }> => {
    const lines = siteLines(mutation, read(mutation.file));
    const reached = lines ? casesReaching(mutation.file, lines[0], lines[1], caseNames) : { kind: "all" as const, why: "its site isn't in its file once" };
    const chosen = reached.kind === "picked" ? reached.cases : caseNames;
    if (chosen.length === 0) return { caught: false, said: `${reached.why}: no case reaches it`, output: "" };
    const named = (names: string[]) => `^(${names.map((name) => name.replaceAll(".", "\\.")).join("|")})$`;
    const parts = Math.max(1, Math.min(caseParts(availableParallelism()), Math.ceil(chosen.length / 50)));
    const ran = await Promise.all(
      Array.from({ length: parts }, (_, i) =>
        beside(() => test(["test/corpus.test.ts", "-t", named(chosen)], mutant, "alone", { env: { RUST_JS_CASES: `${i + 1}/${parts}` } })),
      ),
    );
    const output = ran.map((r) => r.output).join("\n");
    const failed = [...output.matchAll(/^\(fail\) (\S+\.rs)\b/gm)].map((m) => m[1]);
    const of = `${chosen.length} cases${reached.kind === "picked" ? "" : `, every one (${reached.why})`}`;
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
  const one = async (batch: Sited[], slot: number) => {
    const began = performance.now();
    const files = schema(batch, read);
    const unit = batch[0].mutation.unitTests !== undefined;
    const compiler = files instanceof Map ? await build(files, slot, unit) : files;
    const building = (performance.now() - began) / 1000;
    if (typeof compiler !== "string") {
      // A build of many that fails is two of half as many: the mutation that
      // doesn't build alone says so as its own.
      if (batch.length > 1) {
        console.log(`a build of ${batch.length} mutations ${compiler.problem}: as two of ${Math.ceil(batch.length / 2)}`);
        queue.push(batch.slice(0, batch.length / 2), batch.slice(batch.length / 2));
      } else results.set(batch[0].mutation, compiler.problem);
      return;
    }
    // The build's tests run while the worker builds the next: its compiler
    // kept once, and each mutation's name a link to it, which chooses it.
    mkdirSync(join(work, "bin"), { recursive: true });
    const kept = join(work, "bin", `.build-${slot}-${++builds}`);
    cpSync(compiler, kept);
    const tested = batch.map(async ({ mutation }) => {
      if (mutation.unitTests !== undefined) {
        const started = performance.now();
        const p = await beside(() => unitTest(kept, mutation.unitTests!, mutation.name));
        took.set(mutation, { build: building / batch.length, tests: (performance.now() - started) / 1000 });
        const log = join(work, "logs", `${mutation.name}.log`);
        mkdirSync(join(work, "logs"), { recursive: true });
        writeFileSync(log, p.stdout + p.stderr);
        const verdict = judgeUnit(p);
        if (verdict === "inconclusive") console.log(`--- ${mutation.name}, inconclusive: its log's end\n${(p.stdout + p.stderr).split("\n").slice(-60).join("\n")}\n---`);
        results.set(
          mutation,
          verdict === "caught" ? "caught" : verdict === "survived" ? `SURVIVED: \`cargo test ${mutation.unitTests}\` passed with it` : `INCONCLUSIVE: see ${log}`,
        );
        return;
      }
      const mutant = join(work, "bin", mutation.name);
      rmSync(mutant, { force: true });
      try {
        linkSync(kept, mutant);
      } catch {
        cpSync(kept, mutant);
      }
      let started = 0;
      const byTests = await beside(() => {
        started = performance.now();
        return test(mutation.tests!, mutant, mutation.snapshots);
      });
      const { ran, exit } = byTests;
      let output = byTests.output;
      let verdict = judge(exit, output);
      // One of how the JS reads, that its tests miss, by its cases'
      // snapshots (#864).
      if (verdict === "survived" && ofReadabilityOwner(mutation.file)) {
        const bySnapshots = await snapshotsJudge(mutation, mutant);
        output += `\n--- the snapshots of the cases that reach it (#864): ${bySnapshots.said}\n${bySnapshots.output}`;
        if (bySnapshots.caught) {
          verdict = "caught";
          console.log(`${mutation.name}: its tests passed; ${bySnapshots.said}`);
        }
      }
      took.set(mutation, { build: building / batch.length, tests: (performance.now() - started) / 1000 });
      rmSync(mutant, { force: true });
      runtimes.delete(mutant);
      // Its log, whatever it says, for what it caught or didn't.
      const log = join(work, "logs", `${mutation.name}.log`);
      mkdirSync(join(work, "logs"), { recursive: true });
      writeFileSync(log, output);
      // Its log's end, here, as on CI the log is left on the machine: two
      // nightlies' inconclusive mutations, each caught here, said nothing
      // of what stopped them.
      if (verdict === "inconclusive") console.log(`--- ${mutation.name}, inconclusive: its log's end\n${output.split("\n").slice(-60).join("\n")}\n---`);
      results.set(
        mutation,
        verdict === "caught" ? "caught" : verdict === "survived" ? `SURVIVED: its ${ran} tests passed with it` : `INCONCLUSIVE: no test failed, or the runner didn't end; see ${log}`,
      );
    });
    // A mutant's compiler is its tests' alone: a debug build, hundreds of
    // megabytes, which kept for each build would fill a disk.
    buildsTesting.push(Promise.all(tested).finally(() => rmSync(kept, { force: true })));
  };
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
  const harnessRows = await harnessResults;
  for (const [name, result] of harnessRows) console.log(`corpus-mutant ${name}\t${result}`);
  const harnessMissed = harnessRows.filter(([, result]) => !result.startsWith("caught") && !result.startsWith("passed, as it must"));
  if (harnessRows.length > 0) console.log(`${harnessRows.length - harnessMissed.length} of ${harnessRows.length} of the corpus harness's mutants as they must be`);
  if (harnessMissed.length > 0) process.exitCode = 1;
  const missed = rows.filter(([, result]) => result !== "caught");
  // Said last, where a `tail` of the run sees it: the rows above may be
  // more than it shows (#923).
  console.log(`${rows.length - missed.length} of ${rows.length} mutations caught${missed.length > 0 ? `; not: ${missed.map(([m]) => m.name).join(", ")}` : ""}`);
  // Here, each one's time, its build and its tests, as many as run; CI's
  // are its own.
  if (!process.env.GITHUB_ACTIONS && rows.length > 0) {
    console.log(timed("mutation, each", elapsed() / rows.length, `${rows.length} run`));
  }
  if (missed.length > 0) process.exitCode = 1;
}

if (import.meta.main) await main();
