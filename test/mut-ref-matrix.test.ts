// The `&mut` matrix (ADR 1086): each pointee of a `&mut`, held each way,
// through each operation, compiled by rust-js in both builds, the default
// and std's iterator structs, and run as native Rust runs it
// (test/mut-ref-matrix.ts writes the programs). Each push runs its sample;
// the nightly every program (`MUT_REF_MATRIX=all`). A program rust-js
// refuses is listed in `refused`, with why, so the list only shrinks; one
// that runs and differs is a wrong answer, filed P0, listed in `wrong`.
// Each is keyed `<build>:<program>`.
//
//   bun test test/mut-ref-matrix.test.ts                                        # the sample
//   MUT_REF_MATRIX=all RUST_JS_CASES=1/6 bun test test/mut-ref-matrix.test.ts   # every program, a sixth

import { beforeAll, expect, test } from "bun:test";
import { writeFileSync } from "node:fs";
import { join } from "node:path";

import { programs, sample } from "./mut-ref-matrix";
import { agree, compileJs, runJs, runNative, runtimes, show } from "./programs";
import { buildCompiler, fixture } from "./support";
import { shard } from "../scripts/shard";
import { partSetting } from "./part-settings";

beforeAll(() => {
  buildCompiler();
});

const which = partSetting("MUT_REF_MATRIX");
if (which !== undefined && which !== "all") throw new Error(`MUT_REF_MATRIX=${which}: \`all\`, or unset for the sample`);
const all = programs();
const [part, parts] = (process.env.RUST_JS_CASES ?? "1/1").split("/").map(Number);
const chosen = shard(which === "all" ? Object.keys(all) : sample, part, parts);

/** The two builds, keyed in `refused` and `wrong` by name. */
const builds: [string, string[]][] = [
  ["plain", []],
  ["std", ["--std-iterators"]],
];

/** The programs rust-js refuses, each with why. One it compiles is taken
 * off. */
const refused: Record<string, string> = {};

/** The programs that run and differ from native Rust, each a wrong answer
 * filed P0, by its issue. One that agrees is taken off with its fix. */
const wrong: Record<string, string> = {};

test("the sample, the refusals and the wrong answers are the matrix's programs", () => {
  const keyed = [...Object.keys(refused), ...Object.keys(wrong)];
  for (const key of keyed) expect(builds.map(([build]) => build)).toContain(key.split(":")[0]);
  for (const name of [...sample, ...keyed.map((key) => key.slice(key.indexOf(":") + 1))]) expect(Object.keys(all)).toContain(name);
});

for (const name of chosen) {
  test(`a \`&mut\` of ${name.replace("__", ", held as ")}`, () => {
    const dir = fixture(`mut-ref-${name}`);
    const file = join(dir, "main.rs");
    writeFileSync(file, all[name]);
    const native = runNative(file, dir);
    if (typeof native === "string") throw new Error(native);
    const problems: string[] = [];
    for (const [build, flags] of builds) {
      const key = `${build}:${name}`;
      const compiled = compileJs(file, dir, "2024", false, {}, flags);
      let problem = "";
      if (!("js" in compiled)) {
        problem = `refused: ${compiled.error.split("\n").slice(0, 4).join("\n")}`;
      } else {
        const [runtime, cmd] = runtimes[0];
        const ran = runJs(cmd, compiled.js, dir, `${runtime}-${build}`);
        if (!agree(ran, native)) {
          problem = `${show(ran.outcome)}, printing\n${ran.stdout}\n${ran.stderr.slice(0, 600)}\nwhere native Rust ended ${show(native.outcome)}, printing\n${native.stdout}\n${native.stderr.slice(0, 600)}`;
        }
      }
      if (key in refused) {
        if (!problem.startsWith("refused")) problems.push(`${key} compiles now: take it off \`refused\``);
      } else if (key in wrong) {
        if (problem === "") problems.push(`${key} runs as Rust does now: take it off \`wrong\`, ${wrong[key]}`);
      } else if (problem !== "") problems.push(`${key}: ${problem}`);
    }
    expect(problems.join("\n\n")).toBe("");
  }, 300_000);
}
