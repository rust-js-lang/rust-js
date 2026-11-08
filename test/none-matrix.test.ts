// The `None` matrix (#855): an `Option` of each payload written to each
// kind of place, each with its `NoneForm`, by each writer, and read there by
// each reader, compiled by rust-js in both builds and run as native Rust
// runs it (test/none-matrix.ts writes the programs). Each push runs its
// sample where a `None`'s form changes; the nightly every program
// (`NONE_MATRIX=all`). What rust-js refuses is listed in `refused` and what
// it gets wrong in `wrong`, each keyed `<build>:<program>` with its issue,
// so the lists only shrink.
//
//   bun test test/none-matrix.test.ts                                     # the sample
//   NONE_MATRIX=all RUST_JS_CASES=1/6 bun test test/none-matrix.test.ts   # every program, a sixth

import { beforeAll, expect, test } from "bun:test";
import { writeFileSync } from "node:fs";
import { join } from "node:path";

import { programs, sample } from "./none-matrix";
import { agree, compileJs, runJs, runNative, runtimes, show } from "./programs";
import { buildCompiler, fixture } from "./support";
import { shard } from "../scripts/shard";

beforeAll(buildCompiler);

const which = process.env.NONE_MATRIX;
if (which !== undefined && which !== "all") throw new Error(`NONE_MATRIX=${which}: \`all\`, or unset for the sample`);
const all = programs();
const [part, parts] = (process.env.RUST_JS_CASES ?? "1/1").split("/").map(Number);
const chosen = shard(which === "all" ? Object.keys(all) : sample, part, parts);

/** The two builds, by the name `refused` and `wrong` key them by. */
const builds: [string, string[]][] = [
  ["plain", []],
  ["std", ["--std-iterators"]],
];

/** Each of `names` in both builds, with why. */
const keyed = (names: string[], why: string) =>
  Object.fromEntries(names.flatMap((name) => ["plain", "std"].map((build) => [`${build}:${name}`, why])));

/** The programs rust-js refuses or crashes on, each with why. One it
 * compiles is taken off. */
const refused: Record<string, string> = keyed(
  Object.keys(all).filter((name) => name.endsWith("__from_a_cell")),
  "#958: `Cell::new(x).take()`, a temporary cell's take, crashes rust-js",
);

/** The programs that run and differ from native Rust, each a wrong answer
 * filed P0, by its issue. One that agrees is taken off with its fix. */
const wrong: Record<string, string> = {};

test("the sample, the refusals and the wrong answers are the matrix's programs", () => {
  const keyed = [...Object.keys(refused), ...Object.keys(wrong)];
  for (const key of keyed) expect(builds.map(([build]) => build)).toContain(key.split(":")[0]);
  for (const name of [...sample, ...keyed.map((key) => key.slice(key.indexOf(":") + 1))]) expect(Object.keys(all)).toContain(name);
});

for (const name of chosen) {
  test(`an \`Option\` of ${name.replace("__", ", held as ")}`, () => {
    const dir = fixture(`none-${name}`);
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
          const [a, b] = [native.stdout.split("\n"), ran.stdout.split("\n")];
          const at = a.findIndex((line, i) => line !== b[i]);
          const block = a.slice(0, at + 1).findLast((line) => line.startsWith("-- ")) ?? "";
          problem = `${show(ran.outcome)}, at ${block}: native ${JSON.stringify(a.slice(at, at + 3))}, JS ${JSON.stringify(b.slice(at, at + 3))}${ran.stderr ? `\n${ran.stderr.slice(0, 400)}` : ""}`;
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
