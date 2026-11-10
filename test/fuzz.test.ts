// Generated programs (ADR 0092): each seed's program run natively and as JS
// in both builds, the default and std's iterator structs (ADR 0908), as the
// corpus's are, which must print and end the same. One that doesn't is
// written to target/fuzz/ at once, with its seed, the compiler and what
// each run printed, then reduced to as few statements as still fail the
// same way, each smaller one written as it's found, and that again by the
// nets' reducer (#1088), to become a corpus case once it's fixed. With
// `FUZZ_MIRI=1`, each is run under Miri first, and one with undefined
// behaviour is no program to compare (#1085).
//
//   bun test test/fuzz.test.ts                                  # the first 12 seeds, which must compile
//   FUZZ_START=1000 FUZZ_SEEDS=500 bun test test/fuzz.test.ts   # 500 more
//   FUZZ_MIRI=1 FUZZ_START=1000 FUZZ_SEEDS=50 bun test test/fuzz.test.ts   # each under Miri first

import { beforeAll, expect, test } from "bun:test";
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

import { generate, print, type Program } from "./generate";
import { reduce, size } from "./reduce";
import { reducedRepro } from "./reduced";
import { same } from "./oracle";
import { agree, compileJs, jsLimit, runJs, runNative, runtimes, show, type Run } from "./programs";
import { buildCompiler, compiler, contentDirectory, fixture, root, target, writeWhole } from "./support";
import { undefinedBehaviour } from "./miri";
import { partSetting } from "./part-settings";

/** A setting that's a whole number, at least `least`, or none, and then
 * `otherwise`: anything else fails the run before a seed does, not as a
 * run of none. */
function whole(name: string, otherwise: number, least: number, most = Number.MAX_SAFE_INTEGER, value = partSetting(name)): number {
  if (value === undefined) return otherwise;
  if (!/^\d+$/.test(value) || !Number.isSafeInteger(Number(value)) || Number(value) < least || Number(value) > most) {
    throw new Error(`${name}=${value}: a whole number, from ${least} to ${most}, is a setting`);
  }
  return Number(value);
}

// A seed is a 32-bit number to the generator (`random`), so each batch's
// seeds are in that range: past it, a seed would be another's program.
const seedLimit = 2 ** 32;
const start = whole("FUZZ_START", 1, 0, seedLimit - 1);
const seeds = whole("FUZZ_SEEDS", 12, 1, seedLimit);
if (start + seeds > seedLimit) {
  throw new Error(`FUZZ_START=${start} FUZZ_SEEDS=${seeds}: the seeds end past ${seedLimit}, where the generator's seeds do`);
}
// The seeds `bun test` runs are known to compile, so one rust-js rejects is
// a regression; exploring others, a program it doesn't support is skipped.
const exploring = partSetting("FUZZ_START") !== undefined || partSetting("FUZZ_SEEDS") !== undefined;

// A reduction stops after this long, with the smallest program so far.
// No part's setting, the seeds a part runs: read as it is (bbb2dc8c9's
// `partSetting` refused it, #1218).
const reduceBudget = whole("FUZZ_REDUCE_BUDGET", 300_000, 1, Number.MAX_SAFE_INTEGER, process.env.FUZZ_REDUCE_BUDGET);

beforeAll(buildCompiler);

type Verdict =
  | { kind: "same" }
  | { kind: "unsupported"; error: string }
  | { kind: "invalid"; error: string }
  | { kind: "undefined"; error: string }
  | { kind: "crash"; error: string; reason: string; build: Build }
  | { kind: "differs"; native: Run; js: [string, Run][]; build: Build };

/** The two builds, as the corpus's (ADR 0908): `--std-iterators`, std's own
 * iterator structs, beside the default. */
type Build = "plain" | "std";
const builds: [Build, string[]][] = [
  ["plain", []],
  ["std", ["--std-iterators"]],
];

/** Miri, where `FUZZ_MIRI=1` asks for it (test/miri.ts). */
const miri = partSetting("FUZZ_MIRI") === "1";

/** What happens to a program, natively and as JS in each build, the
 * first that goes otherwise than native Rust's. */
function judge(program: Program, seed: number): Verdict {
  const dir = fixture(`fuzz-${seed}`);
  // Where its text says, so the same program is one kept native binary
  // from run to run (`nativeBinary`); the JS is compiled each time.
  const text = print(program, seed);
  const file = join(contentDirectory(text), "program.rs");
  writeWhole(file, text);
  const native = runNative(file, dir);
  if (typeof native === "string") return { kind: "invalid", error: native };
  if (miri) {
    const ub = undefinedBehaviour(file);
    if (ub !== undefined) return { kind: "undefined", error: ub };
  }
  for (const [build, flags] of builds) {
    const compiled = compileJs(file, dir, "2024", false, {}, flags);
    if ("error" in compiled) {
      // A feature rust-js says it doesn't support is its answer; anything
      // else, an error it can't explain or its own panic, isn't, even after it.
      if (compiled.kind === "rejected") return { kind: "unsupported", error: compiled.error };
      return { kind: "crash", error: compiled.error, reason: compiled.reason, build };
    }
    const js = runtimes.map(([name, cmd]): [string, Run] => [name, runJs(cmd, compiled.js, dir, `${name}-${build}`, jsLimit(native))]);
    if (!js.every(([, run]) => agree(run, native))) return { kind: "differs", native, js, build };
  }
  return { kind: "same" };
}

