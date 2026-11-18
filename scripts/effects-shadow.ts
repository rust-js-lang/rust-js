// The effects query's shadow over the corpus (#2112 B, step 2): each case
// compiled with `--intervals-shadow`, its `effects-forgets` and
// `effects-keeps` lines tallied by what each is at and the fact's kind,
// each group with a case that shows it.
//
//   effects-forgets: a fact the walk's forgetting rules keep that the query
//     would forget, an output move, or a fact the rules kept past a write.
//   effects-keeps: one the rules forget that the query would keep. Where the
//     rules forget at a borrow, `assign ref`/`raw`, the query forgets at the
//     write the borrow makes; anywhere else it's a write the query can't
//     see, its own hole, which must be none before the transfer asks it.
//
//   bun scripts/effects-shadow.ts                 # the corpus, a part a core
//   bun scripts/effects-shadow.ts a.rs b.rs       # these cases
//   bun scripts/effects-shadow.ts --json=out.json # the tallies written too

import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { basename, join } from "node:path";
import { machine } from "./shard";

const root = join(import.meta.dir, "..");
const corpus = join(root, "test", "corpus");
const compiler = process.env.RUST_JS_COMPILER ?? join(root, "target", "debug", "rust-js");
const args = process.argv.slice(2);
const json = args.find((arg) => arg.startsWith("--json="))?.slice("--json=".length);
const named = args.filter((arg) => !arg.startsWith("--"));
const cases = named.length > 0 ? named.map((name) => basename(name)) : [...new Bun.Glob("*.rs").scanSync(corpus)].sort();
const out = join(tmpdir(), `effects-shadow-${process.pid}`);
mkdirSync(out, { recursive: true });

/** A line of the shadow: its kind, function, where, fact and what it's at. */
type Line = { kind: string; owner: string; at: string; fact: string; what: string; case: string };

/** `name`'s shadow lines, or `undefined` of a case rust-js refuses. */
async function shadow(name: string): Promise<Line[] | undefined> {
  const stem = basename(name, ".rs");
  const lines = join(out, `${stem}.txt`);
  const p = Bun.spawn([compiler, join(corpus, name), "-o", join(out, `${stem}.js`), "--intervals-shadow", lines], { stdout: "ignore", stderr: "ignore" });
  if ((await p.exited) !== 0) return undefined;
  return readFileSync(lines, "utf8")
    .split("\n")
    .filter((line) => line.startsWith("effects-"))
    .map((line) => {
      const [kind, owner, at, fact, what] = line.split("\t");
      return { kind: kind!, owner: owner!, at: at!, fact: fact!, what: what ?? "", case: name };
    });
}

const all: Line[] = [];
let next = 0;
let refused = 0;
await Promise.all(
  Array.from({ length: machine().cores }, async () => {
    for (let at = next++; at < cases.length; at = next++) {
      const lines = await shadow(cases[at]!);
      if (lines === undefined) refused++;
      else all.push(...lines);
    }
  }),
);

/** A fact's kind, its first words: `interval`, `below`, `length`. */
const factKind = (fact: string) => fact.split(" ")[0]!;
/** What a line is at, a call's callee's crate kept: std's or the crate's. */
const whatKind = (what: string) => (what.startsWith("call ") ? (/^call (std|core|alloc)::|^call <(std|core|alloc)::/.test(what) ? "call std" : what === "call unresolved" ? "call unresolved" : "call crate") : what);

const report: Record<string, unknown> = { cases: cases.length, refused };
for (const kind of ["effects-forgets", "effects-keeps"]) {
  const lines = all.filter((line) => line.kind === kind);
  const groups = new Map<string, { count: number; cases: Set<string>; example: Line }>();
  for (const line of lines) {
    const key = `${whatKind(line.what)} · ${factKind(line.fact)}`;
    const group = groups.get(key) ?? { count: 0, cases: new Set(), example: line };
    group.count++;
    group.cases.add(line.case);
    groups.set(key, group);
  }
  const sorted = [...groups.entries()].sort((a, b) => b[1].count - a[1].count);
  const caseCount = new Set(lines.map((line) => line.case)).size;
  console.log(`\n${kind}: ${lines.length} lines in ${caseCount} cases`);
  for (const [key, group] of sorted.slice(0, 40)) {
    const e = group.example;
    console.log(`  ${String(group.count).padStart(7)}  ${String(group.cases.size).padStart(5)} cases  ${key}\n           e.g. ${e.case} ${e.owner} ${e.at.replace(/^.*\/test\/corpus\//, "")}: ${e.fact} (${e.what})`);
  }
  report[kind] = {
    lines: lines.length,
    cases: caseCount,
    groups: sorted.map(([key, group]) => ({ key, count: group.count, cases: group.cases.size, example: group.example })),
  };
}
console.log(`\n${cases.length} cases, ${refused} refused`);
if (json) writeFileSync(json, JSON.stringify(report, null, 2));
