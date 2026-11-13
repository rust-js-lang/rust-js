// Programs compiled with a flag of rust-js's that writes a file of what
// the compile saw, each on its own core, and that file read: the corpus's
// cases, and the generated programs of a range of seeds (ADR 0092). What
// `scripts/tidy-fired.ts` and `scripts/label-census.ts` sum; and the
// generated `match`es of test/match-trees.ts.
import { mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";

import { cases, corpus, directives } from "../test/corpus-cases";
import { generate, print } from "../test/generate";
import { program as matchTree } from "../test/match-trees";
import { compileJsAsync } from "../test/programs";
import { target } from "../test/support";
import { atOnce } from "./shard";

/** A program to compile: its name, its file, its edition. */
export type Program = { name: string; file: string; edition: string };

/** The corpus's cases rust-js compiles: not one it's to refuse or skip. */
export function corpusPrograms(): Program[] {
  return cases.flatMap((name) => {
    const said = directives(readFileSync(join(corpus, name), "utf8"));
    if (typeof said === "string" || said.kind === "compile-fail" || said.kind === "ignore-rust-js") return [];
    return [{ name, file: join(corpus, name), edition: said.edition }];
  });
}

/** The generated programs of `count` seeds from `start`, written in `dir`. */
export function generatedPrograms(start: number, count: number, dir: string): Program[] {
  return Array.from({ length: count }, (_, i) => {
    const seed = start + i;
    const file = join(dir, `seed${seed}.rs`);
    mkdirSync(dir, { recursive: true });
    writeFileSync(file, print(generate(seed), seed));
    return { name: `seed ${seed}`, file, edition: "2024" };
  });
}

/** The generated `match`es of `count` seeds from `start`
 * (`test/match-trees.ts`, #771), written in `dir`. */
export function matchTreePrograms(start: number, count: number, dir: string): Program[] {
  mkdirSync(dir, { recursive: true });
  return Array.from({ length: count }, (_, i) => {
    const seed = start + i;
    const file = join(dir, `matches${seed}.rs`);
    writeFileSync(file, matchTree(seed));
    return { name: `matches ${seed}`, file, edition: "2024" };
  });
}

/** Each of `programs` compiled with `flags(out)`, `out` the file it writes,
 * and that file's text given to `read`: how many compiled, how many were
 * refused. */
export async function eachCompiled(
  programs: Program[],
  flags: (out: string) => string[],
  read: (text: string, program: Program) => void,
): Promise<{ compiled: number; refused: number }> {
  const work = join(target, "each-compiled", String(process.pid));
  rmSync(work, { recursive: true, force: true });
  let [compiled, refused] = [0, 0];
  const waiting = programs.map((program, i) => ({ program, i }));
  const worker = async () => {
    for (let next = waiting.shift(); next !== undefined; next = waiting.shift()) {
      const dir = join(work, String(next.i));
      mkdirSync(dir, { recursive: true });
      const out = join(dir, "written.txt");
      const js = await compileJsAsync(next.program.file, dir, next.program.edition, false, {}, flags(out));
      if (!("js" in js)) {
        refused++;
        continue;
      }
      compiled++;
      read(readFileSync(out, "utf8"), next.program);
    }
  };
  await Promise.all(Array.from({ length: atOnce("test") }, worker));
  rmSync(work, { recursive: true, force: true });
  return { compiled, refused };
}
