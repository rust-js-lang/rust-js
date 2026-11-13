// The metamorphic net (#939): each corpus case rewritten in a way that keeps
// its meaning, by `scripts/metamorph`, run natively and as JS in both builds
// and compared. Native Rust's run of the rewrite is its oracle, so no
// snapshot is needed; a rewrite rustc rejects is no variant. Each case runs
// as native Rust does as it's written (the corpus); one whose rewrite
// doesn't is a shape around a value that the lowering reads otherwise:
// lesson 2 of docs/finding-wrong-answers.md.
//
//   case.rs ─► metamorph <rewrite> ─┬─ rustc ──► native ─────┐
//                                   └─ rust-js ─► node ──────┴─► the same?

import { copyFileSync, existsSync, mkdirSync, mkdtempSync, readFileSync, renameSync, rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";

import { cases, corpus, directives } from "./corpus-cases";
import { root, target } from "./support";
import { runSync } from "./child";
import { compileJs, runJs, runNative, runtimes, type Run } from "./programs";
import { same } from "./oracle";

/** Each rewrite, `metamorph`'s: `main`'s body outlined; each `let` of a
 * name bound once moved into a box, an `Option`, an `Rc<RefCell>`, a
 * struct's field, an array of one and a tuple of one; each reference
 * parameter of a free function taken as `&dyn` and as `&impl` of an
 * identity trait; each owned one as `impl` of one; a free function made
 * generic over a type it takes; each closure that captures nothing made a
 * `fn`, and each function given as a value a closure; every item but
 * `main` moved into a library crate; a `for` loop made a `for_each` and a
 * `for_each` a `for` loop; a `match` of two arms an `if let` or a `let
 * else`; a method's body a free function's, and a free function's a
 * method's; a trait's default written out in each impl; a struct of named
 * fields a tuple struct; the case made loud, its types' drops and
 * clones, its closures' calls and its loops' steps each a line, and each
 * kind's middle event made a panic (#1203); and each iterator given on as
 * an `impl Iterator`, each closure called through a `Box<dyn Fn>`, and a
 * neighbour that turns on one crate-wide fact, never run (#1204). */
export const rewrites = [
  "outline", "boxed", "dyn-ref", "impl-ref", "impl-owned", "generic", "option", "rc-refcell", "field", "closure-fn", "library",
  "array", "tuple", "for-each", "for-loop", "if-let", "method-fn", "fn-method", "default-copy", "tuple-struct",
  // Equivalence modulo inputs (#1086): what the run never reaches made
  // `unreachable!()`, and code never run added.
  "emi-prune", "emi-insert",
  // The case made loud (#1203): what std does with its values a line each,
  // and a panic at the middle event of each kind.
  "loud", "loud-drop", "loud-clone", "loud-call", "loud-step",
  // Contexts (#1204): each iterator an `impl Iterator`, each closure a
  // `Box<dyn Fn>`, and a neighbour that turns on each crate-wide fact.
  "opaque", "dyn-fn",
  // Generic code given instances (#1237): each consumer and adapter of an
  // iterator the case makes called in generic code, given another instance.
  "generic-instances",
  "fact-addresses", "fact-slices", "fact-stored", "fact-capacity", "fact-fn-identity", "fact-chars", "fact-write-fails", "fact-rc-counts", "fact-clones",
] as const;
export type Rewrite = (typeof rewrites)[number];

/** Where the rewriter's crate is, and what it builds. */
const manifest = join(root, "scripts", "metamorph", "Cargo.toml");
const built = join(target, "metamorph", "debug", "metamorph");
/** What's run: a copy of what's built, put in place whole by a rename.
 * Cargo links its binary again on each build, even one with nothing to
 * do, so a part of a parallel run that spawned `built` while another
 * part's build linked it found no file there (ENOENT). */
export const rewriter = join(target, "metamorph", "metamorph");

/** The rewriter, built once: its own crate, apart from the compiler's. */
export function buildRewriter(): void {
  const p = runSync(["cargo", "build", "--quiet", "--manifest-path", manifest, "--target-dir", join(target, "metamorph")], root, 600_000);
  if (p.code !== 0) throw new Error(`the rewriter doesn't build:\n${p.stderr}`);
  // Copied whole, though another part's build may be linking it now.
  const copy = `${rewriter}.${process.pid}`;
  for (let tries = 1; ; tries++) {
    try {
      copyFileSync(built, copy);
      break;
    } catch (e) {
      if ((e as NodeJS.ErrnoException).code !== "ENOENT" || tries === 50) throw e;
      Bun.sleepSync(20);
    }
  }
  renameSync(copy, rewriter);
}

/** `rewrite` of the case `file`, of so many of the places it finds,
 * `limit`'s, or `undefined` where it changes nothing: the program, and of
 * `library`, the library it uses. `closure-fn` asks rustc for a closure's
 * types, at the case's `edition`. */
export function rewritten(rewrite: Rewrite, file: string, edition: string, limit?: number): { program: string; library?: string } | undefined {
  // `emi-prune` is given the blocks the case's run reaches, its probe's.
  const env: Record<string, string> = { METAMORPH_EDITION: edition };
  // A case whose point is what its iterators skip at once, given instances
  // that keep each place's cost (`//@ instances-keep-cost`, #1109).
  const want = existsSync(file) ? directives(readFileSync(file, "utf8")) : undefined;
  if (typeof want === "object" && want.instancesKeepCost !== undefined) env.METAMORPH_KEEP_COST = "1";
  if (rewrite === "emi-prune") {
    const hits = reached(file, edition);
    if (hits === undefined) return undefined;
    env.METAMORPH_EMI_HITS = hits.join(",");
  }
  // A loud panic's is `loud` with its point, the event `loud`'s run says.
  const panics = /^loud-(\w+)$/.exec(rewrite)?.[1];
  if (panics !== undefined) {
    const at = panicPoint(file, edition, panics, limit);
    if (at === undefined) return undefined;
    env.METAMORPH_PANIC_AT = String(at);
  }
  const by = panics === undefined ? rewrite : "loud";
  const p = runSync([rewriter, by, file, ...(limit === undefined ? [] : [String(limit)])], root, 60_000, env);
  if (p.code === 2) return undefined;
  if (p.code !== 0) throw new Error(`metamorph ${rewrite} ${file}: ${p.stderr}`);
  if (rewrite !== "library") return { program: p.stdout };
  const [library, program] = p.stdout.split(libraryEnds);
  if (program === undefined) throw new Error(`metamorph library ${file}: no \`${libraryEnds.trim()}\` line`);
  return { program, library };
}

/** The blocks of `file` its runs reach, by the numbers `emi-probe` gives
 * them, the probe's runs saying each the first time they run it: native
 * Rust's and its JS's, so a block only the JS reaches, by a difference
 * decided and written down, a panic's payload that downcasts as a `&str`
 * and a `String` both (ADR 0692), is no block pruned; or `undefined` where
 * the native probe doesn't run to its end (#1086). Each probe in a
 * directory of its own, removed after: one a probe's hash named was shared
 * by every part of a parallel run probing the same case, the harness's own
 * test in each of them, one part's build written over while another ran
 * it, and that test failed about once in twelve parts. */
function reached(file: string, edition: string): number[] | undefined {
  const probe = runSync([rewriter, "emi-probe", file], root, 60_000, { METAMORPH_EDITION: edition });
  if (probe.code !== 0) return undefined;
  const probes = join(target, "metamorph-emi");
  mkdirSync(probes, { recursive: true });
  const dir = mkdtempSync(join(probes, `${Bun.hash(probe.stdout).toString(36)}-`));
  try {
    const program = join(dir, "probe.rs");
    writeFileSync(program, probe.stdout);
    const ran = runNative(program, dir, edition);
    if (typeof ran === "string" || typeof ran.outcome === "string") return undefined;
    const said = (stderr: string) => [...stderr.matchAll(/^METAMORPH-EMI-HIT (\d+)$/gm)].map((m) => Number(m[1]));
    const compiled = compileJs(program, dir, edition);
    const js = "js" in compiled ? runJs(runtimes[0][1], compiled.js, dir, runtimes[0][0]).stderr : "";
    return [...new Set([...said(ran.stderr), ...said(js)])];
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
}

/** The event a `loud-<kind>` panics at: of the events of `kind` that
 * `loud`'s native run of `file` says, `~loud <n> <kind> ..`, the middle
 * one, where some are behind it and some ahead; or `undefined` where it
 * says none, or doesn't run to its end. */
function panicPoint(file: string, edition: string, kind: string, limit: number | undefined): number | undefined {
  const made = runSync([rewriter, "loud", file, ...(limit === undefined ? [] : [String(limit)])], root, 60_000, { METAMORPH_EDITION: edition });
  if (made.code !== 0) return undefined;
  const dir = join(target, "metamorph-loud", Bun.hash(made.stdout).toString(36));
  mkdirSync(dir, { recursive: true });
  const program = join(dir, "loud.rs");
  writeFileSync(program, made.stdout);
  const ran = runNative(program, dir, edition);
  if (typeof ran === "string") return undefined;
  return middleEvent(ran.stdout, kind);
}

/** Of the events of `kind` that a loud run's `stdout` says, the middle
 * one's number. */
export function middleEvent(stdout: string, kind: string): number | undefined {
  const events = [...stdout.matchAll(/^~loud (\d+) (\w+)/gm)].filter((m) => m[2] === kind).map((m) => Number(m[1]));
  return events.length === 0 ? undefined : events[Math.floor((events.length - 1) / 2)];
}

/** The line `library` prints between the library and the program
 * (`scripts/metamorph/src/library.rs`'s `MAIN`). */
const libraryEnds = "\n// metamorph: main\n";

/** The library crate `library`'s program uses, by its name. */
export const libraryCrate = "metamorph_lib";

/** Whether a native run of `rewrite` ended in a panic the rewrite made
 * itself, not the case: two borrows of the `RefCell` an `rc-refcell` local
 * is held in that overlap, which the case's own place never checked. Such
 * a rewrite is no variant, as one rustc rejects is not. */
export function overlapping(rewrite: Rewrite, outcome: unknown): boolean {
  if (rewrite !== "rc-refcell" || typeof outcome !== "object" || outcome === null || !("panic" in outcome)) return false;
  const message = String((outcome as { panic: unknown }).panic);
  return message === "RefCell already borrowed" || message === "RefCell already mutably borrowed";
}

/** Why a case is no case of the net, or `undefined`: what its directives
 * expect that a rewrite's run can't carry, a compile error, a refusal, an
 * abort, another target's `usize`. */
export function apart(source: string): string | undefined {
  const want = directives(source);
  if (typeof want === "string") return want;
  if (want.kind !== "run-pass" && want.kind !== "run-fail") return want.kind;
  return undefined;
}

/** The cases a rewrite may run, in name order, with their editions. */
export function rewritable(): { name: string; edition: string; native: "host" | "wasm32" }[] {
  return cases.flatMap((name) => {
    const source = readFileSync(join(corpus, name), "utf8");
    const want = directives(source);
    return apart(source) === undefined && typeof want !== "string" ? [{ name, edition: want.edition, native: want.native }] : [];
  });
}

/** A seeded generator of numbers in [0, 1): mulberry32. */
function random(seed: number): () => number {
  let a = seed >>> 0;
  return () => {
    a = (a + 0x6d2b79f5) >>> 0;
    let t = a;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

/** Whether `name` is in a push's sample, one case in `of` by its name's
 * hash, so a case added changes only whether it is (as the neighbour
 * shuffle's groups are, #853). */
export function sampled(name: string, of = 128): boolean {
  return Math.floor(random(Number(BigInt.asUintN(32, Bun.hash(name) as bigint)))() * of) === 0;
}

/** A rewrite of a case the shuffle found going wrong, listed under its
 * issue: its builds, its rewrite and its case. */
export type Known = { issue: string; builds: string[]; rewrite: string; name: string };

export const knownFile = join(import.meta.dir, "metamorphic-known.txt");

/** test/metamorphic-known.txt's rewrites, or why a line isn't one:
 * `<builds> <rewrite> <case>` under an `#<issue> <title>`. */
export function readKnown(text = existsSync(knownFile) ? readFileSync(knownFile, "utf8") : ""): Known[] {
  const known: Known[] = [];
  let issue: string | undefined;
  for (const line of text.split("\n")) {
    if (line === "" || line.startsWith("//")) continue;
    // An issue's, `#1032`, or a decided difference's, by its number in the
    // guide's list, `#difference 39` (ADR 0262).
    const heading = /^#(\d+|difference \d+) /.exec(line);
    if (heading) {
      issue = heading[1];
      continue;
    }
    const words = line.split(" ");
    const [name, rewrite] = [words.at(-1)!, words.at(-2)!];
    const builds = words.slice(0, -2);
    if (issue === undefined || !(rewrites as readonly string[]).includes(rewrite) || !name.endsWith(".rs") || builds.length === 0 || builds.some((b) => b !== "plain" && b !== "std")) {
      throw new Error(`test/metamorphic-known.txt: \`${line}\` isn't \`<builds> <rewrite> <case.rs>\` under an \`#<issue>\` or a \`#difference <number>\``);
    }
    known.push({ issue, builds, rewrite, name });
  }
  return known;
}

/** A loud rewrite's line, `~loud 20 drop Circle(3)`: its count, and what it
 * says. */
const loudLine = /^(~loud \d+ )(.*)$/;

/** Whether the JS's run, `ran`, agrees with native Rust's, `runs`, its
 * first and its runs again, up to the order of a hash table's drops, which
 * std leaves to `RandomState`'s seed and rust-js gives in insertion order
 * (difference 5). Each drop is a group, its loud line and the lines its
 * `Drop` prints up to the next loud line; where some of native's runs put
 * other lines than its first does, each run of such groups in a row is a
 * hash table's iteration only where every group of it, the JS's too, is a
 * loud drop's, and the JS's are then the same groups, in any order, each
 * loud count in its place. Every other line, standard error and the
 * outcome, as `agree` asks: so a drop native's runs all put in one place,
 * one no hash table orders, is wrong anywhere else, and so is a drop given
 * another's lines. Of a hash table of four, 24 orders, which `nativeTries`
 * runs again seldom hit (#1453). */
export function agreesUpToHashOrder(runs: Run[], ran: Run): boolean {
  const [first] = runs;
  if (first === undefined || typeof ran.outcome === "string") return false;
  const sameRun = (run: Run) =>
    typeof run.outcome !== "string" && same(run.outcome, ran.outcome as never) && run.bytes.stderr.equals(ran.bytes.stderr);
  if (!runs.every(sameRun)) return false;
  const all = [ran, ...runs].map((run) => run.stdout.split("\n"));
  const [js, native] = [all[0], all[1]];
  if (all.some((l) => l.length !== js.length)) return false;
  // Where each group starts, a loud line, in every run alike.
  const starts = js.flatMap((line, i) => (loudLine.test(line) ? [i] : []));
  if (all.some((l) => l.flatMap((line, i) => (loudLine.test(line) ? [i] : [])).join() !== starts.join())) return false;
  const bounds = [0, ...starts.filter((s) => s > 0), js.length];
  const groups = bounds.slice(0, -1).map((from, k) => [from, bounds[k + 1]] as const);
  const varies = ([from, to]: readonly [number, number]) =>
    all.slice(1).some((l) => l.slice(from, to).join("\n") !== native.slice(from, to).join("\n"));
  for (let g = 0; g < groups.length; ) {
    if (!varies(groups[g])) {
      const [from, to] = groups[g];
      if (js.slice(from, to).join("\n") !== native.slice(from, to).join("\n")) return false;
      g++;
      continue;
    }
    let end = g;
    while (end < groups.length && varies(groups[end])) end++;
    // Past the block's last drop, what every run prints in place, after the
    // table's iteration, not the drop's: compared where it is.
    const block = groups.slice(g, end).map(([from, to]) => [from, to] as [number, number]);
    const last = block[block.length - 1];
    const constant = (i: number) => all.every((l) => l[i] === native[i]);
    let tail = last[1];
    while (tail - 1 > last[0] && constant(tail - 1)) tail--;
    if (js.slice(tail, last[1]).join("\n") !== native.slice(tail, last[1]).join("\n")) return false;
    last[1] = tail;
    for (const lines of all) {
      if (!block.every(([from]) => /^~loud \d+ drop /.test(lines[from]))) return false;
    }
    const count = (line: string) => line.match(loudLine)![1];
    if (block.some(([from]) => count(js[from]) !== count(native[from]))) return false;
    const contents = (lines: string[]) =>
      block
        .map(([from, to]) => [lines[from].match(loudLine)![2], ...lines.slice(from + 1, to)].join("\n"))
        .sort()
        .join("\n\u0000");
    if (all.some((lines) => contents(lines) !== contents(native))) return false;
    g = end;
  }
  return true;
}