/** How a program fails, which a smaller one must fail the same way: which
 * runtimes differ from native Rust in which of stdout, stderr and how it
 * ended, or what crashed. */
function signature(verdict: Verdict): string {
  if (verdict.kind === "crash") return `crash of ${verdict.build}: ${verdict.reason}`;
  if (verdict.kind !== "differs") return verdict.kind;
  const { native } = verdict;
  return JSON.stringify(
    verdict.js.map(([name, run]) => [
      verdict.build,
      name,
      run.bytes.stdout.equals(native.bytes.stdout),
      run.bytes.stderr.equals(native.bytes.stderr),
      typeof run.outcome !== "string" && typeof native.outcome !== "string" && same(run.outcome, native.outcome),
    ]),
  );
}

/** What each run printed and how it ended, or what the compiler said. */
function detail(verdict: Verdict): string {
  if (verdict.kind === "differs") {
    const { native, js } = verdict;
    return [`native: ${show(native.outcome)}\n${native.stdout}`, ...js.map(([name, run]) => `${name}: ${show(run.outcome)}\n${run.stdout}${run.stderr}`)].join("\n");
  }
  return "error" in verdict ? verdict.error : "";
}

// Which compiler and source a failure came from, to run it again.
let provenance: { compiler: string; sha256: string; source: string } | undefined;
const origin = () =>
  (provenance ??= {
    compiler,
    sha256: new Bun.CryptoHasher("sha256").update(readFileSync(compiler)).digest("hex"),
    source: Bun.spawnSync(["git", "rev-parse", "HEAD"], { cwd: root }).stdout.toString().trim(),
  });

for (let seed = start; seed < start + seeds; seed++) {
  test(`seed ${seed}`, async () => {
    const program = generate(seed);
    const verdict = judge(program, seed);
    if (process.env.FUZZ_REPORT) console.log(`seed ${seed}: ${verdict.kind}${"error" in verdict ? ` ${verdict.error.split("\n")[0]}` : ""}`);
    // A generated program is valid Rust, or the generator is wrong; one of
    // undefined behaviour, which Miri finds, has no behaviour to compare.
    if (verdict.kind === "invalid") throw new Error(`seed ${seed} made a program rustc rejects:\n${verdict.error}`);
    if (verdict.kind === "undefined") return;
    if (verdict.kind === "unsupported" && !exploring) {
      throw new Error(`seed ${seed} compiled before, and rust-js rejects it now:\n${verdict.error}`);
    }
    if (verdict.kind === "same" || verdict.kind === "unsupported") return;
    // What failed, kept before anything else is tried, so a reduction that
    // never ends, or a job stopped, can't lose it.
    const out = join(target, "fuzz");
    mkdirSync(out, { recursive: true });
    const failure = signature(verdict);
    writeFileSync(join(out, `seed-${seed}.original.rs`), print(program, seed));
    const evidence = { seed, ...origin(), kind: verdict.kind, signature: failure, detail: detail(verdict) };
    writeFileSync(join(out, `seed-${seed}.json`), JSON.stringify(evidence, null, 2));
    // As small as it still fails the same way, each smaller one kept.
    const repro = join(out, `seed-${seed}.rs`);
    const keep = (p: Program) => writeFileSync(repro, print(p, seed));
    keep(program);
    const { program: smallest, complete } = await reduce(program, async (candidate) => signature(judge(candidate, seed)) === failure, {
      until: Date.now() + reduceBudget,
      kept: keep,
    });
    const again = judge(smallest, seed);
    const reduced = complete ? "reduced" : `reduced for ${reduceBudget / 1000}s, not to the end,`;
    // And that again by the nets' reducer, which takes apart what's left
    // within a statement (#1088).
    const build = "build" in again ? again.build : "build" in verdict ? verdict.build : "plain";
    const further = again.kind === "differs" || again.kind === "crash" ? reducedRepro(repro, build) : "";
    expect(`${verdict.kind}, ${reduced} from ${size(program)} statements to ${size(smallest)} in ${repro}:\n\n${print(smallest, seed)}\n${detail(again)}${further}`).toBe("");
  }, 600_000);
}
