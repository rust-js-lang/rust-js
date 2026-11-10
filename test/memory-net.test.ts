// The memory net (#1172): each seed's program of unsafe Rust over plain
// data (test/memory-net.ts writes them), run under Miri, which must find
// it defined, natively on `wasm32-wasip1`, whose `usize` and layout are
// rust-js's (ADR 0090), and as JS in both builds, the default and std's
// iterator structs (ADR 0908), which must print and end as native Rust
// does. One that doesn't is a wrong answer: it's written to
// target/memory-net/ with what each run printed, and reduced (#1088). What
// rust-js refuses is counted by its reason, each part's in
// target/memory-net/refused-<part>.json, so the model's next pieces are
// chosen by need; the seeds a push runs, `slice`, compile but those
// test/memory-net-known.txt lists, with why, so the list only shrinks.
// Each push runs the slice where the pointers' code changes; the nightly
// 400 seeds from the day's (`MEMORY_NET=all`).
//
//   bun test test/memory-net.test.ts                                                        # the slice
//   MEMORY_NET=all MEMORY_NET_START=5000 RUST_JS_CASES=1/4 bun test test/memory-net.test.ts  # 400 seeds from 5000, a quarter

import { afterAll, beforeAll, expect, test } from "bun:test";
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

import { memoryProgram } from "./memory-net";
import { miriInstalled, undefinedBehaviour, installMiri } from "./miri";
import { partSetting } from "./part-settings";
import { agree, compileJs, runJs, runNative, runtimes, show, type Run } from "./programs";
import { reducedRepro } from "./reduced";
import { buildCompiler, contentDirectory, fixture, root, target, writeWhole } from "./support";

beforeAll(buildCompiler);

const which = partSetting("MEMORY_NET");
if (which !== undefined && which !== "all") throw new Error(`MEMORY_NET=${which}: \`all\`, or unset for the slice`);
const startSetting = process.env.MEMORY_NET_START;
if (startSetting !== undefined && !/^\d+$/.test(startSetting)) throw new Error(`MEMORY_NET_START=${startSetting}: a whole number`);

/** The seeds each push runs where the pointers' code changes. */
const slice = Array.from({ length: 24 }, (_, i) => i + 1);
/** How many seeds the nightly runs, from the day's. */
const nightly = 400;
const start = Number(startSetting ?? 1);
const seeds = which === "all" ? Array.from({ length: nightly }, (_, i) => start + i) : slice;
const [part, parts] = (process.env.RUST_JS_CASES ?? "1/1").split("/").map(Number);
const chosen = seeds.filter((_, i) => i % parts === part - 1);

/** The slice's seeds rust-js refuses, each with why, a line each, `<seed>
 * <build>: <why>`: one it compiles is taken off. */
const known = new Map(
  readFileSync(join(root, "test", "memory-net-known.txt"), "utf8")
    .split("\n")
    .filter((line) => line.trim() !== "" && !line.startsWith("#"))
    .map((line) => {
      const at = line.indexOf(":");
      return [line.slice(0, at).trim(), line.slice(at + 1).trim()] as const;
    }),
);

/** The two builds, as the corpus's. */
const builds: [string, string[]][] = [
  ["plain", []],
  ["std", ["--std-iterators"]],
];

// Miri judges each program where it's installed; the nightly's job installs
// it, and a run that asks for it, `RUST_JS_REQUIRE_MIRI=1`, fails without.
const miri = miriInstalled();
if (!miri && partSetting("RUST_JS_REQUIRE_MIRI") === "1") throw new Error(`Miri isn't installed: ${installMiri}`);

/** Each refusal's reason, its spans and numbers taken out, and how many
 * programs it refused, of each build. */
const refusals = new Map<string, { count: number; seeds: number[] }>();

/** A refusal's reason, as one of its kind: its first line, rust-js's own
 * words, without where or how many. */
export function reasonOf(error: string): string {
  const line = error.split("\n").find((l) => /rust-js/.test(l)) ?? error.split("\n")[0];
  return line.replace(/^.*?rust-js( does not support|:)?\s*/, "").replace(/\b\d+\b/g, "N").trim();
}

test("the known refusals are of the slice's seeds and builds", () => {
  for (const key of known.keys()) {
    const [seed, build] = key.split(" ");
    expect(slice).toContain(Number(seed));
    expect(builds.map(([b]) => b)).toContain(build);
  }
});

