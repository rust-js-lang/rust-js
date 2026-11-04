// The `&mut` matrix (ADR 1086): each pointee of a `&mut`, held each way,
// through each operation, compiled by rust-js and run as native Rust runs
// it (test/mut-ref-matrix.ts writes the programs). Each push runs its
// sample; the nightly every program (`MUT_REF_MATRIX=all`). A program
// rust-js refuses is listed in `refused`, with why, so the list only
// shrinks; one that runs and differs is a wrong answer, filed P0.
//
//   bun test test/mut-ref-matrix.test.ts                       # the sample
//   MUT_REF_MATRIX=all bun test test/mut-ref-matrix.test.ts    # every program

import { beforeAll, expect, test } from "bun:test";
import { writeFileSync } from "node:fs";
import { join } from "node:path";

import { programs, sample } from "./mut-ref-matrix";
import { agree, compileJs, runJs, runNative, runtimes, show } from "./programs";
import { buildCompiler, fixture } from "./support";

beforeAll(() => {
  buildCompiler();
});

const which = process.env.MUT_REF_MATRIX;
if (which !== undefined && which !== "all") throw new Error(`MUT_REF_MATRIX=${which}: \`all\`, or unset for the sample`);
const all = programs();
const chosen = which === "all" ? Object.keys(all) : sample;

/** The programs rust-js refuses, each with why. One it compiles is taken
 * off. */
const refused: Record<string, string> = {};

/** The programs that run and differ from native Rust, each a wrong answer
 * filed P0, by its issue. One that agrees is taken off with its fix. */
const wrong: Record<string, string> = {
  // What's moved out of a flattened state lent through its guard is the
  // class itself: `replace`, `take`, `swap` and `Cell::from_mut(..).replace`.
  flattened_state__field: "#636",
  // A `&mut` to a `Range` that's a `Vec`'s item is a new JS iterator of it.
  iterator__item: "#638",
};

test("the sample, the refusals and the wrong answers are the matrix's programs", () => {
  for (const name of [...sample, ...Object.keys(refused), ...Object.keys(wrong)]) expect(Object.keys(all)).toContain(name);
});

for (const name of chosen) {
  test(`a \`&mut\` of ${name.replace("__", ", held as ")}`, () => {
    const dir = fixture(`mut-ref-${name}`);
    const file = join(dir, "main.rs");
    writeFileSync(file, all[name]);
    const native = runNative(file, dir);
    if (typeof native === "string") throw new Error(native);
    const compiled = compileJs(file, dir);
    let problem = "";
    if (!("js" in compiled)) {
      problem = `refused: ${compiled.error.split("\n").slice(0, 4).join("\n")}`;
    } else {
      const [runtime, cmd] = runtimes[0];
      const ran = runJs(cmd, compiled.js, dir, runtime);
      if (!agree(ran, native)) {
        problem = `${show(ran.outcome)}, printing\n${ran.stdout}\n${ran.stderr.slice(0, 600)}\nwhere native Rust ended ${show(native.outcome)}, printing\n${native.stdout}\n${native.stderr.slice(0, 600)}`;
      }
    }
    if (name in refused) expect(problem, `${name} compiles now: take it off \`refused\``).toStartWith("refused");
    else if (name in wrong) expect(problem, `${name} runs as Rust does now: take it off \`wrong\`, ${wrong[name]}`).not.toBe("");
    else expect(problem).toBe("");
  }, 300_000);
}
