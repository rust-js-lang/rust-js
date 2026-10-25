// Tidy's stages, each proved to change nothing a program does (ADR 0659): a
// corpus case's JS as tidy leaves it after each stage, `--tidy-until` its
// last pass, run as the corpus runs it, prints and ends as native Rust
// does. Each stage is a readability rewrite of a body already correct, so
// what it leaves must be correct too: a stage after which a program does
// otherwise, or runs on where Rust ends, is a pass that made it so, which
// `bun scripts/tidy-trace.ts <case>` finds.
//
//   case.rs ─┬─ rustc ──► native ─────────────────────────────┐
//            └─ rust-js --tidy-until <each stage> ──► node ───┴─► the same?
//
//   bun test test/tidy-stages.test.ts                                # a few cases, of every stage
//   TIDY_STAGES=corpus RUST_JS_CASES=1/6 bun test test/tidy-stages.test.ts   # the corpus, a sixth of it

import { beforeAll, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { basename, join } from "node:path";

import { cases, corpus, directives } from "./corpus-cases";
import { agree, compileJs, runJs, runNative, runtimes, show } from "./programs";
import { buildCompiler, caseShard, compiler, fixture, run } from "./support";
import { record, runKey, shared, verified } from "./verified-runs";

beforeAll(() => {
  buildCompiler();
});

// Of the suite's own run, these: tidy's work is in their control flow,
// assignments, matches, early exits and drops.
const few = [
  "assignment_order.rs",
  "calculator.rs",
  "catch_unwind_drops.rs",
  "conditional_regions.rs",
  "interpreter.rs",
  "labeled_blocks.rs",
];
const which = process.env.TIDY_STAGES;
if (which !== undefined && which !== "corpus") throw new Error(`TIDY_STAGES=${which}: \`corpus\`, or unset for a few cases`);
const chosen = which === "corpus" ? caseShard(cases) : few;

/** The last pass of each of tidy's stages, in the order they run: what
 * `--tidy-until` stops after once a stage is done. A pass is
 * `stage/pass`, a stage run again `again/stage/pass`. */
function stageEnds(): string[] {
  const ends = new Map<string, string>();
  for (const pass of run([compiler, "--tidy-passes"]).trim().split("\n")) {
    ends.set(pass.slice(0, pass.lastIndexOf("/")), pass);
  }
  return [...ends.values()];
}

test("the cases a run of a few names are the corpus's", () => {
  for (const name of few) expect(cases).toContain(name);
});

test("a stage's end is the last of its passes", () => {
  buildCompiler();
  const ends = stageEnds();
  expect(ends.length).toBeGreaterThan(10);
  expect(ends).toContain("finish/appends");
  expect(ends.some((end) => end.startsWith("again/"))).toBe(true);
});

// What's checked is each stage's JS, not the finished JS each time: tidy
// stopped after its first stage leaves a body as no later stage does.
test("a case's JS after tidy's first stage isn't its JS after the last", () => {
  buildCompiler();
  const [first, ...rest] = stageEnds();
  const dir = fixture("stages-control");
  const at = (end: string) => {
    const compiled = compileJs(join(corpus, "conditional_regions.rs"), dir, "2024", false, {}, ["--tidy-until", end]);
    if (!("js" in compiled)) throw new Error(compiled.error);
    // Its map is named for the stage, as its file is: the code's what's compared.
    return readFileSync(compiled.js, "utf8").replace(/^\/\/# sourceMappingURL=.*$/m, "");
  };
  expect(at(first)).not.toBe(at(rest.at(-1)!));
});

// A name that's no case is the test above's to report.
for (const name of chosen.filter((name) => cases.includes(name))) {
  const file = join(corpus, name);
  const want = directives(readFileSync(file, "utf8"));
  // What doesn't run as Rust does yet, or doesn't run, has no stage to
  // check; a malformed directive is the corpus's to report.
  if (typeof want === "string" || want.kind === "compile-fail" || want.kind === "ignore-rust-js") continue;
  test(`${basename(name, ".rs")} after each of tidy's stages`, () => {
    const dir = fixture(`stages-${basename(name, ".rs")}`);
    const native = runNative(file, dir, want.edition, want.native);
    if (typeof native === "string") throw new Error(native);
    const problems: string[] = [];
    for (const end of stageEnds()) {
      const compiled = compileJs(file, dir, want.edition, false, {}, ["--tidy-until", end]);
      if (!("js" in compiled)) {
        problems.push(`after ${end}: rust-js failed: ${compiled.error.split("\n").slice(0, 6).join("\n")}`);
        continue;
      }
      const js = readFileSync(compiled.js, "utf8");
      const key = runKey(["tidy stage", end, js, native.bytes.stdout, native.bytes.stderr, show(native.outcome), shared()]);
      if (verified(key)) continue;
      const [runtime, cmd] = runtimes[0];
      const ran = runJs(cmd, compiled.js, dir, `${runtime}-${end.replaceAll(/[^A-Za-z0-9]+/g, "_")}`);
      if (agree(ran, native)) record(key, `${name} after ${end}`);
      else problems.push(`after ${end}: ${show(ran.outcome)}, printing\n${ran.stdout.slice(0, 400)}\nwhere native Rust ended ${show(native.outcome)}, printing\n${native.stdout.slice(0, 400)}`);
    }
    expect(problems.join("\n\n")).toBe("");
  }, 300_000);
}
