// Where a corpus run's time goes (#2296): the lines a run appends to the file
// `RUST_JS_PHASE_TIMES` names, the compiler's phases a compile and the
// harness's steps a case (test/corpus.test.ts), summed into tables.
//
//   RUST_JS_PHASE_TIMES=/tmp/times.jsonl bun test test/corpus.test.ts
//   bun scripts/phase-times.ts /tmp/times.jsonl     # Markdown tables
//
// The harness's `compile` is the whole process; the compiler's phases are
// what it measures inside it, from its `compile`'s start, so what's left is
// the process's start and end, a debug binary's load among them: "process
// start and exit".

import { readFileSync } from "node:fs";

type CompileLine = { input: string | null; library: boolean; phases: [string, number][]; forced?: string[] };
type CaseLine = { case: string; total: number; steps: [string, number][] };

/** Each step's times, by step, in the order they're first met. */
function byName(rows: [string, number][][]): Map<string, number[]> {
  const out = new Map<string, number[]>();
  for (const row of rows) {
    for (const [name, ms] of row) {
      const times = out.get(name) ?? [];
      times.push(ms);
      out.set(name, times);
    }
  }
  return out;
}

const sum = (xs: number[]) => xs.reduce((a, b) => a + b, 0);
function quantile(xs: number[], q: number): number {
  const sorted = [...xs].sort((a, b) => a - b);
  return sorted.length === 0 ? 0 : sorted[Math.min(sorted.length - 1, Math.floor(q * sorted.length))];
}
const seconds = (ms: number) => (ms / 1000).toFixed(1);
const millis = (ms: number) => ms.toFixed(0);
const share = (part: number, whole: number) => `${((100 * part) / whole).toFixed(1)}%`;

/** A Markdown table of `rows`' each step: its total, its share of `whole`,
 * how many, and its median and 90th percentile. */
function table(title: string, rows: Map<string, number[]>, whole: number): string {
  const lines = [`| ${title} | total s | share | n | median ms | p90 ms |`, "|---|---:|---:|---:|---:|---:|"];
  for (const [name, times] of rows) {
    lines.push(`| ${name} | ${seconds(sum(times))} | ${share(sum(times), whole)} | ${times.length} | ${millis(quantile(times, 0.5))} | ${millis(quantile(times, 0.9))} |`);
  }
  return lines.join("\n");
}

/** The tables of the lines in `text`. */
export function report(text: string): string {
  const lines = text.split("\n").filter((line) => line.trim() !== "").map((line) => JSON.parse(line));
  const cases: CaseLine[] = lines.filter((line) => "case" in line);
  const compiles: CompileLine[] = lines.filter((line) => "phases" in line);
  const whole = sum(cases.map((c) => c.total));
  const steps = byName(cases.map((c) => c.steps));
  const out: string[] = [];
  out.push(`${cases.length} cases, ${seconds(whole)} s summed over them (their own time each, not the run's wall time).`, "");
  out.push(table("step of a case", steps, whole), "");
  for (const library of [false, true]) {
    const these = compiles.filter((c) => c.library === library);
    const phases = byName(these.map((c) => c.phases));
    const inside = sum(these.flatMap((c) => c.phases.map(([, ms]) => ms)));
    const step = library ? "compile as a library" : "compile";
    // Of the harness's step, where it timed one: a worker's job, both
    // builds' phases in one line, is timed as `compile` (#2301).
    const harness = steps.get(step);
    if (harness !== undefined) phases.set("process start and exit", [sum(harness) - inside]);
    out.push(table(`${step}: its phases`, phases, sum(harness ?? [inside])), "");
  }
  // rustc's walks of every crate a compile's lowering forced,
  // `visible_parent_map`'s where it printed a type or a path (#2314).
  const forced = new Map<string, number>();
  for (const c of compiles) for (const query of c.forced ?? []) forced.set(query, (forced.get(query) ?? 0) + 1);
  const walks = [...forced].map(([query, n]) => `${query} by ${n} of ${compiles.length} compiles`);
  out.push(`rustc's walks of every crate lowering forced: ${walks.length > 0 ? walks.join(", ") : "none"}`, "");
  const slowest = [...cases].sort((a, b) => b.total - a.total).slice(0, 10);
  out.push("| slowest case | total ms | its steps, ms |", "|---|---:|---|");
  for (const c of slowest) {
    out.push(`| ${c.case} | ${millis(c.total)} | ${c.steps.map(([name, ms]) => `${name} ${millis(ms)}`).join(", ")} |`);
  }
  return out.join("\n");
}

if (import.meta.main) {
  const file = process.argv[2];
  if (file === undefined) {
    console.error("usage: bun scripts/phase-times.ts <times.jsonl>");
    process.exit(2);
  }
  console.log(report(readFileSync(file, "utf8")));
}
