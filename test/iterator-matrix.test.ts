// The iterator conformance matrix (ADR 1090): each source, chain of
// adapters, consumer and panic site `iterator-matrix.ts` makes, its items
// and accumulators printing as they're dropped, run natively and as JS in
// both builds, the recognizers' and std's structs alone (`--std-iterators`,
// ADR 0908), each case's lines compared. Cases run in batches, one
// program each; a case rust-js refuses is its answer, and left out of its
// batch's JS.
//
//   bun test test/iterator-matrix.test.ts                                        # its slice, of every source, adapter, consumer and panic site
//   ITERATOR_MATRIX=all RUST_JS_CASES=1/8 bun test test/iterator-matrix.test.ts  # an eighth of every batch
//   ITERATOR_MATRIX=#651 bun test test/iterator-matrix.test.ts                   # the batches holding #651's known cases
//   ITERATOR_MATRIX='/step_by/' bun test test/iterator-matrix.test.ts           # the cases whose ids match, in batches of their own, a change's (#818)
//   ITERATOR_MATRIX=pairs bun test test/iterator-matrix.test.ts                  # a sample of each adapter with each consumer, of a change to what they share
//   ITERATOR_MATRIX='pairs:20/step_by/' bun test test/iterator-matrix.test.ts    # 20 of each pairing, of the cases whose ids match
//   ITERATOR_TABLE=conformance bun test test/iterator-matrix.test.ts             # the conformance table's (ADR 1092), each likewise
//   ITERATOR_TABLE=points ITERATOR_MATRIX=all bun test test/iterator-matrix.test.ts  # its cases at each panic point (#817)
//   ITERATOR_TABLE=unwind ITERATOR_MATRIX=all bun test test/iterator-matrix.test.ts  # operations beyond iterators at each panic point (#854)
//   ITERATOR_TABLE=format ITERATOR_MATRIX=all bun test test/iterator-matrix.test.ts  # every format spec of each value (#941)
//   ITERATOR_TABLE=numbers ITERATOR_MATRIX=all bun test test/iterator-matrix.test.ts # every number operation of each type's boundary values (#940)
//   ITERATOR_TABLE=async bun test test/iterator-matrix.test.ts                   # panic points across `.await` (#942)
//   ITERATOR_TABLE=consumers bun test test/iterator-matrix.test.ts               # each std method that takes an iterator by value (test/consumer-net.ts)
//   ITERATOR_MATRIX_REPORT=1 ...                                                 # and each case refused, and why
//   bun scripts/iteratorSlice.ts run                                             # both tables' slices, in parts at once (#731)
//
// A case that doesn't run as Rust does fails its batch, but those
// test/iterator-matrix-known.txt lists, each under its issue, which only
// shrinks: one that runs as Rust does fails until it's taken off. Each
// failure is written to target/iterator-matrix/, a program of that case
// alone.

import { beforeAll, expect, test } from "bun:test";
import { appendFileSync, mkdirSync, writeFileSync } from "node:fs";
import { basename, join } from "node:path";

import { builds, failures, fixedFile, knownCases, listedBuild, ofTable, readKnown, segments, table, tableOf, version, type Row } from "./iterator-matrix";
import { same, type Outcome } from "./oracle";
import { aborted, compileJs, runJs, runNative, runtimes, show, type Run } from "./programs";
import { buildCompiler, fixture } from "./support";
import { shard } from "../scripts/shard";
import { partSetting } from "./part-settings";

// What rustc's told of a crate of rust-js's bindings the table's programs
// use, which its `prepare` builds (#942).
let rustc: string[] = [];
beforeAll(() => {
  buildCompiler();
  rustc = of.prepare?.() ?? [];
}, 900_000);

