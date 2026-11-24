// A native program's run, kept by what it was built from (ADR 0567,
// amended): the build's key (`nativeKey`) and what rustc said the build
// read, each file and each variable (`inputs.json`, `keptNative`'s), with
// what the run printed and how it ended, so a program unchanged since it
// ran is neither built nor run again. Its build is the machine's
// (`nativeCache`), some 1.4 GB of the corpus's alone, which CI can't keep
// between runs; its runs are a few MB, which CI does keep: each job that
// runs native programs restores them, and only `main`'s save them.
//
// Kept only of a program whose run says the same each time: not one its
// case marks `//@ nondeterministic: <why>`, which reads the clock,
// randomness, the file system or its environment as it runs (the corpus's
// directives, test/corpus-cases.ts, the one reading of them; a case that
// names such an API is held to the mark by test/native-outcomes.test.ts);
// not one stopped by its time or its memory; and not a run asked for
// afresh, the metamorphic net's runs again of a hash table's seed. Where
// `RUST_JS_NATIVE_RECHECK` gives a share, the nightly's 0.02, a kept run
// is built and run again too, and one that never says what was kept fails
// its test: a key that misses an input.

import { existsSync, mkdirSync, readdirSync, readFileSync, renameSync, rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";

import { markUsed, mayKeep, nativeOutcomes, noteKept } from "./kept";
import { same, type Outcome } from "./oracle";
import type { Run } from "./programs";
import { unchanged } from "./support";

/** A run as kept: its text and its bytes, base64. */
type KeptRun = { stdout: string; stderr: string; bytes: { stdout: string; stderr: string }; outcome: Run["outcome"]; ms?: number };

/** The run kept of the program `key`, whose build read what a kept
 * `inputs.json` says, none of it changed since; or none. */
export function keptRun(key: string, store = nativeOutcomes): Run | undefined {
  const program = join(store, key.slice(0, 32));
  if (!existsSync(program)) return undefined;
  for (const build of readdirSync(program).filter((name) => !name.startsWith("."))) {
    const dir = join(program, build);
    if (!existsSync(join(dir, "run.json")) || !unchanged(join(dir, "inputs.json"))) continue;
    markUsed(dir);
    const kept: KeptRun = JSON.parse(readFileSync(join(dir, "run.json"), "utf8"));
    const bytes = { stdout: Buffer.from(kept.bytes.stdout, "base64"), stderr: Buffer.from(kept.bytes.stderr, "base64") };
    return { stdout: kept.stdout, stderr: kept.stderr, bytes, outcome: kept.outcome, ms: kept.ms };
  }
  return undefined;
}

/** `run`, of the program `key` whose kept build is `build`, kept beside
 * what the build read: not one stopped by its time or memory, and not
 * below the disk's floor, `free`'s answer. Made whole, then moved in. */
export function keepRun(key: string, build: string, run: Run, store = nativeOutcomes, free?: () => number) {
  if (run.stopped || !existsSync(join(build, "inputs.json"))) return;
  mkdirSync(store, { recursive: true });
  if (!mayKeep(store, free)) return;
  const dir = join(store, key.slice(0, 32), build.split("/").at(-1)!);
  if (existsSync(join(dir, "run.json"))) return;
  // Made where a reader skips it, a name beginning with a dot (`keptRun`):
  // `<build>.<pid>` was listed as a build, and its `run.json` read half
  // written, "Unexpected EOF", by another part of the corpus (#2299's push).
  const making = join(store, key.slice(0, 32), `.${build.split("/").at(-1)!}.${process.pid}`);
  rmSync(making, { recursive: true, force: true });
  mkdirSync(making, { recursive: true });
  writeFileSync(join(making, "inputs.json"), readFileSync(join(build, "inputs.json")));
  const kept: KeptRun = {
    stdout: run.stdout,
    stderr: run.stderr,
    bytes: { stdout: run.bytes.stdout.toString("base64"), stderr: run.bytes.stderr.toString("base64") },
    outcome: run.outcome,
    ms: run.ms,
  };
  writeFileSync(join(making, "run.json"), JSON.stringify(kept));
  try {
    renameSync(making, dir);
    noteKept(store, dir);
  } catch {
    rmSync(making, { recursive: true, force: true });
  }
}

/** Whether a kept run is built and run again, of the share
 * `RUST_JS_NATIVE_RECHECK` gives, the nightly's 0.02: at random, so each
 * night checks others. */
export function rechecked(share = Number(process.env.RUST_JS_NATIVE_RECHECK ?? 0)): boolean {
  return share > 0 && Math.random() < share;
}

/** Whether `fresh`, the runs of a kept one's program again, up to
 * `tries`, say what was `kept`: once, and it's the program's; never, each
 * the same other answer, and the key missed an input. A program whose
 * output std leaves to a hash table's seed may say another of its answers
 * once or twice; one whose every run says another, a `RandomState` map's
 * drops each printed in its order (#2461), says two of them apart, and
 * then the kept one is one of its answers too, no input missed. */
export function recheck(kept: Run, fresh: () => Run | string, tries = 8): string | undefined {
  const said: Run[] = [];
  for (let i = 0; i < tries; i++) {
    const ran = fresh();
    if (typeof ran === "string") return `a kept native run's program doesn't build again: ${ran.slice(0, 300)}: its key misses an input`;
    if (sameRun(ran, kept)) return undefined;
    if (said.some((other) => !sameRun(ran, other))) return undefined;
    said.push(ran);
  }
  return `a kept native run isn't what the program says run again, ${tries} times: kept ${JSON.stringify(kept.stdout.slice(0, 200))}, run ${JSON.stringify(said[0]?.stdout.slice(0, 200))}: its key misses an input`;
}

/** Whether two runs said the same: their bytes and how they ended. */
const sameRun = (a: Run, b: Run) => a.bytes.stdout.equals(b.bytes.stdout) && a.bytes.stderr.equals(b.bytes.stderr) && sameOutcome(a.outcome, b.outcome);

const sameOutcome = (a: Run["outcome"], b: Run["outcome"]) =>
  typeof a === "string" || typeof b === "string" ? a === b : same(a as Outcome, b as Outcome);
