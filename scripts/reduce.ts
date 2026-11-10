// A failing program reduced to a small one that fails the same way (#1088),
// C-Reduce style: each reduction the rewriter makes (`metamorph reduce`,
// scripts/metamorph/src/reduce.rs) tried in parallel, and the first, in its
// order, kept where rustc still accepts the program, native Rust still runs
// it, Miri finds no undefined behaviour in it where it has `unsafe` code
// (#1104), and rust-js still fails as it did; again until none is kept.
//
//   bun scripts/reduce.ts <case.rs> [--build plain|std] [--edition <year>]
//                         [--oracle same|differs|error:<text>] [--budget <s>]
//                         [--jobs <n>] [--memory <GB>] [--out <file>]
//
// The oracle, `same` unless it's given: the same first line native Rust
// and the JS print apart, or the same way it ends where they print alike,
// or rust-js's same first error; `differs`, any way at all; `error:<text>`,
// a compile error saying it. The reduced program, formatted, is written to
// `--out`, `target/reduce/<case>.rs` unless it's given, and printed. Its
// variants hold at most `--memory` GB together, the run's memory budget
// unless it's given, each its share, and run at most `variantTimeout`:
// one past either is stopped, it and what it started, and kept as no
// failure (#1306).
//
//   bun scripts/reduce.ts --check <file> --build .. --edition ..
//
// is one variant's failure, as JSON, which the parallel tries run.
import { availableParallelism } from "node:os";
import { mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { basename, join } from "node:path";
import { spawnSync } from "node:child_process";

import { run } from "../test/child";
import { agree, compileJs, jsLimit, runJs, runNative, runtimes, show } from "../test/programs";
import { buildRewriter, rewriter } from "../test/metamorphic";
import { root, target } from "../test/support";
import { installMiri, mayBeUndefined, miriInstalled, miriSysroot, undefinedBehaviour } from "../test/miri";
import { memoryBudget, peakGB } from "./shard";

/** How a program fails: rust-js refuses or crashes on it, with its first
 * error; or its JS ends otherwise than native Rust, a throw where it
 * returns; or prints otherwise from a line on, to stdout or to stderr. */
/** What isn't a failure to keep: see `failure`. */
export type NotAFailure = "rejected" | "undefined" | "passes";

export type Failure =
  | { kind: "rejected" | "crashed"; error: string }
  | { kind: "prints"; native: string; js: string }
  | { kind: "ends"; native: string; js: string }
  | { kind: "stderr"; native: string; js: string };

/** A program's failure, or that it isn't one: rustc rejects it, or native
 * Rust's run doesn't end (`rejected`); Miri finds it's undefined behaviour,
 * whose native run prints what it happens to (`undefined`); or rust-js runs
 * it as native Rust does (`passes`). */
export function failure(file: string, build: "plain" | "std", edition: string, dir: string): Failure | NotAFailure {
  mkdirSync(dir, { recursive: true });
  const native = runNative(file, dir, edition);
  if (typeof native === "string" || typeof native.outcome === "string") return "rejected";
  if (mayBeUndefined(readFileSync(file, "utf8")) && undefinedBehaviour(file, edition) !== undefined) return "undefined";
  const compiled = compileJs(file, dir, edition, false, {}, build === "std" ? ["--std-iterators"] : []);
  if (!("js" in compiled)) {
    const error = compiled.error.split("\n").find((line) => line.startsWith("error")) ?? compiled.reason;
    return { kind: compiled.kind, error: error.trim() };
  }
  const [runtime, cmd] = runtimes[0];
  const js = runJs(cmd, compiled.js, dir, runtime, jsLimit(native));
  if (agree(native, js)) return "passes";
  // Ended otherwise, a throw where native Rust returns: that, whatever it
  // printed before.
  if (show(native.outcome) !== show(js.outcome)) return { kind: "ends", native: show(native.outcome), js: show(js.outcome) };
  const [a, b] = [native.stdout.split("\n"), js.stdout.split("\n")];
  const at = a.findIndex((line, i) => line !== b[i]);
  if (at >= 0 || a.length !== b.length) {
    const i = at >= 0 ? at : Math.min(a.length, b.length);
    return { kind: "prints", native: a[i] ?? "<its end>", js: b[i] ?? "<its end>" };
  }
  const [c, d] = [native.stderr.split("\n"), js.stderr.split("\n")];
  const i = Math.max(0, c.findIndex((line, i) => line !== d[i]));
  return { kind: "stderr", native: c[i] ?? "<its end>", js: d[i] ?? "<its end>" };
}

/** Whether `found`, a variant's, is the failure the oracle keeps. */
function kept(oracle: string, first: Failure, found: Failure | NotAFailure): boolean {
  if (typeof found === "string") return false;
  if (oracle === "differs") return true;
  if (oracle.startsWith("error:")) return "error" in found && found.error.includes(oracle.slice("error:".length));
  return JSON.stringify(found) === JSON.stringify(first);
}

/** What a program's size is taken to be: its text without its spacing,
 * which the rewriter prints otherwise than its source has it, a digit but
 * `0` a little more, so a number made `0` is smaller. */
export const size = (text: string) => {
  const bare = text.replace(/\s+/g, "");
  return bare.length * 16 + (bare.match(/[1-9]/g)?.length ?? 0);
};

/** How many jobs at once: one a core, each a test's, a compile at a time,
 * no more than the run's memory budget holds (`memoryBudget`, #1084). */
export function reduceJobs(): number {
  return Math.max(1, Math.min(availableParallelism(), Math.floor(memoryBudget() / peakGB.test)));
}

/** The rewriter's reductions of `file`: how many, or the program with the
 * n-th made, `undefined` past the last. */
function reductions(file: string): number {
  const p = spawnSync(rewriter, ["reduce", file], { encoding: "utf8" });
  if (p.status !== 0) throw new Error(`metamorph reduce ${file}: ${p.stderr}`);
  return Number(p.stdout.trim());
}
function reduction(file: string, n: number): string | undefined {
  const p = spawnSync(rewriter, ["reduce", file, String(n)], { encoding: "utf8", maxBuffer: 64 * 1024 * 1024 });
  return p.status === 0 ? p.stdout : undefined;
}

/** How long a variant's check may take: its compiles in both builds, its
 * native run, Miri's and its JS's, each with a limit of its own, on a
 * loaded machine. */
export const variantTimeout = 300_000;

/** The least memory a variant's check is given, in bytes: a crate of a
 * neighbour shuffle's group compiled whole. */
const leastVariantMemory = 2 ** 30;

/** `variant`'s failure, by this script's `--check`, run apart so many run
 * at once; within `variantTimeout` and `memory` bytes, it and every
 * process it starts together, or no failure to keep: a variant whose loop
 * no longer ends may allocate without end, which on macOS no `ulimit`
 * stops, and four parts of a net reducing at once took 5 to 10 GB each of
 * a 16 GB Mac (#1306). */
export async function checked(variant: string, build: string, edition: string, memory: number): Promise<Failure | NotAFailure> {
  const cmd = ["bun", join(root, "scripts", "reduce.ts"), "--check", variant, "--build", build, "--edition", edition];
  const p = await run(cmd, root, variantTimeout, {}, memory);
  if (p.timedOut || p.overgrown || p.signal !== null) return "rejected";
  try {
    return JSON.parse(p.stdout.trim().split("\n").pop() ?? "");
  } catch {
    return "rejected";
  }
}

export type Reduced = { program: string; failure: Failure; from: number; to: number; checks: number; seconds: number };

/** `file` reduced as its oracle keeps, within `budget` seconds: the
 * smallest program found, or why there's none, it doesn't fail. */
export async function reduce(
  file: string,
  { build = "plain", edition = "2024", oracle = "same", budget = 600, jobs = reduceJobs(), memory = memoryBudget() }: {
    build?: "plain" | "std";
    edition?: string;
    oracle?: string;
    budget?: number;
    jobs?: number;
    /** The GB its variants may hold together. */
    memory?: number;
  } = {},
): Promise<Reduced | string> {
  const started = Date.now();
  // As many at once as each has its least memory, and each its share.
  jobs = Math.max(1, Math.min(jobs, Math.floor((memory * 2 ** 30) / leastVariantMemory)));
  const each = Math.max(leastVariantMemory, (memory * 2 ** 30) / jobs);
  buildRewriter();
  const work = join(target, "reduce", `${basename(file, ".rs")}-${process.pid}`);
  rmSync(work, { recursive: true, force: true });
  mkdirSync(work, { recursive: true });
  const current = join(work, "current.rs");
  writeFileSync(current, readFileSync(file, "utf8"));
  // A program with `unsafe` code is reduced only where Miri can say which
  // steps make it undefined behaviour: without, one that reads memory it
  // never wrote would be kept, as #1099's was. Its sysroot asked once, and
  // given to each check, `MIRI_SYSROOT`.
  if (mayBeUndefined(readFileSync(current, "utf8"))) {
    if (!miriInstalled()) return `${file} has \`unsafe\` code, and reducing it needs Miri, which keeps no step that makes it undefined behaviour: ${installMiri}`;
    miriSysroot();
  }
  const first = failure(current, build, edition, join(work, "first"));
  if (typeof first === "string") {
    const why = { passes: "runs as native Rust does", undefined: "is undefined behaviour, Miri finds", rejected: "isn't a program native Rust runs" }[first];
    return `${file} ${why}`;
  }
  let text = readFileSync(current, "utf8");
  let checks = 0;
  let changed = true;
  while (changed && (Date.now() - started) / 1000 < budget) {
    changed = false;
    let n = 0;
    let count = reductions(current);
    while (n < count && (Date.now() - started) / 1000 < budget) {
      // A batch of the next reductions, each that's smaller; the first
      // kept, in their order, is the program now.
      const batch: { n: number; path: string; text: string }[] = [];
      let k = n;
      for (; k < count && batch.length < jobs; k++) {
        const made = reduction(current, k);
        if (made === undefined || size(made) >= size(text)) continue;
        const path = join(work, `v${checks + batch.length}`, "variant.rs");
        mkdirSync(join(path, ".."), { recursive: true });
        writeFileSync(path, made);
        batch.push({ n: k, path, text: made });
      }
      if (batch.length === 0) {
        n = k;
        continue;
      }
      const found = await Promise.all(batch.map((v) => checked(v.path, build, edition, each)));
      checks += batch.length;
      const at = found.findIndex((f) => kept(oracle, first, f));
      for (const v of batch) rmSync(join(v.path, ".."), { recursive: true, force: true });
      if (at < 0) {
        n = batch[batch.length - 1].n + 1;
        continue;
      }
      text = batch[at].text;
      writeFileSync(current, text);
      changed = true;
      count = reductions(current);
      n = batch[at].n;
    }
  }
  const formatted = spawnSync("rustfmt", [`--edition=${edition}`, "--emit=stdout", "--quiet", current], { encoding: "utf8" });
  const program = formatted.status === 0 && formatted.stdout ? formatted.stdout : text;
  const length = (text: string) => text.replace(/\s+/g, "").length;
  return { program, failure: first, from: length(readFileSync(file, "utf8")), to: length(text), checks, seconds: Math.round((Date.now() - started) / 1000) };
}

if (import.meta.main) {
  const args = process.argv.slice(2);
  const flag = (name: string) => {
    const at = args.indexOf(name);
    return at >= 0 ? args[at + 1] : undefined;
  };
  const build = (flag("--build") ?? "plain") as "plain" | "std";
  const edition = flag("--edition") ?? "2024";
  const check = flag("--check");
  if (check !== undefined) {
    console.log(JSON.stringify(failure(check, build, edition, join(check, "..", "run"))));
  } else {
    const file = args.find((arg, i) => !arg.startsWith("--") && (i === 0 || !args[i - 1].startsWith("--")));
    if (file === undefined || !["plain", "std"].includes(build)) {
      console.error("bun scripts/reduce.ts <case.rs> [--build plain|std] [--edition <year>] [--oracle same|differs|error:<text>] [--budget <s>] [--jobs <n>] [--memory <GB>] [--out <file>]");
      process.exit(64);
    }
    const reduced = await reduce(file, {
      build,
      edition,
      oracle: flag("--oracle") ?? "same",
      budget: Number(flag("--budget") ?? 600),
      jobs: Number(flag("--jobs") ?? reduceJobs()),
      memory: Number(flag("--memory") ?? memoryBudget()),
    });
    if (typeof reduced === "string") {
      console.error(reduced);
      process.exit(1);
    }
    const out = flag("--out") ?? join(target, "reduce", basename(file));
    mkdirSync(join(out, ".."), { recursive: true });
    writeFileSync(out, reduced.program);
    console.log(reduced.program);
    console.error(
      `reduced ${file} (${build}): ${reduced.from} to ${reduced.to} characters in ${reduced.seconds} s, ${reduced.checks} variants checked; ${JSON.stringify(reduced.failure)}; written to ${out}`,
    );
  }
}