const which = partSetting("ITERATOR_MATRIX");
const issue = /^#(\d+)$/.exec(which ?? "")?.[1];
const picking = /^(pairs(?::(\d+))?)?(?:\/(.+)\/)?$/.exec(which ?? "");
const paired = picking?.[1] === undefined ? undefined : Number(picking[2] ?? (table === "matrix" ? 60 : 10));
const rows = picking?.[3];
if (which !== undefined && which !== "all" && issue === undefined && paired === undefined && rows === undefined) {
  throw new Error(
    `ITERATOR_MATRIX=${which}: \`all\`, \`#<issue>\` for the batches of an issue's known cases, \`/<regex>/\` for the cases whose ids match, \`pairs[:<n>]\` for <n> of each adapter with each consumer, of those a pattern after it matches, or unset for its slice`,
  );
}
const list = readKnown();
const known = knownCases(list);
const of = tableOf(table);
const every = of.cases();
const all = of.batches(every);
// A slice is whole batches, so each case is in the same program as in a
// run of all of them, and judged the same.
const indexes = all.map((_, i) => i);
const perBatch = all[0].length;
const ofIssue = [...known]
  .filter(([name, n]) => String(n) === issue && ofTable(name.split(" ")[0]))
  .map(([name]) => Math.floor(Number(name.split(" ")[1]) / perBatch));
// Of a pattern, the cases whose ids it matches, in batches of their own:
// each is judged by itself, by its number, so what it runs costs as many
// cases as it names, where the batches holding them would be nearly all,
// each of which holds every consumer (#818).
// Of `pairs`, so many of each pairing of an adapter with a consumer, 60 of
// the matrix's and 10 of the table's, that they're some 200 batches and
// some 100 (#818).
const matching = rows === undefined ? every : every.filter((c) => new RegExp(rows).test(c.id));
if (paired !== undefined && of.pairs === undefined) throw new Error(`ITERATOR_MATRIX=${which}: the ${table} table has no pairings of an adapter with a consumer`);
const picked = paired !== undefined ? of.pairs!(matching, paired) : rows !== undefined ? matching : undefined;
const run = picked === undefined ? all : of.batches(picked);
const chosen =
  which === "all" || picked !== undefined
    ? run.map((_, i) => i)
    : issue !== undefined
      ? [...new Set(ofIssue)].sort((a, b) => a - b)
      : of.slice(all);
if (issue !== undefined && chosen.length === 0) throw new Error(`ITERATOR_MATRIX=${which}: no known case is under #${issue}`);
const [part, parts] = (process.env.RUST_JS_CASES ?? "1/1").split("/").map(Number);
const mine = shard(
  chosen.map((i) => String(i).padStart(6, "0")),
  part,
  parts,
).map(Number);
// Its numbers are of the cases the generator made when it was made: of
// another generator, another list, each issue's cases found again.
// The list is both tables'; this run judges its own table's rows.
test("the known failures are of these cases, each one a case", () => {
  const mineListed = [...known.keys()].filter((name) => ofTable(name.split(" ")[0]));
  if (mineListed.length > 0) expect(list.versions[table], `the ${table}'s cases changed: make its rows of test/iterator-matrix-known.txt again`).toBe(version(every));
  const names = new Set(every.flatMap((c) => builds.map(([build]) => `${listedBuild(build)} ${c.n}`)));
  expect(mineListed.filter((name) => !names.has(name))).toEqual([]);
});

// A refusal its error points at the case of (`refusedCases`), `own`; or one
// told apart only by compiling the case alone, which a refusal of the
// program's prelude is too (#854).
type Verdict = { kind: "same" } | { kind: "refused"; why: string; own?: boolean } | { kind: "differs"; js: string; why?: string } | { kind: "crashed"; why: string };

/** Of a batch of one whose native run aborted (#847): the same where the
 * JS's aborted too, `{ exit: 134 }`, after it printed what native Rust did;
 * else how it differs. */
export function abortVerdict(js: { outcome: Outcome | string; printed: string }, native: string): Verdict {
  if (typeof js.outcome === "string" || !same(js.outcome, aborted)) {
    return { kind: "differs", js: js.printed, why: `native Rust aborted, where the run ended ${show(js.outcome)}` };
  }
  return js.printed === native ? { kind: "same" } : { kind: "differs", js: js.printed };
}

