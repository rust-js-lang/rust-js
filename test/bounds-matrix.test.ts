// The bounds matrix (ADR 1137): a collection's length tested, something
// run between that may change it, then an index of it, each holder of the
// collection meeting each test, each thing between and each form of
// index, compiled by rust-js and run as native Rust runs it
// (test/bounds-matrix.ts writes the programs). A check the MIR intervals
// leave out where something between can change the length is a wrong
// answer here. A program that is undefined behaviour under Miri is no
// answer's to judge, and is left out, what Miri says beside it
// (`Holder.undefined`). Every program, 403, runs a night and in `check:fast`
// of a change to the MIR intervals, THIR's ranges or what a loop's range
// is: a part a core, `RUST_JS_CASES=i/n`, 16 s on the 14-core Mac, or
// about 270 s one program after another (DEVELOPMENT.md). One that runs
// and differs is filed P0 and listed in `wrong`, so the list only
// shrinks (#1877, #1878).
//
//   bun test test/bounds-matrix.test.ts                          # every program
//   RUST_JS_CASES=1/6 bun test test/bounds-matrix.test.ts        # a sixth of them

import { beforeAll, expect, test } from "bun:test";
import { writeFileSync } from "node:fs";
import { join } from "node:path";

import { programs } from "./bounds-matrix";
import { agree, compileJs, runJs, runNative, runtimes, show } from "./programs";
import { buildCompiler, fixture } from "./support";
import { shard } from "../scripts/shard";

beforeAll(() => {
  buildCompiler();
});

const all = programs();
const [part, parts] = (process.env.RUST_JS_CASES ?? "1/1").split("/").map(Number);

/** The programs that run and differ from native Rust, each a wrong answer
 * filed P0, by its issue. One that agrees is taken off with its fix. */
const wrong: Record<string, string> = {};

test("the wrong answers are the matrix's programs", () => {
  for (const name of Object.keys(wrong)) expect(Object.keys(all)).toContain(name);
});

for (const name of shard(Object.keys(all), part, parts)) {
  test(`a length of ${name.replace("__", ", tested ")}`, () => {
    const dir = fixture(`bounds-${name}`);
    const file = join(dir, "main.rs");
    writeFileSync(file, all[name]);
    const native = runNative(file, dir);
    if (typeof native === "string") throw new Error(native);
    const compiled = compileJs(file, dir, "2024");
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
    if (name in wrong) {
      expect(problem === "" ? `${name} runs as Rust does now: take it off \`wrong\`, ${wrong[name]}` : "").toBe("");
    } else {
      expect(problem).toBe("");
    }
  }, 300_000);
}