for (const seed of chosen) {
  test(`seed ${seed}`, () => {
    const { text, swarm } = memoryProgram(seed);
    const dir = fixture(`memory-${seed}`);
    // Where its text says, so the same program is one kept native binary.
    const file = join(contentDirectory(text), "program.rs");
    writeWhole(file, text);
    if (miri) {
      const ub = undefinedBehaviour(file);
      if (ub !== undefined) throw new Error(`seed ${seed}'s program (${swarm.join(", ")}) is undefined behaviour, which the generator must never make:\n${text}\n${ub}`);
    }
    const native = runNative(file, dir, "2024", "wasm32");
    if (typeof native === "string") throw new Error(`seed ${seed} made a program rustc rejects:\n${text}\n${native}`);
    const problems: string[] = [];
    for (const [build, flags] of builds) {
      const key = `${seed} ${build}`;
      // What went otherwise than native Rust, a refusal or a wrong answer:
      // of the slice, one test/memory-net-known.txt lists is as it says.
      const went = (problem: string) => {
        if (which !== "all" && known.has(key)) return;
        problems.push(problem);
      };
      const compiled = compileJs(file, dir, "2024", false, {}, flags);
      if ("error" in compiled) {
        if (compiled.kind === "crashed") {
          went(`${key}: rust-js crashed: ${compiled.reason}\n${compiled.error.slice(0, 2000)}`);
          continue;
        }
        refused(build, reasonOf(compiled.error), seed);
        if (which !== "all") went(`${key}: refused, and it isn't in test/memory-net-known.txt:\n${compiled.error.split("\n").slice(0, 6).join("\n")}`);
        continue;
      }
      const [runtime, cmd] = runtimes[0];
      const ran = runJs(cmd, compiled.js, dir, `${runtime}-${build}`);
      // A refusal where it runs, `$unsupported`'s error (#1193), the same
      // as native Rust up to it: counted as a refusal, at run time.
      const error = typeof ran.outcome === "object" && "error" in ran.outcome ? String(ran.outcome.error) : "";
      if (error.includes("rust-js does not support") && native.bytes.stdout.subarray(0, ran.bytes.stdout.length).equals(ran.bytes.stdout)) {
        refused(build, `at run time: ${reasonOf(error)}`, seed);
        if (which !== "all") went(`${key}: refused where it runs, and it isn't in test/memory-net-known.txt:\n${error}`);
        continue;
      }
      if (agree(ran, native)) {
        if (known.has(key)) problems.push(`${key} runs as Rust does now: take it off test/memory-net-known.txt (${known.get(key)})`);
        continue;
      }
      // Reduced only where it's no known one.
      if (which === "all" || !known.has(key)) problems.push(wrong(seed, build, file, text, native, ran));
    }
    expect(problems.join("\n\n")).toBe("");
  }, 600_000);
}

/** One more program `build` refused for `reason`. */
function refused(build: string, reason: string, seed: number) {
  const counted = refusals.get(`${build}: ${reason}`) ?? { count: 0, seeds: [] };
  counted.count++;
  counted.seeds.push(seed);
  refusals.set(`${build}: ${reason}`, counted);
}

/** A wrong answer, kept in target/memory-net/ before it's reduced, so a
 * job stopped can't lose it: what each run printed, and the program
 * reduced. */
function wrong(seed: number, build: string, file: string, text: string, native: Run, ran: Run): string {
  const out = join(target, "memory-net");
  mkdirSync(out, { recursive: true });
  writeFileSync(join(out, `seed-${seed}.rs`), text);
  const detail = `native: ${show(native.outcome)}\n${native.stdout}\nJS (${build}): ${show(ran.outcome)}\n${ran.stdout}${ran.stderr.slice(0, 2000)}`;
  writeFileSync(join(out, `seed-${seed}.${build}.txt`), detail);
  return `seed ${seed}, ${build}: a wrong answer\n${detail}${reducedRepro(file, build)}`;
}

afterAll(() => {
  if (refusals.size === 0) return;
  const out = join(target, "memory-net");
  mkdirSync(out, { recursive: true });
  const ranked = [...refusals].sort((a, b) => b[1].count - a[1].count).map(([reason, { count, seeds }]) => ({ reason, count, seeds }));
  writeFileSync(join(out, `refused-${part}-of-${parts}.json`), JSON.stringify(ranked, null, 2));
  console.log(`refusals, by how many programs each refused:\n${ranked.map((r) => `${String(r.count).padStart(4)}  ${r.reason}`).join("\n")}`);
});
