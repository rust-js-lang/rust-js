// Which rows of the iterator matrix (ADR 1090) and of the conformance table
// (ADR 1092) a change to the iterators' code runs before it lands (#818):
// most of the iterators' regressions were one fix breaking another adapter
// or consumer, which the fixed slices didn't sample and the nightly's full
// run found after it landed (#721, #757, #810, #823, #833, #834).
//
// Each hunk the change makes to the iterators' code (`iteratorPaths`, but
// the runtime's generated module and the tests), with the head of the
// function it's in, names adapters and consumers: in code, its identifiers;
// in a comment, what it quotes in backticks. A name is one the generator
// gives its adapters and consumers, folded onto an identifier's words
// (`$taking`, `StepBy`, `next_back`). The rows of each adapter a hunk names
// run, whatever their consumer; of a hunk that names none, those of each
// consumer it names. `$given`, generic code's, names the generic sources.
// Within `budget` rows each runs; past it, `pairs` of them, as many of each
// pairing of an adapter with a consumer as fit, but the rows of what a hunk
// removes, which decides for every row of what it names, each (#823). A
// hunk in what every adapter shares (`shared`), or that names none, runs
// `pairs` of all of them.
//
//   bun scripts/iteratorRows.ts [base [head]]   what the change picks, and why
import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { allCases, conformanceCases, pairs, type Case, type Table } from "../test/iterator-matrix";

const root = join(import.meta.dir, "..");

/** What every adapter shares, whose change decides for all of them: the
 * runtime's protocol, its steps passed on, closing and consuming, and the
 * lowering's iterator value and its marks (ADR 1092's owners): files, and
 * the functions whose body a hunk is in. */
const shared = {
  files: ["src/runtime/forward.js", "src/runtime/closing.js", "src/lower/iterators/value.rs"],
  functions: ["$consume", "$consumed", "$closing", "$closed", "$forward", "$given", "$givenDrop", "$knowing", "$held", "$iterator"],
};

/** The rows each table runs within: some 3 minutes of the matrix's, at
 * 110 rows a second in 12 parts, and of the table's (2026-11-06). */
// The points table (#817) has no rows a change picks yet: #818 decides.
const budget: Record<Exclude<Table, "points">, number> = { matrix: 20_000, conformance: 6_000 };

/** What names nothing of its own: the runtime's module, generated of
 * src/runtime, and the tests and scripts, which run as they are. */
const unnamed = (file: string) => file === "runtime/index.js" || file.startsWith("test/") || file.startsWith("scripts/");

const keywords = new Set(
  "as async await break case catch const continue crate default do dyn else enum export extern false fn for function if impl import in let loop match mod move mut new of pub ref return self static struct super this throw trait true try type typeof undefined unsafe use var where while yield".split(
    " ",
  ),
);

/** An identifier's words, lower case: `$stepByEnds` is `step by ends`. */
const words = (identifier: string) =>
  identifier
    .replace(/([a-z0-9])([A-Z])/g, "$1 $2")
    .split(/[^A-Za-z0-9]+/)
    .filter(Boolean)
    .map((word) => word.toLowerCase());

/** Whether `name`'s words are some of `identifier`'s, one after another,
 * each as it's inflected: `take` of `taking`, `fuse` of `fused`. */
function names(identifier: string[], name: string[]): boolean {
  const stem = (word: string) => (word.length > 3 && word.endsWith("e") ? word.slice(0, -1) : word);
  return identifier.some((_, at) => name.every((word, i) => identifier[at + i]?.startsWith(stem(word))));
}

/** The identifiers a line says: of a comment, what it quotes; of code,
 * each, but the language's own words. */
