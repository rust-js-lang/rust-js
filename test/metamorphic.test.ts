// The metamorphic net (#939, `test/metamorphic.ts`): each corpus case, each
// rewrite `scripts/metamorph` makes of it, run natively and as JS in both
// builds. A rewrite rustc rejects of every place it finds is tried of its
// first alone; one rustc rejects even so is no variant. What runs otherwise
// than native, or crashes rust-js, is listed under its issue in
// test/metamorphic-known.txt; each listed one runs each night, so the list
// only shrinks.
//
//   bun test test/metamorphic.test.ts                                      # a push's sample
//   METAMORPHIC=all RUST_JS_CASES=1/6 bun test test/metamorphic.test.ts    # every case, a sixth
//   target/metamorph/debug/metamorph <rewrite> test/corpus/<case>.rs       # a rewrite, by hand

import { beforeAll, expect, test } from "bun:test";
import { mkdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";

import { corpus } from "./corpus-cases";
import { buildRewriter, readKnown, rewritable, rewrites, rewritten, sampled, type Rewrite } from "./metamorphic";
import { agree, compileJs, runJs, runNative, runtimes, show, type Run } from "./programs";
import { buildCompiler, fixture } from "./support";
import { shard } from "../scripts/shard";

beforeAll(() => {
  buildCompiler();
  buildRewriter();
}, 900_000);

const which = process.env.METAMORPHIC;
if (which !== undefined && which !== "all") throw new Error(`METAMORPHIC=${which}: \`all\`, or unset for a push's sample`);
const known = readKnown();
const all = rewritable();
// A push's run: the cases a hash of their name samples, one in 128; the
// nightly's, every case, each listed one's too.
const picked = which === "all" ? all : all.filter(({ name }) => sampled(name));
const [part, parts] = (process.env.RUST_JS_CASES ?? "1/1").split("/").map(Number);
const chosen = new Set(shard(picked.map(({ name }) => name), part, parts));

/** The two builds: the default, and std's iterator structs (ADR 0908). */
const builds: [string, string[]][] = [
  ["plain", []],
  ["std", ["--std-iterators"]],
];

/** What went wrong of `rewrite` of a case in each build, or nothing: its
 * JS run otherwise than native, or rust-js's crash. A refusal is safe, and
 * no wrong answer. */
function wrong(rewrite: Rewrite, name: string, edition: string): Map<string, string> | undefined {
  const file = join(corpus, name);
  const dir = fixture(`metamorphic-${rewrite}-${name.replace(/\.rs$/, "")}`);
  let native: Run | undefined;
  let source: string | undefined;
  // Of every place, then of its first alone.
  for (const limit of [undefined, 1]) {
    source = rewritten(rewrite, file, limit);
    if (source === undefined) return undefined;
    writeFileSync(join(dir, "variant.rs"), source);
    const ran = runNative(join(dir, "variant.rs"), dir, edition);
    if (typeof ran !== "string") {
      native = ran;
      break;
    }
  }
  if (native === undefined) return undefined;
  const problems = new Map<string, string>();
  for (const [build, flags] of builds) {
    const own = join(dir, build);
    mkdirSync(own, { recursive: true });
    const compiled = compileJs(join(dir, "variant.rs"), own, edition, false, {}, flags);
    if (!("js" in compiled)) {
      if (compiled.kind === "crashed") problems.set(build, `rust-js crashed: ${compiled.reason}`);
      continue;
    }
    const [runtime, cmd] = runtimes[0];
    const ran = runJs(cmd, compiled.js, own, runtime);
    if (agree(ran, native)) continue;
    const [a, b] = [native.stdout.split("\n"), ran.stdout.split("\n")];
    const at = a.findIndex((line, i) => line !== b[i]);
    problems.set(
      build,
      `ended ${show(ran.outcome)} where native ended ${show(native.outcome)}; native ${JSON.stringify(a.slice(at, at + 3))}, JS ${JSON.stringify(b.slice(at, at + 3))}${ran.stderr ? `; stderr ${JSON.stringify(ran.stderr.slice(0, 300))}` : ""}`,
    );
  }
  return problems;
}

for (const { name, edition } of all.filter(({ name }) => chosen.has(name))) {
  test(`${name}, rewritten`, () => {
    const problems: string[] = [];
    for (const rewrite of rewrites) {
      const found = wrong(rewrite, name, edition) ?? new Map<string, string>();
      for (const [build] of builds) {
        const listedAs = known.find((k) => k.name === name && k.rewrite === rewrite && k.builds.includes(build));
        const problem = found.get(build);
        if (listedAs !== undefined && problem === undefined) {
          problems.push(`${build} ${rewrite} ${name} runs as Rust does now: take it off test/metamorphic-known.txt (#${listedAs.issue})`);
        } else if (listedAs === undefined && problem !== undefined) {
          problems.push(`${build} ${rewrite} ${name}: ${problem}`);
        }
      }
    }
    // What isn't listed: an issue to file, and `<builds> <rewrite> <case>`
    // to list under it.
    expect(problems.join("\n\n")).toBe("");
  }, 900_000);
}

// The harness's own parts.
test("a push's sample is stable: a case added changes only whether it's sampled", () => {
  const names = Array.from({ length: 2000 }, (_, i) => `c${i}.rs`);
  const sample = names.filter((name) => sampled(name));
  expect(sample.length).toBeGreaterThan(5);
  expect(sample.length).toBeLessThan(40);
  expect([...names, "added.rs"].filter((name) => sampled(name) && name !== "added.rs")).toEqual(sample);
});

test("a listed rewrite is read under its issue, and a malformed line is refused", () => {
  expect(readKnown("#7 P0: a title\nplain std boxed a.rs\n")).toEqual([{ issue: "7", builds: ["plain", "std"], rewrite: "boxed", name: "a.rs" }]);
  expect(() => readKnown("plain boxed a.rs\n")).toThrow();
  expect(() => readKnown("#7 t\nplain inlined a.rs\n")).toThrow();
});
