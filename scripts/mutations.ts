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
//
// The mutations themselves are in `mutations/`, one list for each source
// file. Each builds natively a few test programs: on macOS, with the app
// that runs them under Developer Tools, or the first run of each is slow
// (AGENTS.md).

import { createHash } from "node:crypto";
import { chmodSync, cpSync, existsSync, mkdirSync, mkdtempSync, readdirSync, readFileSync, rmSync, statSync, writeFileSync } from "node:fs";
import { availableParallelism, tmpdir } from "node:os";
import { dirname, join } from "node:path";

import { run, runSync, stopped, type Exit } from "../test/child";
import { shard, shardArg } from "./shard";

const root = join(import.meta.dir, "..");

export type Mutation = {
  name: string;
  /** What it breaks, as Rust would see it. */
  breaks: string;
  file: string;
  find: string;
  replace: string;
  /** `bun test` arguments whose tests must catch it. */
  tests: string[];
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

/** A file a change touches, and the lines of it, as it is now, that it
 * changed: each `[first, last]`, from 1. */
export type Change = { file: string; lines: [number, number][] };

/** A change's mutations (DEVELOPMENT.md, ADR 0093): those it adds or edits,
 * of `base`'s; those of a function it changes, the function the mutated
 * code is in; and those whose tests it changes, a test they name, or a
 * corpus case. What a change elsewhere does to them, as making the code
 * one guards redundant, the nightly run of all of them finds. `read` gives
 * a file as it is now. */
export function changedMutations(now: Mutation[], base: Mutation[], changes: Change[], read: (file: string) => string | undefined): Mutation[] {
  const same = (a: Mutation, b: Mutation) => JSON.stringify(a) === JSON.stringify(b);
  const changed = new Map(changes.map((c) => [c.file, c.lines]));
  const overlaps = (lines: [number, number][], [first, last]: [number, number]) => lines.some(([a, b]) => a <= last && b >= first);
  // A test file's tests the change touches, by name: all of them where it
  // changes what they share, above the first. A corpus case is its own test.
  const touchedTests = (file: string): string[] | "all" => {
    const lines = changed.get(file);
    const source = lines && read(file);
    if (!lines || source === undefined) return [];
    const blocks = testBlocks(source);
    if (blocks.length === 0 || overlaps(lines, [1, blocks[0].first - 1])) return "all";
    return blocks.filter((block) => overlaps(lines, [block.first, block.last])).map((block) => block.name);
  };
  const cases = changes.filter((c) => c.file.startsWith("test/corpus/")).map((c) => c.file.slice("test/corpus/".length));
  return now.filter((m) => {
    if (!base.some((b) => same(b, m))) return true;
    const source = changed.has(m.file) ? read(m.file) : undefined;
    if (source !== undefined) {
      const at = source.indexOf(m.find);
      // One that no longer applies runs, to say so.
      if (at < 0) return true;
      const first = source.slice(0, at).split("\n").length;
      const block = enclosingFunction(source, first, first + m.find.split("\n").length - 1);
      if (overlaps(changed.get(m.file)!, block)) return true;
    }
    const [file, ...args] = m.tests;
    const at = args.indexOf("-t");
    const filter = at >= 0 ? new RegExp(args[at + 1]) : undefined;
    const tests = touchedTests(file);
    if (tests === "all" || (tests.length > 0 && (!filter || tests.some((name) => filter.test(name))))) return true;
    return file === "test/corpus.test.ts" && filter !== undefined && cases.some((name) => filter.test(name));
  });
}

/** The mutations, of `among`, a change since `base` runs (`--changed`). */
export async function changedSince(base: string, among: Mutation[] = mutations): Promise<Mutation[]> {
  const { changes, mutations: old } = await since(base);
  const read = (file: string) => {
    try {
      return readFileSync(join(root, file), "utf8");
    } catch {
      return undefined;
    }
  };
  return changedMutations(among, old, changes, read);
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

/** The lines `git diff -U0` says each file's change is in, as it is now. */
export function changedLines(diff: string): Change[] {
  const changes: Change[] = [];
  let current: Change | undefined;
  for (const line of diff.split("\n")) {
    if (line.startsWith("+++ ")) {
      current = line === "+++ /dev/null" ? undefined : { file: line.slice("+++ b/".length), lines: [] };
      if (current) changes.push(current);
      continue;
    }
    const hunk = line.match(/^@@ -\d+(?:,\d+)? \+(\d+)(?:,(\d+))? @@/);
    if (hunk && current) {
      const from = Number(hunk[1]);
      const count = hunk[2] === undefined ? 1 : Number(hunk[2]);
      // Lines taken away are between two: both are where the change is.
      current.lines.push(count === 0 ? [from, from + 1] : [from, from + count - 1]);
    }
  }
  return changes;
}

/** The files changed since where this branch left `base`, and the
 * mutations as they were there. */
async function since(base: string): Promise<{ files: string[]; changes: Change[]; mutations: Mutation[] }> {
  const git = (args: string[]) => {
    const p = runSync(["git", ...args], root, 60_000);
    if (p.code !== 0) throw new Error(`git ${args.join(" ")} failed:\n${p.stderr}`);
    return p.stdout;
  };
  const from = git(["merge-base", base, "HEAD"]).trim();
  const files = git(["diff", "--name-only", from]).split("\n").filter(Boolean);
  const changes = changedLines(git(["diff", "-U0", "--no-color", from]));
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
  return { files, changes, mutations: old };
}

// Where the mutated crate is built, and the compilers kept: one copy of
// the crate, remade for each mutation, and one target, so only rust-js is
// built again.
const work = join(root, "target", "mutants");
const crateFiles = ["Cargo.toml", "Cargo.lock", "build.rs", "rust-toolchain.toml", "src"];
const buildTimeout = 20 * 60_000;
const testTimeout = 20 * 60_000;

/** The source as it is, with `mutation` in it, or why it can't be. */
export function mutate(source: string, mutation: Mutation): string | { problem: string } {
  const at = source.indexOf(mutation.find);
  if (at < 0) return { problem: `doesn't apply: ${mutation.file} has no \`${mutation.find.split("\n")[0].trim()}\`` };
  if (source.indexOf(mutation.find, at + 1) >= 0) return { problem: `applies more than once in ${mutation.file}` };
  return source.slice(0, at) + mutation.replace + source.slice(at + mutation.find.length);
}

/** Each file under `paths` of `dir`, a file or a directory's, by its path from `dir`. */
function files(dir: string, paths: string[]): string[] {
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
  const wanted = new Set(files(from, paths));
  for (const path of wanted) {
    const source = readFileSync(join(from, path));
    const target = join(to, path);
    if (existsSync(target) && readFileSync(target).equals(source)) continue;
    mkdirSync(dirname(target), { recursive: true });
    writeFileSync(target, source);
    touched.push(path);
  }
  for (const path of files(to, paths)) {
    if (wanted.has(path)) continue;
    rmSync(join(to, path));
    touched.push(path);
  }
  return touched;
}

/** A compiler built from this checkout's crate, with `mutation` in it, or
 * without one; or why it can't be built. The crate is kept from the last
 * build, the last mutation's file put back: one built without one is kept
 * too, while the crate's sources are what it was built from. */
async function build(mutation?: Mutation, slot = 0): Promise<string | { problem: string }> {
  // Each worker builds in a crate and a target of its own (`--jobs`).
  const crate = slot === 0 ? join(work, "crate") : join(work, `crate-${slot}`);
  const target = slot === 0 ? join(work, "target") : join(work, `target-${slot}`);
  mkdirSync(crate, { recursive: true });
  syncTree(root, crate, crateFiles);
  const kept = join(work, "bin", mutation?.name ?? "unmutated");
  const sources = createHash("sha256");
  for (const path of files(crate, crateFiles).sort()) sources.update(path).update(readFileSync(join(crate, path)));
  const built = sources.digest("hex");
  const stamp = join(work, "unmutated.stamp");
  if (!mutation && existsSync(kept) && existsSync(stamp) && readFileSync(stamp, "utf8") === built) return kept;
  if (mutation) {
    const file = join(crate, mutation.file);
    const mutated = mutate(readFileSync(file, "utf8"), mutation);
    if (typeof mutated !== "string") return mutated;
    writeFileSync(file, mutated);
  }
  const p = await run(["cargo", "build", "--quiet", "--locked", "--target-dir", target], crate, buildTimeout);
  if (p.code !== 0 || stopped(p, buildTimeout)) {
    const why = stopped(p, buildTimeout) ?? p.stderr.split("\n").find((line) => line.startsWith("error")) ?? `exited ${p.code}`;
    return { problem: `doesn't build: ${why}` };
  }
  mkdirSync(join(work, "bin"), { recursive: true });
  cpSync(join(target, "debug", "rust-js"), kept);
  if (!mutation) writeFileSync(stamp, built);
  return kept;
}

/** `run`, with the checkout's `@rust-js/runtime` the one `compiler` writes:
 * the JS imports its helpers from the package (ADR 0103), so a helper's
 * mutation is the package's too. The checkout's is put back after. One
 * that writes none, as the control that compiles nothing, runs with the
 * checkout's. */
async function withRuntime<T>(compiler: string, work: () => Promise<T>): Promise<T> {
  const file = join(root, "runtime", "index.js");
  const original = readFileSync(file, "utf8");
  const written = runtimeOf(compiler);
  if (written === undefined || written === original) return work();
  writeFileSync(file, written);
  try {
    return await work();
  } finally {
    writeFileSync(file, original);
  }
}

/** The `@rust-js/runtime` `compiler` writes, or none. */
function runtimeOf(compiler: string): string | undefined {
  const written = runSync([compiler, "--runtime-module"], root, buildTimeout);
  return written.code !== 0 || stopped(written, buildTimeout) ? undefined : written.stdout;
}

const count = (output: string, what: string) => Number(new RegExp(String.raw`^ (\d+) ` + what + "$", "m").exec(output)?.[1] ?? 0);

/** What a run of a mutant's tests says of it: `caught` by a test that
 * failed, the runner ending as it does when one does; `survived`, as its
 * tests ran and passed; or `inconclusive`, as the runner, its tests or
 * their hooks ran out of time, it was stopped, or failed before any test
 * did, which says nothing of it. */
export function judge(p: Exit, output: string): "caught" | "survived" | "inconclusive" {
  if (stopped(p, testTimeout)) return "inconclusive";
  if (p.code === 0) return count(output, "pass") > 0 && count(output, "fail") === 0 ? "survived" : "inconclusive";
  // Not only at a line's start: a long diff bun test cuts short doesn't end
  // its last line, and the `(fail)` after it is printed on that line.
  const failed = [...output.matchAll(/\(fail\) (.*)$\n?(  \^ .* timed out\b)?/gm)].filter(
    (m) => !m[1].startsWith("(unnamed)") && m[2] === undefined,
  );
  return p.code === 1 && failed.length > 0 ? "caught" : "inconclusive";
}

/** How `tests` do with `compiler`, what they printed, and how many ran. */
async function test(tests: string[], compiler: string, snapshots = false): Promise<{ passed: boolean; ran: number; output: string; exit: Exit }> {
  // What the JS does is what's checked: a corpus snapshot differs with
  // nearly any change to the compiler, a mutation's or not. One that only
  // changes how the JS reads is checked by its snapshots.
  const p = await withRuntime(compiler, () =>
    run([process.execPath, "test", ...tests], root, testTimeout, {
      RUST_JS_COMPILER: compiler,
      // A compile that takes long, as an exponential one, is stopped, and
      // its test fails, before the runner runs out of time on it, which
      // would say nothing: a corpus case compiles in a second or two.
      RUST_JS_COMPILE_TIMEOUT: "60000",
      ...(snapshots ? {} : { RUST_JS_SNAPSHOTS: "ignore" }),
      // On GitHub's machines bun test writes a failure as an annotation, one
      // line of its whole diff, which a run cut short as it exited, leaving no
      // `(fail)` to judge by: "inconclusive" of what was caught. Its plain
      // output is the same everywhere.
      GITHUB_ACTIONS: undefined,
    }),
  );
  const output = p.stdout + p.stderr;
  return { passed: p.code === 0 && !stopped(p, testTimeout), ran: count(output, "pass") + count(output, "fail"), output, exit: p };
}

async function main() {
  const args = process.argv.slice(2);
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
  if (chosen.length === 0) {
    console.log("no mutations to run");
    return;
  }
  // A killed run's compilers, which it didn't get to remove; not the one
  // built without a mutation, kept while the crate is what it was built from.
  for (const entry of existsSync(join(work, "bin")) ? readdirSync(join(work, "bin")) : []) {
    if (entry !== "unmutated") rmSync(join(work, "bin", entry), { recursive: true, force: true });
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
  // Each set of tests as its mutations run them, with snapshots or without.
  for (const key of new Set(chosen.map((m) => [m.snapshots ? "snapshots" : "", ...m.tests].join("\0")))) {
    const [mode, ...tests] = key.split("\0");
    const control = await test(tests, unmutated, mode === "snapshots");
    if (!control.passed || control.ran === 0) {
      throw new Error(`\`bun test ${tests.join(" ")}\` doesn't pass, or runs nothing, as the compiler is:\n${control.output.slice(-2000)}`);
    }
    if ((await test(tests, broken, mode === "snapshots")).passed) {
      throw new Error(`\`bun test ${tests.join(" ")}\` passes with a compiler that compiles nothing: it isn't using the one it's given`);
    }
  }
  // Each mutation by one of `--jobs` workers, each building in a crate of its
  // own: one for each three cores, up to four, as a build uses several,
  // and one on a CI machine's four, where each would build cold. One whose compiler
  // writes another runtime, which the checkout's `runtime/index.js` holds
  // while its tests run, waits for the others and runs alone.
  const said = Number(args.find((arg) => arg.startsWith("--jobs="))?.slice("--jobs=".length));
  const jobs = said >= 1 ? said : Math.min(4, Math.max(1, Math.floor(availableParallelism() / 3)));
  const results = new Map<Mutation, string>();
  const alone: Mutation[] = [];
  const original = readFileSync(join(root, "runtime", "index.js"), "utf8");
  const one = async (mutation: Mutation, slot: number, shared: boolean) => {
    const compiler = await build(mutation, slot);
    if (typeof compiler !== "string") {
      results.set(mutation, compiler.problem);
      return;
    }
    if (shared && runtimeOf(compiler) !== original) {
      alone.push(mutation);
      rmSync(compiler, { force: true });
      return;
    }
    const { ran, output, exit } = await test(mutation.tests, compiler, mutation.snapshots);
    // A mutant's compiler is its tests' alone: a debug build, hundreds of
    // megabytes, which kept for each of hundreds would fill a disk.
    rmSync(compiler, { force: true });
    // Its log, whatever it says, for what it caught or didn't.
    const log = join(work, "logs", `${mutation.name}.log`);
    mkdirSync(join(work, "logs"), { recursive: true });
    writeFileSync(log, output);
    const verdict = judge(exit, output);
    results.set(
      mutation,
      verdict === "caught" ? "caught" : verdict === "survived" ? `SURVIVED: its ${ran} tests passed with it` : `INCONCLUSIVE: no test failed, or the runner didn't end; see ${log}`,
    );
  };
  const queue = [...chosen];
  const workers = Math.min(jobs, chosen.length);
  await Promise.all(
    Array.from({ length: workers }, async (_, slot) => {
      for (let next = queue.shift(); next; next = queue.shift()) await one(next, slot, workers > 1);
    }),
  );
  for (const mutation of alone) await one(mutation, 0, false);
  const rows: [Mutation, string][] = chosen.map((mutation) => [mutation, results.get(mutation) ?? "not run"]);
  for (const [mutation, result] of rows) console.log(`${mutation.name}\t${result}`);
  const summary = process.env.GITHUB_STEP_SUMMARY;
  if (summary) {
    const cell = (s: string) => s.replaceAll("|", "\\|");
    const lines = ["## Mutations", "", "| Mutation | Breaks | Result |", "|---|---|---|"];
    for (const [mutation, result] of rows) lines.push(`| ${mutation.name} | ${cell(mutation.breaks)} | ${cell(result)} |`);
    writeFileSync(summary, lines.join("\n") + "\n", { flag: "a" });
  }
  const missed = rows.filter(([, result]) => result !== "caught");
  console.log(`${rows.length - missed.length} of ${rows.length} mutations caught`);
  if (missed.length > 0) process.exitCode = 1;
}

if (import.meta.main) await main();