function identifiers(line: string): string[] {
  const comment = /^\s*(\/\/|\/\*|\*)/.test(line);
  const said = comment ? [...line.matchAll(/`([^`]+)`/g)].map((m) => m[1]).join(" ") : line;
  return [...said.matchAll(/\$?[A-Za-z_][A-Za-z0-9_$]*/g)].map((m) => m[0]).filter((id) => !keywords.has(id));
}

export type Hunk = { file: string; at: number; added: string[]; removed: string[]; head: string };

/** The change's hunks to the iterators' code from `base` to `head` (the
 * working tree where it's none), each with the head of the function it's
 * in. */
function hunks(base: string, head: string | undefined, files: string[]): Hunk[] {
  const range = head === undefined ? [base] : [base, head];
  const diff = Bun.spawnSync(["git", "diff", "-U0", ...range, "--", ...files], { cwd: root }).stdout.toString();
  const out: Hunk[] = [];
  let file = "";
  for (const line of diff.split("\n")) {
    const named = /^\+\+\+ (?:b\/(.*)|\/dev\/null)$/.exec(line);
    if (named) {
      file = named[1] ?? "";
      continue;
    }
    if (line.startsWith("--- ")) continue;
    const at = /^@@ -\d+(?:,\d+)? \+(\d+)(?:,\d+)? @@/.exec(line);
    if (at) out.push({ file, at: Number(at[1]), added: [], removed: [], head: "" });
    else if (out.length > 0 && line.startsWith("+")) out.at(-1)!.added.push(line.slice(1));
    else if (out.length > 0 && line.startsWith("-")) out.at(-1)!.removed.push(line.slice(1));
  }
  const isHead = (line: string) => /^\s*(export\s+)?(async\s+)?function\s|^\s*(pub(\([^)]*\))?\s+)?(const\s+)?fn\s/.test(line);
  for (const hunk of out) {
    if (hunk.file === "") continue;
    const text =
      head === undefined
        ? existsSync(join(root, hunk.file)) && readFileSync(join(root, hunk.file), "utf8")
        : Bun.spawnSync(["git", "show", `${head}:${hunk.file}`], { cwd: root }).stdout.toString();
    if (!text) continue;
    const lines = text.split("\n");
    let at = Math.min(hunk.at, lines.length) - 1;
    while (at > 0 && !isHead(lines[at])) at--;
    if (isHead(lines[at])) hunk.head = lines[at];
  }
  return out.filter((hunk) => !unnamed(hunk.file));
}

/** What a run of a table is given: `ITERATOR_MATRIX`'s setting, how many
 * rows it runs, and why. */
export type Picked = { table: Table; setting: string; rows: number; why: string[] };

const escaped = (name: string) => name.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");

/** What a hunk names of a table's `adapters` and `consumers`: the
 * adapters, or, where it names none, the consumers; whether it names
 * generic code's (`$given`); whether it's in what every adapter shares;
 * and whether it takes away more than it puts back. */
export function named(hunk: Hunk, adapters: Set<string>, consumers: Set<string>) {
  const ids = [...hunk.added, ...hunk.removed, hunk.head].flatMap(identifiers);
  const idWords = ids.map(words);
  const of = (list: Set<string>, types: boolean) =>
    [...list].filter((name) => ids.some((id, i) => (types || !/^[A-Z]/.test(id)) && names(idWords[i], words(name)))).sort();
  const a = of(adapters, true);
  return {
    adapters: a,
    consumers: a.length > 0 ? [] : of(consumers, false),
    generic: ids.some((id) => id.startsWith("$given")),
    shared: shared.files.includes(hunk.file) || identifiers(hunk.head).some((id) => shared.functions.includes(id)),
    removes: hunk.removed.filter((line) => line.trim() !== "").length > hunk.added.filter((line) => line.trim() !== "").length,
  };
}

/** The rows of a table's cases each name is of: of adapters, those whose
 * chain has it; of consumers, those whose consumer it begins; generic, the
 * generic sources'. */
export function pattern(adapters: Set<string>, consumers: Set<string>, generic: boolean): RegExp | undefined {
  const alt = (list: Set<string>) => [...list].sort().map(escaped).join("|");
  const parts = [
    adapters.size > 0 ? `^[^|]+\\| (?:[^|]*\\.)?(?:${alt(adapters)})(?: [^.|]+)?(?:\\.[^|]*)? \\|` : undefined,
    consumers.size > 0 ? `^[^|]+\\|[^|]+\\| (?:${alt(consumers)})(?: [^|]+)? \\|` : undefined,
    generic ? "^generic " : undefined,
  ].filter((part) => part !== undefined);
  return parts.length === 0 ? undefined : new RegExp(parts.join("|"));
}

/** What the change from `base` to `head` runs of each table, in
 * `iteratorPaths`' `files`. */
export function pick(base: string, head: string | undefined, files: string[]): Picked[] {
  const found = hunks(base, head, files);
  if (found.length === 0) return [];
  return (["matrix", "conformance"] as const).flatMap((table) => {
    const cases: Case[] = table === "matrix" ? allCases() : conformanceCases();
    const adapters = new Set(cases.flatMap((c) => c.chain.map((a) => a.name.split(" ")[0])));
    const consumers = new Set(cases.map((c) => c.consumer.name.split(" ")[0]));
    const why: string[] = [];
    const picked = { adapters: new Set<string>(), consumers: new Set<string>(), generic: false };
    const removing = new Set<string>();
    let everyAdapter = false;
    for (const hunk of found) {
      const { adapters: a, consumers: c, generic, shared: sharedHunk, removes } = named(hunk, adapters, consumers);
      // What's taken away more than put back decides for each row of what
      // it names: every one runs (#823).
      for (const name of a) (removes ? removing : picked.adapters).add(name);
      for (const name of c) picked.consumers.add(name);
      picked.generic ||= generic;
      const none = a.length === 0 && c.length === 0 && !generic;
      if (sharedHunk || none) everyAdapter = true;
      const said = [...a, ...c, ...(generic ? ["generic"] : [])].join(", ") || "none";
      why.push(`${hunk.file}:${hunk.at}: ${said}${removes && a.length > 0 ? " (removes: each row)" : ""}${sharedHunk ? " (shared: pairs)" : none ? " (names none: pairs)" : ""}`);
    }
    const picks: Picked[] = [];
    const whole = pattern(removing, new Set(), false);
    if (whole) picks.push({ table, setting: `/${whole.source}/`, rows: cases.filter((c) => whole.test(c.id)).length, why: [] });
    const some = pattern(picked.adapters, picked.consumers, picked.generic);
    if (some) {
      const matching = cases.filter((c) => some.test(c.id) && !whole?.test(c.id));
      let setting = `/${some.source}/`;
      let rows = matching.length;
      if (rows > budget[table]) {
        // As many of each pairing as fit.
        let per = 60;
        while (per > 1 && pairs(matching, per).length > budget[table]) per = Math.floor(per * 0.8);
        setting = `pairs:${per}/${some.source}/`;
        rows = pairs(matching, per).length;
        why.push(`past ${budget[table]} rows (${matching.length}): ${per} of each pairing; ITERATOR_MATRIX='/${some.source}/' runs each`);
      }
      picks.push({ table, setting, rows, why: [] });
    }
    if (everyAdapter) picks.push({ table, setting: "pairs", rows: pairs(cases, table === "matrix" ? 60 : 10).length, why: [] });
    if (picks.length > 0) picks[0].why = why;
    return picks;
  });
}

if (import.meta.main) {
  const [given, head] = process.argv.slice(2);
  const base = given ?? Bun.spawnSync(["git", "merge-base", "HEAD", "origin/main"], { cwd: root }).stdout.toString().trim();
  const { iteratorPaths } = await import("./iteratorSlice");
  const range = head === undefined ? [base] : [base, head];
  const files = Bun.spawnSync(["git", "diff", "--name-only", ...range], { cwd: root }).stdout.toString().split("\n").filter(Boolean);
  const globs = iteratorPaths.map((path) => new Bun.Glob(path));
  for (const p of pick(base, head, files.filter((file) => globs.some((glob) => glob.match(file))))) {
    for (const line of p.why) console.log(`  ${line}`);
    console.log(`${p.table}: ${p.rows} rows, ITERATOR_MATRIX='${p.setting}'`);
  }
}
