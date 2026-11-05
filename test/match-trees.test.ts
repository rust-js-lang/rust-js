// Generated `match`es MIR tests as trees (`test/match-trees.ts`): each
// seed's program run natively and as JS, which must print and end the
// same; and of its JS, the functions that print a label the compiler made,
// `bb12:`, the Rust writing none (#523's C1, #771), are those
// test/match-trees-labeled.txt lists, which only shrinks: a function
// labeled that isn't listed fails, and so does one listed that isn't, to
// be taken off with `BLESS=1`; a change to the generator makes the list
// again, `BLESS=1` then listing what's labeled. The nightly runs it (ADR
// 1030).
//
//   bun test test/match-trees.test.ts
//   BLESS=1 bun test test/match-trees.test.ts   # what's labeled now, once fewer are

import { afterAll, beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

import { labeled, program } from "./match-trees";
import { agree, compileJs, runJs, runNative, runtimes, show } from "./programs";
import { buildCompiler, contentDirectory, fixture, root, writeWhole } from "./support";

const SEEDS = 24;
const listFile = join(root, "test", "match-trees-labeled.txt");
const bless = process.env.BLESS !== undefined;

/** The seeds' programs, by which the list's functions are named: a change
 * to the generator makes the list again. */
const programs = new Bun.CryptoHasher("sha256");
for (let seed = 1; seed <= SEEDS; seed++) programs.update(program(seed));
const version = programs.digest("hex").slice(0, 12);

/** test/match-trees-labeled.txt: its programs' version, and of each seed
 * the functions labeled. */
function read(): { version: string | undefined; seeds: Map<number, string[]> } {
  const seeds = new Map<number, string[]>();
  let listed: string | undefined;
  for (const line of readFileSync(listFile, "utf8").split("\n")) {
    if (line.startsWith("//") || line.trim() === "") continue;
    const programs = /^programs ([0-9a-f]+)$/.exec(line);
    const seed = /^seed (\d+):((?: m\d+)*)$/.exec(line);
    if (programs) listed = programs[1];
    else if (seed) seeds.set(Number(seed[1]), seed[2].trim().split(" ").filter(Boolean));
    else throw new Error(`test/match-trees-labeled.txt: \`${line}\` is neither its programs' version nor a seed's functions`);
  }
  return { version: listed, seeds };
}

const list = read();
const now = new Map<number, string[]>();

beforeAll(buildCompiler);

afterAll(() => {
  if (!bless || now.size !== SEEDS) return;
  const header = readFileSync(listFile, "utf8")
    .split("\n")
    .filter((line) => line.startsWith("//"));
  const seeds = [...now].sort(([a], [b]) => a - b).map(([seed, fns]) => `seed ${seed}:${fns.map((f) => ` ${f}`).join("")}`);
  const count = [...now.values()].reduce((n, fns) => n + fns.length, 0);
  console.log(`match trees: ${count} labeled functions of ${SEEDS} seeds`);
  writeFileSync(listFile, [...header, `programs ${version}`, ...seeds, ""].join("\n"));
});

test("the list is of these programs", () => {
  if (!bless) expect(list.version, "the generator changed: make the list again, `BLESS=1`").toBe(version);
});

for (let seed = 1; seed <= SEEDS; seed++) {
  test(`seed ${seed}`, () => {
    const text = program(seed);
    const file = join(contentDirectory(text), "program.rs");
    writeWhole(file, text);
    const dir = fixture(`match-trees-${seed}`);
    const native = runNative(file, dir);
    if (typeof native === "string") throw new Error(`seed ${seed} made a program rustc rejects:\n${native}`);
    const compiled = compileJs(file, dir);
    if ("error" in compiled) throw new Error(`seed ${seed}: rust-js: ${compiled.kind}\n${compiled.error}`);
    for (const [name, cmd] of runtimes) {
      const run = runJs(cmd, compiled.js, dir, name);
      if (!agree(run, native)) {
        expect(`${name}: ${show(run.outcome)}\n${run.stdout}${run.stderr}`).toBe(`native: ${show(native.outcome)}\n${native.stdout}`);
      }
    }
    const fns = labeled(readFileSync(compiled.js, "utf8"));
    now.set(seed, fns);
    const listed = list.seeds.get(seed) ?? [];
    // A list made again, of programs the generator changed, is what's
    // labeled; otherwise it only shrinks, blessed or not.
    if (bless && list.version !== version) return;
    expect(
      fns.filter((f) => !listed.includes(f)),
      `seed ${seed}: a function prints a label the compiler made, \`bbN:\`, that test/match-trees-labeled.txt doesn't list`,
    ).toEqual([]);
    if (!bless) {
      expect(
        listed.filter((f) => !fns.includes(f)),
        `seed ${seed}: fewer functions are labeled: take them off test/match-trees-labeled.txt, \`BLESS=1\``,
      ).toEqual([]);
    }
  }, 300_000);
}