/** Which case each line of `text` is in, from `program`'s `fn case_<n>`. */
function caseAtLine(text: string, cases: Row[]): (line: number) => Row | undefined {
  const starts: [number, number][] = [];
  text.split("\n").forEach((line, i) => {
    const n = /^fn case_(\d+)\(/.exec(line)?.[1];
    if (n !== undefined) starts.push([i + 1, Number(n)]);
  });
  return (line) => {
    const at = starts.filter(([start]) => start <= line).at(-1);
    return at === undefined ? undefined : cases[at[1]];
  };
}

/** The cases of `cases` each refusal of rust-js's `stderr` points into. */
function refusedCases(stderr: string, text: string, cases: Row[], file: string): Map<Row, string> {
  const at = caseAtLine(text, cases);
  const out = new Map<Row, string>();
  let error = "";
  for (const line of stderr.split("\n")) {
    if (line.startsWith("error")) error = line;
    // Where it points, of `file`, its path as rust-js remapped it.
    const where = /--> (.*):(\d+):\d+$/.exec(line);
    if (!where || basename(where[1]) !== basename(file)) continue;
    const c = at(Number(where[2]));
    if (c && !out.has(c)) out.set(c, error);
  }
  return out;
}

/** Each case of `cases`, compiled with `flags` and run, against what
 * native Rust printed of it. */
function judge(cases: Row[], native: Map<string, string>, dir: string, build: string, flags: string[], aborts = false): Map<Row, Verdict> {
  const verdicts = new Map<Row, Verdict>();
  mkdirSync(dir, { recursive: true });
  let pending = cases;
  for (let round = 0; pending.length > 0; round++) {
    const text = of.program(pending);
    const file = join(dir, `${build}-${round}.rs`);
    const at = join(dir, `${build}-${round}`);
    writeFileSync(file, text);
    mkdirSync(at, { recursive: true });
    const compiled = compileJs(file, at, "2024", false, {}, flags, rustc);
    if ("error" in compiled) {
      const refused = compiled.kind === "rejected" ? refusedCases(compiled.error, text, pending, file) : new Map<Row, string>();
      if (refused.size === 0) {
        // What can't be told apart by where it points, a crash: each
        // half on its own, until it's one case.
        if (pending.length === 1) {
          verdicts.set(pending[0], compiled.kind === "rejected" ? { kind: "refused", why: compiled.reason } : { kind: "crashed", why: compiled.reason });
          return verdicts;
        }
        const half = Math.ceil(pending.length / 2);
        [pending.slice(0, half), pending.slice(half)].forEach((cases, i) => {
          for (const [c, v] of judge(cases, native, join(at, `half-${i}`), build, flags)) verdicts.set(c, v);
        });
        return verdicts;
      }
      for (const [c, why] of refused) verdicts.set(c, { kind: "refused", why, own: true });
      pending = pending.filter((c) => !refused.has(c));
      continue;
    }
    const [runtime, cmd] = runtimes[0];
    const ran = runJs(cmd, compiled.js, at, runtime);
    const js = segments(ran.stdout);
    if (aborts) {
      const [c] = pending;
      verdicts.set(c, abortVerdict({ outcome: ran.outcome, printed: js.get(c.id) ?? "" }, native.get(c.id) ?? ""));
      return verdicts;
    }
    const ended = typeof ran.outcome !== "string" && "value" in ran.outcome;
    // A run that ended early ended in the last case it began: those after
    // it run again, without it.
    const began = pending.filter((c) => js.has(c.id));
    const last = ended ? undefined : began.at(-1);
    for (const c of began) {
      const printed = js.get(c.id)!;
      if (c === last) {
        const why = `the run ended ${show(ran.outcome)}${ran.stderr ? `, printing to stderr\n${ran.stderr.slice(0, 2000)}` : ""}`;
        verdicts.set(c, { kind: "differs", js: printed, why });
      } else {
        verdicts.set(c, printed === native.get(c.id) ? { kind: "same" } : { kind: "differs", js: printed });
      }
    }
    if (ended || last === undefined) {
      const why = ended ? "it never ran" : `the run ended ${show(ran.outcome)} before any case`;
      for (const c of pending.filter((c) => !verdicts.has(c))) verdicts.set(c, { kind: "differs", js: "", why });
      return verdicts;
    }
    pending = pending.filter((c) => !verdicts.has(c));
  }
  return verdicts;
}

for (const index of mine) {
  const batch = run[index];
  test(`batch ${index + 1} of ${run.length}${picked === undefined ? "" : ` of ${which}`}: ${batch.length} cases`, () => {
    const dir = fixture(`iterator-${table}-${picked === undefined ? "" : "picked-"}${index}`);
    const file = join(dir, "native.rs");
    writeFileSync(file, of.program(batch));
    const native: Run | string = runNative(file, dir);
    if (typeof native === "string") throw new Error(`the matrix made a program rustc rejects:\n${native.slice(0, 4000)}`);
    // A batch of one may abort, as Rust does where a drop panics as a panic
    // unwinds: what it printed till then is what the JS must print, then
    // abort too (#847). Any other run must end.
    const aborts = batch.length === 1 && typeof native.outcome !== "string" && same(native.outcome, aborted);
    if (!aborts && (typeof native.outcome === "string" || !("value" in native.outcome))) {
      throw new Error(`a native run of the matrix ended ${show(native.outcome)}:\n${native.stderr.slice(0, 2000)}`);
    }
    const expected = segments(native.stdout);
    const problems: string[] = [];
    let refused = 0;
    for (const [build, flags] of builds) {
      for (const [c, verdict] of judge(batch, expected, dir, build, flags, aborts)) {
        // As the one list names it, of either table.
        const listed = listedBuild(build);
        const name = `${listed}: ${c.id}`;
        const issue = known.get(`${listed} ${c.n}`);
        if (verdict.kind === "refused" && !verdict.own) refused++;
        // Each refusal, and why, of a run that reports them.
        if (verdict.kind === "refused" && process.env.ITERATOR_MATRIX_REPORT) console.log(`refused ${name}: ${verdict.why.split("\n")[0]}`);
        if (verdict.kind === "same" || verdict.kind === "refused") {
          if (issue === undefined) continue;
          // What `bun test/iterator-matrix.ts take-off` takes off the list.
          mkdirSync(failures, { recursive: true });
          appendFileSync(fixedFile, `${listed} ${c.n}\n`);
          problems.push(`${name} (${listed} ${c.n}) is no wrong answer now (${verdict.kind}): take it off test/iterator-matrix-known.txt (#${issue})`);
          continue;
        }
        // Each that differs, its program alone and what each printed, a
        // known one's too, for its fixer.
        mkdirSync(failures, { recursive: true });
        const repro = join(failures, `${name.replaceAll(/[^A-Za-z0-9]+/g, "-")}.rs`);
        writeFileSync(repro, of.program([c]));
        writeFileSync(repro.replace(/\.rs$/, ".json"), JSON.stringify({ name, build: listed, n: c.n, issue, verdict, native: expected.get(c.id) }, null, 2));
        if (issue !== undefined) continue;
        const said =
          verdict.kind === "crashed"
            ? `rust-js crashed: ${verdict.why}`
            : `${verdict.why ? `${verdict.why}\n` : ""}native printed\n${expected.get(c.id)}\nwhere the JS printed\n${verdict.js}`;
        problems.push(problems.length < 3 ? `${name}, in ${repro}:\n${said}` : `${name}, in ${repro}`);
      }
    }
    if (refused > 0 && process.env.ITERATOR_MATRIX_REPORT) console.log(`batch ${index + 1}: ${refused} refused`);
    // Every case refused, none by an error that points at it, is the
    // program's own code refused, its prelude's, where each case would pass
    // by refusing (#854). A batch a pattern picks may be all of one
    // refused family, each refused where it's written (#945's rows).
    if (batch.length > 1 && refused === batch.length * builds.length) problems.push("every case of the batch is refused: its prelude is, not a case");
    expect(problems.join("\n\n")).toBe("");
  }, 900_000);
}

// A batch of one that aborts natively passes where the JS aborts too, after
// printing the same, and fails where only one side aborts (#847).
test("a batch of one that aborts is judged by its abort", () => {
  const printed = "case x\ndrop 3\ndrop 4\n";
  expect(abortVerdict({ outcome: aborted, printed }, printed)).toEqual({ kind: "same" });
  expect(abortVerdict({ outcome: { value: null }, printed }, printed).kind).toBe("differs");
  expect(abortVerdict({ outcome: { panic: "drop again" }, printed }, printed).kind).toBe("differs");
  expect(abortVerdict({ outcome: aborted, printed: "case x\ndrop 3\n" }, printed).kind).toBe("differs");
  // And a native run that doesn't abort is judged as every other: the JS's
  // abort there ends the run early, which `judge` holds a difference.
  expect(same(aborted, { value: null })).toBe(false);
});
